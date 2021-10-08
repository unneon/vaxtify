use crate::dbus::client::DevPustaczekVaxtify;
use std::time::Duration;

#[derive(Debug)]
struct Args {
	permit: String,
	is_end: bool,
}

pub fn run() {
	let argv = parse_args().unwrap();
	let conn = dbus::blocking::Connection::new_session().unwrap();
	let proxy = conn.with_proxy("dev.pustaczek.Vaxtify", "/", Duration::from_millis(500));
	let permit = argv.permit.as_str();
	let r = if argv.is_end { proxy.permit_end(permit) } else { proxy.permit_start(permit) };
	match r {
		Ok(()) => {}
		Err(e) => {
			println!("\x1B[1;31merror:\x1B[0m {}", e);
			std::process::exit(1);
		}
	}
}

fn parse_args() -> Result<Args, &'static str> {
	let argv = std::env::args().collect::<Vec<_>>();
	let argv = argv.iter().map(String::as_str).collect::<Vec<_>>();
	let (permit, is_end) = match argv.as_slice() {
		[_, "permit", permit, "end"] => (permit, true),
		[_, "permit", permit] => (permit, false),
		_ => return Err("arguments don't match the pattern"),
	};
	let permit = (*permit).to_owned();
	let args = Args { permit, is_end };
	Ok(args)
}
