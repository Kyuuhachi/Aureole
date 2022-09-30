pub mod prelude {
	pub use super::{In, InBase, Bytes};
}

pub mod coverage;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("out-of-bounds seek to {pos:#X} (size {size:#X})")]
	Seek { pos: usize, size: usize },
	#[error("out-of-bounds read of {pos:#X}+{len} (size {size:#X})")]
	Read { pos: usize, len: usize, size: usize },
	#[error("mismatched {type_} at {pos:#X}\n  got:      {got}\n  expected: {expected}")]
	Check { pos: usize, type_: String, got: String, expected: String },
}
pub type Result<T, E=Error> = std::result::Result<T, E>;

#[allow(clippy::len_without_is_empty)]
pub trait InBase<'a> {
	fn pos(&self) -> usize;
	fn len(&self) -> usize;
	fn seek(&mut self, pos: usize) -> Result<()>;
	fn slice(&mut self, len: usize) -> Result<&'a [u8]>;
	fn dump(&self) -> beryl::Dump;
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
			return Err(Error::Check {
				pos,
				type_:    format!("[u8; {}]", v.len()),
				got:      format!("b\"{}\"", got.into_iter().collect::<String>()),
				expected: format!("b\"{}\"", exp.into_iter().collect::<String>()),
			})
		}
		Ok(())
	}
}

impl<'a, T> In<'a> for T where T: InBase<'a> + ?Sized {}

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
						return Err(Error::Check {
							pos,
							type_: stringify!($type).to_owned(),
							got: u.to_string(),
							expected: v.to_string(),
						})
					}
					Ok(())
				}
			)*
		}
		impl<'a, T: In<'a> + ?Sized> $name<'a> for T {}

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
		if pos > self.len() {
			return Err(Error::Seek { pos, size: self.len() })
		}
		self.pos = pos;
		Ok(())
	}

	fn slice(&mut self, len: usize) -> Result<&'a [u8]> {
		if len > self.remaining() {
			return Err(Error::Read { pos: self.pos(), len, size: self.len() });
		}
		let pos = self.pos;
		self.pos += len;
		Ok(&self.data[pos..pos+len])
	}

	fn dump(&self) -> beryl::Dump {
		let mut cursor = std::io::Cursor::new(&self.data);
		cursor.set_position(self.pos as u64);
		beryl::Dump::new(cursor, self.pos)
			.num_width_from(self.len())
	}
}
