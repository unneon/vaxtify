use crate::slots::Slots;
use chrono::{DateTime, Utc};
use std::time::Duration;

mod input_webext;
mod slots;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Activity {
	Website { domain: String },
}

#[derive(Debug)]
pub struct Event {
	activity: Activity,
	timestamp: DateTime<Utc>,
	is_active: bool,
}

fn main() {
	let mut slots = Slots::new();
	let mut conn = input_webext::Connection::new();
	loop {
		let event = conn.next_timeout(Duration::from_secs(1));
		if let Some(event) = event {
			println!("{:?}", event);
			slots.process_event(event);
		} else {
			println!("..");
		}
	}
}
