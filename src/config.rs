mod kdl_duration;
mod kdl_time;

use chrono::{DateTime, Local, NaiveTime};
use knuffel::Decode;
use std::collections::HashSet;
use std::hash::Hash;
#[cfg(test)]
use std::time::Duration;

#[derive(Debug, Decode)]
pub struct Config {
	#[knuffel(child)]
	pub prevent_browser_close: bool,
	#[knuffel(child)]
	pub close_all_on_block: bool,
	#[knuffel(child)]
	pub close_all_after_block: Option<kdl_duration::Duration>,
	#[knuffel(child)]
	pub reload_delay: Option<kdl_duration::Duration>,
	#[knuffel(child, default = crate::processes::DEFAULT_SCAN_EACH.into())]
	pub processes_scan_each: kdl_duration::Duration,
	#[knuffel(children(name = "category"))]
	pub categories: Vec<Category>,
	#[knuffel(children(name = "rule"))]
	pub rules: Vec<Rule>,
	#[knuffel(children(name = "permit"))]
	pub permits: Vec<Permit>,
}

#[derive(Debug, Decode)]
pub struct Category {
	#[knuffel(argument)]
	pub name: String,
	#[knuffel(child, unwrap(arguments))]
	pub domains: Option<Vec<String>>,
	#[knuffel(child, unwrap(arguments))]
	pub subreddits: Option<Vec<String>>,
	#[knuffel(child, unwrap(arguments))]
	pub githubs: Option<Vec<String>>,
	#[knuffel(child, unwrap(arguments))]
	pub regexes: Option<Vec<String>>,
	#[knuffel(child, unwrap(arguments))]
	pub processes: Option<Vec<String>>,
}

#[derive(Clone, Copy, Debug, Decode, Eq, PartialEq)]
pub struct TimeRange {
	#[knuffel(child)]
	pub since: kdl_time::NaiveTime,
	#[knuffel(child)]
	pub until: kdl_time::NaiveTime,
}

#[derive(Debug, Decode)]
pub struct Rule {
	#[knuffel(argument)]
	pub name: String,
	#[knuffel(child)]
	pub allowed: Option<TimeRange>,
	#[knuffel(child, unwrap(arguments))]
	pub categories: Vec<String>,
}

#[derive(Debug, Decode)]
pub struct Permit {
	#[knuffel(argument)]
	pub name: String,
	#[knuffel(child)]
	pub length: kdl_duration::Duration,
	#[knuffel(child)]
	pub cooldown: Option<kdl_duration::Duration>,
	#[knuffel(child)]
	pub available: Option<TimeRange>,
	#[knuffel(child, unwrap(arguments))]
	pub categories: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
	#[error("config parse error")]
	ParseError(#[from] knuffel::Error),
	#[error("config validation failed ({details})")]
	ValidationFailure { details: &'static str },
}

const CONFIG_FILE_NAME: &str = "vaxtify.kdl";

impl Config {
	pub fn load() -> Result<Config, ConfigError> {
		let path = dirs::config_dir().unwrap().join(CONFIG_FILE_NAME);
		let file = std::fs::read_to_string(path).unwrap();
		Config::parse(&file)
	}

	pub fn parse(file: &str) -> Result<Config, ConfigError> {
		let config: Config = knuffel::parse(CONFIG_FILE_NAME, file)?;
		if config.prevent_browser_close && config.close_all_after_block.is_some() {
			return Err(ConfigError::ValidationFailure {
				details: "prevent-browser-close and close-all-after-block can't both be set",
			});
		}
		if config.close_all_on_block && config.close_all_after_block.is_some() {
			return Err(ConfigError::ValidationFailure {
				details: "close-all-on-block and close-all-after-block can't both be set",
			});
		}
		check_unique_names(&config.categories, |c| &c.name)?;
		check_unique_names(&config.rules, |r| &r.name)?;
		check_unique_names(&config.permits, |p| &p.name)?;
		Ok(config)
	}
}

impl TimeRange {
	fn contains(&self, now: &DateTime<Local>) -> bool {
		let since: NaiveTime = self.since.into();
		let until: NaiveTime = self.until.into();
		let time = now.naive_local().time();
		if since <= until {
			time >= since && time < until
		} else {
			time >= since || time < until
		}
	}
}

impl Rule {
	pub fn is_active(&self, now: &DateTime<Local>) -> bool {
		self.allowed.map_or(true, |allowed| !allowed.contains(now))
	}

	pub fn next_change_time(&self, now: &DateTime<Local>) -> Option<DateTime<Local>> {
		let TimeRange { since, until } = self.allowed?;
		let next_start = upper_bound_with_time(now, &since.into());
		let next_end = upper_bound_with_time(now, &until.into());
		Some(next_start.min(next_end))
	}
}

impl Permit {
	pub fn is_available(&self, now: &DateTime<Local>) -> bool {
		self.available.map_or(true, |available| available.contains(now))
	}
}

fn upper_bound_with_time(greater_than: &DateTime<Local>, set_time: &NaiveTime) -> DateTime<Local> {
	let mut candidate = greater_than.date();
	while candidate.and_time(*set_time).unwrap() <= *greater_than {
		candidate = candidate.succ();
	}
	candidate.and_time(*set_time).unwrap()
}

fn check_unique_names<T, U: Eq + Hash>(blocks: &[T], name: impl Fn(&T) -> &U) -> Result<(), ConfigError> {
	let set: HashSet<&U> = blocks.iter().map(name).collect();
	if set.len() != blocks.len() {
		return Err(ConfigError::ValidationFailure { details: "blocks of the same type can't have identical names" });
	}
	Ok(())
}

#[test]
fn example() {
	let text = r#"
prevent-browser-close

category "example" {
	domains "example.com"
	subreddits "all"
	githubs "unneon/icie"
	regexes r"example\.org"
	processes "chrome"
}

category "other" {
	githubs "unneon/vaxtify"
}

rule "things" {
	allowed {
		since hour=23 min=30
		until hour=0
	}
	categories "example"
}

rule "never" {
	categories "other"
}

permit "example" {
	length mins=30
	cooldown hours=20
	available {
		since hour=20
		until hour=0
	}
	categories "other"
}
"#;
	let config = Config::parse(text).unwrap();
	assert_eq!(config.prevent_browser_close, true);
	assert_eq!(Duration::from(config.processes_scan_each), Duration::from_secs(10));
	assert_eq!(config.categories.len(), 2);
	assert_eq!(config.categories[0].name, "example");
	assert_eq!(config.categories[0].domains, Some(vec!["example.com".to_string()]));
	assert_eq!(config.categories[0].subreddits, Some(vec!["all".to_string()]));
	assert_eq!(config.categories[0].githubs, Some(vec!["unneon/icie".to_string()]));
	assert_eq!(config.categories[0].regexes, Some(vec!["example\\.org".to_string()]));
	assert_eq!(config.categories[0].processes, Some(vec!["chrome".to_string()]));
	assert_eq!(config.categories[1].name, "other");
	assert_eq!(config.categories[1].githubs, Some(vec!["unneon/vaxtify".to_string()]));
	assert_eq!(config.rules.len(), 2);
	assert_eq!(config.rules[0].name, "things");
	assert_eq!(config.rules[0].allowed.map(|r| NaiveTime::from(r.since)), Some(NaiveTime::from_hms(23, 30, 0)));
	assert_eq!(config.rules[0].allowed.map(|r| NaiveTime::from(r.until)), Some(NaiveTime::from_hms(0, 0, 0)));
	assert_eq!(config.rules[0].categories, ["example"]);
	assert_eq!(config.rules[1].name, "never");
	assert_eq!(config.rules[1].allowed, None);
	assert_eq!(config.rules[1].categories, ["other"]);
	assert_eq!(config.permits.len(), 1);
	assert_eq!(config.permits[0].name, "example");
	assert_eq!(Duration::from(config.permits[0].length), Duration::from_secs(30 * 60));
	assert_eq!(config.permits[0].cooldown.map(Duration::from), Some(Duration::from_secs(20 * 60 * 60)));
	assert_eq!(config.permits[0].available.map(|r| NaiveTime::from(r.since)), Some(NaiveTime::from_hms(20, 0, 0)));
	assert_eq!(config.permits[0].available.map(|r| NaiveTime::from(r.until)), Some(NaiveTime::from_hms(0, 0, 0)));
	assert_eq!(config.permits[0].categories, ["other"]);
}

#[test]
fn duplicate_categories() {
	let text = r#"
category "example" {
	domains "example.com"
}

category "example" {
	domains "example.org"
}
"#;
	assert_duplicate_error(text);
}

#[test]
fn duplicate_rules() {
	let text = r#"
category "example" {
	domains "example.org"
}

rule "never" {
	categories "example"
}

rule "never" {
	categories "example"
}
"#;
	assert_duplicate_error(text);
}

#[test]
fn duplicate_permits() {
	let text = r#"
category "example" {
	domains "example.com"
}

permit "sometimes" {
	length mins=30
	categories "example"
}

permit "sometimes" {
	length mins=60
	categories "example"
}
"#;
	assert_duplicate_error(text);
}

#[cfg(test)]
fn assert_duplicate_error(text: &str) {
	let result = Config::parse(text);
	if let Err(e) = &result {
		if let ConfigError::ValidationFailure { details } = e {
			assert_eq!(*details, "blocks of the same type can't have identical names");
			return;
		}
	}
	panic!("{:?}", result);
}
