mod interval;

use crate::activity::Activity;
use crate::event::Event;
use crate::timeline::interval::Interval;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Default)]
struct ActivityTimeline {
	slices: Vec<Interval>,
	active_since: Option<DateTime<Utc>>,
}

pub struct Timeline {
	activities: HashMap<Activity, ActivityTimeline>,
}

impl ActivityTimeline {
	pub fn add_event(&mut self, timestamp: DateTime<Utc>, is_active: bool) {
		match (is_active, self.active_since) {
			(true, None) => self.active_since = Some(timestamp),
			(false, Some(since)) => {
				self.slices.push(Interval { since, until: timestamp });
				self.active_since = None;
			}
			_ => panic!("event source returned inconsistent data"),
		}
	}
}

impl Timeline {
	pub fn new() -> Timeline {
		Timeline { activities: HashMap::new() }
	}

	pub fn add_event(&mut self, event: Event) {
		self.activities.entry(event.activity).or_default().add_event(event.timestamp, event.is_active);
	}

	pub fn compute_individual_time(&self, activities: &[Activity], now: DateTime<Utc>) -> Duration {
		let intervals = self.finished_intervals(activities);
		match (intervals.last(), self.earliest_active_since(activities)) {
			(Some(last), Some(active_since)) if active_since <= last.until => (now - last.since).to_std().unwrap(),
			(_, Some(active_since)) => (now - active_since).to_std().unwrap(),
			(_, None) => Duration::new(0, 0),
		}
	}

	fn finished_intervals(&self, activities: &[Activity]) -> Vec<Interval> {
		Interval::merge(
			self.activities
				.iter()
				.filter(|(activity, _)| activities.contains(activity))
				.flat_map(|(_, timeline)| &timeline.slices)
				.copied()
				.collect(),
		)
	}

	fn earliest_active_since(&self, activities: &[Activity]) -> Option<DateTime<Utc>> {
		self.activities
			.iter()
			.filter(|(activity, _)| activities.contains(activity))
			.filter_map(|(_, timeline)| timeline.active_since)
			.max()
	}
}

#[cfg(test)]
fn check_time(names: &[&str], time: u32, timeline: &Timeline) -> u64 {
	use crate::util::example_time;
	let activities: Vec<_> = names.iter().copied().map(Activity::example).collect();
	timeline.compute_individual_time(&activities, example_time(time)).as_secs()
}

#[test]
fn double_active_and_inactive() {
	let mut timeline = Timeline::new();
	timeline.add_event(Event::example("a", 0, true));
	timeline.add_event(Event::example("b", 1, true));
	timeline.add_event(Event::example("a", 10, false));
	assert_eq!(check_time(&["a", "b"], 15, &timeline), 15);
}
