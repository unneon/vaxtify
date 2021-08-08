use crate::config::Config;
use crate::{config, filters};
use fixedbitset::FixedBitSet;
use regex::RegexSet;
use std::collections::HashMap;
use url::Url;

pub struct Lookups<'a> {
	pub config: &'a Config,
	// TODO: Precompute basic masks.
	pub domain: HashMap<&'a str, Vec<usize>>,
	pub subreddit: HashMap<&'a str, Vec<usize>>,
	pub github: HashMap<&'a str, Vec<usize>>,
	pub process: HashMap<&'a str, Vec<usize>>,
	pub category: Table<'a, &'a config::Category>,
	pub permit: Table<'a, &'a config::Permit>,
	pub regex_category: Vec<usize>,
	pub regex_set: RegexSet,
}

pub struct Table<'a, T> {
	pub id: HashMap<&'a str, usize>,
	pub name: Vec<&'a str>,
	pub details: Vec<T>,
}

impl<'a> Lookups<'a> {
	pub fn new(config: &'a Config) -> Self {
		let mut domain: HashMap<_, Vec<usize>> = HashMap::new();
		let mut subreddit: HashMap<_, Vec<usize>> = HashMap::new();
		let mut github: HashMap<_, Vec<usize>> = HashMap::new();
		let mut process: HashMap<_, Vec<usize>> = HashMap::new();
		let mut category = Table::new();
		let mut permit = Table::new();
		let mut regex_category = Vec::new();
		let mut regex_set_vec = Vec::new();
		for (cat_name, cat_details) in &config.category {
			let cat = category.insert(cat_name.as_str(), cat_details);
			for dom in &cat_details.domains {
				domain.entry(dom.as_str()).or_default().push(cat);
			}
			for sub in &cat_details.subreddits {
				subreddit.entry(sub.as_str()).or_default().push(cat);
			}
			for git in &cat_details.githubs {
				github.entry(git.as_str()).or_default().push(cat);
			}
			for reg in &cat_details.regexes {
				regex_category.push(cat);
				regex_set_vec.push(reg);
			}
			for proc in &cat_details.processes {
				process.entry(proc.as_str()).or_default().push(cat);
			}
		}
		for (per_name, per_details) in &config.permit {
			permit.insert(per_name.as_str(), per_details);
		}
		let regex_set = RegexSet::new(regex_set_vec).unwrap();
		Lookups { config, domain, subreddit, github, process, category, permit, regex_category, regex_set }
	}

	pub fn url_to_mask(&self, url: &Url) -> FixedBitSet {
		let mut mask = FixedBitSet::with_capacity(self.category.len());
		if let Some(domain) = url.domain() {
			if let Some(categories) = self.domain.get(domain) {
				mask.extend(categories.iter().copied());
			}
		}
		if let Some(subreddit) = filters::extract_subreddit(url) {
			if let Some(categories) = self.subreddit.get(subreddit.as_str()) {
				mask.extend(categories.iter().copied());
			}
		}
		if let Some(github) = filters::extract_github(url) {
			if let Some(categories) = self.github.get(github.as_str()) {
				mask.extend(categories.iter().copied());
			}
		}
		mask.extend(
			self.regex_set.matches(url.as_str()).into_iter().map(|regex_index| self.regex_category[regex_index]),
		);
		mask
	}

	// TODO: Do this more efficiently?
	pub fn process_to_mask(&self, process: &str) -> FixedBitSet {
		let mut mask = FixedBitSet::with_capacity(self.category.len());
		if let Some(process) = self.process.get(process) {
			mask.extend(process.iter().copied());
		}
		mask
	}
}

impl<'a, T> Table<'a, T> {
	fn new() -> Self {
		Table { id: HashMap::new(), name: Vec::new(), details: Vec::new() }
	}

	fn insert(&mut self, name: &'a str, details: T) -> usize {
		let index = self.id.len();
		self.id.insert(name, index);
		self.details.push(details);
		self.name.push(name);
		index
	}

	pub fn len(&self) -> usize {
		self.id.len()
	}
}
