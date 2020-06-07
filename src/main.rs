fn add(a: i32, b: i32) -> i32 {
	match a.checked_add(b) {
		Some(c) => c,
		None => i32::MAX,
	}
}

fn main() {
	assert_eq!(add(2, 2), 4);
	assert_eq!(add(2_000_000_000, 2_000_000_000), i32::MAX);
}
