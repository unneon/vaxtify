use crate::sources::webext::{Message, MessageKind};
use crate::{Activity, Event};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use url::Url;

pub struct Tabs {
	tabs: HashMap<i64, String>,
	sites: HashMap<String, i64>,
}

impl Tabs {
	pub fn new() -> Self {
		Tabs { tabs: HashMap::new(), sites: HashMap::new() }
	}

	pub(super) fn process_message(&mut self, message: Message) -> Vec<Event> {
		let mut events = Vec::new();
		match message.kind {
			MessageKind::Created { .. } => (),
			MessageKind::Removed { tab } => {
				let domain = self.tabs.remove(&tab);
				if let Some(domain) = domain {
					events.extend(self.site_decrement(domain, message.timestamp));
				}
			}
			MessageKind::Updated { tab, url } => {
				let new_domain = Url::parse(&url).unwrap().domain().map(str::to_owned);
				let old_domain = if let Some(new_domain) = new_domain {
					events.extend(self.site_increment(new_domain.clone(), message.timestamp));
					self.tabs.insert(tab, new_domain)
				} else {
					self.tabs.remove(&tab)
				};
				if let Some(old_domain) = old_domain {
					events.extend(self.site_decrement(old_domain, message.timestamp));
				}
			}
			MessageKind::Activated { .. } => (),
			MessageKind::BrowserLaunch => {}
			MessageKind::BrowserShutdown => {
				for (_, domain) in std::mem::replace(&mut self.tabs, HashMap::new()) {
					events.extend(self.site_decrement(domain, message.timestamp));
				}
			}
		}
		events
	}

	fn site_increment(&mut self, domain: String, timestamp: DateTime<Utc>) -> Option<Event> {
		let refcount = self.sites.entry(domain.clone()).or_insert(0);
		*refcount += 1;
		if *refcount == 1 {
			Some(Event { activity: Activity::Website { domain }, timestamp, is_active: true })
		} else {
			None
		}
	}

	fn site_decrement(&mut self, domain: String, timestamp: DateTime<Utc>) -> Option<Event> {
		let refcount = self.sites.get_mut(&domain).unwrap();
		*refcount -= 1;
		if *refcount == 0 {
			self.sites.remove(&domain);
			Some(Event { activity: Activity::Website { domain }, timestamp, is_active: false })
		} else {
			None
		}
	}
}

#[test]
fn multiple_tabs() {
	let timestamp = Utc::now();
	let mut tabs = Tabs::new();
	let mut send = |kind| tabs.process_message(Message { timestamp, kind });
	assert_eq!(
		send(MessageKind::Updated { tab: 0, url: "https://github.com/pustaczek".to_owned() }),
		&[Event { timestamp, activity: Activity::Website { domain: "github.com".to_owned() }, is_active: true }]
	);
	assert_eq!(
		send(MessageKind::Updated { tab: 1, url: "https://github.com/pustaczek/distraction-oni".to_owned() }),
		&[]
	);
	assert_eq!(send(MessageKind::Removed { tab: 0 }), &[]);
	assert_eq!(
		send(MessageKind::Removed { tab: 1 }),
		&[Event { timestamp, activity: Activity::Website { domain: "github.com".to_owned() }, is_active: false }]
	);
}

#[test]
fn clean_browser_shutdown() {
	let timestamp = Utc::now();
	let mut tabs = Tabs::new();
	let mut send = |kind| tabs.process_message(Message { timestamp, kind });
	assert_eq!(
		send(MessageKind::Updated { tab: 0, url: "https://github.com".to_owned() }),
		&[Event { timestamp, activity: Activity::Website { domain: "github.com".to_owned() }, is_active: true }]
	);
	assert_eq!(
		send(MessageKind::BrowserShutdown),
		&[Event { timestamp, activity: Activity::Website { domain: "github.com".to_owned() }, is_active: false }]
	);
}
