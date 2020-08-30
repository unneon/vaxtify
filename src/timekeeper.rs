use crate::config::{Config, Enforce, Limit, Rule};
use crate::timeline::Timeline;
use chrono::{DateTime, Utc};

struct State<'a> {
	rule: &'a Rule,
	last_enforced: Option<DateTime<Utc>>,
}

pub struct Timekeeper<'a> {
	states: Vec<State<'a>>,
	config: &'a Config,
}

impl<'a> Timekeeper<'a> {
	pub fn new(config: &'a Config) -> Timekeeper {
		let states = config.rules.iter().map(|rule| State { rule, last_enforced: None }).collect();
		Timekeeper { states, config }
	}

	/// Computes a set of categories that should be enforced now.
	pub fn update_enforcements(&mut self, timeline: &Timeline) -> Vec<String> {
		let now = Utc::now();
		let mut categories = Vec::new();
		for state in &mut self.states {
			let activities = state.rule.all_activities(self.config);
			let Limit::Individual(limit) = state.rule.allowed;
			let time = timeline.compute_individual_time(&activities, now);
			if time.as_secs() != 0 {
				let used_ratio = time.as_secs_f64() / limit.as_secs_f64();
				println!("{:>6.2}% of rule {:?}", used_ratio * 100., state.rule.categories);
			}
			if time > limit && delay_passed(state, now) {
				categories.extend(state.rule.categories.iter().cloned());
				state.last_enforced = Some(now);
			} else {
				state.last_enforced = None;
			}
		}
		categories.sort();
		categories.dedup();
		categories
	}
}

fn delay_passed(state: &State, now: DateTime<Utc>) -> bool {
	let Enforce::Stepwise { delay } = state.rule.enforce;
	match state.last_enforced {
		Some(last_enforced) => (now - last_enforced).to_std().unwrap() >= delay,
		None => true,
	}
}
