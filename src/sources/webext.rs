mod protocol;
pub mod proxy;
mod socket;

use crate::sources::webext::socket::Socket;
use crate::{Activity, Event};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use url::Url;

pub struct WebExt {
	tabs: HashMap<i64, String>,
	sites: HashMap<String, i64>,
	buffer: VecDeque<Event>,
	socket: Socket,
}

#[derive(Debug, PartialEq)]
struct Message {
	timestamp: DateTime<Utc>,
	kind: MessageKind,
}

#[derive(Debug, PartialEq)]
enum MessageKind {
	Created { tab: i64 },
	Removed { tab: i64 },
	Updated { tab: i64, url: String },
	Activated { tab: i64 },
	BrowserLaunch,
	BrowserShutdown,
}

impl WebExt {
	pub fn new(port: u16) -> WebExt {
		let socket = Socket::new(port);
		WebExt { tabs: HashMap::new(), sites: HashMap::new(), buffer: VecDeque::new(), socket }
	}

	pub fn next_timeout(&mut self, timeout: Duration) -> Option<Event> {
		let deadline = Instant::now() + timeout;
		loop {
			let timeout = deadline.checked_duration_since(Instant::now()).unwrap_or_default();
			match self.buffer.pop_front() {
				Some(event) => break Some(event),
				None => match self.fill_buffer(timeout) {
					Some(()) => (),
					None => break None,
				},
			}
		}
	}

	fn fill_buffer(&mut self, timeout: Duration) -> Option<()> {
		let message = self.socket.recv_timeout(timeout)?;
		self.process_message(message);
		Some(())
	}

	fn process_message(&mut self, message: Message) {
		match message.kind {
			MessageKind::Created { .. } => (),
			MessageKind::Removed { tab } => {
				let domain = self.tabs.remove(&tab);
				if let Some(domain) = domain {
					self.site_decrement(domain, message.timestamp);
				}
			}
			MessageKind::Updated { tab, url } => {
				let new_domain = Url::parse(&url).unwrap().domain().map(str::to_owned);
				let old_domain = if let Some(new_domain) = new_domain {
					self.site_increment(new_domain.clone(), message.timestamp);
					self.tabs.insert(tab, new_domain)
				} else {
					self.tabs.remove(&tab)
				};
				if let Some(old_domain) = old_domain {
					self.site_decrement(old_domain, message.timestamp);
				}
			}
			MessageKind::Activated { .. } => (),
			MessageKind::BrowserLaunch => {}
			MessageKind::BrowserShutdown => {
				for (_, domain) in std::mem::replace(&mut self.tabs, HashMap::new()) {
					self.site_decrement(domain, message.timestamp);
				}
			}
		}
	}

	fn site_increment(&mut self, domain: String, timestamp: DateTime<Utc>) {
		let refcount = self.sites.entry(domain.clone()).or_insert(0);
		*refcount += 1;
		if *refcount == 1 {
			self.add_event(domain, timestamp, true);
		}
	}

	fn site_decrement(&mut self, domain: String, timestamp: DateTime<Utc>) {
		let refcount = self.sites.get_mut(&domain).unwrap();
		*refcount -= 1;
		if *refcount == 0 {
			self.sites.remove(&domain);
			self.add_event(domain, timestamp, false);
		}
	}

	fn add_event(&mut self, domain: String, timestamp: DateTime<Utc>, is_active: bool) {
		let event = Event { activity: Activity::Website { domain }, timestamp, is_active };
		self.buffer.push_back(event);
	}
}
