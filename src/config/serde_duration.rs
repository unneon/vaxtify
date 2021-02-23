use serde::{Deserialize, Deserializer};
use std::fmt;
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

struct DurationOptionVisitor;

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

impl<'de> serde::de::Visitor<'de> for DurationOptionVisitor {
	type Value = Option<Duration>;

	fn expecting<'a>(&self, formatter: &mut fmt::Formatter<'a>) -> fmt::Result {
		write!(formatter, "an optional duration")
	}

	fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
		Ok(None)
	}

	fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, <D as Deserializer<'de>>::Error> {
		Ok(Some(deserialize(deserializer)?))
	}
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
	Unit::deserialize(deserializer).map(Duration::from)
}

pub fn deserialize_option<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Duration>, D::Error> {
	deserializer.deserialize_option(DurationOptionVisitor)
}
