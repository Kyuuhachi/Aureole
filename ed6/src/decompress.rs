use std::io::{Read, Result, Error, ErrorKind};

macro_rules! ioe {
	($kind:ident, $e1:literal, $($args:tt)*) => {
		Error::new(ErrorKind::$kind, format!($e1, $($args)*))
	};
	($kind:ident, $e:expr) => {
		Error::new(ErrorKind::$kind, $e)
	}
}

fn u8(data: &mut impl Read) -> Result<u8> {
	let mut buf = [0];
	data.read_exact(&mut buf)?;
	Ok(buf[0])
}

struct Ctx<'b> {
	out: Vec<u8>,
	data: &'b [u8],
	pos: usize,
}

impl <'a> Ctx<'a> {
	fn new(data: &'a [u8]) -> Self {
		Ctx {
			out: Vec::with_capacity(0xFFF0), // TODO I am not sure allocating here is good. Probably more performant to do it outside.
			data,
			pos: 0,
		}
	}

	fn u8(&mut self) -> Result<u8> {
		let pos = self.pos.min(self.data.len());
		let mut buf = [0];
		Read::read_exact(&mut &self.data[pos..], &mut buf)?;
		self.pos += 1;
		Ok(buf[0])
	}

	fn extend(&mut self, b: usize) -> Result<usize> {
		Ok(b << 8 | self.u8()? as usize)
	}

	fn constant(&mut self, n: usize) -> Result<()> {
		let b = self.u8()?;
		for _ in 0..n {
			self.out.push(b);
		}
		Ok(())
	}

	fn verbatim(&mut self, n: usize) -> Result<()> {
		for _ in 0..n {
			let b = self.u8()?;
			self.out.push(b);
		}
		Ok(())
	}

	fn repeat(&mut self, n: usize, o: usize) -> Result<()> {
		if !(1..=self.out.len()).contains(&o) {
			return Err(ioe!(InvalidData, "tried to repeat {n} bytes from offset -{o} (length {len})", len=self.out.len()))
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
		self.bits = u16::from_le_bytes([self.u8()?, self.u8()?]);
		self.nextbit = 1;
		Ok(())
	}

	fn bits(&mut self, n: usize) -> Result<usize> {
		let mut x = 0;
		for _ in 0..n%8 {
			x = x << 1 | usize::from(self.bit()?);
		}
		for _ in 0..n/8 {
			x = self.extend(x)?;
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
	while c.pos < c.data.len() {
		#[bitmatch] match c.u8()? as usize {
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

pub fn decompress_chunk(data: &[u8]) -> Result<Vec<u8>> {
	if data.get(0) == Some(&0) {
		Ok(decompress1(data)?)
	} else {
		Ok(decompress2(data)?)
	}
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
	let mut out = Vec::new();
	for chunk in decompress_stream(&mut std::io::Cursor::new(data)) {
		out.append(&mut chunk?);
	}
	Ok(out)
}

pub fn decompress_stream(data: &mut impl Read) -> impl Iterator<Item=Result<Vec<u8>>> + '_ {
	let mut has_next = true;
	let mut buf = Vec::new();
	let mut buf2 = [0u8;2];
	std::iter::from_fn(move || has_next.then(|| {
		data.read_exact(&mut buf2)?;
		let chunklen = u16::from_le_bytes(buf2) as usize;
		let chunklen = chunklen.checked_sub(2).ok_or_else(|| ioe!(InvalidData, "chunk len < 2"))?;
		if chunklen > buf.len() {
			buf = vec![0; chunklen];
		}
		let buf = &mut buf[..chunklen];
		data.read_exact(buf)?;
		let chunk = decompress_chunk(buf)?;
		has_next = u8(data)? != 0;
		Ok(chunk)
	}))
}
