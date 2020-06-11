mod input_webext;
// mod input_x11;

fn main() {
	let mut conn = input_webext::Connection::new();
	loop {
		println!("{:?}", conn.read());
	}
}
