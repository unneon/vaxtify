mod serde_time;

use chrono::Duration;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
	pub sources: Sources,
	pub rules: Vec<Rule>,
}

#[derive(Debug, Deserialize)]
pub struct Sources {
	pub webext: SourceWebext,
}

#[derive(Debug, Deserialize)]
pub struct SourceWebext {
	pub port: u16,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Rule {
	#[serde(deserialize_with = "serde_time::minutes")]
	pub allowed_minutes: Duration,
	#[serde(deserialize_with = "serde_time::hours")]
	pub cooldown_hours: Duration,
	pub domains: Vec<String>,
}

impl Config {
	pub fn load() -> Config {
		let path = dirs::config_dir().unwrap().join("distraction-oni.toml");
		let file = std::fs::read(path).unwrap();
		toml::from_slice(&file).unwrap()
	}
}
