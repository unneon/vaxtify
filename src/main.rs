mod cli;
mod config;
mod dbus;
mod filters;
mod logger;
mod lookups;
mod permits;
mod processes;
mod rules;
mod tabs;
mod webext;

use crate::config::{Config, ConfigError};
use crate::dbus::server::DBus;
use crate::permits::{PermitResult, PermitSaveState};
use crate::processes::Processes;
use crate::tabs::{TabId, TabsSaveState};
use chrono::{DateTime, Local};
use permits::PermitManager;
use rules::RuleManager;
use std::sync::mpsc;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;
use url::Url;

#[derive(Debug)]
pub enum Event {
	PermitRequest { name: String, err_tx: mpsc::SyncSender<PermitResult> },
	PermitEnd { name: String, err_tx: mpsc::SyncSender<PermitResult> },
	TabUpdate { tab: TabId, url: Url },
	TabDelete { tab: TabId },
	TabDeleteAll { pid: u32 },
	ServiceReload { err_tx: mpsc::SyncSender<Result<(), ConfigError>> },
}

struct SaveState {
	config: Config,
	tabs: TabsSaveState,
	permits: PermitSaveState,
}

fn main() {
	logger::init().unwrap();
	webext::proxy::check_and_run();
	if std::env::args().nth(1).as_deref() == Some("daemon") {
		run_daemon_outer()
	} else {
		cli::run();
	}
}

fn run_daemon_outer() {
	let config = Config::load().unwrap();
	let event_queue = mpsc::channel();
	let dbus = DBus::new(event_queue.0);
	dbus.refresh();

	let mut save_state = SaveState { config, tabs: Default::default(), permits: Default::default() };
	loop {
		save_state = run_daemon(save_state, &dbus, &event_queue.1);
	}
}

fn run_daemon(save_state: SaveState, dbus: &DBus, event_queue: &mpsc::Receiver<Event>) -> SaveState {
	let config = save_state.config;
	let lookups = lookups::Lookups::new(&config);
	let mut tabs = tabs::Tabs::new(&lookups, save_state.tabs);
	let mut processes = Processes::new(&lookups);
	let mut rules = RuleManager::new(&lookups);
	let mut permits = PermitManager::new(&lookups, save_state.permits);

	let initial_time = Local::now();
	rules.reload(&initial_time);
	permits.reload(&initial_time);
	tabs.rescan(rules.blocked(), permits.unblocked(), dbus, &initial_time);
	processes.rescan(rules.blocked(), permits.unblocked(), &initial_time);
	let mut when_reload = compute_when_reload(&rules, &permits, &processes, &initial_time);

	loop {
		let timeout = when_reload.and_then(|when| (when - Local::now()).to_std().ok());
		let event = recv_maybe(event_queue, timeout).unwrap();
		let now = Local::now();

		if let Some(event) = event {
			match event {
				Event::PermitRequest { name, err_tx } => {
					err_tx.send(permits.activate(&name, &now)).unwrap();
					permits.reload(&now);
					tabs.rescan(rules.blocked(), permits.unblocked(), dbus, &now);
					processes.rescan(rules.blocked(), permits.unblocked(), &now);
					when_reload = compute_when_reload(&rules, &permits, &processes, &now);
				}
				Event::PermitEnd { name, err_tx } => {
					err_tx.send(permits.deactivate(&name)).unwrap();
					permits.reload(&now);
					tabs.rescan(rules.blocked(), permits.unblocked(), dbus, &now);
					processes.rescan(rules.blocked(), permits.unblocked(), &now);
					when_reload = compute_when_reload(&rules, &permits, &processes, &now);
				}
				Event::TabUpdate { tab, url } => {
					tabs.insert(tab, url, rules.blocked(), permits.unblocked(), dbus, &now)
				}
				Event::TabDelete { tab } => tabs.remove(tab),
				Event::TabDeleteAll { pid } => tabs.clear(pid),
				Event::ServiceReload { err_tx } => match Config::load() {
					Ok(new_config) => {
						return SaveState {
							config: new_config,
							tabs: tabs.save_state(),
							permits: permits.save_state(),
						};
					}
					Err(err) => err_tx.send(Err(err)).unwrap(),
				},
			}
		} else {
			rules.reload(&now);
			permits.reload(&now);
			tabs.rescan(rules.blocked(), permits.unblocked(), dbus, &now);
			processes.rescan(rules.blocked(), permits.unblocked(), &now);
			when_reload = compute_when_reload(&rules, &permits, &processes, &now);
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

fn compute_when_reload(
	rules: &RuleManager,
	permits: &PermitManager,
	processes: &Processes,
	now: &DateTime<Local>,
) -> Option<DateTime<Local>> {
	IntoIterator::into_iter([rules.when_reload(now), permits.when_reload(), processes.when_reload()]).flatten().min()
}
