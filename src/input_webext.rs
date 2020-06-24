use crate::{Activity, Event};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, VecDeque};
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use url::Url;

pub struct Connection {
	tabs: HashMap<i64, String>,
	sites: HashMap<String, i64>,
	buffer: VecDeque<Event>,
	message_rx: mpsc::Receiver<Message>,
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

impl Connection {
	pub fn new() -> Connection {
		let listener = TcpListener::bind("localhost:56154").unwrap();
		let (message_tx, message_rx) = mpsc::sync_channel(0);
		std::thread::spawn(move || receive_message_loop(listener, message_tx));
		Connection { tabs: HashMap::new(), sites: HashMap::new(), buffer: VecDeque::new(), message_rx }
	}

	pub fn next_timeout(&mut self, timeout: Duration) -> Option<Event> {
		let deadline = Instant::now() + timeout;
		loop {
			let timeout = deadline.checked_duration_since(Instant::now()).unwrap_or_default();
			match self.buffer.pop_front() {
				Some(event) => break Some(event),
				None => match self.fill_buffer(timeout) {
					Ok(()) => (),
					Err(mpsc::RecvTimeoutError::Timeout) => break None,
					e => e.unwrap(),
				},
			}
		}
	}

	fn fill_buffer(&mut self, timeout: Duration) -> Result<(), mpsc::RecvTimeoutError> {
		let message = self.message_rx.recv_timeout(timeout)?;
		self.process_message(message);
		Ok(())
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

fn receive_message_loop(listener: TcpListener, tx: mpsc::SyncSender<Message>) {
	let mut connection_slot = None;
	loop {
		let message = match &mut connection_slot {
			Some(connection) => match webext::read(connection) {
				Ok(raw) => parse_message(&raw),
				Err(_) => {
					connection_slot = None;
					Message { timestamp: Utc::now(), kind: MessageKind::BrowserShutdown }
				}
			},
			None => {
				connection_slot = Some(listener.accept().unwrap().0);
				Message { timestamp: Utc::now(), kind: MessageKind::BrowserLaunch }
			}
		};
		tx.send(message).unwrap();
	}
}

fn parse_message(raw: &[u8]) -> Message {
	let string = std::str::from_utf8(&raw).unwrap();
	let value = json::parse(string).unwrap();
	let tab = value["tab"].as_i64().unwrap();
	let timestamp = value["timestamp"].as_str().unwrap().parse().unwrap();
	let kind = value["kind"].as_str().unwrap();
	let kind = match kind {
		"Created" => MessageKind::Created { tab },
		"Removed" => MessageKind::Removed { tab },
		"Updated" => MessageKind::Updated { tab, url: value["url"].as_str().unwrap().to_owned() },
		"Activated" => MessageKind::Activated { tab },
		_ => unreachable!(),
	};
	Message { timestamp, kind }
}

#[test]
fn test_parse() {
	let c_str = "{\"kind\":\"Created\",\"timestamp\":\"2020-06-11T22:07:54.925Z\",\"tab\":20}";
	let r_str = "{\"kind\":\"Removed\",\"timestamp\":\"2020-06-11T22:07:55.885Z\",\"tab\":20}";
	let u_str = "{\"kind\":\"Updated\",\"timestamp\":\"2020-06-11T22:07:49.692Z\",\"tab\":19,\"url\":\"about:blank\"}";
	let a_str = "{\"kind\":\"Activated\",\"timestamp\":\"2020-06-11T22:07:49.651Z\",\"tab\":19}";
	let c_time: DateTime<Utc> = "2020-06-11T22:07:54.925Z".parse().unwrap();
	let r_time: DateTime<Utc> = "2020-06-11T22:07:55.885Z".parse().unwrap();
	let u_time: DateTime<Utc> = "2020-06-11T22:07:49.692Z".parse().unwrap();
	let a_time: DateTime<Utc> = "2020-06-11T22:07:49.651Z".parse().unwrap();
	assert_eq!(parse_message(c_str.as_bytes()), Message { timestamp: c_time, kind: MessageKind::Created { tab: 20 } });
	assert_eq!(parse_message(r_str.as_bytes()), Message { timestamp: r_time, kind: MessageKind::Removed { tab: 20 } });
	assert_eq!(
		parse_message(u_str.as_bytes()),
		Message { timestamp: u_time, kind: MessageKind::Updated { tab: 19, url: "about:blank".to_owned() } }
	);
	assert_eq!(
		parse_message(a_str.as_bytes()),
		Message { timestamp: a_time, kind: MessageKind::Activated { tab: 19 } }
	);
}
