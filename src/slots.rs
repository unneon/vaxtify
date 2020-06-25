use crate::config::Rule;
use crate::{Activity, Event};
use chrono::Duration;
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

	pub fn filter_overused(&self, rules: &[Rule]) -> Vec<&Activity> {
		self.slots
			.iter()
			.filter(|(activity, slot)| self.is_overused(rules, activity, slot))
			.map(|entry| entry.0)
			.collect()
	}

	fn is_overused(&self, rules: &[Rule], activity: &Activity, slot: &Slot) -> bool {
		if slot.is_active.is_none() {
			return false;
		}
		let rule = match rules.iter().find(|rule| activity.matches(rule)) {
			Some(rule) => rule,
			None => return false,
		};
		let usage_time = slot.usage_time(rule.cooldown_hours);
		let usage_ratio = usage_time.num_milliseconds() as f64 / rule.allowed_minutes.num_milliseconds() as f64;
		println!("{:>6.2}% in {:?}", usage_ratio * 100., activity);
		slot.is_active.is_some() && usage_time > rule.allowed_minutes
	}

	fn get_slot(&mut self, activity: Activity) -> &mut Slot {
		self.slots.entry(activity).or_insert_with(|| Slot { history: Vec::new(), is_active: None })
	}
}

impl Slot {
	pub fn usage_time(&self, cooldown: Duration) -> Duration {
		let now = Utc::now();
		let cutoff = now - cooldown;
		let current_session = self.is_active.map(|since| now - since).unwrap_or_else(Duration::zero);
		let past_sessions = self
			.history
			.iter()
			.map(|time_range| {
				let since = cutoff.max(time_range.since);
				(time_range.until - since).max(Duration::zero())
			})
			.fold(Duration::zero(), |a, b| a + b);
		current_session + past_sessions
	}
}

#[test]
fn basic_overusage() {
	let now = Utc::now();
	let rules = [
		Rule {
			allowed_minutes: Duration::minutes(4),
			cooldown_hours: Duration::hours(1),
			domains: vec!["a".to_owned()],
		},
		Rule {
			allowed_minutes: Duration::minutes(4),
			cooldown_hours: Duration::hours(1),
			domains: vec!["b".to_owned()],
		},
		Rule {
			allowed_minutes: Duration::minutes(4),
			cooldown_hours: Duration::hours(1),
			domains: vec!["c".to_owned()],
		},
	];
	let website = |name: &str| Activity::Website { domain: name.to_owned() };
	let mut slots = Slots::new();
	slots.process_event(Event { timestamp: now - Duration::minutes(2), activity: website("a"), is_active: true });
	slots.process_event(Event { timestamp: now - Duration::minutes(6), activity: website("b"), is_active: true });
	slots.process_event(Event { timestamp: now - Duration::minutes(8), activity: website("c"), is_active: true });
	slots.process_event(Event { timestamp: now - Duration::minutes(2), activity: website("c"), is_active: false });
	slots.process_event(Event { timestamp: now - Duration::minutes(6), activity: website("d"), is_active: true });
	assert_eq!(slots.filter_overused(&rules), &[&website("b")]);
}

#[test]
fn usage_time() {
	let now = Utc::now();
	let slot = Slot {
		history: vec![
			TimeRange { since: now - Duration::minutes(10), until: now - Duration::minutes(8) },
			TimeRange { since: now - Duration::minutes(7), until: now - Duration::minutes(5) },
			TimeRange { since: now - Duration::minutes(4), until: now - Duration::minutes(2) },
		],
		is_active: Some(now - Duration::minutes(1)),
	};
	assert_eq!(slot.usage_time(Duration::minutes(6)), Duration::minutes(4));
}
