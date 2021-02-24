use crate::webext::{protocol, PORT};
use std::io::Write;
use std::net::TcpStream;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::thread;
use std::time::Duration;

pub fn check_and_run() {
	if std::env::args().nth(2).as_deref() == Some("vaxtify@pustaczek.dev") {
		run();
	}
}

fn run() -> ! {
	let tx = spawn_reverse_pipe();
	run_pipe(tx);
	std::process::exit(0);
}

fn spawn_reverse_pipe() -> SyncSender<TcpStream> {
	let (tx, rx) = sync_channel(0);
	thread::spawn(move || {
		let mut stdout = std::io::stdout();
		while let Ok(mut socket) = rx.recv() {
			while let Ok(message) = protocol::read(&mut socket) {
				protocol::write(&message, &mut stdout).unwrap();
				stdout.flush().unwrap();
			}
		}
	});
	tx
}

fn run_pipe(tx: SyncSender<TcpStream>) {
	let mut socket_slot = None;
	let mut stdin = std::io::stdin();
	let mut handshake = None;
	while let Ok(message) = protocol::read(&mut stdin) {
		let is_handshake = handshake.is_none();
		if is_handshake {
			handshake = Some(message.clone());
		}
		let mut socket = get_socket(&mut socket_slot, &tx, handshake.as_deref());
		if !is_handshake && (protocol::write(&message, &mut socket).is_err() || socket.flush().is_err()) {
			socket_slot = None;
		}
	}
}

fn get_socket<'a>(
	socket_slot: &'a mut Option<TcpStream>,
	tx: &SyncSender<TcpStream>,
	handshake: Option<&[u8]>,
) -> &'a mut TcpStream {
	socket_slot.get_or_insert_with(|| loop {
		match TcpStream::connect(("localhost", PORT)) {
			Ok(mut socket) => {
				// Resend the handshake message for every new connection. The proxy has to do this, because the
				// extension does not know when connections start and end.
				if let Some(handshake) = handshake {
					if protocol::write(handshake, &mut socket).is_err() {
						std::thread::sleep(Duration::from_secs(1));
						continue;
					}
				}
				tx.send(socket.try_clone().unwrap()).unwrap();
				break socket;
			}
			Err(_) => std::thread::sleep(Duration::from_secs(1)),
		}
	})
}
