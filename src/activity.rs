use crate::config::Config;
use url::Url;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Activity {
	Github { repo: String },
	Internet { domain: String },
	Reddit { subreddit: String },
}

impl Activity {
	pub fn from_url(url: &Url, config: &Config) -> Option<Activity> {
		if config.general.reddit {
			reddit_from_url(url).or_else(|| github_from_url(url)).or_else(|| internet_from_url(url))
		} else {
			github_from_url(url).or_else(|| internet_from_url(url))
		}
	}

	#[cfg(test)]
	pub fn example(name: &str) -> Activity {
		Activity::Internet { domain: name.to_owned() }
	}
}

fn reddit_from_url(url: &Url) -> Option<Activity> {
	if url.domain()? != "www.reddit.com" {
		return None;
	}
	let path_segments = url.path_segments()?.collect::<Vec<_>>();
	if path_segments.len() < 2 || path_segments[0] != "r" {
		return None;
	}
	Some(Activity::Reddit { subreddit: path_segments[1].to_lowercase() })
}

fn github_from_url(url: &Url) -> Option<Activity> {
	if url.domain()? != "github.com" {
		return None;
	}
	let path_segments = url.path_segments()?.collect::<Vec<_>>();
	if path_segments.len() < 2 {
		return None;
	}
	let repo = format!("{}/{}", path_segments[0], path_segments[1]);
	Some(Activity::Github { repo })
}

fn internet_from_url(url: &Url) -> Option<Activity> {
	Some(Activity::Internet { domain: url.domain()?.to_owned() })
}

#[test]
fn reddit_uppercase() {
	let config = Config::default();
	let url = "https://www.reddit.com/r/PrOgRaMmInG/".parse().unwrap();
	assert_eq!(Activity::from_url(&url, &config), Some(Activity::Reddit { subreddit: "programming".to_owned() }));
}

#[test]
fn reddit_ignore() {
	use crate::config::General;
	let config = Config { general: General { reddit: false }, ..Config::default() };
	let url = "https://www.reddit.com/r/PrOgRaMmInG/".parse().unwrap();
	assert_eq!(Activity::from_url(&url, &config), Some(Activity::Internet { domain: "www.reddit.com".to_owned() }));
}

#[test]
fn github() {
	let config = Config::default();
	let url = "https://github.com/pustaczek/icie".parse().unwrap();
	assert_eq!(Activity::from_url(&url, &config), Some(Activity::Github { repo: "pustaczek/icie".to_owned() }));
}
