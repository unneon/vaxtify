use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum Event {
	Removed { tab: i64 },
	Updated { tab: i64, url: String },
	Handshake { version: String },
}

#[derive(Serialize)]
#[serde(tag = "kind")]
pub enum Command {
	Close { tab: i64 },
	CreateEmpty {},
}

pub fn deserialize_event(raw: &[u8]) -> Event {
	serde_json::from_slice(raw).unwrap()
}

pub fn serialize_command(command: Command) -> Vec<u8> {
	serde_json::to_vec(&command).unwrap()
}

#[test]
fn parsing() {
	let r_str = "{\"kind\":\"Removed\",\"tab\":20}";
	let u_str = "{\"kind\":\"Updated\",\"tab\":19,\"url\":\"about:blank\"}";
	assert_eq!(deserialize_event(r_str.as_bytes()), Event::Removed { tab: 20 });
	assert_eq!(deserialize_event(u_str.as_bytes()), Event::Updated { tab: 19, url: "about:blank".to_owned() });
}
