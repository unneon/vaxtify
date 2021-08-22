use crate::config;
use crate::config::Config;
use crate::lookups::Lookups;
use chrono::{DateTime, Local, NaiveTime};
use fixedbitset::FixedBitSet;
use log::info;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum PermitError {
	#[error("permit {name:?} does not exist")]
	PermitDoesNotExist { name: String },
	#[error("permit is not active")]
	PermitIsNotActive,
	#[error("permit extension refused")]
	PermitExtensionRefused(#[source] Box<PermitError>),
	#[error("duration is too long (got: {got:?}, maximum: {maximum:?})")]
	DurationTooLong { got: Duration, permit: String, maximum: Duration },
	#[error("duration is not specified")]
	DurationNotSpecified,
	#[error("cooldown is not finished ({left:?} left)")]
	CooldownNotFinished { left: Duration },
	#[error("permit is not available (only since {since} until {until})")]
	AvailableBadTime { since: NaiveTime, until: NaiveTime },
	#[error("cooldown after restart is not finished ({left:?} left)")]
	CooldownAfterRestart { left: Duration },
}

pub struct PermitManager<'a> {
	lookups: &'a Lookups<'a>,
	unblocked: FixedBitSet,
	state: Vec<PermitState>,
}

#[derive(Default)]
pub struct PermitSaveState {
	state: HashMap<String, PermitState>,
}

#[derive(Clone)]
struct PermitState {
	expires: Option<DateTime<Local>>,
	last_active: Option<DateTime<Local>>,
}

pub type PermitResult = Result<(), PermitError>;

impl<'a> PermitManager<'a> {
	pub fn new(lookups: &'a Lookups<'a>, save_state: PermitSaveState) -> Self {
		let mut unblocked = FixedBitSet::with_capacity(lookups.category.len());
		unblocked.extend(
			save_state
				.state
				.iter()
				.filter(|(_, state)| state.expires.is_some())
				.filter_map(|(name, _)| lookups.permit.id.get(name.as_str()))
				.copied(),
		);
		let mut state = vec![PermitState { expires: None, last_active: None }; lookups.permit.len()];
		for (permit_name, permit_state) in save_state.state {
			if let Some(permit_index) = lookups.permit.id.get(permit_name.as_str()) {
				state[*permit_index] = permit_state;
			}
		}
		PermitManager { lookups, unblocked, state }
	}

	pub fn unblocked(&self) -> &FixedBitSet {
		&self.unblocked
	}

	pub fn activate(&mut self, name: &str, now: &DateTime<Local>, restart_time: &DateTime<Local>) -> PermitResult {
		let id = self.get_permit(name)?;
		let details = self.lookups.permit.details[id];
		let state = &mut self.state[id];
		check_cooldown(now, state, details)?;
		check_restart_cooldown(now, restart_time, self.lookups.config)?;
		check_reload_cooldown(now, restart_time, self.lookups.config)?;
		check_available(now, details)?;
		state.last_active = Some(*now);
		state.expires = Some(*now + chrono::Duration::from_std(details.length).unwrap());
		info!("Permit {:?} activated on request.", name);
		Ok(())
	}

	pub fn deactivate(&mut self, name: &str) -> PermitResult {
		let id = self.get_permit(name)?;
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

	pub fn save_state(self) -> PermitSaveState {
		PermitSaveState {
			state: self.lookups.permit.name.iter().copied().map(str::to_owned).zip(self.state.into_iter()).collect(),
		}
	}

	fn get_permit(&self, name: &str) -> Result<usize, PermitError> {
		self.lookups
			.permit
			.id
			.get(name)
			.copied()
			.ok_or_else(|| PermitError::PermitDoesNotExist { name: name.to_owned() })
	}
}

fn check_cooldown(now: &DateTime<Local>, state: &PermitState, details: &config::Permit) -> PermitResult {
	match (state.last_active, details.cooldown) {
		(Some(last_active), Some(cooldown)) if last_active + chrono::Duration::from_std(cooldown).unwrap() > *now => {
			let error = PermitError::CooldownNotFinished { left: cooldown - (*now - last_active).to_std().unwrap() };
			if state.expires.is_some() {
				Err(PermitError::PermitExtensionRefused(Box::new(error)))
			} else {
				Err(error)
			}
		}
		_ => Ok(()),
	}
}

fn check_restart_cooldown(now: &DateTime<Local>, restart_time: &DateTime<Local>, config: &Config) -> PermitResult {
	match config.after.restart.block.permits {
		Some(cooldown) if (*now - *restart_time).to_std().unwrap() < cooldown => {
			Err(PermitError::CooldownAfterRestart { left: cooldown - (*now - *restart_time).to_std().unwrap() })
		}
		_ => Ok(()),
	}
}

fn check_reload_cooldown(now: &DateTime<Local>, restart_time: &DateTime<Local>, config: &Config) -> PermitResult {
	match config.after.reload.block.permits {
		Some(cooldown) if (*now - *restart_time).to_std().unwrap() < cooldown => {
			Err(PermitError::CooldownAfterRestart { left: cooldown - (*now - *restart_time).to_std().unwrap() })
		}
		_ => Ok(()),
	}
}

fn check_available(now: &DateTime<Local>, details: &config::Permit) -> PermitResult {
	if details.is_available(now) {
		Ok(())
	} else {
		Err(PermitError::AvailableBadTime {
			since: details.available.unwrap().since,
			until: details.available.unwrap().until,
		})
	}
}

fn check_active(state: &PermitState) -> PermitResult {
	if state.expires.is_some() {
		Ok(())
	} else {
		Err(PermitError::PermitIsNotActive)
	}
}
