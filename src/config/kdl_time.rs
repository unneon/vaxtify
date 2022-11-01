use knuffel::Decode;

#[derive(Clone, Copy, Debug, Decode, Eq, PartialEq)]
pub struct NaiveTime {
	#[knuffel(property)]
	hour: u32,
	#[knuffel(property, default)]
	min: u32,
}

impl From<NaiveTime> for chrono::NaiveTime {
	fn from(time: NaiveTime) -> Self {
		chrono::NaiveTime::from_hms(time.hour, time.min, 0)
	}
}
