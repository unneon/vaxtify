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
use crate::permits::PermitResult;
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
	PermitRequest { name: String, duration: Option<Duration>, err_tx: mpsc::SyncSender<PermitResult> },
	PermitEnd { name: String, err_tx: mpsc::SyncSender<PermitResult> },
	TabUpdate { tab: i64, url: Url },
	TabDelete { tab: i64 },
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
		let timeout = when_reload.and_then(|when| (when - Local::now()).to_std().ok());
		let event = recv_maybe(&event_queue.1, timeout).unwrap();
		let now = Local::now();

		if let Some(event) = event {
			match event {
				Event::PermitRequest { name, duration, err_tx } => {
					err_tx.send(permits.activate(&name, duration, &now)).unwrap();
					permits.reload(&now);
					tabs.rescan(rules.blocked(), permits.unblocked(), &webext);
					when_reload = compute_when_reload(&rules, &permits, &now);
				}
				Event::PermitEnd { name, err_tx } => {
					err_tx.send(permits.deactivate(&name)).unwrap();
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

fn recv_maybe<T>(rx: &mpsc::Receiver<T>, timeout: Option<Duration>) -> Result<Option<T>, mpsc::RecvError> {
	match timeout {
		Some(timeout) => match rx.recv_timeout(timeout) {
			Ok(event) => Ok(Some(event)),
			Err(RecvTimeoutError::Timeout) => Ok(None),
			Err(RecvTimeoutError::Disconnected) => Err(mpsc::RecvError),
		},
		None => Ok(Some(rx.recv()?)),
	}
}

fn compute_when_reload(rules: &RuleManager, permits: &PermitManager, now: &DateTime<Local>) -> Option<DateTime<Local>> {
	match (rules.when_reload(now), permits.when_reload()) {
		(Some(a), Some(b)) => Some(a.max(b)),
		(Some(a), None) | (None, Some(a)) => Some(a),
		(None, None) => None,
	}
}
