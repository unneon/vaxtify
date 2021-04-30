use crate::webext::dbus::{DevPustaczekVaxtify, DevPustaczekVaxtifyTabClose, DevPustaczekVaxtifyTabCreateEmpty};
use crate::webext::message::{deserialize_event, serialize_command, Command, Event};
use crate::webext::protocol;
use dbus::blocking::LocalConnection;
use dbus::Message;
use std::io::Write;
use std::thread;
use std::time::Duration;

pub fn check_and_run() {
	if std::env::args().nth(2).as_deref() == Some("vaxtify@pustaczek.dev") {
		run();
	}
}

fn run() -> ! {
	spawn_signals_to_commands();
	run_events_to_calls();
	std::process::exit(0);
}

fn spawn_signals_to_commands() {
	thread::spawn(move || {
		// TODO: Avoid creating two connections? This caused dropped return values before.
		let pid = std::process::id();
		let conn = LocalConnection::new_session().unwrap();
		let proxy = conn.with_proxy("dev.pustaczek.Vaxtify", "/", Duration::from_millis(5000));
		proxy
			.match_signal(move |h: DevPustaczekVaxtifyTabClose, _: &LocalConnection, _: &Message| {
				// TODO: Delegate PID filter to dbus instead, somehow?
				if h.pid == pid {
					let stdout = std::io::stdout();
					let mut stdout = stdout.lock();
					protocol::write(&serialize_command(Command::Close { tab: h.tab }), &mut stdout).unwrap();
					stdout.flush().unwrap();
				}
				true
			})
			.unwrap();
		proxy
			.match_signal(move |h: DevPustaczekVaxtifyTabCreateEmpty, _: &LocalConnection, _: &Message| {
				if h.pid == pid {
					let stdout = std::io::stdout();
					let mut stdout = stdout.lock();
					protocol::write(&serialize_command(Command::CreateEmpty {}), &mut stdout).unwrap();
					stdout.flush().unwrap();
				}
				true
			})
			.unwrap();
		loop {
			conn.process(Duration::from_millis(5000)).unwrap();
		}
	});
}

fn run_events_to_calls() {
	let pid = std::process::id();
	let conn = LocalConnection::new_session().unwrap();
	let stdin = std::io::stdin();
	let mut stdin = stdin.lock();
	let proxy = conn.with_proxy("dev.pustaczek.Vaxtify", "/", Duration::from_millis(5000));
	proxy.browser_register(pid).unwrap();
	while let Ok(message) = protocol::read(&mut stdin) {
		match deserialize_event(&message) {
			Event::Removed { tab } => proxy.browser_tab_delete(pid, tab).unwrap(),
			Event::Updated { tab, url } => proxy.browser_tab_update(pid, tab, &url).unwrap(),
			Event::Handshake { .. } => {}
		}
	}
	proxy.browser_unregister(pid).unwrap();
}
