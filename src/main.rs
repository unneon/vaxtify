mod activity;
mod config;
mod event;
mod ipc;
mod timekeeper;
mod timeline;
mod util;
mod webext;

use crate::config::{Config, Enforce};
use crate::timekeeper::Timekeeper;
use crate::timeline::Timeline;
use crate::webext::WebExt;
use chrono::Utc;
use std::time::Duration;

const IDLE_TIMEOUT: Duration = Duration::from_millis(1000);

fn main() {
	webext::proxy::check_and_run();
	let config = Config::load();
	let mut timekeeper = Timekeeper::new(&config);
	let mut timeline = Timeline::new();
	let mut webext = WebExt::new(&config);
	loop {
		if let Some(event) = webext.next() {
			timeline.add_event(event);
		} else {
			std::thread::sleep(IDLE_TIMEOUT);
		}
		let now = Utc::now();
		for (category, enforce) in timekeeper.update_enforcements(&timeline, now) {
			match enforce {
				Enforce::Close => webext.close_all(&config.category[&category].all_activities()),
			}
		}
	}
}
