use hamu::read::le::*;

#[derive(Debug, snafu::Snafu)]
pub enum Error {
	#[snafu(context(false))]
	Read { source: ReadError, backtrace: snafu::Backtrace },
	#[snafu(display("tried to repeat {count} bytes from offset -{offset} (length {len})"))]
	Repeat { count: usize, offset: usize, len: usize },
}

pub type Result<T, E=Error> = std::result::Result<T, E>;

struct Ctx<'b> {
	out: Vec<u8>,
	data: Bytes<'b>,
}

impl <'a> Ctx<'a> {
	fn new(data: &'a [u8]) -> Self {
		Ctx {
			out: Vec::with_capacity(0xFFF0),
			data: Bytes::new(data),
		}
	}

	fn byte(&mut self) -> Result<usize> {
		Ok(self.data.u8()? as usize)
	}

	fn constant(&mut self, n: usize) -> Result<()> {
		let b = self.data.u8()?;
		for _ in 0..n {
			self.out.push(b);
		}
		Ok(())
	}

	fn verbatim(&mut self, n: usize) -> Result<()> {
		for _ in 0..n {
			let b = self.data.u8()?;
			self.out.push(b);
		}
		Ok(())
	}

	fn repeat(&mut self, n: usize, o: usize) -> Result<()> {
		snafu::ensure!((1..=self.out.len()).contains(&o), RepeatSnafu {
			count: n,
			offset: o,
			len: self.out.len(),
		});
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
	fn new(data: &'a [u8]) -> Self {
		ByteCtx {
			ctx: Ctx::new(data),
			bits: 0,
			nextbit: 0,
		}
	}

	fn bit(&mut self) -> Result<bool> {
		if self.nextbit == 0 {
			self.renew_bits()?;
		}
		let v = self.bits & self.nextbit != 0;
		self.nextbit <<= 1;
		Ok(v)
	}

	fn renew_bits(&mut self) -> Result<()> {
		self.bits = self.data.u16()?;
		self.nextbit = 1;
		Ok(())
	}

	fn bits(&mut self, n: usize) -> Result<usize> {
		let mut x = 0;
		for _ in 0..n%8 {
			x = x << 1 | usize::from(self.bit()?);
		}
		for _ in 0..n/8 {
			x = x << 8 | self.byte()?;
		}
		Ok(x)
	}

	fn read_count(&mut self) -> Result<usize> {
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

fn decompress1(data: &[u8]) -> Result<Vec<u8>> {
	let mut c = ByteCtx::new(data);
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
fn decompress2(data: &[u8]) -> Result<Vec<u8>> {
	let mut c = Ctx::new(data);

	let mut last_o = 0;
	while c.data.remaining() > 0 {
		#[bitmatch] match c.byte()? {
			"00xnnnnn" => {
				let n = if x == 1 { n << 8 | c.byte()? } else { n };
				c.verbatim(n)?;
			}
			"010xnnnn" => {
				let n = if x == 1 { n << 8 | c.byte()? } else { n };
				c.constant(4 + n)?;
			}
			"011nnnnn" => {
				c.repeat(n, last_o)?;
			}
			"1nnooooo" => {
				last_o = o << 8 | c.byte()?;
				c.repeat(4 + n, last_o)?;
			},
		}
	}

	Ok(c.out)
}

pub fn decompress_chunk(data: &[u8]) -> Result<Vec<u8>> {
	if data.get(0) == Some(&0) {
		Ok(decompress1(data)?)
	} else {
		Ok(decompress2(data)?)
	}
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
	let mut out = Vec::new();
	let mut i = Bytes::new(data);
	loop {
		let chunklen = i.u16()? as usize;
		let chunk = i.slice(chunklen - 2)?;
		out.append(&mut decompress_chunk(chunk)?);
		if i.u8()? == 0 { break }
	}
	Ok(out)
}
