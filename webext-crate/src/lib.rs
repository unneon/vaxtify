use std::io;
use std::io::{Read, Write};

pub fn read(mut input: impl Read) -> Result<Vec<u8>, io::Error> {
	let mut header = [0; 4];
	input.read_exact(&mut header)?;
	let len = u32::from_le_bytes(header) as usize;
	let mut buf = vec![0; len];
	input.read_exact(&mut buf)?;
	Ok(buf)
}

pub fn write(data: &[u8], mut output: impl Write) -> Result<(), io::Error> {
	output.write_all(&(data.len() as u32).to_le_bytes())?;
	output.write_all(data)?;
	Ok(())
}
