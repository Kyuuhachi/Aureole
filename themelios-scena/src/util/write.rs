use hamu::write::le::*;
use std::ops::*;

use super::{Backtrace, ensure};

#[derive(Debug, thiserror::Error)]
pub enum WriteError {
	#[error("{source}")]
	Write { #[from] source: hamu::write::Error, backtrace: Backtrace },

	#[error("{source}")]
	Encoding { #[from] source: EncodeError, backtrace: Backtrace },

	#[error("{source}")]
	Cast { #[from] source: super::CastError, backtrace: Backtrace },

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

impl std::convert::From<&str> for WriteError {
	fn from(assertion: &str) -> Self {
		assertion.to_owned().into()
	}
}

impl std::convert::From<std::convert::Infallible> for WriteError {
	fn from(v: std::convert::Infallible) -> Self {
		match v {}
	}
}

#[derive(Debug, thiserror::Error)]
#[error("Cannot encode {text:?} as SJIS")]
pub struct EncodeError { text: String }

pub fn encode(text: &str) -> Result<Vec<u8>, EncodeError> {
	cp932::encode(text).map_err(|_| EncodeError { text: text.to_owned() })
}

pub trait WriteStreamExt1: WriteStream {
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
		ensure!(items.len() <= N, super::cast_error::<[A; N]>(format!("{items:?}"), "too large").into());
		for i in items {
			f(self, i)?;
		}
		for _ in items.len()..N {
			self.slice(nil);
		}
		Ok(())
	}

	fn multiple_loose<const N: usize, A: PartialEq + std::fmt::Debug>(
		&mut self,
		nil: &[u8],
		items: &[Option<A>; N],
		mut f: impl FnMut(&mut Self, &A) -> Result<(), WriteError>,
	) -> Result<(), WriteError> {
		for i in items {
			if let Some(i) = i {
				f(self, i)?;
			} else {
				self.slice(nil);
			}
		}
		Ok(())
	}

	fn sized_string<const N: usize>(&mut self, s: &str) -> Result<(), WriteError> {
		let s = encode(s)?;
		// Not using multiple() here to include the string in the error
		ensure!(s.len() <= N, super::cast_error::<[u8; N]>(format!("{s:?}"), "too large").into());
		let mut buf = [0; N];
		buf[..s.len()].copy_from_slice(&s);
		self.array::<N>(buf);
		Ok(())
	}

	#[deprecated]
	fn name_desc(&mut self, nd: &super::NameDesc) -> Result<(), WriteError> where Self: Write {
		let super::NameDesc { name, desc } = nd;
		let (l1, l1_) = Label::new();
		let (l2, l2_) = Label::new();
		self.delay_u16(l1);
		self.delay_u16(l2);
		self.label(l1_);
		self.string(name)?;
		self.label(l2_);
		self.string(desc)?;
		Ok(())
	}
}
impl<T: WriteStream + ?Sized> WriteStreamExt1 for T {}

#[extend::ext]
pub impl Writer {
	fn ptr(&mut self) -> Writer {
		let mut g = Writer::new();
		self.delay_u16(g.here());
		g
	}

	fn ptr32(&mut self) -> Writer {
		let mut g = Writer::new();
		self.delay_u32(g.here());
		g
	}
}
