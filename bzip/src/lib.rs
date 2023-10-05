/// An implementation of Falcom's BZip compression algorithm,
/// used in *Trails in the Sky* as well as in their `itp` and `it3` file formats.
///
/// Note that this algorithm has no relation whatsoever to the bzip2 algorithm in common use.
///
/// BZip has two modes:
/// - Mode 1 appears to suffer less from barely-compressible data, but is only known to be supported by *Trails in the Sky*, which uses it in its 3d model files.
/// - Mode 2 is supported by all known games that use this algorithm, including *Trails in the Sky*.
///
/// There are also two framing modes. They have no known names, so I call them ed6 and ed7 from which game I first encountered them:
/// There is no known benefit for either of them, other than being in different contexts:
/// - *Trails in the Sky* uses `ed6` framing in its archive files.
/// - Certain forms of `itp` files also use ed6 framing. (Others use C77 compression, another proprietary Falcom algorithm.)
/// - `it3` files use ed7 framing.
///
/// Mode 2 is sometimes inofficially known as FALCOM2, and ed7 framing as FALCOM3.

use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _, Label};

mod decompress;
mod compress;

pub use decompress::Error;

/// Decompresses a single chunk of compressed data. Both mode 1 and 2 are supported.
/// There are no notable limitations regarding input or output size.
///
/// In most cases you will likely want to use the framed formats instead, [`decompress_ed6`] or [`decompress_ed7`].
pub use decompress::decompress as decompress_chunk;

pub fn decompress_ed6(f: &mut Reader) -> Result<Vec<u8>, Error> {
	let mut out = Vec::new();
	loop {
		let Some(chunklen) = (f.u16()? as usize).checked_sub(2) else {
			return Err(Error::Frame)
		};
		decompress_chunk(f.slice(chunklen)?, &mut out)?;
		if f.u8()? == 0 {
			break
		}
	}
	Ok(out)
}

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

pub fn decompress_ed6_from_slice(data: &[u8]) -> Result<Vec<u8>, Error> {
	decompress_ed6(&mut Reader::new(data))
}

pub fn decompress_ed7_from_slice(data: &[u8]) -> Result<Vec<u8>, Error> {
	decompress_ed7(&mut Reader::new(data))
}

pub fn compression_info_ed6(data: &[u8]) -> Option<(usize, Option<CompressMode>)> {
	let f = &mut Reader::new(data);
	let mut len = 0;
	let mut has_mode1 = false;
	let mut has_mode2 = false;
	loop {
		let chunklen = (f.u16().ok()? as usize).checked_sub(2)?;
		let chunk = f.slice(chunklen).ok()?;
		if chunk.is_empty() { return None }
		if chunk[0] == 0 {
			has_mode2 = true;
		} else {
			has_mode1 = true
		};
		if f.u8().ok()? == 0 {
			if !f.remaining().is_empty() {
				return None
			}
			len += decompress::get_size(chunk)?;
			break
		} else {
			len += 0xFFF0;
		}
	}

	let mode = match (has_mode1, has_mode2) {
		(true, true) => None,
		(true, false) => Some(CompressMode::Mode1),
		(false, _) => Some(CompressMode::Mode2),
	};

	Some((len, mode))
}

pub use compress::CompressMode;
/// Compresses a single chunk of compressed data, in the specified mode.
/// The mode 2 compressor can currently not handle chunks larger than `0xFFFF` bytes,
/// but mode 1 has no such restrictions.
/// Usually, chunks no larger than `0xFFF0` bytes are used, in either mode.
///
/// In most cases you will likely want to use the framed formats instead, [`compress_ed6`] or [`compress_ed7`].
pub use compress::compress as compress_chunk;

pub fn compress_ed6(f: &mut Writer, data: &[u8], mode: CompressMode) {
	let mut nchunks = data.chunks(0xFFF0).count();
	for chunk in data.chunks(0xFFF0) {
		let mut data = Vec::new();
		compress_chunk(chunk, &mut data, mode);
		f.u16(data.len() as u16 + 2);
		f.slice(&data);
		nchunks -= 1;
		f.u8(nchunks as u8);
	}
}

pub fn compress_ed7(f: &mut Writer, data: &[u8], mode: CompressMode) {
	let start = Label::new();
	let end = Label::new();
	f.delay(move |ctx| Ok(u32::to_le_bytes((ctx.label(end)? - ctx.label(start)?) as u32)));
	f.label(start);
	f.u32(data.len() as u32);
	f.u32(1+data.chunks(0xFFF0).count() as u32);
	for chunk in data.chunks(0xFFF0) {
		let mut data = Vec::new();
		compress_chunk(chunk, &mut data, mode);
		f.u16(data.len() as u16 + 2);
		f.slice(&data);
		f.u8(1);
	}
	f.u32(0x06000006);
	f.slice(&[0,0,0]);
	f.label(end);
}

pub fn compress_ed6_to_vec(data: &[u8], mode: CompressMode) -> Vec<u8> {
	let mut w = Writer::new();
	compress_ed6(&mut w, data, mode);
	w.finish().unwrap()
}

pub fn compress_ed7_to_vec(data: &[u8], mode: CompressMode) -> Vec<u8> {
	let mut w = Writer::new();
	compress_ed7(&mut w, data, mode);
	w.finish().unwrap()
}

#[test]
fn framing_should_roundtrip() {
	let data = std::fs::read("../data/fc.extract2/03/mt4301._ch").unwrap();
	let start = std::time::Instant::now();
	let decomp = decompress_ed6_from_slice(&data).unwrap();
	let d1 = start.elapsed();
	let start = std::time::Instant::now();
	let recomp = compress_ed6_to_vec(&decomp, CompressMode::Mode2);
	let d2 = start.elapsed();
	assert!(data == recomp);

	println!("Decompress {}, compress {}", d1.as_secs_f64(), d2.as_secs_f64());
}

#[test]
#[ignore = "it is slow"]
fn mode2_should_roundtrip() {
	use gospel::read::{Reader, Le as _};

	let data = std::fs::read("../data/fc.extract2/00/font64._da").unwrap();
	let mut f = Reader::new(&data);
	let start = std::time::Instant::now();
	let mut d1 = std::time::Duration::ZERO;
	let mut d2 = std::time::Duration::ZERO;

	loop {
		let chunklen = f.u16().unwrap() as usize - 2;
		let inchunk = f.slice(chunklen).unwrap();
		assert!(inchunk[0] == 0);
		println!("{} / {}", f.pos(), f.len());

		let mut chunk = Vec::new();
		let start = std::time::Instant::now();
		decompress_chunk(inchunk, &mut chunk).unwrap();
		let end = std::time::Instant::now();
		d1 += end - start;

		let mut outchunk = Vec::new();
		let start = std::time::Instant::now();
		compress_chunk(&chunk, &mut outchunk, CompressMode::Mode2);
		let end = std::time::Instant::now();
		d2 += end - start;

		assert!(inchunk == outchunk);

		if f.u8().unwrap() == 0 {
			break
		}
	}
	let end = std::time::Instant::now();

	println!("Decompress {}, compress {}, total {}", d1.as_secs_f64(), d2.as_secs_f64(), (end-start).as_secs_f64());
}

#[test]
fn mode1_should_roundtrip() {
	use gospel::read::{Reader, Le as _};

	let data = std::fs::read("../data/3rd.extract2/33/val2._x3").unwrap();
	let mut f = Reader::new(&data);
	let start = std::time::Instant::now();
	let mut d1 = std::time::Duration::ZERO;
	let mut d2 = std::time::Duration::ZERO;

	loop {
		let chunklen = f.u16().unwrap() as usize - 2;
		let inchunk = f.slice(chunklen).unwrap();
		assert!(inchunk[0] != 0);
		println!("{} / {}", f.pos(), f.len());

		let mut chunk = Vec::new();
		let start = std::time::Instant::now();
		decompress_chunk(inchunk, &mut chunk).unwrap();
		let end = std::time::Instant::now();
		d1 += end - start;

		let mut outchunk = Vec::new();
		let start = std::time::Instant::now();
		compress_chunk(&chunk, &mut outchunk, CompressMode::Mode1);
		let end = std::time::Instant::now();
		d2 += end - start;

		assert!(inchunk == outchunk);

		if f.u8().unwrap() == 0 {
			break
		}
	}
	let end = std::time::Instant::now();

	println!("Decompress {}, compress {}, total {}", d1.as_secs_f64(), d2.as_secs_f64(), (end-start).as_secs_f64());
}
