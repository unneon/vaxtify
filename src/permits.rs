use crate::lookups::Lookups;
use chrono::{DateTime, Local};
use fixedbitset::FixedBitSet;
use log::info;
use std::time::Duration;

// enum PermitError {
// 	PermitDoesNotExist,
// 	PermitIsNotActive,
// 	DurationTooLong,
// 	DurationNotSpecified,
// 	CooldownNotFinished,
// }

pub struct PermitManager<'a> {
	lookups: &'a Lookups<'a>,
	unblocked: FixedBitSet,
	state: Vec<PermitState>,
}

#[derive(Clone)]
struct PermitState {
	expires: Option<DateTime<Local>>,
	last_active: Option<DateTime<Local>>,
}

impl<'a> PermitManager<'a> {
	pub fn new(lookups: &'a Lookups<'a>) -> Self {
		let unblocked = FixedBitSet::with_capacity(lookups.category.len());
		let state = vec![PermitState { expires: None, last_active: None }; lookups.permit.len()];
		PermitManager { lookups, unblocked, state }
	}

	pub fn unblocked(&self) -> &FixedBitSet {
		&self.unblocked
	}

	pub fn activate(&mut self, name: &str, duration: Option<Duration>, now: &DateTime<Local>) {
		let id = self.lookups.permit.id[name];
		let details = self.lookups.permit.details[id];
		let state = &mut self.state[id];
		let duration = duration.or(details.length.default).unwrap();
		if let Some(max_duration) = details.length.maximum {
			assert!(duration <= max_duration);
		}
		if let (Some(last_active), Some(cooldown)) = (state.last_active, details.cooldown) {
			let cooldown = chrono::Duration::from_std(cooldown).unwrap();
			assert!(last_active + cooldown <= *now);
		}
		let duration = chrono::Duration::from_std(duration).unwrap();
		state.last_active = Some(*now);
		state.expires = Some(*now + duration);
		info!("Permit {:?} activated on request.", name);
	}

	pub fn deactivate(&mut self, name: &str) {
		let id = self.lookups.permit.id[name];
		let state = &mut self.state[id];
		assert!(state.expires.is_some());
		state.expires = None;
		info!("Permit {:?} deactivated on request.", name);
	}

	pub fn reload(&mut self, now: &DateTime<Local>) {
		self.unblocked.clear();
		for (per_id, state) in self.state.iter_mut().enumerate() {
			let name = self.lookups.permit.name[per_id];
			let details = self.lookups.permit.details[per_id];
			if let Some(expires) = state.expires {
				if expires <= *now {
					state.expires = None;
					info!("Permit {:?} deactivated after using allotted time.", name);
				}
			}
			if state.expires.is_some() {
				for category in &details.categories {
					self.unblocked.insert(self.lookups.category.id[category.as_str()]);
				}
			}
		}
	}

	pub fn when_reload(&self) -> Option<DateTime<Local>> {
		self.state.iter().filter_map(|state| state.expires).min()
	}
}

// pub type PermitResponse = Result<(), PermitError>;
