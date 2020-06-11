use chrono::{DateTime, Utc};
use std::net::{TcpListener, TcpStream};

pub struct Connection {
	listener: TcpListener,
	connection: Option<TcpStream>,
}

#[derive(Debug, PartialEq)]
pub struct Message {
	tab: i64,
	timestamp: DateTime<Utc>,
	event: Event,
}

#[derive(Debug, PartialEq)]
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
				break parse_message(&raw);
			} else {
				self.connection = None;
			}
		}
	}
}

fn parse_message(raw: &[u8]) -> Message {
	let string = std::str::from_utf8(&raw).unwrap();
	let value = json::parse(string).unwrap();
	let tab = value["tab"].as_i64().unwrap();
	let timestamp = value["timestamp"].as_str().unwrap().parse().unwrap();
	let kind = value["kind"].as_str().unwrap();
	let event = match kind {
		"Created" => Event::Created,
		"Removed" => Event::Removed,
		"Updated" => Event::Updated { url: value["url"].as_str().unwrap().to_owned() },
		"Activated" => Event::Activated,
		_ => unreachable!(),
	};
	Message { tab, timestamp, event }
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
	assert_eq!(parse_message(c_str.as_bytes()), Message { tab: 20, timestamp: c_time, event: Event::Created });
	assert_eq!(parse_message(r_str.as_bytes()), Message { tab: 20, timestamp: r_time, event: Event::Removed });
	assert_eq!(
		parse_message(u_str.as_bytes()),
		Message { tab: 19, timestamp: u_time, event: Event::Updated { url: "about:blank".to_owned() } }
	);
	assert_eq!(parse_message(a_str.as_bytes()), Message { tab: 19, timestamp: a_time, event: Event::Activated });
}
