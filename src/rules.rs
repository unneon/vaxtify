use crate::lookups::Lookups;
use chrono::{DateTime, Local};
use fixedbitset::FixedBitSet;

pub struct RuleManager<'a> {
	lookups: &'a Lookups<'a>,
	blocked: FixedBitSet,
	state: Vec<bool>,
	restart_time: &'a DateTime<Local>,
	restart_completed: bool,
	reload_time: &'a DateTime<Local>,
	reload_completed: bool,
}

impl<'a> RuleManager<'a> {
	pub fn new(lookups: &'a Lookups<'a>, restart_time: &'a DateTime<Local>, reload_time: &'a DateTime<Local>) -> Self {
		let blocked = FixedBitSet::with_capacity(lookups.category.len());
		let last_state = vec![false; lookups.config.rule.len()];
		RuleManager {
			lookups,
			blocked,
			state: last_state,
			restart_time,
			restart_completed: false,
			reload_time,
			reload_completed: false,
		}
	}

	pub fn blocked(&self) -> &FixedBitSet {
		&self.blocked
	}

	pub fn reload(&mut self, now: &DateTime<Local>) {
		self.blocked.clear();
		if self.when_reload_after_restart_cooldown().map_or(true, |when| when <= *now) {
			self.restart_completed = true;
		}
		if self.when_reload_after_reload_cooldown().map_or(true, |when| when <= *now) {
			self.reload_completed = true;
		}
		for (index, rule) in self.lookups.config.rule.values().enumerate() {
			let is_active = rule.is_active(now) || !self.restart_completed || !self.reload_completed;
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
		for (index, rule) in self.lookups.config.rule.values().enumerate() {
			if rule.is_active(now) != self.state[index] {
				return Some(*now);
			}
		}
		self.lookups
			.config
			.rule
			.values()
			.filter_map(|rule| rule.next_change_time(now))
			.chain(self.when_reload_after_restart_cooldown())
			.chain(self.when_reload_after_reload_cooldown())
			.min()
	}

	fn when_reload_after_restart_cooldown(&self) -> Option<DateTime<Local>> {
		if self.restart_completed {
			None
		} else {
			self.lookups
				.config
				.after
				.restart
				.block
				.rules
				.map(|cooldown| *self.restart_time + chrono::Duration::from_std(cooldown).unwrap())
		}
	}

	fn when_reload_after_reload_cooldown(&self) -> Option<DateTime<Local>> {
		if self.reload_completed {
			None
		} else {
			self.lookups
				.config
				.after
				.reload
				.block
				.rules
				.map(|cooldown| *self.reload_time + chrono::Duration::from_std(cooldown).unwrap())
		}
	}
}
