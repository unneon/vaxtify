use crate::permits::{PermitError, PermitResult};
use crate::Event;
use dbus_tree::{MTFn, MethodInfo};
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

fn build_tree(event_tx: mpsc::Sender<Event>) -> dbus_tree::Tree<dbus_tree::MTFn, ()> {
	let event_tx1 = event_tx;
	let event_tx2 = event_tx1.clone();
	let event_tx3 = event_tx1.clone();
	let f = dbus_tree::Factory::new_fn::<()>();
	f.tree(()).add(
		f.object_path("/", ()).introspectable().add(
			f.interface("dev.pustaczek.Vaxtify", ())
				.add_m(
					f.method("PermitStart", (), move |m| {
						let name = m.msg.read1()?;
						let (err_tx, err_rx) = mpsc::sync_channel(0);
						let event = Event::PermitRequest { name, duration: None, err_tx };
						dbus_wait(m, &event_tx1, event, err_rx)
					})
					.inarg::<&str, _>("permit"),
				)
				.add_m(
					f.method("PermitStartWithDuration", (), move |m| {
						let (name, duration) = m.msg.read2()?;
						let duration = Some(Duration::from_secs(duration));
						let (err_tx, err_rx) = mpsc::sync_channel(0);
						let event = Event::PermitRequest { name, duration, err_tx };
						dbus_wait(m, &event_tx2, event, err_rx)
					})
					.inarg::<&str, _>("permit")
					.inarg::<u64, _>("duration"),
				)
				.add_m(
					f.method("PermitEnd", (), move |m| {
						let name = m.msg.read1()?;
						let (err_tx, err_rx) = mpsc::sync_channel(0);
						let event = Event::PermitEnd { name, err_tx };
						dbus_wait(m, &event_tx3, event, err_rx)
					})
					.inarg::<&str, _>("permit"),
				),
		),
	)
}

fn dbus_wait(
	m: &MethodInfo<MTFn, ()>,
	event_tx: &mpsc::Sender<Event>,
	event: Event,
	err_rx: mpsc::Receiver<PermitResult>,
) -> dbus_tree::MethodResult {
	event_tx.send(event).unwrap();
	match err_rx.recv().unwrap() {
		Ok(()) => Ok(vec![m.msg.method_return()]),
		Err(err) => {
			let message = match err {
				PermitError::PermitDoesNotExist => "permit does not exist",
				PermitError::PermitIsNotActive => "permit is not active",
				PermitError::DurationTooLong => "duration is too long",
				PermitError::DurationNotSpecified => "duration is not specified",
				PermitError::CooldownNotFinished => "cooldown is not finished",
			};
			Err(dbus::Error::new_custom("dev.pustaczek.Vaxtify.Error", message).into())
		}
	}
}
