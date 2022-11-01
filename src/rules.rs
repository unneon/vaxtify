use crate::lookups::Lookups;
use chrono::{DateTime, Local};
use fixedbitset::FixedBitSet;

pub struct RuleManager<'a> {
	lookups: &'a Lookups<'a>,
	blocked: FixedBitSet,
	state: Vec<bool>,
}

impl<'a> RuleManager<'a> {
	pub fn new(lookups: &'a Lookups<'a>) -> Self {
		let blocked = FixedBitSet::with_capacity(lookups.category.len());
		let last_state = vec![false; lookups.config.rules.len()];
		RuleManager { lookups, blocked, state: last_state }
	}

	pub fn blocked(&self) -> &FixedBitSet {
		&self.blocked
	}

	pub fn reload(&mut self, now: &DateTime<Local>) {
		self.blocked.clear();
		for (index, rule) in self.lookups.config.rules.iter().enumerate() {
			let is_active = rule.is_active(now);
			if is_active != self.state[index] {
				self.state[index] = is_active;
			}
			if is_active {
				for category in &rule.categories {
					self.blocked.insert(self.lookups.category.id[category.as_str()]);
				}
			}
		}
	}

	pub fn when_reload(&self, now: &DateTime<Local>) -> Option<DateTime<Local>> {
		for (index, rule) in self.lookups.config.rules.iter().enumerate() {
			if rule.is_active(now) != self.state[index] {
				return Some(*now);
			}
		}
		self.lookups.config.rules.iter().filter_map(|rule| rule.next_change_time(now)).min()
	}
}
