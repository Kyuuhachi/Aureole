#![feature(test)]

mod decompress;
mod compress;

pub use decompress::Error;
pub use decompress::decompress;
pub use compress::compress;

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
