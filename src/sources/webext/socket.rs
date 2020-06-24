use crate::sources::webext::protocol;
use crate::sources::webext::{Message, MessageKind};
use chrono::Utc;
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::Duration;

pub struct Socket {
	rx: mpsc::Receiver<Message>,
}

impl Socket {
	pub fn new(port: u16) -> Socket {
		let listener = TcpListener::bind(("localhost", port)).unwrap();
		let (tx, rx) = mpsc::sync_channel(0);
		std::thread::spawn(move || receive_loop(listener, tx));
		Socket { rx }
	}

	pub(super) fn recv_timeout(&mut self, timeout: Duration) -> Option<Message> {
		self.rx.recv_timeout(timeout).ok()
	}
}

fn receive_loop(listener: TcpListener, tx: mpsc::SyncSender<Message>) {
	let mut connection_slot = None;
	loop {
		let message = match &mut connection_slot {
			Some(connection) => match protocol::read(connection) {
				Ok(raw) => protocol(&raw),
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

fn protocol(raw: &[u8]) -> Message {
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
	assert_eq!(protocol(c_str.as_bytes()), Message { timestamp: c_time, kind: MessageKind::Created { tab: 20 } });
	assert_eq!(protocol(r_str.as_bytes()), Message { timestamp: r_time, kind: MessageKind::Removed { tab: 20 } });
	assert_eq!(
		protocol(u_str.as_bytes()),
		Message { timestamp: u_time, kind: MessageKind::Updated { tab: 19, url: "about:blank".to_owned() } }
	);
	assert_eq!(protocol(a_str.as_bytes()), Message { timestamp: a_time, kind: MessageKind::Activated { tab: 19 } });
}
