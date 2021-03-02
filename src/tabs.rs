use crate::lookups::Lookups;
use crate::webext::WebExt;
use fixedbitset::FixedBitSet;
use log::debug;
use std::collections::{HashMap, HashSet};
use url::Url;

pub struct Tabs<'a> {
	lookups: &'a Lookups<'a>,
	tabs: HashMap<i64, TabState>,
	alive: HashSet<i64>,
}

struct TabState {
	mask: FixedBitSet,
	url: Url,
}

impl<'a> Tabs<'a> {
	pub fn new(lookups: &'a Lookups<'a>) -> Tabs<'a> {
		Tabs { lookups, tabs: HashMap::new(), alive: HashSet::new() }
	}

	pub fn insert(&mut self, tab: i64, url: Url, blocked: &FixedBitSet, unblocked: &FixedBitSet, webext: &WebExt) {
		let mask = self.lookups.url_to_mask(&url);
		let should_close = should_block_mask(&mask, blocked, unblocked);
		let state = TabState { mask, url };
		if self.tabs.insert(tab, state).is_none() {
			self.alive.insert(tab);
		}
		if should_close {
			self.close(tab, webext);
		}
	}

	pub fn remove(&mut self, tab: i64) {
		self.tabs.remove(&tab);
		self.alive.remove(&tab);
	}

	pub fn clear(&mut self) {
		self.tabs.clear();
		self.alive.clear();
	}

	// TODO: Figure out how to avoid dependency on webext here?
	pub fn rescan(&mut self, blocked: &FixedBitSet, unblocked: &FixedBitSet, webext: &WebExt) {
		let to_close: Vec<i64> = self
			.alive
			.iter()
			.copied()
			.filter(|tab| should_block_mask(&self.tabs[tab].mask, blocked, unblocked))
			.collect();
		for tab in to_close {
			self.close(tab, webext);
		}
	}

	pub fn close(&mut self, tab: i64, webext: &WebExt) {
		debug!("Tab blocked on {}.", self.tabs[&tab].url);
		let is_last = self.alive.remove(&tab) && self.alive.is_empty();
		if is_last {
			webext.create_empty_tab();
		}
		webext.close_tab(tab);
	}
}

fn should_block_mask(mask: &FixedBitSet, blocked: &FixedBitSet, unblocked: &FixedBitSet) -> bool {
	mask.intersection(blocked).count() > 0 && mask.intersection(unblocked).count() == 0
}
