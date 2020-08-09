use std::io;
use std::net::TcpStream;

pub enum Socket {
	Active { stream: TcpStream },
	Disconnected,
	Offline,
}

impl io::Read for Socket {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		match self {
			Socket::Active { stream } => match stream.read(buf) {
				Ok(0) => {
					*self = Socket::Disconnected;
					Ok(0)
				}
				r => r,
			},
			Socket::Disconnected | Socket::Offline => Ok(0),
		}
	}
}

impl io::Write for Socket {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		match self {
			Socket::Active { stream } => match stream.write(buf) {
				Ok(0) => {
					*self = Socket::Disconnected;
					Ok(0)
				}
				r => r,
			},
			Socket::Disconnected | Socket::Offline => Ok(0),
		}
	}

	fn flush(&mut self) -> io::Result<()> {
		if let Socket::Active { stream } = self {
			stream.flush()
		} else {
			Ok(())
		}
	}
}
