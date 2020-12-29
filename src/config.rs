mod serde_duration;
mod serde_time;

use crate::activity::Activity;
use chrono::NaiveTime;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct Category {
	#[serde(default)]
	domains: Vec<String>,
	#[serde(default)]
	subreddits: Vec<String>,
	#[serde(default)]
	github: Vec<String>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct Hour {
	hour: u8,
	#[serde(default)]
	minute: u8,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub enum Limit {
	#[serde(rename = "individual")]
	Individual(#[serde(with = "serde_duration")] Duration),
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum Enforce {
	#[serde(rename = "close")]
	Close,
}

#[derive(Debug, Deserialize)]
pub struct Rule {
	pub allowed: Limit,
	pub categories: Vec<String>,
	pub enforce: Enforce,
}

#[derive(Debug, Deserialize)]
pub struct General {
	#[serde(default = "default_reddit")]
	pub reddit: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct Config {
	#[serde(default)]
	pub general: General,
	pub category: HashMap<String, Category>,
	pub rules: Vec<Rule>,
}

impl Category {
	pub fn all_activities(&self) -> Vec<Activity> {
		let domains = self.domains.iter().cloned().map(|domain| Activity::Internet { domain });
		let subreddits = self.subreddits.iter().cloned().map(|subreddit| Activity::Reddit { subreddit });
		let github = self.github.iter().cloned().map(|repo| Activity::Github { repo });
		domains.chain(subreddits).chain(github).collect()
	}
}

impl Rule {
	pub fn all_activities(&self, config: &Config) -> Vec<Activity> {
		self.categories.iter().flat_map(|category| config.category[category].all_activities()).collect()
	}
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

impl Default for General {
	fn default() -> Self {
		General { reddit: default_reddit() }
	}
}

fn default_reddit() -> bool {
	true
}

#[test]
fn example() {
	let text = r#"
[category.example]
domains = ["example.com", "example.org"]
subreddits = ["all"]
github = ["pustaczek/icie"]

[[rules]]
allowed.individual.minutes = 4
categories = ["example"]
enforce.close = {}
"#;
	let config = Config::parse(text);
	assert_eq!(config.general.reddit, true);
	assert_eq!(config.category.len(), 1);
	assert_eq!(config.category["example"].domains, ["example.com", "example.org"]);
	assert_eq!(config.category["example"].subreddits, ["all"]);
	assert_eq!(config.category["example"].github, ["pustaczek/icie"]);
	assert_eq!(config.rules.len(), 1);
	assert_eq!(config.rules[0].allowed, Limit::Individual(Duration::from_secs(240)));
	assert_eq!(config.rules[0].categories, ["example"]);
	assert_eq!(config.rules[0].enforce, Enforce::Close);
}
