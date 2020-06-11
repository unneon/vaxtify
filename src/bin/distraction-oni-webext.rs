use std::io::stdin;
use std::net::TcpStream;

fn main() {
	let mut socket = TcpStream::connect("localhost:56154").expect("could not connect to distraction oni");
	std::io::copy(&mut stdin(), &mut socket).expect("could not copy entire input");
}
