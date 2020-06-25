use crate::config::{Config, Rule};
use crate::slots::Slots;
use chrono::{DateTime, Utc};
use std::time::Duration;

mod config;
mod slots;
mod sources;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Activity {
	Website { domain: String },
}

#[derive(Debug, PartialEq)]
pub struct Event {
	activity: Activity,
	timestamp: DateTime<Utc>,
	is_active: bool,
}

impl Activity {
	fn matches(&self, rule: &Rule) -> bool {
		let Activity::Website { domain } = self;
		rule.domains.contains(domain)
	}
}

fn main() {
	let config = Config::load();
	sources::webext::proxy::check_and_run(config.sources.webext.port);
	let mut slots = Slots::new();
	let mut conn = sources::webext::WebExt::new(config.sources.webext.port);
	loop {
		if let Some(event) = conn.next_timeout(Duration::from_secs(10)) {
			slots.process_event(event);
		}
		let now = Utc::now();
		let overused = slots.filter_overused(&config.rules, now);
		if !overused.is_empty() {
			std::process::Command::new("killall").arg("firefox").status().unwrap();
		}
	}
}
