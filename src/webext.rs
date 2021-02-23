mod protocol;
pub mod proxy;

use crate::Event;
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::sync::mpsc;

pub struct WebExt {
	command_tx: mpsc::Sender<WebCommand>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "kind")]
enum WebEvent {
	Removed { tab: i64 },
	Updated { tab: i64, url: String },
}

#[derive(Serialize)]
#[serde(tag = "kind")]
enum WebCommand {
	Close { tab: i64 },
	CreateEmpty {},
}

const PORT: u16 = 7487;

impl WebExt {
	pub fn new(tx: mpsc::Sender<Event>) -> WebExt {
		let (socket_tx, socket_rx) = mpsc::channel();
		let (command_tx, command_rx) = mpsc::channel();
		let listener = TcpListener::bind(("localhost", PORT)).unwrap();
		std::thread::spawn(move || loop {
			let mut socket = listener.accept().unwrap().0;
			socket_tx.send(socket.try_clone().unwrap()).unwrap();
			loop {
				let buffer = match protocol::read(&mut socket) {
					Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
					buffer => buffer.unwrap(),
				};
				let web_event = deserialize_web_event(&buffer);
				let event = match web_event {
					WebEvent::Removed { tab } => Event::TabDelete { tab },
					WebEvent::Updated { tab, url } => {
						let url = url.parse().unwrap();
						Event::TabUpdate { tab, url }
					}
				};
				tx.send(event).unwrap();
			}
			socket.shutdown(std::net::Shutdown::Both).unwrap();
			tx.send(Event::TabDeleteAll).unwrap();
		});
		std::thread::spawn(move || {
			let mut socket = socket_rx.recv().unwrap();
			loop {
				let command = command_rx.recv().unwrap();
				let data = serialize_web_command(command);
				loop {
					match protocol::write(&data, &mut socket) {
						Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => socket = socket_rx.recv().unwrap(),
						r => break r.unwrap(),
					}
				}
			}
		});
		WebExt { command_tx }
	}

	pub fn close_tab(&self, tab: i64) {
		self.command_tx.send(WebCommand::Close { tab }).unwrap();
	}

	pub fn create_empty_tab(&self) {
		self.command_tx.send(WebCommand::CreateEmpty {}).unwrap();
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
	let r_str = "{\"kind\":\"Removed\",\"tab\":20}";
	let u_str = "{\"kind\":\"Updated\",\"tab\":19,\"url\":\"about:blank\"}";
	assert_eq!(deserialize_web_event(r_str.as_bytes()), WebEvent::Removed { tab: 20 });
	assert_eq!(deserialize_web_event(u_str.as_bytes()), WebEvent::Updated { tab: 19, url: "about:blank".to_owned() });
}
