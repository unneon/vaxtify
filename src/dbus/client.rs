// This code was autogenerated with `dbus-codegen-rust -d solar.unneon.Vaxtify -p / -m None`, see https://github.com/diwic/dbus-rs
use dbus;
#[allow(unused_imports)]
use dbus::arg;
use dbus::blocking;

pub trait SolarUnneonVaxtify {
	fn browser_register(&self, pid: u32) -> Result<(), dbus::Error>;
	fn browser_unregister(&self, pid: u32) -> Result<(), dbus::Error>;
	fn permit_end(&self, permit: &str) -> Result<(), dbus::Error>;
	fn permit_start(&self, permit: &str) -> Result<(), dbus::Error>;
	fn service_reload(&self) -> Result<(), dbus::Error>;
	fn tab_delete(&self, pid: u32, tab: i32) -> Result<(), dbus::Error>;
	fn tab_update(&self, pid: u32, tab: i32, url: &str) -> Result<(), dbus::Error>;
}

impl<'a, T: blocking::BlockingSender, C: ::std::ops::Deref<Target = T>> SolarUnneonVaxtify for blocking::Proxy<'a, C> {
	fn browser_register(&self, pid: u32) -> Result<(), dbus::Error> {
		self.method_call("solar.unneon.Vaxtify", "BrowserRegister", (pid,))
	}

	fn browser_unregister(&self, pid: u32) -> Result<(), dbus::Error> {
		self.method_call("solar.unneon.Vaxtify", "BrowserUnregister", (pid,))
	}

	fn permit_end(&self, permit: &str) -> Result<(), dbus::Error> {
		self.method_call("solar.unneon.Vaxtify", "PermitEnd", (permit,))
	}

	fn permit_start(&self, permit: &str) -> Result<(), dbus::Error> {
		self.method_call("solar.unneon.Vaxtify", "PermitStart", (permit,))
	}

	fn service_reload(&self) -> Result<(), dbus::Error> {
		self.method_call("solar.unneon.Vaxtify", "ServiceReload", ())
	}

	fn tab_delete(&self, pid: u32, tab: i32) -> Result<(), dbus::Error> {
		self.method_call("solar.unneon.Vaxtify", "TabDelete", (pid, tab))
	}

	fn tab_update(&self, pid: u32, tab: i32, url: &str) -> Result<(), dbus::Error> {
		self.method_call("solar.unneon.Vaxtify", "TabUpdate", (pid, tab, url))
	}
}

#[derive(Debug)]
pub struct SolarUnneonVaxtifyTabClose {
	pub pid: u32,
	pub tab: i32,
}

impl arg::AppendAll for SolarUnneonVaxtifyTabClose {
	fn append(&self, i: &mut arg::IterAppend) {
		arg::RefArg::append(&self.pid, i);
		arg::RefArg::append(&self.tab, i);
	}
}

impl arg::ReadAll for SolarUnneonVaxtifyTabClose {
	fn read(i: &mut arg::Iter) -> Result<Self, arg::TypeMismatchError> {
		Ok(SolarUnneonVaxtifyTabClose { pid: i.read()?, tab: i.read()? })
	}
}

impl dbus::message::SignalArgs for SolarUnneonVaxtifyTabClose {
	const NAME: &'static str = "TabClose";
	const INTERFACE: &'static str = "solar.unneon.Vaxtify";
}

#[derive(Debug)]
pub struct SolarUnneonVaxtifyTabCreateEmpty {
	pub pid: u32,
}

impl arg::AppendAll for SolarUnneonVaxtifyTabCreateEmpty {
	fn append(&self, i: &mut arg::IterAppend) {
		arg::RefArg::append(&self.pid, i);
	}
}

impl arg::ReadAll for SolarUnneonVaxtifyTabCreateEmpty {
	fn read(i: &mut arg::Iter) -> Result<Self, arg::TypeMismatchError> {
		Ok(SolarUnneonVaxtifyTabCreateEmpty { pid: i.read()? })
	}
}

impl dbus::message::SignalArgs for SolarUnneonVaxtifyTabCreateEmpty {
	const NAME: &'static str = "TabCreateEmpty";
	const INTERFACE: &'static str = "solar.unneon.Vaxtify";
}

#[derive(Debug)]
pub struct SolarUnneonVaxtifyTabRefresh {}

impl arg::AppendAll for SolarUnneonVaxtifyTabRefresh {
	fn append(&self, _: &mut arg::IterAppend) {}
}

impl arg::ReadAll for SolarUnneonVaxtifyTabRefresh {
	fn read(_: &mut arg::Iter) -> Result<Self, arg::TypeMismatchError> {
		Ok(SolarUnneonVaxtifyTabRefresh {})
	}
}

impl dbus::message::SignalArgs for SolarUnneonVaxtifyTabRefresh {
	const NAME: &'static str = "TabRefresh";
	const INTERFACE: &'static str = "solar.unneon.Vaxtify";
}

pub trait OrgFreedesktopDBusIntrospectable {
	fn introspect(&self) -> Result<String, dbus::Error>;
}

impl<'a, T: blocking::BlockingSender, C: ::std::ops::Deref<Target = T>> OrgFreedesktopDBusIntrospectable
	for blocking::Proxy<'a, C>
{
	fn introspect(&self) -> Result<String, dbus::Error> {
		self.method_call("org.freedesktop.DBus.Introspectable", "Introspect", ()).and_then(|r: (String,)| Ok(r.0))
	}
}
