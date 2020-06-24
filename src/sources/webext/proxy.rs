use crate::sources::webext::protocol;
use std::io::stdin;
use std::net::TcpStream;
use std::time::Duration;

pub fn check_and_run(port: u16) {
	if std::env::args().nth(2).as_deref() == Some("distraction_oni@pustaczek.dev") {
		run(port);
	}
}

fn run(port: u16) -> ! {
	let mut socket_slot = None;
	let mut stdin = stdin();
	while let Ok(message) = protocol::read(&mut stdin) {
		let socket = socket_slot.get_or_insert_with(|| create_socket(port));
		if protocol::write(&message, socket).is_err() {
			socket_slot = None;
		}
	}
	std::process::exit(0);
}

fn create_socket(port: u16) -> TcpStream {
	loop {
		let socket = TcpStream::connect(("localhost", port));
		match socket {
			Ok(socket) => break socket,
			Err(_) => std::thread::sleep(Duration::from_secs(1)),
		}
	}
}
