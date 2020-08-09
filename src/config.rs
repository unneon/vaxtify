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

#[derive(Debug, Deserialize)]
pub enum Limit {
	#[serde(rename = "individual")]
	Individual(#[serde(with = "serde_duration")] Duration),
}

#[derive(Debug, Deserialize)]
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
		toml::from_slice(&file).unwrap()
	}
}
