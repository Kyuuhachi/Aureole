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

struct Ctx<'b> {
	out: Vec<u8>,
	r: Reader<'b>,
}

impl<'a> Ctx<'a> {
	fn new(r: Reader<'a>) -> Self {
		Ctx {
			out: Vec::with_capacity(0xFFF0), // TODO I am not sure allocating here is good. Probably more performant to do it outside.
			r,
		}
	}

	fn extend(&mut self, b: usize) -> Result<usize, Error> {
		Ok(b << 8 | self.r.u8()? as usize)
	}

	fn constant(&mut self, n: usize) -> Result<(), Error> {
		let b = self.r.u8()?;
		for _ in 0..n {
			self.out.push(b);
		}
		Ok(())
	}

	fn verbatim(&mut self, n: usize) -> Result<(), Error> {
		for _ in 0..n {
			let b = self.r.u8()?;
			self.out.push(b);
		}
		Ok(())
	}

	fn repeat(&mut self, n: usize, o: usize) -> Result<(), Error> {
		if !(1..=self.out.len()).contains(&o) {
			return Err(Reader::to_error(self.r.pos(), DecompressError::BadRepeat { count: n, offset: o, len: self.out.len() }.into()))
		}
		for _ in 0..n {
			self.out.push(self.out[self.out.len()-o]);
		}
		Ok(())
	}
}

#[derive(derive_more::Deref, derive_more::DerefMut)]
struct ByteCtx<'b> {
	#[deref]
	#[deref_mut]
	ctx: Ctx<'b>,
	bits: u16,
	// Zero's decompressor counts number of remaining bits instead,
	// but this method is simpler.
	nextbit: u16,
}

impl <'a> ByteCtx<'a> {
	fn new(data: Reader<'a>) -> Self {
		ByteCtx {
			ctx: Ctx::new(data),
			bits: 0,
			nextbit: 0,
		}
	}

	fn bit(&mut self) -> Result<bool, Error> {
		if self.nextbit == 0 {
			self.renew_bits()?;
		}
		let v = self.bits & self.nextbit != 0;
		self.nextbit <<= 1;
		Ok(v)
	}

	fn renew_bits(&mut self) -> Result<(), Error> {
		self.bits = self.ctx.r.u16()?;
		self.nextbit = 1;
		Ok(())
	}

	fn bits(&mut self, n: usize) -> Result<usize, Error> {
		let mut x = 0;
		for _ in 0..n%8 {
			x = x << 1 | usize::from(self.bit()?);
		}
		for _ in 0..n/8 {
			x = self.extend(x)?;
		}
		Ok(x)
	}

	fn read_count(&mut self) -> Result<usize, Error> {
		Ok(
			if      self.bit()? {  2 }
			else if self.bit()? {  3 }
			else if self.bit()? {  4 }
			else if self.bit()? {  5 }
			else if self.bit()? {  6 + self.bits(3)? } //  6..=13
			else                { 14 + self.bits(8)? } // 14..=269
		)
	}
}

fn decompress1(data: &[u8]) -> Result<Vec<u8>, Error> {
	let mut c = ByteCtx::new(Reader::new(data));
	c.renew_bits()?;
	c.nextbit <<= 8;

	loop {
		if !c.bit()? {
			c.verbatim(1)?
		} else if !c.bit()? {
			let o = c.bits(8)?;
			let n = c.read_count()?;
			c.repeat(n, o)?
		} else {
			match c.bits(13)? {
				0 => break,
				1 => {
					let n = if c.bit()? {
						c.bits(12)?
					} else {
						c.bits(4)?
					};
					c.constant(14 + n)?;
				}
				o => {
					let n = c.read_count()?;
					c.repeat(n, o)?;
				}
			}
		}
	}

	Ok(c.ctx.out)
}

#[bitmatch::bitmatch]
fn decompress2(data: &[u8]) -> Result<Vec<u8>, Error> {
	let mut c = Ctx::new(Reader::new(data));

	let mut last_o = 0;
	while c.r.remaining() > 0 {
		#[bitmatch] match c.r.u8()? as usize {
			"00xnnnnn" => {
				let n = if x == 1 { c.extend(n)? } else { n };
				c.verbatim(n)?;
			}
			"010xnnnn" => {
				let n = if x == 1 { c.extend(n)? } else { n };
				c.constant(4 + n)?;
			}
			"011nnnnn" => {
				c.repeat(n, last_o)?;
			}
			"1nnooooo" => {
				last_o = c.extend(o)?;
				c.repeat(4 + n, last_o)?;
			},
		}
	}

	Ok(c.out)
}

pub fn decompress_chunk(data: &[u8]) -> Result<Vec<u8>, Error> {
	if data.first() == Some(&0) {
		Ok(decompress1(data)?)
	} else {
		Ok(decompress2(data)?)
	}
}

pub fn decompress_ed6<'a, T: Read<'a>>(f: &mut T) -> Result<Vec<u8>, T::Error> {
	let mut out = Vec::new();
	loop {
		let pos = f.error_state();
		let Some(chunklen) = (f.u16()? as usize).checked_sub(2) else {
			return Err(T::to_error(pos, DecompressError::BadChunkLength.into()))
		};
		out.extend(decompress_chunk(f.slice(chunklen)?)?);
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
		out.extend(decompress_chunk(f.slice(chunklen)?)?);
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
