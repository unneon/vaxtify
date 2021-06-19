use crate::config;
use crate::lookups::Lookups;
use chrono::{DateTime, Local};
use fixedbitset::FixedBitSet;
use log::info;
use std::time::Duration;

#[derive(Debug)]
pub enum PermitError {
	PermitDoesNotExist,
	PermitIsNotActive,
	DurationTooLong,
	DurationNotSpecified,
	CooldownNotFinished,
	AvailableBadTime,
}

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

pub type PermitResult = Result<(), PermitError>;

impl<'a> PermitManager<'a> {
	pub fn new(lookups: &'a Lookups<'a>) -> Self {
		let unblocked = FixedBitSet::with_capacity(lookups.category.len());
		let state = vec![PermitState { expires: None, last_active: None }; lookups.permit.len()];
		PermitManager { lookups, unblocked, state }
	}

	pub fn unblocked(&self) -> &FixedBitSet {
		&self.unblocked
	}

	pub fn activate(&mut self, name: &str, duration: Option<Duration>, now: &DateTime<Local>) -> PermitResult {
		let id = *self.lookups.permit.id.get(name).ok_or(PermitError::PermitDoesNotExist)?;
		let details = self.lookups.permit.details[id];
		let state = &mut self.state[id];
		let duration = duration.or(details.length.default).ok_or(PermitError::DurationNotSpecified)?;
		check_duration(duration, details)?;
		check_cooldown(now, state, details)?;
		check_available(now, details)?;
		state.last_active = Some(*now);
		state.expires = Some(*now + chrono::Duration::from_std(duration).unwrap());
		info!("Permit {:?} activated on request.", name);
		Ok(())
	}

	pub fn deactivate(&mut self, name: &str) -> PermitResult {
		let id = *self.lookups.permit.id.get(name).ok_or(PermitError::PermitDoesNotExist)?;
		let state = &mut self.state[id];
		check_active(state)?;
		state.expires = None;
		info!("Permit {:?} deactivated on request.", name);
		Ok(())
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

fn check_duration(duration: Duration, details: &config::Permit) -> PermitResult {
	match details.length.maximum {
		Some(mx) if duration > mx => Err(PermitError::DurationTooLong),
		_ => Ok(()),
	}
}

fn check_cooldown(now: &DateTime<Local>, state: &PermitState, details: &config::Permit) -> PermitResult {
	match (state.last_active, details.cooldown) {
		(Some(last_active), Some(cooldown)) if last_active + chrono::Duration::from_std(cooldown).unwrap() > *now => {
			Err(PermitError::CooldownNotFinished)
		}
		_ => Ok(()),
	}
}

fn check_available(now: &DateTime<Local>, details: &config::Permit) -> PermitResult {
	if details.is_available(now) {
		Ok(())
	} else {
		Err(PermitError::AvailableBadTime)
	}
}

fn check_active(state: &PermitState) -> PermitResult {
	if state.expires.is_some() {
		Ok(())
	} else {
		Err(PermitError::PermitIsNotActive)
	}
}
