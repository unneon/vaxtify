mod protocol;
pub mod proxy;
mod tabs;

use crate::activity::Activity;
use crate::config::Config;
use crate::event::Event;
use crate::ipc::Socket;
use crate::webext::protocol::ReadError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io;
use std::io::Write;
use std::net::TcpListener;
use tabs::Tabs;

pub struct WebExt<'a> {
	tabs: Tabs<'a>,
	buffer: VecDeque<Event>,
	listener: TcpListener,
	socket: Socket,
	protocol_state: protocol::ReadState,
}

#[derive(Debug, Deserialize, PartialEq)]
struct WebEvent {
	timestamp: DateTime<Utc>,
	#[serde(flatten)]
	kind: WebEventKind,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "kind")]
enum WebEventKind {
	Created { tab: i64 },
	Removed { tab: i64 },
	Updated { tab: i64, url: String },
	Activated { tab: i64 },
	BrowserLaunch,
	BrowserShutdown,
}

#[derive(Serialize)]
#[serde(tag = "kind")]
enum WebCommand {
	Close { tab: i64 },
}

const PORT: u16 = 7487;

impl WebExt<'_> {
	pub fn new(config: &Config) -> WebExt {
		WebExt {
			tabs: Tabs::new(config),
			buffer: VecDeque::new(),
			listener: TcpListener::bind(("localhost", PORT)).unwrap(),
			socket: Socket::Offline,
			protocol_state: protocol::ReadState::new(),
		}
	}

	pub fn next(&mut self) -> Option<Event> {
		self.fill_buffer();
		self.buffer.pop_front()
	}

	fn fill_buffer(&mut self) {
		while self.buffer.is_empty() {
			match self.read_web_event() {
				Some(web_event) => {
					let events = self.tabs.process_web_event(web_event);
					self.buffer.extend(events);
				}
				None => break,
			}
		}
	}

	fn read_web_event(&mut self) -> Option<WebEvent> {
		match self.socket {
			Socket::Active { .. } => {
				let message = match self.protocol_state.read(&mut self.socket) {
					Err(ReadError::Wait) | Err(ReadError::EOF) => return None,
					Err(ReadError::IO(e)) if e.kind() == io::ErrorKind::WouldBlock => return None,
					Err(ReadError::IO(e)) => Err(e).unwrap(),
					Ok(message) => message,
				};
				let web_event = deserialize_web_event(&message);
				Some(web_event)
			}
			Socket::Disconnected => {
				self.socket = Socket::Offline;
				self.protocol_state = protocol::ReadState::new();
				Some(WebEvent { timestamp: Utc::now(), kind: WebEventKind::BrowserShutdown })
			}
			Socket::Offline => {
				let stream = self.listener.accept().unwrap().0;
				stream.set_nonblocking(true).unwrap();
				self.socket = Socket::Active { stream };
				Some(WebEvent { timestamp: Utc::now(), kind: WebEventKind::BrowserLaunch })
			}
		}
	}

	pub fn close_all(&mut self, activities: &[Activity]) {
		for tab in self.tabs.filter_by_activities(activities) {
			let data = serialize_web_command(WebCommand::Close { tab });
			let _ = protocol::write(&data, &mut self.socket);
		}
		let _ = self.socket.flush();
	}
}

fn deserialize_web_event(raw: &[u8]) -> WebEvent {
	serde_json::from_slice(raw).unwrap()
}

fn serialize_web_command(web_command: WebCommand) -> Vec<u8> {
	serde_json::to_vec(&web_command).unwrap()
}

#[test]
fn parsing() {
	let c_str = "{\"kind\":\"Created\",\"timestamp\":\"2020-06-11T22:07:54.925Z\",\"tab\":20}";
	let r_str = "{\"kind\":\"Removed\",\"timestamp\":\"2020-06-11T22:07:55.885Z\",\"tab\":20}";
	let u_str = "{\"kind\":\"Updated\",\"timestamp\":\"2020-06-11T22:07:49.692Z\",\"tab\":19,\"url\":\"about:blank\"}";
	let a_str = "{\"kind\":\"Activated\",\"timestamp\":\"2020-06-11T22:07:49.651Z\",\"tab\":19}";
	let c_time: DateTime<Utc> = "2020-06-11T22:07:54.925Z".parse().unwrap();
	let r_time: DateTime<Utc> = "2020-06-11T22:07:55.885Z".parse().unwrap();
	let u_time: DateTime<Utc> = "2020-06-11T22:07:49.692Z".parse().unwrap();
	let a_time: DateTime<Utc> = "2020-06-11T22:07:49.651Z".parse().unwrap();
	assert_eq!(
		deserialize_web_event(c_str.as_bytes()),
		WebEvent { timestamp: c_time, kind: WebEventKind::Created { tab: 20 } }
	);
	assert_eq!(
		deserialize_web_event(r_str.as_bytes()),
		WebEvent { timestamp: r_time, kind: WebEventKind::Removed { tab: 20 } }
	);
	assert_eq!(
		deserialize_web_event(u_str.as_bytes()),
		WebEvent { timestamp: u_time, kind: WebEventKind::Updated { tab: 19, url: "about:blank".to_owned() } }
	);
	assert_eq!(
		deserialize_web_event(a_str.as_bytes()),
		WebEvent { timestamp: a_time, kind: WebEventKind::Activated { tab: 19 } }
	);
}
