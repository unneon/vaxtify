mod input_x11;

fn main() -> Result<(), ()> {
	let display = input_x11::Display::open()?;
	let tree = display.query_tree()?;
	for child in tree.children {
		let attributes = display.get_window_attributes(child)?;
		let name = display.get_window_name(child);
		println!("Found window called: {:?}\n{:#?}", name, attributes);
	}
	Ok(())
}
