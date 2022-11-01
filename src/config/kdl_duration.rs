use knuffel::Decode;

#[derive(Clone, Copy, Debug, Decode)]
pub struct Duration {
	#[knuffel(property, default)]
	days: u64,
	#[knuffel(property, default)]
	hours: u64,
	#[knuffel(property, default)]
	mins: u64,
	#[knuffel(property, default)]
	seconds: u64,
}

impl PartialEq for Duration {
	fn eq(&self, other: &Self) -> bool {
		std::time::Duration::from(*self) == std::time::Duration::from(*other)
	}
}

impl From<std::time::Duration> for Duration {
	fn from(duration: std::time::Duration) -> Self {
		Duration { days: 0, hours: 0, mins: 0, seconds: duration.as_secs() }
	}
}

impl From<Duration> for std::time::Duration {
	fn from(duration: Duration) -> Self {
		let days = duration.days;
		let hours = days * 24 + duration.hours;
		let mins = hours * 60 + duration.mins;
		let seconds = mins * 60 + duration.seconds;
		std::time::Duration::from_secs(seconds)
	}
}
