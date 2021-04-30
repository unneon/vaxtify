use crate::dbusapi::DBus;
use crate::lookups::Lookups;
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
}

struct TabState {
	mask: FixedBitSet,
	url: Url,
}

impl<'a> Tabs<'a> {
	pub fn new(lookups: &'a Lookups<'a>) -> Tabs<'a> {
		Tabs { lookups, tabs: HashMap::new(), alive: HashSet::new() }
	}

	pub fn insert(&mut self, tab: TabId, url: Url, blocked: &FixedBitSet, unblocked: &FixedBitSet, dbus: &DBus) {
		let mask = self.lookups.url_to_mask(&url);
		let should_close = should_block_mask(&mask, blocked, unblocked);
		let state = TabState { mask, url };
		if self.tabs.insert(tab, state).is_none() {
			self.alive.insert(tab);
		}
		if should_close {
			self.close(tab, dbus);
		}
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
	pub fn rescan(&mut self, blocked: &FixedBitSet, unblocked: &FixedBitSet, dbus: &DBus) {
		let to_close: Vec<TabId> = self
			.alive
			.iter()
			.copied()
			.filter(|tab| should_block_mask(&self.tabs[tab].mask, blocked, unblocked))
			.collect();
		for tab in to_close {
			self.close(tab, dbus);
		}
	}

	pub fn close(&mut self, tab: TabId, dbus: &DBus) {
		debug!("Tab blocked on {}.", self.tabs[&tab].url);
		let is_last = self.alive.remove(&tab) && self.alive.is_empty();
		if is_last && self.lookups.config.general.prevent_browser_close {
			dbus.tab_create_empty(tab.pid);
		}
		dbus.tab_close(tab);
	}
}

fn should_block_mask(mask: &FixedBitSet, blocked: &FixedBitSet, unblocked: &FixedBitSet) -> bool {
	mask.intersection(blocked).count() > 0 && mask.intersection(unblocked).count() == 0
}
