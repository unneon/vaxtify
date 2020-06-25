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

#[derive(Clone, Debug)]
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

	pub fn filter_overused<'a>(&self, rules: &'a [Rule], now: DateTime<Utc>) -> Vec<&'a Rule> {
		rules.iter().filter(|rule| self.is_overused(rule, now)).collect()
	}

	fn is_overused(&self, rule: &Rule, now: DateTime<Utc>) -> bool {
		let activities = self.slots.keys().filter(|activity| activity.matches(rule)).collect::<Vec<_>>();
		if !activities.iter().any(|activity| self.slots[activity].is_active.is_some()) {
			return false;
		}
		let usage_time = self.usage_time(rule, now);
		let usage_ratio = usage_time.num_milliseconds() as f64 / rule.allowed_minutes.num_milliseconds() as f64;
		println!("{:>6.2}% in {:?}", usage_ratio * 100., rule);
		usage_time > rule.allowed_minutes
	}

	fn usage_time(&self, rule: &Rule, now: DateTime<Utc>) -> Duration {
		let ranges: Vec<_> = self
			.slots
			.iter()
			.filter(|(activity, _)| activity.matches(rule))
			.flat_map(|(_, slot)| slot.time_ranges_cooldown(rule.cooldown_hours, now))
			.collect();
		TimeRange::union_duration(ranges)
	}

	fn get_slot(&mut self, activity: Activity) -> &mut Slot {
		self.slots.entry(activity).or_insert_with(|| Slot { history: Vec::new(), is_active: None })
	}
}

impl Slot {
	fn time_ranges_cooldown(&self, cooldown: Duration, now: DateTime<Utc>) -> Vec<TimeRange> {
		self.time_ranges(now)
			.into_iter()
			.filter_map(|range| {
				let since = range.since.max(now - cooldown);
				if since < range.until {
					Some(TimeRange { since, until: range.until })
				} else {
					None
				}
			})
			.collect()
	}

	fn time_ranges(&self, now: DateTime<Utc>) -> Vec<TimeRange> {
		let mut ranges = self.history.clone();
		if let Some(since) = self.is_active {
			ranges.push(TimeRange { since, until: now });
		}
		ranges
	}
}

impl TimeRange {
	pub fn union_duration(mut ranges: Vec<TimeRange>) -> Duration {
		ranges.sort_by_key(|range| range.since);
		let mut last_until = ranges[0].since;
		let mut total = Duration::zero();
		for range in ranges {
			last_until = last_until.max(range.since);
			total = total + (range.until - last_until).max(Duration::zero());
			last_until = last_until.max(range.until);
		}
		total
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
	assert_eq!(slots.filter_overused(&rules, now), &[&rules[1]]);
}

#[test]
fn group_nonoverlapping() {
	let now = Utc::now();
	let rules = [Rule {
		allowed_minutes: Duration::minutes(4),
		cooldown_hours: Duration::hours(24),
		domains: vec!["a".to_owned(), "b".to_owned()],
	}];
	let website = |name: &str| Activity::Website { domain: name.to_owned() };
	let mut slots = Slots::new();
	slots.process_event(Event { timestamp: now - Duration::minutes(7), activity: website("a"), is_active: true });
	slots.process_event(Event { timestamp: now - Duration::minutes(4), activity: website("a"), is_active: false });
	slots.process_event(Event { timestamp: now - Duration::minutes(3), activity: website("b"), is_active: true });
	assert_eq!(slots.filter_overused(&rules, now), &[&rules[0]]);
}

#[test]
fn group_overlapping() {
	let now = Utc::now();
	let rules = [Rule {
		allowed_minutes: Duration::minutes(6),
		cooldown_hours: Duration::hours(24),
		domains: vec!["a".to_owned(), "b".to_owned()],
	}];
	let website = |name: &str| Activity::Website { domain: name.to_owned() };
	let mut slots = Slots::new();
	slots.process_event(Event { timestamp: now - Duration::minutes(5), activity: website("a"), is_active: true });
	slots.process_event(Event { timestamp: now - Duration::minutes(1), activity: website("a"), is_active: false });
	slots.process_event(Event { timestamp: now - Duration::minutes(4), activity: website("b"), is_active: true });
	assert_eq!(slots.filter_overused(&rules, now), &[] as &[&Rule]);
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
	let total_usage = TimeRange::union_duration(slot.time_ranges_cooldown(Duration::minutes(6), now));
	assert_eq!(total_usage, Duration::minutes(4));
}
