use crate::{Activity, Event};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Slots {
	slots: HashMap<Activity, Slot>,
}

#[derive(Debug)]
struct Slot {
	history: Vec<TimeRange>,
	is_active: Option<DateTime<Utc>>,
}

#[derive(Debug)]
struct TimeRange {
	since: DateTime<Utc>,
	until: DateTime<Utc>,
}

impl Slots {
	pub fn new() -> Slots {
		Slots { slots: HashMap::new() }
	}

	pub fn process_event(&mut self, event: Event) {
		let slot = self.get_slot(event.activity.clone());
		if event.is_active {
			slot.is_active = Some(event.timestamp);
		} else {
			let since = slot.is_active.take().unwrap();
			slot.history.push(TimeRange { since, until: event.timestamp });
		}
	}

	fn get_slot(&mut self, activity: Activity) -> &mut Slot {
		self.slots.entry(activity).or_insert_with(|| Slot { history: Vec::new(), is_active: None })
	}
}
