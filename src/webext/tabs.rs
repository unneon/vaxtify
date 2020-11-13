use crate::activity::Activity;
use crate::event::Event;
use crate::webext::{WebEvent, WebEventKind};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub struct Tabs {
	tabs: HashMap<i64, Activity>,
	sites: HashMap<Activity, i64>,
}

impl Tabs {
	pub fn new() -> Self {
		Tabs { tabs: HashMap::new(), sites: HashMap::new() }
	}

	pub(super) fn process_web_event(&mut self, web_event: WebEvent) -> Vec<Event> {
		let mut events = Vec::new();
		match web_event.kind {
			WebEventKind::Created { .. } => (),
			WebEventKind::Removed { tab } => {
				let domain = self.tabs.remove(&tab);
				if let Some(domain) = domain {
					events.extend(self.site_decrement(domain, web_event.timestamp));
				}
			}
			WebEventKind::Updated { tab, url } => {
				let new_activity = Activity::from_url(&url.parse().unwrap());
				let old_activity = if let Some(new_activity) = new_activity {
					events.extend(self.site_increment(new_activity.clone(), web_event.timestamp));
					self.tabs.insert(tab, new_activity)
				} else {
					self.tabs.remove(&tab)
				};
				if let Some(old_domain) = old_activity {
					events.extend(self.site_decrement(old_domain, web_event.timestamp));
				}
			}
			WebEventKind::Activated { .. } => (),
			WebEventKind::BrowserLaunch => {}
			WebEventKind::BrowserShutdown => {
				for (_, domain) in std::mem::replace(&mut self.tabs, HashMap::new()) {
					events.extend(self.site_decrement(domain, web_event.timestamp));
				}
			}
		}
		events
	}

	fn site_increment(&mut self, activity: Activity, timestamp: DateTime<Utc>) -> Option<Event> {
		let refcount = self.sites.entry(activity.clone()).or_insert(0);
		*refcount += 1;
		if *refcount == 1 {
			Some(Event { activity, timestamp, is_active: true })
		} else {
			None
		}
	}

	fn site_decrement(&mut self, activity: Activity, timestamp: DateTime<Utc>) -> Option<Event> {
		let refcount = self.sites.get_mut(&activity).unwrap();
		*refcount -= 1;
		if *refcount == 0 {
			self.sites.remove(&activity);
			Some(Event { activity, timestamp, is_active: false })
		} else {
			None
		}
	}

	pub fn filter_by_activities(&self, activities: &[Activity]) -> Vec<i64> {
		self.tabs.iter().filter(|(_, activity)| activities.contains(activity)).map(|(tab, _)| *tab).collect()
	}
}

#[test]
fn multiple_tabs() {
	let timestamp = Utc::now();
	let mut tabs = Tabs::new();
	let mut send = |kind| tabs.process_web_event(WebEvent { timestamp, kind });
	assert_eq!(
		send(WebEventKind::Updated { tab: 0, url: "https://example.com".to_owned() }),
		&[Event { timestamp, activity: Activity::Internet { domain: "example.com".to_owned() }, is_active: true }]
	);
	assert_eq!(send(WebEventKind::Updated { tab: 1, url: "https://example.com/robots.txt".to_owned() }), &[]);
	assert_eq!(send(WebEventKind::Removed { tab: 0 }), &[]);
	assert_eq!(
		send(WebEventKind::Removed { tab: 1 }),
		&[Event { timestamp, activity: Activity::Internet { domain: "example.com".to_owned() }, is_active: false }]
	);
}

#[test]
fn clean_browser_shutdown() {
	let timestamp = Utc::now();
	let mut tabs = Tabs::new();
	let mut send = |kind| tabs.process_web_event(WebEvent { timestamp, kind });
	assert_eq!(
		send(WebEventKind::Updated { tab: 0, url: "https://example.com".to_owned() }),
		&[Event { timestamp, activity: Activity::Internet { domain: "example.com".to_owned() }, is_active: true }]
	);
	assert_eq!(
		send(WebEventKind::BrowserShutdown),
		&[Event { timestamp, activity: Activity::Internet { domain: "example.com".to_owned() }, is_active: false }]
	);
}
