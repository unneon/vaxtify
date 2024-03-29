use crate::lookups::Lookups;
use chrono::{DateTime, Local};
use fixedbitset::FixedBitSet;
use std::time::Duration;

pub struct Processes<'a> {
	lookups: &'a Lookups<'a>,
	when_last_scan: DateTime<Local>,
}

pub const DEFAULT_SCAN_EACH: Duration = Duration::from_secs(10);

impl<'a> Processes<'a> {
	pub fn new(lookups: &'a Lookups<'a>) -> Self {
		Processes { lookups, when_last_scan: Local::now() }
	}

	pub fn when_reload(&self) -> Option<DateTime<Local>> {
		Some(self.when_last_scan + chrono::Duration::from_std(self.lookups.config.processes_scan_each.into()).unwrap())
	}

	pub fn rescan(&mut self, blocked: &FixedBitSet, unblocked: &FixedBitSet, now: &DateTime<Local>) {
		let processes: Vec<_> = self
			.lookups
			.process
			.keys()
			.filter(|process| should_block_mask(&self.lookups.process_to_mask(process), blocked, unblocked))
			.collect();
		if !processes.is_empty() {
			std::process::Command::new("killall")
				.arg("-9")
				.args(processes)
				.stdout(std::process::Stdio::null())
				.stderr(std::process::Stdio::null())
				.status()
				.unwrap();
		}
		self.when_last_scan = *now;
	}
}

fn should_block_mask(mask: &FixedBitSet, blocked: &FixedBitSet, unblocked: &FixedBitSet) -> bool {
	mask.intersection(blocked).count() > 0 && mask.intersection(unblocked).count() == 0
}
