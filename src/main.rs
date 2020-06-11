#![allow(unused)]

mod input_x11;

fn main() -> Result<(), ()> {
	let display = input_x11::Display::open()?;
	let tree = display.query_tree()?;
	for window in tree.children {
		println!("PID = {:?}", display.get_window_pid(window));
	}
	Ok(())
}
