mod input_webext;

fn main() {
	let mut conn = input_webext::Connection::new();
	loop {
		println!("{:?}", conn.read());
	}
}
