use crate::activity::Activity;
use crate::event::Event;
use crate::webext::{WebEvent, WebEventKind};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use url::Url;

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
				let new_activity = url_to_activity(&url.parse().unwrap());
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

	pub fn find_by_activity(&self, activity: Activity) -> Option<i64> {
		self.tabs.iter().filter(|(_, haystack)| **haystack == activity).map(|(tab, _)| *tab).next()
	}
}

fn url_to_activity(url: &Url) -> Option<Activity> {
	url_to_reddit(url).or_else(|| url_to_internet(url))
}

fn url_to_reddit(url: &Url) -> Option<Activity> {
	if url.domain()? != "www.reddit.com" {
		return None;
	}
	let path_segments = url.path_segments()?.collect::<Vec<_>>();
	if path_segments.len() < 2 || path_segments[0] != "r" {
		return None;
	}
	Some(Activity::Reddit { subreddit: path_segments[1].to_owned() })
}

fn url_to_internet(url: &Url) -> Option<Activity> {
	Some(Activity::Internet { domain: url.domain()?.to_owned() })
}

#[test]
fn multiple_tabs() {
	let timestamp = Utc::now();
	let mut tabs = Tabs::new();
	let mut send = |kind| tabs.process_web_event(WebEvent { timestamp, kind });
	assert_eq!(
		send(WebEventKind::Updated { tab: 0, url: "https://github.com/pustaczek".to_owned() }),
		&[Event { timestamp, activity: Activity::Internet { domain: "github.com".to_owned() }, is_active: true }]
	);
	assert_eq!(send(WebEventKind::Updated { tab: 1, url: "https://github.com/pustaczek/vaxtify".to_owned() }), &[]);
	assert_eq!(send(WebEventKind::Removed { tab: 0 }), &[]);
	assert_eq!(
		send(WebEventKind::Removed { tab: 1 }),
		&[Event { timestamp, activity: Activity::Internet { domain: "github.com".to_owned() }, is_active: false }]
	);
}

#[test]
fn clean_browser_shutdown() {
	let timestamp = Utc::now();
	let mut tabs = Tabs::new();
	let mut send = |kind| tabs.process_web_event(WebEvent { timestamp, kind });
	assert_eq!(
		send(WebEventKind::Updated { tab: 0, url: "https://github.com".to_owned() }),
		&[Event { timestamp, activity: Activity::Internet { domain: "github.com".to_owned() }, is_active: true }]
	);
	assert_eq!(
		send(WebEventKind::BrowserShutdown),
		&[Event { timestamp, activity: Activity::Internet { domain: "github.com".to_owned() }, is_active: false }]
	);
}
