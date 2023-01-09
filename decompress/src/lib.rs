use hamu::read::le::*;
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

struct Ctx<'a> {
	start: usize,
	out: &'a mut Vec<u8>,
}

impl<'a> Ctx<'a> {
	fn new(out: &'a mut Vec<u8>) -> Self {
		Ctx {
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

struct ByteCtx {
	bits: u16,
	// Zero's decompressor counts number of remaining bits instead,
	// but this method is simpler.
	nextbit: u16,
}

impl ByteCtx {
	fn new() -> Self {
		ByteCtx {
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

fn decompress1(data: &[u8], w: &mut Ctx) -> Result<(), Error> {
	let f = &mut Reader::new(data);
	let mut c = ByteCtx::new();
	c.renew_bits(f)?;
	c.nextbit <<= 8;

	loop {
		if !c.bit(f)? {
			w.verbatim(1, f)?
		} else if !c.bit(f)? {
			let o = c.bits(8, f)?;
			let n = c.read_count(f)?;
			w.repeat(n, o, f)?
		} else {
			match c.bits(13, f)? {
				0 => break,
				1 => {
					let n = if c.bit(f)? {
						c.bits(12, f)?
					} else {
						c.bits(4, f)?
					};
					w.constant(14 + n, f)?;
				}
				o => {
					let n = c.read_count(f)?;
					w.repeat(n, o, f)?;
				}
			}
		}
	}
	Ok(())
}

#[bitmatch::bitmatch]
fn decompress2(data: &[u8], w: &mut Ctx) -> Result<(), Error> {
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

fn decompress_chunk(data: &[u8], w: &mut Ctx) -> Result<(), Error> {
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
		decompress_chunk(f.slice(chunklen)?, &mut Ctx::new(&mut out))?;
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
		decompress_chunk(f.slice(chunklen)?, &mut Ctx::new(&mut out))?;
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
