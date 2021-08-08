use crate::dbusapi::DBus;
use crate::lookups::Lookups;
use chrono::{DateTime, Local};
use fixedbitset::FixedBitSet;
use log::debug;
use std::collections::{HashMap, HashSet};
use url::Url;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TabId {
	pub pid: u32,
	pub tab: i32,
}

pub struct Tabs<'a> {
	lookups: &'a Lookups<'a>,
	tabs: HashMap<TabId, TabState>,
	alive: HashSet<TabId>,
	block_all_until: Option<DateTime<Local>>,
}

#[derive(Default)]
pub struct TabsSaveState {
	tabs: HashMap<TabId, Url>,
	alive: HashSet<TabId>,
	block_all_until: Option<DateTime<Local>>,
}

struct TabState {
	mask: FixedBitSet,
	url: Url,
}

impl<'a> Tabs<'a> {
	pub fn new(lookups: &'a Lookups<'a>, save_state: TabsSaveState) -> Tabs<'a> {
		Tabs {
			lookups,
			tabs: save_state
				.tabs
				.into_iter()
				.map(|(id, url)| (id, TabState { mask: lookups.url_to_mask(&url), url }))
				.collect(),
			alive: save_state.alive,
			block_all_until: save_state.block_all_until,
		}
	}

	pub fn insert(
		&mut self,
		tab: TabId,
		url: Url,
		blocked: &FixedBitSet,
		unblocked: &FixedBitSet,
		dbus: &DBus,
		now: &DateTime<Local>,
	) {
		let mask = self.lookups.url_to_mask(&url);
		let should_close = self.should_block_all(now) || should_block_mask(&mask, blocked, unblocked);
		let state = TabState { mask, url };
		if self.tabs.insert(tab, state).is_none() {
			self.alive.insert(tab);
		}
		if should_close {
			self.close(tab, dbus, now);
		}
	}

	fn should_block_all(&self, now: &DateTime<Local>) -> bool {
		self.block_all_until.map_or(false, |block_all_until| *now <= block_all_until)
	}

	pub fn remove(&mut self, tab: TabId) {
		self.tabs.remove(&tab);
		self.alive.remove(&tab);
	}

	pub fn clear(&mut self, pid: u32) {
		self.tabs.retain(|tab, _| tab.pid != pid);
		self.alive.retain(|tab| tab.pid != pid);
	}

	// TODO: Figure out how to avoid dependency on webext here?
	pub fn rescan(&mut self, blocked: &FixedBitSet, unblocked: &FixedBitSet, dbus: &DBus, now: &DateTime<Local>) {
		let to_close: Vec<TabId> = self
			.alive
			.iter()
			.copied()
			.filter(|tab| should_block_mask(&self.tabs[tab].mask, blocked, unblocked))
			.collect();
		for tab in to_close {
			self.close(tab, dbus, now);
		}
	}

	pub fn close(&mut self, tab: TabId, dbus: &DBus, now: &DateTime<Local>) {
		debug!("Tab blocked on {}.", self.tabs[&tab].url);
		let is_last = self.alive.remove(&tab) && self.alive.is_empty();
		if let Some(close_all_after_block) = self.lookups.config.general.close_all_after_block {
			self.block_all_until = Some(*now + chrono::Duration::from_std(close_all_after_block).unwrap());
		}
		if is_last && self.lookups.config.general.prevent_browser_close {
			dbus.tab_create_empty(tab.pid);
		}
		dbus.tab_close(tab);
		if self.lookups.config.general.close_all_on_block {
			if let Some(other_alive) = self.alive.iter().next().copied() {
				self.close(other_alive, dbus, now);
			}
		}
	}

	pub fn save_state(self) -> TabsSaveState {
		TabsSaveState {
			tabs: self.tabs.into_iter().map(|(id, state)| (id, state.url)).collect(),
			alive: self.alive,
			block_all_until: self.block_all_until,
		}
	}
}

fn should_block_mask(mask: &FixedBitSet, blocked: &FixedBitSet, unblocked: &FixedBitSet) -> bool {
	mask.intersection(blocked).count() > 0 && mask.intersection(unblocked).count() == 0
}
