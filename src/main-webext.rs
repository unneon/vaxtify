use std::io::stdin;
use std::net::TcpStream;
use std::time::Duration;

fn create_socket() -> TcpStream {
	loop {
		let socket = TcpStream::connect("localhost:56154");
		match socket {
			Ok(socket) => break socket,
			Err(_) => std::thread::sleep(Duration::from_secs(1)),
		}
	}
}

fn main() {
	let mut socket_slot = None;
	let mut stdin = stdin();
	while let Ok(message) = webext::read(&mut stdin) {
		let socket = socket_slot.get_or_insert_with(create_socket);
		if webext::write(&message, socket).is_err() {
			socket_slot = None;
		}
	}
}
