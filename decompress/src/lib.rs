#![feature(test)]

use hamu::read::le::*;
use hamu::write::le::*;

mod decompress;
mod compress;

#[derive(Debug, thiserror::Error)]
enum DecompressError {
	#[error(transparent)]
	Decompress {
		#[from] source: decompress::Error,
	},
	#[error("invalid chunk length")]
	BadChunkLength,
	#[error("wrong compresed len: got {size}, expected {expected_size}")]
	WrongCompressedLen {
		size: usize,
		expected_size: usize,
	},
	#[error("wrong uncompresed len: got {size}, expected {expected_size}")]
	WrongUncompressedLen {
		size: usize,
		expected_size: usize,
	},
}

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

pub fn decompress_ed7<'a, T: Read<'a>>(f: &mut T) -> Result<Vec<u8>, T::Error> {
	let csize = f.u32()? as usize;
	let start = f.pos();
	let usize = f.u32()? as usize;
	let mut out = Vec::with_capacity(usize);
	for _ in 1..f.u32()? {
		let pos = f.error_state();
		let Some(chunklen) = (f.u16()? as usize).checked_sub(2) else {
			return Err(T::to_error(pos, DecompressError::BadChunkLength.into()))
		};
		decompress::decompress_chunk(f.slice(chunklen)?, &mut out)
			.map_err(|e| T::to_error(pos, Box::new(e)))?;
		f.check_u8(1)?;
	}

	f.check_u32(0x06000006)?;
	f.slice(3)?; // unknown

	if f.pos() != csize+start {
		return Err(Reader::to_error(f.pos(), DecompressError::WrongCompressedLen {
			size: f.pos() - start,
			expected_size: csize,
		}.into()))
	}

	if out.len() != usize {
		return Err(Reader::to_error(f.pos(), DecompressError::WrongUncompressedLen {
			size: out.len(),
			expected_size: usize,
		}.into()))
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

pub fn compress_ed7(data: &[u8]) -> Vec<u8> {
	let mut f = Writer::new();
	let (csize_r, csize_w) = Label::new();
	f.delay(|l| Ok(u32::to_le_bytes(hamu::write::cast_usize::<u32>(l(csize_r)?)? - 4)));
	f.u32(data.len() as u32);
	f.u32(1+data.chunks(0xFFF0).count() as u32);
	for chunk in data.chunks(0xFFF0) {
		let data = compress::compress_chunk(chunk);
		f.u16(data.len() as u16 + 2);
		f.slice(&data);
		f.u8(1);
	}
	f.u32(0x06000006);
	f.slice(&[0,0,0]);
	f.label(csize_w);
	f.finish().unwrap()
}

#[cfg(test)]
mod test {
	const DATA: &[u8] = include_bytes!("lib.rs");

	#[test]
	fn compress_ed6() {
		let c = super::compress_ed6(DATA);
		let d = super::decompress_ed6(&mut super::Reader::new(&c)).unwrap();
		assert!(d == DATA);
	}

	#[test]
	fn ed7() {
		let c = super::compress_ed7(DATA);
		let d = super::decompress_ed7(&mut super::Reader::new(&c)).unwrap();
		assert!(d == DATA);
	}
}
