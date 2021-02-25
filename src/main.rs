mod cli;
mod config;
mod filters;
mod logger;
mod webext;

use crate::config::Config;
use crate::webext::WebExt;
use chrono::{DateTime, Local};
use fixedbitset::FixedBitSet;
use log::{debug, info};
use regex::RegexSet;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;
use url::Url;

pub enum PermitError {
	PermitDoesNotExist,
	PermitIsNotActive,
	DurationTooLong,
	DurationNotSpecified,
	CooldownNotFinished,
}

#[derive(Debug)]
pub enum Event {
	PermitRequest {
		name: String,
		duration: Option<Duration>,
		// responder: mpsc::Sender<PermitResponse>
	},
	PermitEnd {
		name: String,
		// responder: mpsc::Sender<PermitResponse>
	},
	TabUpdate {
		tab: i64,
		url: Url,
	},
	TabDelete {
		tab: i64,
	},
	TabDeleteAll,
}

struct Lookups<'a> {
	domain: HashMap<&'a str, Vec<usize>>,
	subreddit: HashMap<&'a str, Vec<usize>>,
	github: HashMap<&'a str, Vec<usize>>,
	regex_category: Vec<usize>,
	regex_set: RegexSet,
	category_count: usize,
	category_id: HashMap<&'a str, usize>,
	permit_id: HashMap<&'a str, usize>,
	permit_rev: Vec<&'a str>,
}

struct AllowManager<'a> {
	config: &'a Config,
	lookups: &'a Lookups<'a>,
	blocked: FixedBitSet,
	last_state: Vec<bool>,
}

struct PermitManager<'a> {
	lookups: &'a Lookups<'a>,
	unblocked: FixedBitSet,
	state: Vec<PermitState<'a>>,
}

struct PermitState<'a> {
	expires: Option<DateTime<Local>>,
	last_active: Option<DateTime<Local>>,
	details: &'a config::Permit,
}

impl<'a> AllowManager<'a> {
	fn new(config: &'a Config, lookups: &'a Lookups<'a>) -> Self {
		let blocked = FixedBitSet::with_capacity(config.category.len());
		let last_state = vec![false; config.rule.len()];
		AllowManager { config, lookups, blocked, last_state }
	}

	fn blocked(&self) -> &FixedBitSet {
		&self.blocked
	}

	fn reload(&mut self, now: &DateTime<Local>) {
		self.blocked.clear();
		for (index, rule) in self.config.rule.iter().enumerate() {
			let is_active = rule.is_active(now);
			if is_active != self.last_state[index] {
				self.last_state[index] = is_active;
				info!(
					"Rule {} {} according to schedule.",
					index + 1,
					if is_active { "activated" } else { "deactivated" },
				);
			}
			if is_active {
				for category in &rule.categories {
					self.blocked.insert(self.lookups.category_id[category.as_str()]);
				}
			}
		}
	}

	fn next_reload_time(&self, now: &DateTime<Local>) -> Option<DateTime<Local>> {
		self.config.rule.iter().filter_map(|rule| rule.next_change_time(now)).min()
	}
}

impl<'a> PermitManager<'a> {
	fn new(config: &'a Config, lookups: &'a Lookups<'a>) -> Self {
		let unblocked = FixedBitSet::with_capacity(config.category.len());
		let state = (0..config.permit.len())
			.map(|index| PermitState {
				expires: None,
				last_active: None,
				details: &config.permit[lookups.permit_rev[index]],
			})
			.collect();
		PermitManager { lookups, unblocked, state }
	}

	fn unblocked(&self) -> &FixedBitSet {
		&self.unblocked
	}

	fn reload(&mut self, now: &DateTime<Local>) {
		self.unblocked.clear();
		for (state_index, state) in self.state.iter_mut().enumerate() {
			if let Some(expires) = state.expires {
				if expires <= *now {
					state.expires = None;
					info!("Permit {:?} deactivated after using allotted time.", self.lookups.permit_rev[state_index]);
				}
			}
			if state.expires.is_some() {
				for category in &state.details.categories {
					self.unblocked.insert(self.lookups.category_id[category.as_str()]);
				}
			}
		}
	}

	fn next_reload_time(&self) -> Option<DateTime<Local>> {
		self.state.iter().filter_map(|state| state.expires).min()
	}
}

pub type PermitResponse = Result<(), PermitError>;

fn main() {
	logger::init().unwrap();
	webext::proxy::check_and_run();
	if std::env::args().nth(1).as_deref() == Some("daemon") {
		run_daemon()
	} else {
		cli::run();
	}
}

fn run_daemon() {
	let config = Config::load();

	let event_queue = mpsc::channel();
	let webext = WebExt::new(event_queue.0.clone());
	let event_queue_tx = event_queue.0.clone();
	std::thread::spawn(move || setup_dbus(event_queue_tx));

	let lookups = build_lookups(&config);
	let mut tabs = HashMap::new();
	let mut alive_tabs = HashSet::new();
	let mut allow_manager = AllowManager::new(&config, &lookups);
	let mut permit_manager = PermitManager::new(&config, &lookups);

	let initial_time = Local::now();
	allow_manager.reload(&initial_time);
	permit_manager.reload(&initial_time);
	let mut rule_reload_time =
		allow_manager.next_reload_time(&initial_time).into_iter().chain(permit_manager.next_reload_time()).min();

	loop {
		let now_before = Local::now();
		let timeout = match rule_reload_time {
			Some(rrt) if rrt > now_before => Some((rrt - now_before).to_std().unwrap()),
			Some(_) => Some(Duration::from_secs(0)),
			None => None,
		};
		let event = match timeout {
			Some(timeout) => match event_queue.1.recv_timeout(timeout) {
				Err(RecvTimeoutError::Timeout) => None,
				event => Some(event.unwrap()),
			},
			None => Some(event_queue.1.recv().unwrap()),
		};

		if let Some(event) = event {
			match event {
				Event::PermitRequest { name, duration } => {
					let now = Local::now();
					let id = lookups.permit_id[name.as_str()];
					let state = &mut permit_manager.state[id];
					let details = state.details;
					let duration = duration.or(details.length.default).unwrap();
					if let Some(max_duration) = details.length.maximum {
						assert!(duration <= max_duration);
					}
					if let (Some(last_active), Some(cooldown)) = (state.last_active, details.cooldown) {
						let cooldown = chrono::Duration::from_std(cooldown).unwrap();
						assert!(last_active + cooldown <= now);
					}
					let duration = chrono::Duration::from_std(duration).unwrap();
					state.last_active = Some(now);
					state.expires = Some(now + duration);
					info!("Permit {:?} activated on request.", name);
					allow_manager.reload(&now);
					permit_manager.reload(&now);
					recheck_tabs(&webext, &tabs, &mut alive_tabs, &mut allow_manager, &mut permit_manager);
					rule_reload_time =
						allow_manager.next_reload_time(&now).into_iter().chain(permit_manager.next_reload_time()).min();
				}
				Event::PermitEnd { name } => {
					let now = Local::now();
					let state = &mut permit_manager.state[lookups.permit_id[name.as_str()]];
					assert!(state.expires.is_some());
					state.expires = None;
					info!("Permit {:?} deactivated on request.", name);
					allow_manager.reload(&now);
					permit_manager.reload(&now);
					recheck_tabs(&webext, &tabs, &mut alive_tabs, &mut allow_manager, &mut permit_manager);
					rule_reload_time =
						allow_manager.next_reload_time(&now).into_iter().chain(permit_manager.next_reload_time()).min();
				}
				Event::TabUpdate { tab, url } => {
					let mask = compute_mask(&url, &lookups);
					let is_blocked = mask.intersection(allow_manager.blocked()).count() > 0
						&& mask.intersection(permit_manager.unblocked()).count() == 0;
					if tabs.insert(tab, (mask, url)).is_none() {
						alive_tabs.insert(tab);
					}
					if is_blocked {
						close_tab(tab, &tabs, &mut alive_tabs, &webext);
					}
				}
				Event::TabDelete { tab } => {
					tabs.remove(&tab);
					alive_tabs.remove(&tab);
				}
				Event::TabDeleteAll => {
					tabs.clear();
					alive_tabs.clear();
				}
			}
		} else {
			let now = Local::now();
			allow_manager.reload(&now);
			permit_manager.reload(&now);
			recheck_tabs(&webext, &tabs, &mut alive_tabs, &mut allow_manager, &mut permit_manager);
			rule_reload_time =
				allow_manager.next_reload_time(&now).into_iter().chain(permit_manager.next_reload_time()).min();
		}
	}
}

fn recheck_tabs(
	webext: &WebExt,
	tabs: &HashMap<i64, (FixedBitSet, Url)>,
	alive_tabs: &mut HashSet<i64>,
	allow_manager: &mut AllowManager,
	permit_manager: &mut PermitManager,
) {
	let tabs_to_close = alive_tabs
		.iter()
		.copied()
		.filter(|tab| {
			let mask = &tabs[tab].0;
			mask.intersection(allow_manager.blocked()).count() > 0
				&& mask.intersection(permit_manager.unblocked()).count() == 0
		})
		.collect::<Vec<_>>();
	for tab in tabs_to_close {
		close_tab(tab, tabs, alive_tabs, &webext);
	}
}

fn close_tab(tab: i64, tabs: &HashMap<i64, (FixedBitSet, Url)>, alive_tabs: &mut HashSet<i64>, webext: &WebExt) {
	debug!("Tab blocked with URL {:?}.", tabs[&tab].1);
	let last_removed_tab = alive_tabs.remove(&tab) && alive_tabs.is_empty();
	if last_removed_tab {
		webext.create_empty_tab();
	}
	webext.close_tab(tab);
}

fn setup_dbus(tx: mpsc::Sender<Event>) {
	let tx1 = tx;
	let tx2 = tx1.clone();
	let tx3 = tx1.clone();
	let conn = dbus::blocking::LocalConnection::new_session().unwrap();
	conn.request_name("dev.pustaczek.Vaxtify", false, false, false).unwrap();
	let f = dbus_tree::Factory::new_fn::<()>();
	let tree = f.tree(()).add(
		f.object_path("/", ()).introspectable().add(
			f.interface("dev.pustaczek.Vaxtify", ())
				.add_m(
					f.method("PermitStart", (), move |m| {
						let permit: &str = m.msg.get1().unwrap();
						tx1.send(Event::PermitRequest { name: permit.to_owned(), duration: None }).unwrap();
						Ok(vec![m.msg.method_return()])
					})
					.inarg::<&str, _>("permit"),
				)
				.add_m(
					f.method("PermitStartWithDuration", (), move |m| {
						let (permit, duration) = m.msg.get2();
						let permit: &str = permit.unwrap();
						let duration: u64 = duration.unwrap();
						tx2.send(Event::PermitRequest {
							name: permit.to_owned(),
							duration: Some(Duration::from_secs(duration)),
						})
						.unwrap();
						Ok(vec![m.msg.method_return()])
					})
					.inarg::<&str, _>("permit")
					.inarg::<u64, _>("duration"),
				)
				.add_m(
					f.method("PermitEnd", (), move |m| {
						let permit: &str = m.msg.get1().unwrap();
						tx3.send(Event::PermitEnd { name: permit.to_owned() }).unwrap();
						Ok(vec![m.msg.method_return()])
					})
					.inarg::<&str, _>("permit"),
				),
		),
	);
	tree.start_receive(&conn);
	loop {
		conn.process(Duration::from_millis(1000)).unwrap();
	}
}

fn build_lookups(config: &Config) -> Lookups<'_> {
	let category_count = config.category.len();
	let mut domain: HashMap<&str, Vec<usize>> = HashMap::new();
	let mut subreddit: HashMap<&str, Vec<usize>> = HashMap::new();
	let mut github: HashMap<&str, Vec<usize>> = HashMap::new();
	let mut regex_category = Vec::new();
	let mut regex_set_vec = Vec::new();
	let mut category_id = HashMap::new();
	for (index, (name, category)) in config.category.iter().enumerate() {
		for dom in &category.domains {
			domain.entry(dom.as_str()).or_default().push(index);
		}
		for sub in &category.subreddits {
			subreddit.entry(sub.as_str()).or_default().push(index);
		}
		for git in &category.githubs {
			github.entry(git.as_str()).or_default().push(index);
		}
		for reg in &category.regexes {
			regex_category.push(index);
			regex_set_vec.push(reg);
		}
		category_id.insert(name.as_ref(), index);
	}
	let regex_set = RegexSet::new(regex_set_vec).unwrap();
	let mut permit_id = HashMap::new();
	let mut permit_rev = Vec::new();
	for (index, name) in config.permit.keys().enumerate() {
		permit_id.insert(name.as_str(), index);
		permit_rev.push(name.as_str());
	}
	Lookups { domain, subreddit, github, regex_category, regex_set, category_count, category_id, permit_id, permit_rev }
}

fn compute_mask(url: &Url, lookups: &Lookups<'_>) -> FixedBitSet {
	let mut mask = FixedBitSet::with_capacity(lookups.category_count);
	if let Some(domain) = url.domain() {
		if let Some(categories) = lookups.domain.get(domain) {
			mask.extend(categories.iter().copied());
		}
	}
	if let Some(subreddit) = filters::extract_subreddit(url) {
		if let Some(categories) = lookups.subreddit.get(subreddit.as_str()) {
			mask.extend(categories.iter().copied());
		}
	}
	if let Some(github) = filters::extract_github(url) {
		if let Some(categories) = lookups.github.get(github.as_str()) {
			mask.extend(categories.iter().copied());
		}
	}
	mask.extend(
		lookups.regex_set.matches(url.as_str()).into_iter().map(|regex_index| lookups.regex_category[regex_index]),
	);
	mask
}
