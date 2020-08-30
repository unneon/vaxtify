use crate::activity::Activity;
use chrono::{DateTime, Utc};

#[derive(Debug, Eq, PartialEq)]
pub struct Event {
	pub activity: Activity,
	pub timestamp: DateTime<Utc>,
	pub is_active: bool,
}

impl Event {
	#[cfg(test)]
	pub fn example(name: &str, time: u32, is_active: bool) -> Event {
		use crate::util::example_time;
		Event { activity: Activity::example(name), timestamp: example_time(time), is_active }
	}
}
