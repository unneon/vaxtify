use std::io;
use std::io::{Read, Write};

pub enum ReadState {
	Header { header: [u8; 4], filled: usize },
	Data { data: Vec<u8>, filled: usize },
}

pub enum ReadError {
	Wait,
	EOF,
	IO(io::Error),
}

impl ReadState {
	pub fn new() -> ReadState {
		ReadState::Header { header: [0u8; 4], filled: 0 }
	}

	pub fn read(&mut self, mut read: impl Read) -> Result<Vec<u8>, ReadError> {
		match self {
			ReadState::Header { header, filled } => {
				read_to(header, filled, &mut read)?;
				let len = u32::from_ne_bytes(*header) as usize;
				let data = vec![0u8; len];
				*self = ReadState::Data { data, filled: 0 };
				self.read(read)
			}
			ReadState::Data { data, filled } => {
				read_to(data, filled, read)?;
				let data = std::mem::replace(data, Vec::new());
				*self = ReadState::new();
				Ok(data)
			}
		}
	}
}

fn read_to(buffer: &mut [u8], filled: &mut usize, mut read: impl Read) -> Result<(), ReadError> {
	match read.read(&mut buffer[*filled..]) {
		Ok(0) => Err(ReadError::EOF),
		Ok(n) => {
			*filled += n;
			if *filled == buffer.len() {
				Ok(())
			} else {
				Err(ReadError::Wait)
			}
		}
		Err(e) => Err(ReadError::IO(e)),
	}
}

pub fn read(mut read: impl Read) -> Result<Vec<u8>, io::Error> {
	let mut state = ReadState::new();
	loop {
		match state.read(&mut read) {
			Ok(data) => break Ok(data),
			Err(ReadError::Wait) => (),
			Err(ReadError::EOF) => break Err(io::ErrorKind::UnexpectedEof.into()),
			Err(ReadError::IO(e)) => break Err(e),
		}
	}
}

pub fn write(data: &[u8], mut output: impl Write) -> Result<(), io::Error> {
	output.write_all(&(data.len() as u32).to_ne_bytes())?;
	output.write_all(data)?;
	Ok(())
}

#[test]
fn reading() {
	let mut data: &[_] = &[2, 0, 0, 0, 1, 2, 3, 4];
	let message = read(&mut data).unwrap();
	assert_eq!(message, [1, 2]);
	assert_eq!(data, [3, 4]);
}

#[test]
fn reading_eof() {
	let data: &[_] = &[];
	assert_eq!(read(data).unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
}

#[test]
fn writing() {
	let mut data = Vec::new();
	write(&[1, 2], &mut data).unwrap();
	assert_eq!(data, [2, 0, 0, 0, 1, 2]);
}
