use chrono::{DateTime, Utc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Interval {
	pub since: DateTime<Utc>,
	pub until: DateTime<Utc>,
}

impl Interval {
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

	#[cfg(test)]
	pub fn example(since: u32, until: u32) -> Interval {
		use crate::util::example_time;
		let since = example_time(since);
		let until = example_time(until);
		Interval { since, until }
	}
}

#[cfg(test)]
fn test_merge(input: &[(u32, u32)], output: &[(u32, u32)]) {
	let input = input.iter().map(|(a, b)| Interval::example(*a, *b)).collect();
	let output: Vec<_> = output.iter().map(|(a, b)| Interval::example(*a, *b)).collect();
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
