use encoding_rs::SHIFT_JIS;
use hamu::read::le::*;
use hamu::write::le::*;

type Backtrace = Box<std::backtrace::Backtrace>;

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
	#[error("{source}")]
	Io { #[from] source: std::io::Error, backtrace: Backtrace },

	#[error("{source}")]
	Read { #[from] source: hamu::read::Error, backtrace: Backtrace },

	#[error("{source}")]
	Coverage { #[from] source: hamu::read::coverage::Error, backtrace: Backtrace },

	#[error("{source}")]
	Encoding { #[from] source: crate::util::DecodeError, backtrace: Backtrace },

	#[error("{source}")]
	Cast { #[from] source: CastError, backtrace: Backtrace },

	#[error("{assertion}")]
	Assert { assertion: Box<str>, backtrace: Backtrace },
}

impl std::convert::From<String> for ReadError {
	fn from(assertion: String) -> Self {
		Self::Assert {
			assertion: assertion.into(),
			backtrace: std::backtrace::Backtrace::capture().into(),
		}
	}
}

#[derive(Debug, thiserror::Error)]
pub enum WriteError {
	#[error("{source}")]
	Io { #[from] source: std::io::Error, backtrace: Backtrace },

	#[error("{source}")]
	Write { #[from] source: hamu::write::Error, backtrace: Backtrace },

	#[error("{source}")]
	Encoding { #[from] source: crate::util::EncodeError, backtrace: Backtrace },

	#[error("{source}")]
	Cast { #[from] source: CastError, backtrace: Backtrace },

	#[error("{assertion}")]
	Assert { assertion: Box<str>, backtrace: Backtrace },
}

impl std::convert::From<String> for WriteError {
	fn from(assertion: String) -> Self {
		Self::Assert {
			assertion: assertion.into(),
			backtrace: std::backtrace::Backtrace::capture().into(),
		}
	}
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid SJIS string {text:?}")]
pub struct DecodeError { text: String }

pub fn decode(bytes: &[u8]) -> Result<String, DecodeError> {
	let (text, _, error) = SHIFT_JIS.decode(bytes);
	if error {
		return Err(DecodeError { text: text.into_owned() });
	}
	if text.contains('\0') {
		return Err(DecodeError { text: text.into_owned() });
	}
	Ok(text.into_owned())
}

#[derive(Debug, thiserror::Error)]
#[error("Cannot encode {text:?} as SJIS")]
pub struct EncodeError { text: String }

pub fn encode(text: &str) -> Result<Vec<u8>, EncodeError> {
	if text.contains('\0') {
		return Err(EncodeError { text: text.to_owned() });
	}
	let (bytes, _, error) = SHIFT_JIS.encode(text);
	if error {
		return Err(EncodeError { text: text.to_owned() });
	}
	Ok(bytes.into_owned())
}

#[derive(Debug, thiserror::Error)]
#[error("cannot convert {value} into {type_}\n{source}")]
pub struct CastError {
	source: Box<dyn std::error::Error>,
	type_: &'static str,
	value: String,
}

pub fn cast<A, B>(a: A) -> Result<B, CastError> where
	A: std::fmt::Debug + Clone,
	B: TryFrom<A>,
	B::Error: std::error::Error + 'static,
{
	a.clone().try_into().map_err(|e| cast_error::<B>(a, e))
}

pub fn cast_bool(a: impl Into<u64> + std::fmt::Debug + Clone) -> Result<bool, CastError> {
	match a.into() {
		0 => Ok(false),
		1 => Ok(true),
		n => Err(cast_error::<bool>(n, "out of range integral type conversion attempted")),
	}
}

pub fn cast_error<T>(
	val: impl std::fmt::Debug,
	cause: impl Into<Box<dyn std::error::Error>>,
) -> CastError {
	CastError {
		type_: std::any::type_name::<T>(),
		value: format!("{:?}", val),
		source: cause.into()
	}
}

pub trait InExt<'a>: In<'a> {
	fn string(&mut self) -> Result<String, ReadError> {
		let mut buf = Vec::new();
		loop {
			match self.array()? {
				[0] => break,
				[n] => buf.push(n),
			}
		}
		Ok(decode(&buf)?)
	}

	fn multiple<const N: usize, A: PartialEq + std::fmt::Debug>(
		&mut self,
		nil: &[u8],
		mut f: impl FnMut(&mut Self) -> Result<A, ReadError>,
	) -> Result<Vec<A>, ReadError> {
		let mut out = Vec::with_capacity(N);
		let mut has_junk = false;
		for _ in 0..N {
			let i = self.pos();
			if self.slice(nil.len())? == nil {
				has_junk = true;
			} else {
				let j = self.pos();
				self.seek(i)?;
				let v = f(self)?;

				if self.pos() != j {
					return Err(format!("inconsistent position: {i} != {j}").into())
				}

				if has_junk {
					return Err(format!("junk after end: {v:?}").into())
				}

				out.push(v);
			}
		}
		Ok(out)
	}

	fn sized_string<const N: usize>(&mut self) -> Result<String, ReadError> {
		let buf = self.multiple::<N, _>(&[0], |a| Ok(a.u8()?))?;
		Ok(decode(&buf)?)
	}
}
impl<'a, T: In<'a>> InExt<'a> for T {}

pub trait OutExt<L: Eq + std::hash::Hash + std::fmt::Debug> {
	fn string(&mut self, s: &str) -> Result<(), WriteError>;
	fn multiple<const N: usize, A: PartialEq + std::fmt::Debug>(
		&mut self,
		nil: &[u8],
		items: &[A],
		f: impl FnMut(&mut Self, &A) -> Result<(), WriteError>,
	) -> Result<(), WriteError>;
	fn sized_string<const N: usize>(&mut self, s: &str) -> Result<(), WriteError>;
}
impl<L: Eq + std::hash::Hash + std::fmt::Debug + Clone> OutExt<L> for Out<'_, L> {
	fn string(&mut self, s: &str) -> Result<(), WriteError> {
		let s = encode(s)?;
		self.slice(&s);
		self.array([0]);
		Ok(())
	}

	fn multiple<const N: usize, A: PartialEq + std::fmt::Debug>(
		&mut self,
		nil: &[u8],
		items: &[A],
		mut f: impl FnMut(&mut Self, &A) -> Result<(), WriteError>,
	) -> Result<(), WriteError> {
		if items.len() > N {
			return Err(cast_error::<[A; N]>(format!("{items:?}"), "too large").into());
		}
		for i in items {
			f(self, i)?;
		}
		for _ in items.len()..N {
			self.slice(nil);
		}
		Ok(())
	}

	fn sized_string<const N: usize>(&mut self, s: &str) -> Result<(), WriteError> {
		let s = encode(s)?;
		// Not using multiple() here to include the string in the error
		if s.len() > N {
			return Err(cast_error::<[u8; N]>(format!("{s:?}"), "too large").into());
		}
		let mut buf = [0; N];
		buf[..s.len()].copy_from_slice(&s);
		self.array::<N>(buf);
		Ok(())
	}
}

#[cfg(test)]
pub mod test {
	use crate::archive::Archives;

	#[derive(Debug, thiserror::Error)]
	pub enum Error {
		#[error("{source}")]
		Io { #[from] source: std::io::Error, backtrace: std::backtrace::Backtrace },

		#[error(transparent)]
		Read { #[from] #[backtrace] source: crate::util::ReadError },

		#[error(transparent)]
		Write { #[from] #[backtrace] source: crate::util::WriteError },
	}

	lazy_static::lazy_static! {
		pub static ref FC: Archives = Archives::new("../data/fc").unwrap();
	}

	pub fn check_equal<T: PartialEq + std::fmt::Debug>(a: &T, b: &T) -> Result<(), Error> {
		if a != b {
			use similar::{TextDiff, ChangeTag};

			let a = format!("{:#?}", a);
			let b = format!("{:#?}", b);
			let diff = TextDiff::configure().diff_lines(&a, &b);

			for (i, hunk) in diff.unified_diff().iter_hunks().enumerate() {
				if i > 0 {
					println!("\x1B[34m…\x1B[39m");
				}
				for change in hunk.iter_changes() {
					match change.tag() {
						ChangeTag::Delete => print!("\x1B[31m-{change}\x1B[39m"),
						ChangeTag::Insert => print!("\x1B[32m+{change}\x1B[39m"),
						ChangeTag::Equal => print!(" {change}"),
					};
				}
			}
			panic!("{} differs", std::any::type_name::<T>());
		}
		Ok(())
	}

	pub fn check_roundtrip<T>(
		arc: &Archives,
		name: &str,
		read: impl Fn(&Archives, &[u8]) -> Result<T, super::ReadError>,
		write: impl Fn(&Archives, &T) -> Result<Vec<u8>, super::WriteError>,
	) -> Result<T, Error> where
		T: PartialEq + std::fmt::Debug,
	{
		let data = arc.get_decomp(name)?;
		let parsed = read(arc, &data)?;
		let data2 = write(arc, &parsed)?;
		let parsed2 = read(arc, &data2)?;
		check_equal(&parsed, &parsed2)?;
		Ok(parsed2)
	}
}
