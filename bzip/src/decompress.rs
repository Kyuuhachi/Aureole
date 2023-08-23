use gospel::read::{Reader, Le};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error(transparent)]
	Read { #[from] source: gospel::read::Error },
	#[error("attempted to repeat {count} bytes from offset -{offset}, but only have {len} bytes")]
	BadRepeat {
		count: usize,
		offset: usize,
		len: usize,
	},
	#[error("invalid frame")]
	Frame,
}

type Result<A, E=Error> = std::result::Result<A, E>;

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

	fn constant(&mut self, n: usize, f: &mut Reader) -> Result<()> {
		let b = f.u8()?;
		for _ in 0..n {
			self.out.push(b);
		}
		Ok(())
	}

	fn verbatim(&mut self, n: usize, f: &mut Reader) -> Result<()> {
		for _ in 0..n {
			let b = f.u8()?;
			self.out.push(b);
		}
		Ok(())
	}

	fn repeat(&mut self, n: usize, o: usize, _f: &mut Reader) -> Result<()> {
		if !(1..=self.out.len()-self.start).contains(&o) {
			return Err(Error::BadRepeat { count: n, offset: o, len: self.out.len() })
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

	fn bit(&mut self, f: &mut Reader) -> Result<bool> {
		if self.nextbit == 0 {
			self.renew_bits(f)?;
		}
		let v = self.bits & self.nextbit != 0;
		self.nextbit <<= 1;
		Ok(v)
	}

	fn renew_bits(&mut self, f: &mut Reader) -> Result<()> {
		self.bits = f.u16()?;
		self.nextbit = 1;
		Ok(())
	}

	fn bits(&mut self, n: usize, f: &mut Reader) -> Result<usize> {
		let mut x = 0;
		for _ in 0..n%8 {
			x = x << 1 | usize::from(self.bit(f)?);
		}
		for _ in 0..n/8 {
			x = x << 8 | f.u8()? as usize;
		}
		Ok(x)
	}

	fn read_count(&mut self, f: &mut Reader) -> Result<usize> {
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

fn decompress_mode2(data: &[u8], w: &mut OutBuf) -> Result<(), Error> {
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
fn decompress_mode1(data: &[u8], w: &mut OutBuf) -> Result<(), Error> {
	let f = &mut Reader::new(data);

	let mut last_o = 0;
	while !f.is_empty() {
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

pub fn decompress(data: &[u8], w: &mut Vec<u8>) -> Result<()> {
	let w = &mut OutBuf::new(w);
	if data.first() == Some(&0) {
		decompress_mode2(data, w)
	} else {
		decompress_mode1(data, w)
	}
}
