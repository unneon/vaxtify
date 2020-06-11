use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::io;
use std::io::Read;
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
			if let Ok(raw) = read_message(connection) {
				break serde_json::from_slice(&raw).unwrap();
			} else {
				self.connection = None;
			}
		}
	}
}

fn read_message(mut input: impl Read) -> Result<Vec<u8>, io::Error> {
	let mut header = [0; 4];
	input.read_exact(&mut header)?;
	let len = u32::from_le_bytes(header) as usize;
	let mut buf = vec![0; len];
	input.read_exact(&mut buf)?;
	Ok(buf)
}
