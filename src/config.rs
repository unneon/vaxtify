mod serde_duration;
mod serde_time;

use chrono::{DateTime, Local, NaiveTime};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct Category {
	#[serde(default)]
	pub domains: Vec<String>,
	#[serde(default)]
	pub subreddits: Vec<String>,
	#[serde(default)]
	pub githubs: Vec<String>,
	#[serde(default)]
	pub regexes: Vec<String>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub enum Limit {
	#[serde(rename = "during")]
	During {
		#[serde(with = "serde_time")]
		since: NaiveTime,
		#[serde(with = "serde_time")]
		until: NaiveTime,
	},
	#[serde(rename = "never")]
	Never,
}

#[derive(Debug, Deserialize)]
pub struct Rule {
	pub allowed: Limit,
	pub categories: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct PermitLength {
	#[serde(default, deserialize_with = "serde_duration::deserialize_option")]
	pub default: Option<Duration>,
	#[serde(default, deserialize_with = "serde_duration::deserialize_option")]
	pub maximum: Option<Duration>,
}

#[derive(Debug, Deserialize)]
pub struct Permit {
	#[serde(default)]
	pub length: PermitLength,
	#[serde(default, deserialize_with = "serde_duration::deserialize_option")]
	pub cooldown: Option<Duration>,
	pub categories: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Config {
	pub category: HashMap<String, Category>,
	pub rule: Vec<Rule>,
	#[serde(default)]
	pub permit: HashMap<String, Permit>,
}

impl Config {
	pub fn load() -> Config {
		let path = dirs::config_dir().unwrap().join("vaxtify.toml");
		let file = std::fs::read_to_string(path).unwrap();
		Config::parse(&file)
	}

	pub fn parse(file: &str) -> Config {
		toml::from_str(&file).unwrap()
	}
}

impl Rule {
	pub fn is_active(&self, now: &DateTime<Local>) -> bool {
		match self.allowed {
			Limit::During { since, until } => {
				let time = now.naive_local().time();
				if since <= until {
					time < since || time > until
				} else {
					time > until && time < since
				}
			}
			Limit::Never => true,
		}
	}

	pub fn next_change_time(&self, now: &DateTime<Local>) -> Option<DateTime<Local>> {
		let (since, until) = match self.allowed {
			Limit::During { since, until } => (since, until),
			Limit::Never => return None,
		};
		let next_start = upper_bound_with_time(now, &since);
		let next_end = upper_bound_with_time(now, &until);
		Some(next_start.min(next_end))
	}
}

fn upper_bound_with_time(greater_than: &DateTime<Local>, set_time: &NaiveTime) -> DateTime<Local> {
	let mut candidate = greater_than.date();
	while candidate.and_time(*set_time).unwrap() <= *greater_than {
		candidate = candidate.succ();
	}
	candidate.and_time(*set_time).unwrap()
}

#[test]
fn example() {
	let text = r#"
[category.example]
domains = ["example.com"]
subreddits = ["all"]
githubs = ["pustaczek/icie"]
regexes = ["example\\.org"]

[[rule]]
allowed.during.since = { hour = 23, min = 0 }
allowed.during.until = { hour = 0, min = 0 }
categories = ["example"]

[permit.example]
length.default.minutes = 30
length.maximum.minutes = 40
cooldown.hours = 20
categories = ["example"]
"#;
	let config = Config::parse(text);
	assert_eq!(config.category.len(), 1);
	assert_eq!(config.category["example"].domains, ["example.com"]);
	assert_eq!(config.category["example"].subreddits, ["all"]);
	assert_eq!(config.category["example"].githubs, ["pustaczek/icie"]);
	assert_eq!(config.category["example"].regexes, ["example\\.org"]);
	assert_eq!(config.rule.len(), 1);
	assert_eq!(
		config.rule[0].allowed,
		Limit::During { since: NaiveTime::from_hms(23, 0, 0), until: NaiveTime::from_hms(0, 0, 0) }
	);
	assert_eq!(config.rule[0].categories, ["example"]);
	assert_eq!(config.permit.len(), 1);
	assert_eq!(config.permit["example"].length.default, Some(Duration::from_secs(30 * 60)));
	assert_eq!(config.permit["example"].length.maximum, Some(Duration::from_secs(40 * 60)));
	assert_eq!(config.permit["example"].cooldown, Some(Duration::from_secs(20 * 60 * 60)));
	assert_eq!(config.permit["example"].categories, ["example"]);
}
