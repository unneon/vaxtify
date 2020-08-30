use url::Url;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Activity {
	Internet { domain: String },
	Reddit { subreddit: String },
}

impl Activity {
	pub fn from_url(url: &Url) -> Option<Activity> {
		reddit_from_url(url).or_else(|| internet_from_url(url))
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

fn internet_from_url(url: &Url) -> Option<Activity> {
	Some(Activity::Internet { domain: url.domain()?.to_owned() })
}

#[test]
fn reddit_uppercase() {
	let url = "https://www.reddit.com/r/PrOgRaMmInG/".parse().unwrap();
	assert_eq!(Activity::from_url(&url), Some(Activity::Reddit { subreddit: "programming".to_owned() }));
}
