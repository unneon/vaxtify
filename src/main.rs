use chrono::{DateTime, Utc};

mod input_webext;

#[derive(Debug, PartialEq)]
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
	for event in input_webext::Connection::new() {
		println!("{:?}", event);
	}
}
