use crate::config::{Config, Enforce, Limit, Rule};
use crate::timeline::Timeline;
use chrono::{DateTime, Local, Utc};

struct Fuse {
	limit: f64,
	triggered: bool,
	enforce: Enforce,
}

struct State<'a> {
	rule: &'a Rule,
	fuses: Vec<Fuse>,
}

pub struct Timekeeper<'a> {
	states: Vec<State<'a>>,
	config: &'a Config,
}

impl<'a> Timekeeper<'a> {
	pub fn new(config: &'a Config) -> Timekeeper {
		let states = config.rules.iter().map(State::new).collect();
		Timekeeper { states, config }
	}

	/// Computes a set of categories that should be enforced now.
	pub fn update_enforcements(&mut self, timeline: &Timeline, now: DateTime<Utc>) -> Vec<(String, Enforce)> {
		let mut enforces = Vec::new();
		for state in &mut self.states {
			match &state.rule.allowed {
				Limit::Individual(limit) => {
					let activities = state.rule.all_activities(self.config);
					let time = timeline.compute_individual_time(&activities, now);
					let time_ratio = time.as_secs_f64() / limit.as_secs_f64();
					for fuse in &mut state.fuses {
						if !fuse.triggered && time_ratio >= fuse.limit {
							for category in &state.rule.categories {
								enforces.push((category.clone(), fuse.enforce));
							}
							fuse.triggered = true;
						} else if fuse.triggered && time_ratio < fuse.limit {
							fuse.triggered = false;
						}
					}
				}
				Limit::During { since, until } => {
					let now = now.with_timezone(&Local).time();
					let is_allowed =
						if since <= until { now >= *since && now <= *until } else { now >= *since || now <= *until };
					if !is_allowed {
						for category in &state.rule.categories {
							enforces.push((category.clone(), state.rule.enforce));
						}
					}
				}
				Limit::Never => {
					for category in &state.rule.categories {
						enforces.push((category.clone(), state.rule.enforce));
					}
				}
			}
		}
		enforces
	}
}

impl<'a> State<'a> {
	pub fn new(rule: &'a Rule) -> State<'a> {
		State { rule, fuses: vec![Fuse { limit: 1.0, triggered: false, enforce: rule.enforce }] }
	}
}

#[test]
fn no_call_twice() {
	use crate::event::Event;
	use crate::util::example_time;

	let config = r#"
[category.example]
domains = ["example.com"]

[[rules]]
allowed.individual.seconds = 5
categories = ["example"]
enforce.close = {}
"#;
	let config = Config::parse(config);

	let mut timeline = Timeline::new();
	timeline.add_event(Event::example("example.com", 0, true));

	let mut timekeeper = Timekeeper::new(&config);
	let enforces1 = timekeeper.update_enforcements(&timeline, example_time(10));
	let enforces2 = timekeeper.update_enforcements(&timeline, example_time(15));
	let enforces3 = timekeeper.update_enforcements(&timeline, example_time(20));
	assert_eq!(enforces1, [("example".to_owned(), Enforce::Close)]);
	assert_eq!(enforces2, []);
	assert_eq!(enforces3, []);
}
