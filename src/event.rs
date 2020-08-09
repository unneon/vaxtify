use crate::activity::Activity;
use chrono::{DateTime, Utc};

#[derive(Debug, Eq, PartialEq)]
pub struct Event {
	pub activity: Activity,
	pub timestamp: DateTime<Utc>,
	pub is_active: bool,
}
