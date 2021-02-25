use crate::config::Config;
use crate::{config, filters};
use fixedbitset::FixedBitSet;
use regex::RegexSet;
use std::collections::HashMap;
use url::Url;

pub struct Lookups<'a> {
	pub config: &'a Config,
	pub domain: HashMap<&'a str, Vec<usize>>,
	pub subreddit: HashMap<&'a str, Vec<usize>>,
	pub github: HashMap<&'a str, Vec<usize>>,
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
		}
		for (per_name, per_details) in &config.permit {
			permit.insert(per_name.as_str(), per_details);
		}
		let regex_set = RegexSet::new(regex_set_vec).unwrap();
		Lookups { config, domain, subreddit, github, category, permit, regex_category, regex_set }
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
