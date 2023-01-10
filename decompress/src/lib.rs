use hamu::read::le::*;
use hamu::write::le::*;
use hamu::read::Error;

#[derive(Debug, thiserror::Error)]
enum DecompressError {
	#[error("invalid chunk length")]
	BadChunkLength,
	#[error("attempted to repeat {count} bytes from offset -{offset}, but only have {len} bytes")]
	BadRepeat {
		count: usize,
		offset: usize,
		len: usize,
	},
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

struct OutBuf<'a> {
	start: usize,
	out: &'a mut Vec<u8>,
}

impl<'a> OutBuf<'a> {
	fn new(out: &'a mut Vec<u8>) -> Self {
		OutBuf {
			start: out.len(),
			out,
		}
	}

	fn constant<T: ReadStream>(&mut self, n: usize, f: &mut T) -> Result<(), T::Error> {
		let b = f.u8()?;
		for _ in 0..n {
			self.out.push(b);
		}
		Ok(())
	}

	fn verbatim<T: ReadStream>(&mut self, n: usize, f: &mut T) -> Result<(), T::Error> {
		for _ in 0..n {
			let b = f.u8()?;
			self.out.push(b);
		}
		Ok(())
	}

	fn repeat<T: ReadStream>(&mut self, n: usize, o: usize, f: &mut T) -> Result<(), T::Error> {
		if !(1..=self.out.len()-self.start).contains(&o) {
			return Err(T::to_error(f.error_state(), DecompressError::BadRepeat { count: n, offset: o, len: self.out.len() }.into()))
		}
		for _ in 0..n {
			self.out.push(self.out[self.out.len()-o]);
		}
		Ok(())
	}
}

struct Bits {
	bits: u16,
	// Zero's decompressor counts number of remaining bits instead,
	// but this method is simpler.
	nextbit: u16,
}

impl Bits {
	fn new() -> Self {
		Bits {
			bits: 0,
			nextbit: 0,
		}
	}

	fn bit<T: ReadStream>(&mut self, f: &mut T) -> Result<bool, T::Error> {
		if self.nextbit == 0 {
			self.renew_bits(f)?;
		}
		let v = self.bits & self.nextbit != 0;
		self.nextbit <<= 1;
		Ok(v)
	}

	fn renew_bits<T: ReadStream>(&mut self, f: &mut T) -> Result<(), T::Error> {
		self.bits = f.u16()?;
		self.nextbit = 1;
		Ok(())
	}

	fn bits<T: ReadStream>(&mut self, n: usize, f: &mut T) -> Result<usize, T::Error> {
		let mut x = 0;
		for _ in 0..n%8 {
			x = x << 1 | usize::from(self.bit(f)?);
		}
		for _ in 0..n/8 {
			x = x << 8 | f.u8()? as usize;
		}
		Ok(x)
	}

	fn read_count<T: ReadStream>(&mut self, f: &mut T) -> Result<usize, T::Error> {
		Ok(
			if      self.bit(f)? {  2 }
			else if self.bit(f)? {  3 }
			else if self.bit(f)? {  4 }
			else if self.bit(f)? {  5 }
			else if self.bit(f)? {  6 + self.bits(3, f)? } //  6..=13
			else                 { 14 + self.bits(8, f)? } // 14..=269
		)
	}
}

fn decompress1(data: &[u8], w: &mut OutBuf) -> Result<(), Error> {
	let f = &mut Reader::new(data);
	let mut b = Bits::new();
	b.renew_bits(f)?;
	b.nextbit <<= 8;

	loop {
		if !b.bit(f)? {
			w.verbatim(1, f)?
		} else if !b.bit(f)? {
			let o = b.bits(8, f)?;
			let n = b.read_count(f)?;
			w.repeat(n, o, f)?
		} else {
			match b.bits(13, f)? {
				0 => break,
				1 => {
					let n = if b.bit(f)? {
						b.bits(12, f)?
					} else {
						b.bits(4, f)?
					};
					w.constant(14 + n, f)?;
				}
				o => {
					let n = b.read_count(f)?;
					w.repeat(n, o, f)?;
				}
			}
		}
	}
	Ok(())
}

#[bitmatch::bitmatch]
fn decompress2(data: &[u8], w: &mut OutBuf) -> Result<(), Error> {
	let f = &mut Reader::new(data);

	let mut last_o = 0;
	while f.remaining() > 0 {
		#[bitmatch] match f.u8()? as usize {
			"00xnnnnn" => {
				let n = if x == 1 { n << 8 | f.u8()? as usize } else { n };
				w.verbatim(n, f)?;
			}
			"010xnnnn" => {
				let n = if x == 1 { n << 8 | f.u8()? as usize } else { n };
				w.constant(4 + n, f)?;
			}
			"011nnnnn" => {
				w.repeat(n, last_o, f)?;
			}
			"1nnooooo" => {
				last_o = o << 8 | f.u8()? as usize;
				w.repeat(4 + n, last_o, f)?;
			},
		}
	}
	Ok(())
}

fn decompress_chunk(data: &[u8], w: &mut OutBuf) -> Result<(), Error> {
	if data.first() == Some(&0) {
		decompress1(data, w)
	} else {
		decompress2(data, w)
	}
}

pub fn decompress_ed6<'a, T: Read<'a>>(f: &mut T) -> Result<Vec<u8>, T::Error> {
	let mut out = Vec::new();
	loop {
		let pos = f.error_state();
		let checked_sub = (f.u16()? as usize).checked_sub(2);
		let Some(chunklen) = checked_sub else {
			return Err(T::to_error(pos, DecompressError::BadChunkLength.into()))
		};
		decompress_chunk(f.slice(chunklen)?, &mut OutBuf::new(&mut out))?;
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
		decompress_chunk(f.slice(chunklen)?, &mut OutBuf::new(&mut out))?;
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

pub fn compress_chunk(data: &[u8]) -> Vec<u8> {
	let mut f = Writer::new();
	for c in data.chunks(1<<13) {
		f.u8(0x20 | (c.len() >> 8) as u8);
		f.u8(c.len() as u8);
		f.slice(c);
	}
	f.finish().unwrap()
}

pub fn compress_ed6(data: &[u8]) -> Vec<u8> {
	let mut f = Writer::new();
	for chunk in data.chunks(0xFFF0) {
		let data = compress_chunk(chunk);
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
		let data = compress_chunk(chunk);
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
	const DATA: &[u8] = "The Legend of Heroes: Trails in the Sky[c] is a 2004 role-playing video game developed by Nihon Falcom. The game is the first in what later became known as the Trails series, itself a part of the larger The Legend of Heroes series. Trails in the Sky was first released in Japan for Windows and was later ported to the PlayStation Portable in 2006. North American video game publisher Xseed Games acquired the rights from Falcom, but did not release it until 2011 due to the game's large amount of text necessary to translate and localize. A high-definition port to the PlayStation 3 was released in 2012, while a remaster for the PlayStation Vita was released in 2015; both were only released in Japan. A direct sequel, Trails in the Sky SC, was released in 2006.".as_bytes();

	#[test]
	fn ed6() {
		let c = super::compress_ed6(DATA);
		println!("{:?}", c);
		let d = super::decompress_ed6(&mut super::Reader::new(&c)).unwrap();
		println!("{:?}", String::from_utf8_lossy(&d));
		assert_eq!(DATA, d);
	}

	#[test]
	fn ed7() {
		let c = super::compress_ed7(DATA);
		println!("{:?}", c);
		let d = super::decompress_ed7(&mut super::Reader::new(&c)).unwrap();
		println!("{:?}", String::from_utf8_lossy(&d));
		assert_eq!(DATA, d);
	}
}
