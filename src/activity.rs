#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Activity {
	Internet { domain: String },
	Reddit { subreddit: String },
}
