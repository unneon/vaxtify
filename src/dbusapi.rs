use crate::Event;
use std::sync::mpsc;
use std::time::Duration;

pub fn spawn(tx: mpsc::Sender<Event>) {
	std::thread::spawn(move || {
		let tree = build_tree(tx);
		let conn = dbus::blocking::LocalConnection::new_session().unwrap();
		conn.request_name("dev.pustaczek.Vaxtify", false, false, false).unwrap();
		tree.start_receive(&conn);
		loop {
			conn.process(Duration::from_millis(1000)).unwrap();
		}
	});
}

fn build_tree(tx: mpsc::Sender<Event>) -> dbus_tree::Tree<dbus_tree::MTFn, ()> {
	let tx1 = tx;
	let tx2 = tx1.clone();
	let tx3 = tx1.clone();
	let f = dbus_tree::Factory::new_fn::<()>();
	f.tree(()).add(
		f.object_path("/", ()).introspectable().add(
			f.interface("dev.pustaczek.Vaxtify", ())
				.add_m(
					f.method("PermitStart", (), move |m| {
						let permit: &str = m.msg.get1().unwrap();
						tx1.send(Event::PermitRequest { name: permit.to_owned(), duration: None }).unwrap();
						Ok(vec![m.msg.method_return()])
					})
					.inarg::<&str, _>("permit"),
				)
				.add_m(
					f.method("PermitStartWithDuration", (), move |m| {
						let (permit, duration) = m.msg.get2();
						let permit: &str = permit.unwrap();
						let duration: u64 = duration.unwrap();
						tx2.send(Event::PermitRequest {
							name: permit.to_owned(),
							duration: Some(Duration::from_secs(duration)),
						})
						.unwrap();
						Ok(vec![m.msg.method_return()])
					})
					.inarg::<&str, _>("permit")
					.inarg::<u64, _>("duration"),
				)
				.add_m(
					f.method("PermitEnd", (), move |m| {
						let permit: &str = m.msg.get1().unwrap();
						tx3.send(Event::PermitEnd { name: permit.to_owned() }).unwrap();
						Ok(vec![m.msg.method_return()])
					})
					.inarg::<&str, _>("permit"),
				),
		),
	)
}
