use serde::{Deserialize, Deserializer};
use std::time::Duration;

#[derive(Deserialize)]
enum Unit {
	#[serde(rename = "days")]
	Days(u32),
	#[serde(rename = "hours")]
	Hours(u32),
	#[serde(rename = "minutes")]
	Minutes(u32),
	#[serde(rename = "seconds")]
	Seconds(u32),
}

const DAY: Duration = Duration::from_secs(60 * 60 * 24);
const HOUR: Duration = Duration::from_secs(60 * 60);
const MINUTE: Duration = Duration::from_secs(60);
const SECOND: Duration = Duration::from_secs(1);

impl From<Unit> for Duration {
	fn from(unit: Unit) -> Self {
		match unit {
			Unit::Days(days) => days * DAY,
			Unit::Hours(hours) => hours * HOUR,
			Unit::Minutes(minutes) => minutes * MINUTE,
			Unit::Seconds(seconds) => seconds * SECOND,
		}
	}
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
	Unit::deserialize(deserializer).map(Duration::from)
}
