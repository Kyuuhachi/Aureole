pub mod prelude {
	pub use super::{In, InBase, Dump, Bytes};
}

pub mod coverage;

#[derive(Debug, snafu::Snafu)]
pub enum Error {
	#[snafu(display("out-of-bounds seek to {pos:#X} (size {size:#X})"))]
	Seek { pos: usize, size: usize },
	#[snafu(display("out-of-bounds read of {pos:#X}+{len} (size {size:#X})"))]
	Read { pos: usize, len: usize, size: usize },
	#[snafu(display("mismatched {type_} at {pos:#X}\n  got:      {got}\n  expected: {expected}"))]
	Check { pos: usize, type_: String, got: String, expected: String },
}
pub type Result<T, E=Error> = std::result::Result<T, E>;

#[allow(clippy::len_without_is_empty)]
pub trait InBase<'a> {
	fn pos(&self) -> usize;
	fn len(&self) -> usize;
	fn seek(&mut self, pos: usize) -> Result<()>;
	fn slice(&mut self, len: usize) -> Result<&'a [u8]>;
}

pub trait In<'a>: InBase<'a> {
	fn remaining(&self) -> usize {
		self.len() - self.pos()
	}

	fn at(mut self, pos: usize) -> Result<Self> where Self: Sized {
		self.seek(pos)?;
		Ok(self)
	}

	fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
		Ok(self.slice(N)?.try_into().unwrap())
	}

	fn align(&mut self, size: usize) -> Result<()> {
		self.slice((size-(self.pos()%size))%size)?;
		Ok(())
	}

	fn check(&mut self, v: &[u8]) -> Result<()> {
		let pos = self.pos();
		let u = self.slice(v.len())?;
		if u != v {
			let _ = self.seek(pos);
			let mut got = Vec::new();
			let mut exp = Vec::new();
			for (&g, &e) in std::iter::zip(u, v) {
				got.extend(std::ascii::escape_default(g).map(char::from));
				exp.extend(std::ascii::escape_default(e).map(char::from));
				while got.len() < exp.len() { got.push('░') }
				while exp.len() < got.len() { exp.push('░') }
			}
			return CheckSnafu {
				pos,
				type_:    format!("[u8; {}]", v.len()),
				got:      format!("b\"{}\"", got.into_iter().collect::<String>()),
				expected: format!("b\"{}\"", exp.into_iter().collect::<String>()),
			}.fail()
		}
		Ok(())
	}
}

impl<'a, T> In<'a> for T where T: InBase<'a> {}

pub trait Dump<'a>: In<'a> {
	fn dump(&self) -> beryl::Dump;
}

macro_rules! primitives {
	($name:ident, $suf:ident, $conv:ident; $($type:ident),*) => { paste::paste! {
		pub trait $name<'a>: In<'a> {
			$(
				fn $type(&mut self) -> Result<$type> {
					self.[<$type _ $suf>]()
				}

				fn [<$type _ $suf>](&mut self) -> Result<$type> {
					Ok($type::$conv(self.array()?))
				}

				fn [<check_ $type>](&mut self, v: $type) -> Result<()> {
					self.[<check_ $type _ $suf>](v)
				}

				fn [<check_ $type _ $suf>](&mut self, v: $type) -> Result<()> {
					let pos = self.pos();
					let u = $name::$type(self)?;
					if u != v {
						let _ = self.seek(pos);
						return CheckSnafu {
							pos,
							type_: stringify!($type).to_owned(),
							got: u.to_string(),
							expected: v.to_string(),
						}.fail()
					}
					Ok(())
				}
			)*
		}
		impl<'a, T: In<'a>> $name<'a> for T {}

		pub mod $suf {
			pub use super::prelude::*;
			pub use super::$name;
		}
	} }
}

primitives!(InLe, le, from_le_bytes; u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);
primitives!(InBe, be, from_be_bytes; u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

#[derive(Clone)]
pub struct Bytes<'a> {
	data: &'a [u8],
	pos: usize,
}

impl<'a> Bytes<'a> {
	pub fn new(data: &'a [u8]) -> Self {
		Self {
			data,
			pos: 0,
		}
	}
}

impl<'a> InBase<'a> for Bytes<'a> {
	fn pos(&self) -> usize {
		self.pos
	}

	fn len(&self) -> usize {
		self.data.len()
	}

	fn seek(&mut self, pos: usize) -> Result<()> {
		snafu::ensure!(pos <= self.len(), SeekSnafu { pos, size: self.len() });
		self.pos = pos;
		Ok(())
	}

	fn slice(&mut self, len: usize) -> Result<&'a [u8]> {
		snafu::ensure!(len <= self.remaining(), ReadSnafu { pos: self.pos(), len, size: self.len() });
		let pos = self.pos;
		self.pos += len;
		Ok(&self.data[pos..pos+len])
	}
}

impl<'a> Dump<'a> for Bytes<'a> {
	fn dump(&self) -> beryl::Dump {
		let mut cursor = std::io::Cursor::new(&self.data);
		cursor.set_position(self.pos as u64);
		beryl::Dump::new(cursor, self.pos)
			.num_width_from(self.len())
	}
}
