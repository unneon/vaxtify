mod serde_duration;

use crate::activity::Activity;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct Category {
	#[serde(default)]
	domains: Vec<String>,
	#[serde(default)]
	subreddits: Vec<String>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub enum Limit {
	#[serde(rename = "individual")]
	Individual(#[serde(with = "serde_duration")] Duration),
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub enum Enforce {
	#[serde(rename = "stepwise")]
	Stepwise {
		#[serde(with = "serde_duration")]
		delay: Duration,
	},
}

#[derive(Debug, Deserialize)]
pub struct Rule {
	pub allowed: Limit,
	pub categories: Vec<String>,
	pub enforce: Enforce,
}

#[derive(Debug, Deserialize)]
pub struct Config {
	pub category: HashMap<String, Category>,
	pub rules: Vec<Rule>,
}

impl Category {
	pub fn all_activities(&self) -> Vec<Activity> {
		let domains = self.domains.iter().cloned().map(|domain| Activity::Internet { domain });
		let subreddits = self.subreddits.iter().cloned().map(|subreddit| Activity::Reddit { subreddit });
		domains.chain(subreddits).collect()
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
		let file = std::fs::read(path).unwrap();
		Config::parse(&file)
	}

	fn parse(file: &[u8]) -> Config {
		toml::from_slice(&file).unwrap()
	}
}

#[test]
fn example() {
	let text = r#"
[category.example]
domains = ["example.com", "example.org"]
subreddits = ["all"]

[[rules]]
allowed.individual.minutes = 4
categories = ["example"]
enforce.stepwise.delay.seconds = 1
"#;
	let config = Config::parse(text.as_bytes());
	assert_eq!(config.category.len(), 1);
	assert_eq!(config.category["example"].domains, ["example.com", "example.org"]);
	assert_eq!(config.category["example"].subreddits, ["all"]);
	assert_eq!(config.rules.len(), 1);
	assert_eq!(config.rules[0].allowed, Limit::Individual(Duration::from_secs(240)));
	assert_eq!(config.rules[0].categories, ["example"]);
	assert_eq!(config.rules[0].enforce, Enforce::Stepwise { delay: Duration::from_secs(1) });
}
