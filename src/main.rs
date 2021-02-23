mod config;
mod filters;
mod webext;

use crate::config::Config;
use crate::webext::WebExt;
use chrono::{DateTime, Local};
use fixedbitset::FixedBitSet;
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
	PermitRequest { name: String, duration: Option<Duration>, responder: mpsc::Sender<PermitResponse> },
	PermitEnd { name: String, responder: mpsc::Sender<PermitResponse> },
	TabUpdate { tab: i64, url: Url },
	TabDelete { tab: i64 },
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
}

struct AllowManager<'a> {
	config: &'a Config,
	lookups: &'a Lookups<'a>,
	blocked: FixedBitSet,
}

impl<'a> AllowManager<'a> {
	fn new(config: &'a Config, lookups: &'a Lookups<'a>) -> Self {
		let blacklist = FixedBitSet::with_capacity(config.category.len());
		AllowManager { config, lookups, blocked: blacklist }
	}

	fn blocked(&self) -> &FixedBitSet {
		&self.blocked
	}

	fn reload(&mut self, now: &DateTime<Local>) {
		self.blocked.clear();
		for rule in &self.config.rule {
			if rule.is_active(now) {
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

pub type PermitResponse = Result<(), PermitError>;

fn main() {
	webext::proxy::check_and_run();

	let config = Config::load();

	let event_queue = mpsc::channel();
	let webext = WebExt::new(event_queue.0.clone());

	let lookups = build_lookups(&config);
	let mut tabs = HashMap::new();
	let mut alive_tabs = HashSet::new();
	let mut allow_manager = AllowManager::new(&config, &lookups);

	let initial_time = Local::now();
	allow_manager.reload(&initial_time);
	let mut rule_reload_time = allow_manager.next_reload_time(&initial_time);

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
				Event::PermitRequest { .. } => {}
				Event::PermitEnd { .. } => {}
				Event::TabUpdate { tab, url } => {
					let mask = compute_mask(&url, &lookups);
					let is_blocked = mask.intersection(allow_manager.blocked()).count() > 0;
					if tabs.insert(tab, mask).is_none() {
						alive_tabs.insert(tab);
					}
					if is_blocked {
						let last_removed_tab = alive_tabs.remove(&tab) && alive_tabs.is_empty();
						if last_removed_tab {
							webext.create_empty_tab();
						}
						webext.close_tab(tab);
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
			rule_reload_time = allow_manager.next_reload_time(&now);
		}
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
	Lookups { domain, subreddit, github, regex_category, regex_set, category_count, category_id }
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
