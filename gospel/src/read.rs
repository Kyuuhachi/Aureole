#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("out-of-bounds seek to {pos:#X} (size {size:#X})")]
	Seek { pos: usize, size: usize },
	#[error("out-of-bounds read of {pos:#X}+{len} (size {size:#X})")]
	Read { pos: usize, len: usize, size: usize },
	#[error("error at {pos:#X}: {source}")]
	Other { pos: usize, #[source] source: Box<dyn std::error::Error + Send + Sync> },
}

pub type Result<T, E=Error> = std::result::Result<T, E>;

impl Error {
	pub fn pos(&self) -> usize {
		match self {
			Error::Seek { pos, .. } => *pos,
			Error::Read { pos, .. } => *pos,
			Error::Other { pos, .. } => *pos,
		}
	}

	pub fn pos_mut(&mut self) -> &mut usize {
		match self {
			Error::Seek { pos, .. } => pos,
			Error::Read { pos, .. } => pos,
			Error::Other { pos, .. } => pos,
		}
	}
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("mismatched {type_}. expected: {expected}, got: {got}", type_ = std::any::type_name::<T>())]
pub struct CheckError<T: std::fmt::Display> {
	pub expected: T,
	pub got: T,
}

#[derive(Clone, Debug, thiserror::Error)]
pub struct CheckBytesError {
	pub expected: Vec<u8>,
	pub got: Vec<u8>,
}

impl std::fmt::Display for CheckBytesError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let mut got = Vec::new();
		let mut exp = Vec::new();
		for (&g, &e) in std::iter::zip(&self.got, &self.expected) {
			got.extend(std::ascii::escape_default(g).map(char::from));
			exp.extend(std::ascii::escape_default(e).map(char::from));
			while got.len() < exp.len() { got.push('░') }
			while exp.len() < got.len() { exp.push('░') }
		}
		writeln!(f, "mismatched bytes.")?;
		writeln!(f, "expected: b\"{}\"", String::from_iter(exp))?;
		write  !(f, "got:      b\"{}\"", String::from_iter(got))
	}
}

/// An incremental reader from a byte slice.
///
/// Cloning this type is cheap, but it does not implement [`Copy`] for similar reasons as
/// [`Range`](`std::ops::Range`).
#[derive(Clone)]
pub struct Reader<'a> {
	pos: usize,
	data: &'a [u8],
}

impl<'a> std::fmt::Debug for Reader<'a> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Reader")
			.field("pos", &self.pos)
			.field("data", &format_args!("[_; {}]", self.data.len()))
			.finish()
	}
}

impl<'a> Reader<'a> {
	/// Constructs a new `Reader`.
	pub fn new(data: &'a [u8]) -> Reader<'a> {
		Self {
			pos: 0,
			data,
		}
	}

	/// Reads a slice of data from the input. No copying is done.
	///
	/// Returns an error if there is not enough data left, in which case the read position is
	/// unchanged.
	pub fn slice(&mut self, len: usize) -> Result<&'a [u8]> {
		if len > self.remaining().len() {
			return Err(Error::Read { pos: self.pos(), len, size: self.len() });
		}
		let pos = self.pos;
		self.pos += len;
		Ok(&self.data[pos..pos+len])
	}

	/// Reads a fixed-size slice of data from the input.
	///
	/// Handles errors identically to [`slice`](`Self::slice`).
	pub fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
		let mut x = [0; N];
		self.read_into(&mut x)?;
		Ok(x)
	}

	/// Reads a slice of data into a preexisting buffer.
	///
	/// Handles errors identically to [`slice`](`Self::slice`).
	pub fn read_into(&mut self, buf: &mut [u8]) -> Result<()> {
		buf.copy_from_slice(self.slice(buf.len())?);
		Ok(())
	}

	/// Returns the read position of the reader.
	///
	/// To change the position, use [`seek`](`Self::seek`) or [`at`](`Self::at`).
	#[must_use]
	pub fn pos(&self) -> usize {
		self.pos
	}

	/// Returns the total length of the input.
	///
	/// Note that this is the total length, which does not change. See
	/// [`remaining`](`Self::remaining`) for the number of bytes left that can be read.
	#[must_use]
	pub fn len(&self) -> usize {
		self.data.len()
	}

	/// Returns true if there are no more bytes left to read.
	///
	/// Note that unlike slices, this is based on [`remaining`](`Self::remaining`), not
	/// [`len`](`Self::len`).
	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.remaining().is_empty()
	}

	/// Returns the remaining data in the buffer.
	#[must_use]
	pub fn remaining(&self) -> &'a [u8] {
		&self.data[self.pos()..]
	}

	/// Returns the data being read from.
	///
	/// This is the full slice; for only the remainder, see [`remaining`](`Self::remaining`).
	#[must_use]
	pub fn data(&self) -> &'a [u8] {
		self.data
	}

	/// Sets the read position.
	///
	/// Returns an error if the position is out of bounds.
	///
	/// See [`at`](`Self::at`) for a version that returns a copy.
	pub fn seek(&mut self, pos: usize) -> Result<()> {
		if pos > self.len() {
			return Err(Error::Seek { pos, size: self.len() })
		}
		self.pos = pos;
		Ok(())
	}

	/// Returns a copy of the reader at the specified position.
	///
	/// See also [`ptrN`](`Self::ptrN`) for a shorthand for the common pattern of
	/// `f.clone().at(f.u32()? as usize)?`.
	pub fn at(&self, pos: usize) -> Result<Self> {
		let mut a = self.clone();
		a.seek(pos)?;
		Ok(a)
	}

	/// Rounds the read position up to the next multiple of `size`.
	pub fn align(&mut self, size: usize) -> Result<&'a [u8]> {
		self.slice((size-(self.pos()%size))%size)
	}

	/// Reads a number of bytes and returns an error if they are not as expected.
	///
	/// If it does not match, the read position is not affected.
	pub fn check(&mut self, v: &[u8]) -> Result<()> {
		let pos = self.pos();
		let u = self.slice(v.len())?;
		if u != v {
			self.pos = pos;
			return Err(Error::Other { pos, source: CheckBytesError {
				got:      u.to_owned(),
				expected: v.to_owned(),
			}.into() })
		}
		Ok(())
	}
}

#[cfg(doc)]
#[doc(hidden)]
pub type T = ();

/// Functions for reading primitives from the stream. The underlying functions are hidden from
/// the docs for brevity.
///
/// Supported primitives are `u8..=u128`, `i8..=i128`, `f32`, `f64`.
///
/// The functions are suffixed with either `_le` or `_be`, for endianness. To use unsuffixed
/// versions, import either the [`Le`] or [`Be`] trait.
#[cfg(doc)]
impl Reader<'_> {
	// Could add a shitload of #[doc(alias = _)], but don't wanna.

	/// Read a primitive from the input.
	pub fn T(&mut self) -> Result<T> {}

	/// Read a primitive from the input, giving an error if it is not as expected.
	///
	/// If it not match, the read position is not affected.
	pub fn check_T(&mut self, v: T) -> Result<()> {}

	/// Read a `uN` primitive from the input, and return a new `Reader` at that position.
	///
	/// This is a shorthand for the common pattern of `f.clone().at(f.uN()? as usize)?`.
	///
	/// Note that no checking is made that the value actually fits inside a `usize`; the higher bits are simply discarded.
	pub fn ptrN(&mut self) -> Result<Self> {}
}

mod seal { pub trait Sealed: Sized {} }
impl seal::Sealed for Reader<'_> {}

macro_rules! primitives {
	(
		$(#[$trait_attrs:meta])* trait $trait:ident;
		$suf:ident, $conv:ident;
		{ $($type:ident),* }
		{ $($ptr:tt),* }
	) => { paste::paste! {
		#[doc(hidden)]
		impl<'a> Reader<'a> {
			$(pub fn [<$type $suf>](&mut self) -> Result<$type> {
				Ok($type::$conv(self.array()?))
			})*
			$(pub fn [<check_ $type $suf>](&mut self, v: $type) -> Result<()> {
				let pos = self.pos();
				let u = self.[< $type $suf >]()?;
				if u != v {
					self.pos = pos;
					return Err(Error::Other { pos, source: CheckError {
						got: u,
						expected: v,
					}.into() })
				}
				Ok(())
			})*
			$(pub fn [<ptr$ptr $suf>](&mut self) -> Result<Self> {
				self.clone().at(self.[<u$ptr $suf>]()? as usize)
			})*
		}

		$(#[$trait_attrs])*
		pub trait $trait: seal::Sealed {
			$(#[doc(hidden)] fn $type(&mut self) -> Result<$type>;)*
			$(#[doc(hidden)] fn [<check_ $type>](&mut self, v: $type) -> Result<()>;)*
			$(#[doc(hidden)] fn [<ptr $ptr>](&mut self) -> Result<Self>;)*
		}

		impl<'a> $trait for Reader<'a> {
			$(#[doc(hidden)] fn $type(&mut self) -> Result<$type> {
				self.[<$type $suf>]()
			})*
			$(#[doc(hidden)] fn [<check_ $type>](&mut self, v: $type) -> Result<()> {
				self.[<check_ $type $suf>](v)
			})*
			$(#[doc(hidden)] fn [<ptr$ptr>](&mut self) -> Result<Self> {
				self.[<ptr$ptr $suf>]()
			})*
		}
	} }
}

primitives!(
	/// Allows reading little-endian primitives without `_le` suffix.
	///
	/// It is recommended to import this as `use gospel::read::Le as _;`.
	trait Le;
	_le, from_le_bytes;
	{
		u8, u16, u32, u64, u128,
		i8, i16, i32, i64, i128,
		f32, f64
	}
	{ 8, 16, 32, 64, 128 }
);
primitives!(
	/// Allows reading little-endian primitives without `_be` suffix.
	///
	/// It is recommended to import this as `use gospel::read::Be as _;`.
	trait Be;
	_be, from_be_bytes;
	{
		u8, u16, u32, u64, u128,
		i8, i16, i32, i64, i128,
		f32, f64
	}
	{ 8, 16, 32, 64, 128 }
);
