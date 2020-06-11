use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::net::{TcpListener, TcpStream};

pub struct Connection {
	listener: TcpListener,
	connection: Option<TcpStream>,
}

#[derive(Debug, Deserialize)]
pub struct Message {
	tab: i64,
	timestamp: DateTime<Utc>,
	#[serde(flatten)]
	event: Event,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
pub enum Event {
	Created,
	Removed,
	Updated { url: String },
	Activated,
}

impl Connection {
	pub fn new() -> Connection {
		let listener = TcpListener::bind("localhost:56154").unwrap();
		Connection { listener, connection: None }
	}

	pub fn read(&mut self) -> Message {
		loop {
			let listener = &mut self.listener;
			let connection = self.connection.get_or_insert_with(|| listener.accept().unwrap().0);
			if let Ok(raw) = webext::read(connection) {
				break serde_json::from_slice(&raw).unwrap();
			} else {
				self.connection = None;
			}
		}
	}
}
