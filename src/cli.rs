use once_cell::sync::Lazy;
use regex::Regex;
use std::time::Duration;

#[derive(Debug)]
struct Args {
	permit: String,
	duration: Option<Duration>,
	is_end: bool,
}

pub fn run() {
	let argv = parse_args().unwrap();
	let conn = dbus::blocking::Connection::new_session().unwrap();
	let proxy = conn.with_proxy("dev.pustaczek.Vaxtify", "/", Duration::from_millis(500));
	let permit = argv.permit.as_str();
	let duration = argv.duration.map_or(0, |duration| duration.as_secs());
	let _: () = if argv.is_end {
		proxy.method_call("dev.pustaczek.Vaxtify", "PermitEnd", (permit,)).unwrap()
	} else if argv.duration.is_some() {
		proxy.method_call("dev.pustaczek.Vaxtify", "PermitStartWithDuration", (permit, duration)).unwrap()
	} else {
		proxy.method_call("dev.pustaczek.Vaxtify", "PermitStart", (permit,)).unwrap()
	};
}

fn parse_args() -> Result<Args, &'static str> {
	let argv = std::env::args().collect::<Vec<_>>();
	let argv = argv.iter().map(String::as_str).collect::<Vec<_>>();
	let (permit, duration_str, is_end) = match argv.as_slice() {
		[_, "permit", permit, "end"] => (permit, None, true),
		[_, "permit", permit, duration] => (permit, Some(*duration), false),
		[_, "permit", permit] => (permit, None, false),
		_ => return Err("arguments don't match the pattern"),
	};
	let permit = (*permit).to_owned();
	let duration = match duration_str {
		Some(duration_str) => {
			let (hours, minutes, seconds) = parse_duration(duration_str)?;
			Some(Duration::from_secs(60 * 60 * hours + 60 * minutes + seconds))
		}
		None => None,
	};
	let args = Args { permit, duration, is_end };
	Ok(args)
}

fn parse_duration(text: &str) -> Result<(u64, u64, u64), &'static str> {
	static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("(?:(\\d+)h)?(?:(\\d+)min)?(?:(\\d+)s)?").unwrap());
	let cap = match REGEX.captures(text) {
		Some(cap) => cap,
		None => return Err("duration does not match (\\d+h)?(\\d+min)?(\\d+s)?"),
	};
	let hours = if let Some(hours) = cap.get(1) { hours.as_str().parse().unwrap() } else { 0 };
	let minutes = if let Some(minutes) = cap.get(2) { minutes.as_str().parse().unwrap() } else { 0 };
	let seconds = if let Some(seconds) = cap.get(3) { seconds.as_str().parse().unwrap() } else { 0 };
	if hours == 0 && minutes == 0 && seconds == 0 {
		return Err("duration must be nonzero");
	}
	Ok((hours, minutes, seconds))
}

#[test]
fn duration_format() {
	assert_eq!(parse_duration("1h").unwrap(), (1, 0, 0));
	assert_eq!(parse_duration("1h30min").unwrap(), (1, 30, 0));
	assert_eq!(parse_duration("20s").unwrap(), (0, 0, 20));
}
