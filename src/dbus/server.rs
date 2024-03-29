use crate::tabs::TabId;
use crate::Event;
use dbus::blocking::stdintf::org_freedesktop_dbus::RequestNameReply;
use dbus::blocking::LocalConnection;
use dbus::channel::Sender;
use dbus::strings::Interface;
use dbus::Path;
use dbus_tree::DataType;
use dbus_tree::{MTFn, MethodInfo, Signal, Tree};
use std::sync::{mpsc, Arc};
use std::time::Duration;
// TODO: Figure out a better way of communicating between these threads?
// The main problem is that callbacks contain mpsc::Sender so they aren't Sync, and for some reason dbus-tree only
// supports non-Send and Send-Sync callbacks. Could figure out why and maybe fix it, could try to call .channel() which
// returns a raw Send-Sync socket-like thingy and figure out lifetimes instead.

#[derive(Debug)]
enum Command {
	TabClose { pid: u32, tab: i32 },
	TabCreateEmpty { pid: u32 },
	Refresh {},
}

pub struct DBus {
	command_tx: mpsc::Sender<Command>,
}

struct TreeInfo {
	tree: Tree<MTFn<TreeData>, TreeData>,
	signal_close: Arc<Signal<TreeData>>,
	signal_create_empty: Arc<Signal<TreeData>>,
	signal_refresh: Arc<Signal<TreeData>>,
}

#[derive(Default)]
struct TreeData;

#[derive(Debug)]
struct EventChannel {
	event_tx: mpsc::Sender<Event>,
}

impl DBus {
	pub fn new(tx: mpsc::Sender<Event>) -> DBus {
		let (command_tx, command_rx) = mpsc::channel();
		std::thread::spawn(move || {
			let path = Path::new("/").unwrap();
			let iface = Interface::new("solar.unneon.Vaxtify").unwrap();
			let info = build_tree(tx);
			let conn = LocalConnection::new_session().unwrap();
			let name_reply = conn.request_name("solar.unneon.Vaxtify", false, false, true).unwrap();
			assert_eq!(name_reply, RequestNameReply::PrimaryOwner);
			info.tree.start_receive(&conn);
			loop {
				conn.process(Duration::from_millis(100)).unwrap();
				while let Ok(command) = command_rx.try_recv() {
					let msg = match command {
						Command::TabClose { pid, tab } => info.signal_close.msg(&path, &iface).append2(pid, tab),
						Command::TabCreateEmpty { pid } => info.signal_create_empty.msg(&path, &iface).append1(pid),
						Command::Refresh {} => info.signal_refresh.msg(&path, &iface),
					};
					conn.send(msg).unwrap();
				}
			}
		});
		DBus { command_tx }
	}

	pub fn tab_close(&self, tab: TabId) {
		self.command_tx.send(Command::TabClose { pid: tab.pid, tab: tab.tab }).unwrap();
	}

	pub fn tab_create_empty(&self, pid: u32) {
		self.command_tx.send(Command::TabCreateEmpty { pid }).unwrap();
	}

	pub fn refresh(&self) {
		self.command_tx.send(Command::Refresh {}).unwrap();
	}
}

fn build_tree(event_tx: mpsc::Sender<Event>) -> TreeInfo {
	let method_channel = EventChannel { event_tx };
	let f = dbus_tree::Factory::new_fn();
	let signal_close = Arc::new(f.signal("TabClose", ()).sarg::<u32, _>("pid").sarg::<i32, _>("tab"));
	let signal_create_empty = Arc::new(f.signal("TabCreateEmpty", ()).sarg::<u32, _>("pid"));
	let signal_refresh = Arc::new(f.signal("TabRefresh", ()));
	let tree = f.tree(method_channel).add(
		f.object_path("/", ()).introspectable().add(
			f.interface("solar.unneon.Vaxtify", ())
				.add_s(signal_close.clone())
				.add_s(signal_create_empty.clone())
				.add_s(signal_refresh.clone())
				.add_m(f.method("ServiceReload", (), |m| {
					let (err_tx, err_rx) = mpsc::sync_channel(0);
					let event = Event::ServiceReload { err_tx };
					dbus_wait(m, event, err_rx)
				}))
				.add_m(
					f.method("PermitStart", (), |m| {
						let name = m.msg.read1()?;
						let (err_tx, err_rx) = mpsc::sync_channel(0);
						let event = Event::PermitRequest { name, err_tx };
						dbus_wait(m, event, err_rx)
					})
					.inarg::<&str, _>("permit"),
				)
				.add_m(
					f.method("PermitEnd", (), |m| {
						let name = m.msg.read1()?;
						let (err_tx, err_rx) = mpsc::sync_channel(0);
						let event = Event::PermitEnd { name, err_tx };
						dbus_wait(m, event, err_rx)
					})
					.inarg::<&str, _>("permit"),
				)
				.add_m(
					f.method("BrowserRegister", (), |m| {
						let _pid: u32 = m.msg.read1()?;
						Ok(vec![m.msg.method_return()])
					})
					.inarg::<u32, _>("pid"),
				)
				.add_m(
					f.method("TabUpdate", (), |m| {
						let (pid, tab, url): (_, _, &str) = m.msg.read3()?;
						let url = url.parse().unwrap();
						m.tree.get_data().event_tx.send(Event::TabUpdate { tab: TabId { pid, tab }, url }).unwrap();
						Ok(vec![m.msg.method_return()])
					})
					.inarg::<u32, _>("pid")
					.inarg::<i32, _>("tab")
					.inarg::<&str, _>("url"),
				)
				.add_m(
					f.method("TabDelete", (), |m| {
						let (pid, tab) = m.msg.read2()?;
						m.tree.get_data().event_tx.send(Event::TabDelete { tab: TabId { pid, tab } }).unwrap();
						Ok(vec![m.msg.method_return()])
					})
					.inarg::<u32, _>("pid")
					.inarg::<i32, _>("tab"),
				)
				.add_m(
					f.method("BrowserUnregister", (), |m| {
						let pid = m.msg.read1()?;
						m.tree.get_data().event_tx.send(Event::TabDeleteAll { pid }).unwrap();
						Ok(vec![m.msg.method_return()])
					})
					.inarg::<u32, _>("pid"),
				),
		),
	);
	TreeInfo { tree, signal_close, signal_create_empty, signal_refresh }
}

impl DataType for TreeData {
	type Tree = EventChannel;
	type ObjectPath = ();
	type Property = ();
	type Interface = ();
	type Method = ();
	type Signal = ();
}

impl Default for EventChannel {
	fn default() -> Self {
		EventChannel { event_tx: mpsc::channel().0 }
	}
}

fn dbus_wait<E: std::error::Error + 'static>(
	m: &MethodInfo<MTFn<TreeData>, TreeData>,
	event: Event,
	err_rx: mpsc::Receiver<Result<(), E>>,
) -> dbus_tree::MethodResult {
	m.tree.get_data().event_tx.send(event).unwrap();
	match err_rx.recv().unwrap() {
		Ok(()) => Ok(vec![m.msg.method_return()]),
		Err(err) => Err(dbus::Error::new_custom("solar.unneon.Vaxtify.Error", format_error(&err).as_str()).into()),
	}
}

fn format_error(mut error: &(dyn std::error::Error + 'static)) -> String {
	let mut output = error.to_string();
	while let Some(source) = error.source() {
		output += " \x1B[1;33mcaused by\x1B[0m ";
		output += &source.to_string();
		error = source;
	}
	output
}
