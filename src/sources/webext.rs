mod protocol;
pub mod proxy;
mod socket;
mod tabs;

use crate::sources::webext::socket::Socket;
use crate::sources::webext::tabs::Tabs;
use crate::Event;
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub struct WebExt {
	tabs: Tabs,
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
		WebExt { tabs: Tabs::new(), buffer: VecDeque::new(), socket }
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
		let events = self.tabs.process_message(message);
		self.buffer.extend(events);
		Some(())
	}
}
