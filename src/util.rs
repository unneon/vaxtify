#[cfg(test)]
pub fn example_time(seconds: u32) -> chrono::DateTime<chrono::Utc> {
	use chrono::{TimeZone, Utc};
	Utc.ymd(2020, 1, 1).and_hms(0, 0, seconds)
}
