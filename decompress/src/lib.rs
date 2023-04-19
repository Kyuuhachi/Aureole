#![feature(test)]

use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _, Label};

mod decompress;
mod compress;

pub use decompress::Error;
pub use decompress::decompress as decompress_chunk;
pub use compress::compress as compress_chunk;

/*
pub fn decompress_ed6<'a, T: Read<'a>>(f: &mut T) -> Result<Vec<u8>, T::Error> {
	let mut out = Vec::new();
	loop {
		let pos = f.error_state();
		let checked_sub = (f.u16()? as usize).checked_sub(2);
		let Some(chunklen) = checked_sub else {
			return Err(T::to_error(pos, DecompressError::BadChunkLength.into()))
		};
		decompress::decompress_chunk(f.slice(chunklen)?, &mut out)
			.map_err(|e| T::to_error(pos, Box::new(e)))?;
		if f.u8()? == 0 {
			break
		}
	}
	Ok(out)
}

pub fn compress_ed6(data: &[u8]) -> Vec<u8> {
	let mut f = Writer::new();
	for chunk in data.chunks(0xFFF0) {
		let data = compress::compress_chunk(chunk);
		f.u16(data.len() as u16 + 2);
		f.slice(&data);
		f.u8((chunk.as_ptr_range().end == data.as_ptr_range().end).into());
	}
	f.finish().unwrap()
}
*/

pub fn decompress_ed7(f: &mut Reader) -> Result<Vec<u8>, Error> {
	let csize = f.u32()? as usize;
	let start = f.pos();
	let usize = f.u32()? as usize;
	let mut out = Vec::with_capacity(usize);
	for _ in 1..f.u32()? {
		let Some(chunklen) = (f.u16()? as usize).checked_sub(2) else {
			return Err(Error::Frame)
		};
		decompress_chunk(f.slice(chunklen)?, &mut out)?;
		f.check_u8(1)?;
	}

	f.check_u32(0x06000006)?;
	f.slice(3)?; // unknown

	if csize != f.pos() - start {
		return Err(Error::Frame)
	}
	if usize != out.len() {
		return Err(Error::Frame)
	}

	Ok(out)
}

pub fn compress_ed7(f: &mut Writer, data: &[u8]) {
	let start = Label::new();
	let end = Label::new();
	f.delay(move |ctx| Ok(u32::to_le_bytes((ctx.label(end)? - ctx.label(start)?) as u32)));
	f.label(start);
	f.u32(data.len() as u32);
	f.u32(1+data.chunks(0xFFF0).count() as u32);
	for chunk in data.chunks(0xFFF0) {
		let data = compress_chunk(chunk);
		f.u16(data.len() as u16 + 2);
		f.slice(&data);
		f.u8(1);
	}
	f.u32(0x06000006);
	f.slice(&[0,0,0]);
	f.label(end);
}

pub fn decompress_ed7_from_slice(data: &[u8]) -> Result<Vec<u8>, Error> {
	decompress_ed7(&mut Reader::new(data))
}

pub fn compress_ed7_to_vec(data: &[u8]) -> Vec<u8> {
	let mut w = Writer::new();
	compress_ed7(&mut w, data);
	w.finish().unwrap()
}
