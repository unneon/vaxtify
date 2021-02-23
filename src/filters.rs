use url::Url;

pub fn extract_subreddit(url: &Url) -> Option<String> {
	if url.domain()? != "www.reddit.com" {
		return None;
	}
	let path_segments = url.path_segments()?.collect::<Vec<_>>();
	match path_segments.as_slice() {
		["r", subreddit, ..] => Some(subreddit.to_lowercase()),
		_ => None,
	}
}

pub fn extract_github(url: &Url) -> Option<String> {
	if url.domain()? != "github.com" {
		return None;
	}
	let path_segments = url.path_segments()?.collect::<Vec<_>>();
	match path_segments.as_slice() {
		[user, repo, ..] => Some(format!("{}/{}", user, repo)),
		_ => None,
	}
}

#[test]
fn reddit_lowercase() {
	let url = "https://www.reddit.com/r/PrOgRaMmInG/".parse().unwrap();
	assert_eq!(extract_subreddit(&url).as_deref(), Some("programming"));
}

#[test]
fn github() {
	let url = "https://github.com/pustaczek/icie".parse().unwrap();
	assert_eq!(extract_github(&url).as_deref(), Some("pustaczek/icie"));
}
