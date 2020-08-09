mod activity;
mod config;
mod event;
mod ipc;
mod timekeeper;
mod timeline;
mod webext;

use crate::config::Config;
use crate::timekeeper::Timekeeper;
use crate::timeline::Timeline;
use crate::webext::WebExt;
use std::time::Duration;

const IDLE_TIMEOUT: Duration = Duration::from_millis(200);

fn main() {
	webext::proxy::check_and_run();
	let config = Config::load();
	println!("{:#?}", config);
	let mut timekeeper = Timekeeper::new(&config);
	let mut timeline = Timeline::new();
	let mut webext = WebExt::new();
	loop {
		if let Some(event) = webext.next() {
			println!("{:?}", event);
			timeline.add_event(event);
		} else {
			std::thread::sleep(IDLE_TIMEOUT);
		}
		for category in timekeeper.update_enforcements(&timeline) {
			for activity in config.category[&category].all_activities() {
				webext.close_one(activity);
			}
		}
	}
}
