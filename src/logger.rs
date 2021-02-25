use log::{Level, Metadata, Record};

struct Logger;

static LOGGER: Logger = Logger;

impl log::Log for Logger {
	fn enabled(&self, _: &Metadata) -> bool {
		true
	}

	fn log(&self, record: &Record) {
		let level = match record.level() {
			Level::Error => "<3>",
			Level::Warn => "<4>",
			Level::Info => "<6>",
			Level::Debug => "<7>",
			Level::Trace => "<7>",
		};
		println!("{}{}", level, record.args());
	}

	fn flush(&self) {}
}

pub fn init() -> Result<(), log::SetLoggerError> {
	log::set_logger(&LOGGER)?;
	log::set_max_level(log::LevelFilter::Debug);
	Ok(())
}
