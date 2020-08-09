use chrono::{DateTime, Utc};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Interval {
	pub since: DateTime<Utc>,
	pub until: DateTime<Utc>,
}

impl Interval {
	pub fn duration(&self) -> Duration {
		(self.until - self.since).to_std().unwrap()
	}

	pub fn merge(mut intervals: Vec<Interval>) -> Vec<Interval> {
		intervals.sort_by_key(|interval| interval.since);
		let mut merged = Vec::<Interval>::new();
		for interval in intervals {
			match merged.last_mut() {
				Some(last) if last.until >= interval.since => last.until = last.until.max(interval.until),
				_ => merged.push(interval),
			}
		}
		merged
	}
}

#[cfg(test)]
fn test_merge(input: &[(i64, i64)], output: &[(i64, i64)]) {
	let now = Utc::now();
	let make_time = |a| now + chrono::Duration::seconds(a);
	let make_interval = |a, b| Interval { since: make_time(a), until: make_time(b) };
	let make_intervals = |a: &[(i64, i64)]| a.iter().map(|(a, b)| make_interval(*a, *b)).collect::<Vec<_>>();
	let input = make_intervals(input);
	let output = make_intervals(output);
	assert_eq!(Interval::merge(input), output);
}

#[test]
fn merge_empty() {
	test_merge(&[], &[]);
}

#[test]
fn merge_nonoverlapping() {
	test_merge(&[(0, 1)], &[(0, 1)]);
	test_merge(&[(0, 1), (2, 3)], &[(0, 1), (2, 3)]);
	test_merge(&[(2, 3), (0, 1)], &[(0, 1), (2, 3)]);
	test_merge(&[(0, 1), (2, 3), (4, 6)], &[(0, 1), (2, 3), (4, 6)]);
}

#[test]
fn merge_overlapping() {
	test_merge(&[(0, 1), (1, 2)], &[(0, 2)]);
	test_merge(&[(0, 4), (1, 6), (6, 7)], &[(0, 7)]);
}
