use chrono::Duration;
use serde::de::Visitor;
use serde::Deserializer;
use std::fmt;

struct TimeVisitor {
	unit: Duration,
	name: &'static str,
}

impl Visitor<'_> for TimeVisitor {
	type Value = Duration;

	fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "an u32 number of {}", self.name)
	}

	fn visit_i64<E: std::error::Error>(self, v: i64) -> Result<Self::Value, E> {
		Ok(self.unit * v as i32)
	}
}

pub fn minutes<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
	generic(d, Duration::minutes(1), "minutes")
}

pub fn hours<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
	generic(d, Duration::hours(1), "hours")
}

fn generic<'de, D: Deserializer<'de>>(d: D, unit: Duration, name: &'static str) -> Result<Duration, D::Error> {
	d.deserialize_u32(TimeVisitor { unit, name })
}
