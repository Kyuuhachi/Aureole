
pub trait Bits {
	fn bit(&mut self, v: bool);
	fn bits(&mut self, n: usize, v: usize);
}

pub struct BitW {
	out: Vec<u8>,
	nextbit: u16,
	bitpos: usize,
}

impl BitW {
	pub fn new() -> Self {
		Self { out: vec![0,0], nextbit: 0x0100, bitpos: 0 }
	}

	pub fn finish(self) -> Vec<u8> {
		self.out
	}
}

impl Bits for BitW {
	fn bit(&mut self, v: bool) {
		if self.nextbit == 0 {
			self.bitpos = self.out.len();
			self.out.extend([0,0]);
			self.nextbit = 0x0001;
		}
		if v {
			if self.nextbit < 256 {
				self.out[self.bitpos] |= self.nextbit as u8;
			} else {
				self.out[self.bitpos+1] |= (self.nextbit>>8) as u8;
			}
		}
		self.nextbit <<= 1;
	}

	fn bits(&mut self, n: usize, v: usize) {
		assert!(v < (1<<n), "{v} < (1<<{n})");
		for k in (n/8*8..n).rev() {
			self.bit((v>>k) & 1 != 0);
		}
		for k in (0..n/8).rev() {
			self.out.push((v>>(k*8)) as u8);
		}
	}
}

struct BitC(usize);

impl Bits for BitC {
	fn bit(&mut self, _v: bool) {
		self.0 += 1;
	}

	fn bits(&mut self, n: usize, _v: usize) {
		self.0 += n;
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressCommand {
	Byte { val: u8 },
	Match { offset: usize, count: usize },
	Fill { val: u8, count: usize },
}

impl CompressCommand {
	pub const NO_COMMAND: CompressCommand = CompressCommand::Match { offset: 0, count: 0 };

	pub fn size(self) -> (usize, usize) {
		let mut c = BitC(0);
		let bytes = self.write_to(&mut c);
		let bits = c.0;
		(bytes, bits)
	}

	pub fn write_to(self, b: &mut impl Bits) -> usize {
		match self {
			CompressCommand::Byte { val: v } => {
				b.bit(false);
				b.bits(8, v as usize);
				1
			}

			CompressCommand::Match { offset: o, count: n } => {
				assert!(n >= 2);
				assert!(n <= 269);
				assert!(o < 1<<13);
				b.bit(true);
				if o >= 256 {
					b.bit(true);
					b.bits(13, o)
				} else {
					b.bit(false);
					b.bits(8, o);
				}

				for i in 2..=5 {
					if n > i {
						b.bit(false);
					}
				}
				if n < 6 {
					b.bit(true);
				} else if n < 14 {
					b.bit(true);
					b.bits(3, n-6);
				} else {
					b.bit(false);
					b.bits(8, n-14);
				}

				n
			}

			CompressCommand::Fill { val: v, count: n } => {
				assert!(n >= 14);
				assert!(n < 14+(1<<12));
				b.bit(true);
				b.bit(true);
				b.bits(13, 1);
				if n >= 30 { // This doesn't make sense, but it seems to be what the algorithm says.
					b.bit(true);
					b.bits(12, n-14)
				} else {
					b.bit(false);
					b.bits(4, n-14);
				}
				b.bits(8, v as usize);

				n
			}
		}
	}
}
