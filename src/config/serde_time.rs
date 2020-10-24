use chrono::NaiveTime;
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
struct Format {
	hour: u32,
	#[serde(default)]
	min: u32,
}

impl From<Format> for NaiveTime {
	fn from(f: Format) -> Self {
		NaiveTime::from_hms(f.hour, f.min, 0)
	}
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<NaiveTime, D::Error> {
	Ok(NaiveTime::from(Format::deserialize(deserializer)?))
}
