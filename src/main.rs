mod cli;
mod config;
mod dbusapi;
mod filters;
mod logger;
mod lookups;
mod permits;
mod rules;
mod tabs;
mod webext;

use crate::config::Config;
use crate::webext::WebExt;
use chrono::{DateTime, Local};
use permits::PermitManager;
use rules::RuleManager;
use std::sync::mpsc;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;
use url::Url;

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
	dbusapi::spawn(event_queue.0.clone());

	let lookups = lookups::Lookups::new(&config);
	let mut tabs = tabs::Tabs::new(&lookups);
	let mut rules = RuleManager::new(&lookups);
	let mut permits = PermitManager::new(&lookups);

	let initial_now = Local::now();
	rules.reload(&initial_now);
	permits.reload(&initial_now);
	let mut when_reload = compute_when_reload(&rules, &permits, &initial_now);

	loop {
		let timeout = compute_timeout(when_reload, Local::now());
		let event = recv_maybe(&event_queue.1, timeout);
		let now = Local::now();

		if let Some(event) = event {
			match event {
				Event::PermitRequest { name, duration } => {
					permits.activate(&name, duration, &now);
					permits.reload(&now);
					tabs.rescan(rules.blocked(), permits.unblocked(), &webext);
					when_reload = compute_when_reload(&rules, &permits, &now);
				}
				Event::PermitEnd { name } => {
					permits.deactivate(&name);
					permits.reload(&now);
					tabs.rescan(rules.blocked(), permits.unblocked(), &webext);
					when_reload = compute_when_reload(&rules, &permits, &now);
				}
				Event::TabUpdate { tab, url } => tabs.insert(tab, url, rules.blocked(), permits.unblocked(), &webext),
				Event::TabDelete { tab } => tabs.remove(tab),
				Event::TabDeleteAll => tabs.clear(),
			}
		} else {
			rules.reload(&now);
			permits.reload(&now);
			tabs.rescan(rules.blocked(), permits.unblocked(), &webext);
			when_reload = compute_when_reload(&rules, &permits, &now);
		}
	}
}

fn compute_timeout(when: Option<DateTime<Local>>, now: DateTime<Local>) -> Option<Duration> {
	match when {
		Some(when) if when > now => Some((when - now).to_std().unwrap()),
		Some(_) => Some(Duration::from_secs(0)),
		None => None,
	}
}

fn recv_maybe<T>(rx: &mpsc::Receiver<T>, timeout: Option<Duration>) -> Option<T> {
	match timeout {
		Some(timeout) => match rx.recv_timeout(timeout) {
			Err(RecvTimeoutError::Timeout) => None,
			event => Some(event.unwrap()),
		},
		None => Some(rx.recv().unwrap()),
	}
}

fn compute_when_reload(rules: &RuleManager, permits: &PermitManager, now: &DateTime<Local>) -> Option<DateTime<Local>> {
	match (rules.when_reload(now), permits.when_reload()) {
		(Some(a), Some(b)) => Some(a.max(b)),
		(Some(a), None) | (None, Some(a)) => Some(a),
		(None, None) => None,
	}
}
