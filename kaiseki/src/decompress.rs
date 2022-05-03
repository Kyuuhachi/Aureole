use bitmatch::bitmatch;
use hamu::read::{In, Le};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Read error")]
	Read(#[from] hamu::read::Error),
	#[error("Tried to repeat {count} bytes from offset 0")]
	ZeroRepeat { count: usize },
	#[error("Tried to repeat {count} bytes from offset -{offset}, but only have {len}")]
	TooLongRepeat { count: usize, offset: usize, len: usize },
	#[error("Bytes remaining at end: {bytes:02X?}")]
	RemainingBytes { bytes: Vec<u8> },
	#[error("Bits remaining at end: {bits:nbits$b}")]
	RemainingBits { bits: u16, nbits: usize },
}
pub type Result<T, E=Error> = std::result::Result<T, E>;

struct Ctx<'b> {
	out: Vec<u8>,
	data: In<'b>,
	bits: u16,
	bitidx: usize,
}

impl Ctx<'_> {
	fn byte(&mut self) -> Result<u8> {
		let v = self.data.u8()?;
		Ok(v)
	}

	fn bit(&mut self) -> Result<bool> {
		if self.bitidx == 16 {
			self.renew_bits()?;
			self.bitidx = 0;
		}
		let v = self.bits & (1 << self.bitidx) != 0;
		self.bitidx += 1;
		Ok(v)
	}

	fn renew_bits(&mut self) -> Result<()> {
		self.bits = self.data.u16()?;
		Ok(())
	}

	fn bits(&mut self, n: usize) -> Result<usize> {
		let mut x = 0;
		for _ in 0..n%8 {
			x = x << 1 | if self.bit()? { 1 } else { 0 };
		}
		for _ in 0..n/8 {
			x = extend(x, self)?;
		}
		Ok(x)
	}
}

// These can't be associated since c needs to be last
fn extend(acc: usize, c: &mut Ctx) -> Result<usize> {
	Ok(acc << 8 | c.byte()? as usize)
}

fn extend_if(v: bool, acc: usize, c: &mut Ctx) -> Result<usize> {
	Ok(if v { extend(acc, c)? } else { acc })
}

fn constant(count: usize, c: &mut Ctx) -> Result<()> {
	let b = c.byte()?;
	for _ in 0..count {
		c.out.push(b);
	}
	Ok(())
}

fn verbatim(count: usize, c: &mut Ctx) -> Result<()> {
	for _ in 0..count {
		let b = c.byte()?;
		c.out.push(b);
	}
	Ok(())
}

fn repeat(offset: usize, count: usize, c: &mut Ctx) -> Result<()> {
	if offset == 0 { return Err(Error::ZeroRepeat { count }) }
	if offset > c.out.len() { return Err(Error::TooLongRepeat { count, offset, len: c.out.len() }) }
	for _ in 0..count {
		c.out.push(c.out[c.out.len()-offset]);
	}
	Ok(())
}

fn repeat2(offset: usize, c: &mut Ctx) -> Result<()> {
	let count = match () {
		() if c.bit()? => 2,
		() if c.bit()? => 3,
		() if c.bit()? => 4,
		() if c.bit()? => 5,
		() if c.bit()? => 6 + c.bits(3)?,
		()             => 14 + c.bits(8)?
	};
	repeat(offset, count, c)?;
	Ok(())
}

#[bitmatch]
pub fn decompress_chunk(data: &[u8]) -> Result<Vec<u8>> {
	let mut c_ = Ctx {
		out: Vec::with_capacity(0xFFF0),
		data: In::new(data),
		bits: 0,
		bitidx: 0,
	};
	let c = &mut c_;

	if data[0] == 0 {
		c.renew_bits()?;
		c.bitidx = 8;

		loop {
			match () {
				() if !c.bit()? => verbatim(1, c)?,
				() if !c.bit()? => repeat2(c.bits(8)?, c)?,
				() => match c.bits(13)? {
					0 => break,
					1 => constant(14 + extend_if(c.bit()?, c.bits(4)?, c)?, c)?,
					x => repeat2(x, c)?,
				},
			}
		}

		if c.data.remaining() != 0 {
			return Err(Error::RemainingBytes { bytes: c.data.slice(c.data.remaining())?.to_owned() });
		}

		let bits = c.bits as u32 >> c.bitidx;
		if bits != 0 {
			return Err(Error::RemainingBits { bits: c.bits, nbits: 16-c.bitidx })
		}
	} else {
		let mut last_o = 0;
		while c.data.remaining() > 0 {
			#[bitmatch] match c.byte()? as usize {
				"00xnnnnn" => verbatim(    extend_if(x == 1, n, c)?, c)?,
				"010xnnnn" => constant(4 + extend_if(x == 1, n, c)?, c)?,
				"011nnnnn" => repeat(last_o, n, c)?,
				"1nnooooo" => { last_o = extend(o, c)?; repeat(last_o, 4 + n, c)?; },
			}
		}
	}

	Ok(c_.out)
}

#[tracing::instrument(skip(data))]
pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
	let mut out = Vec::new();
	let mut i = In::new(data);
	loop {
		let chunklen = i.u16()? as usize;
		let chunk = i.slice(chunklen - 2)?;
		out.append(&mut decompress_chunk(chunk)?);
		if i.u8()? == 0 { break }
	}
	Ok(out)
}
