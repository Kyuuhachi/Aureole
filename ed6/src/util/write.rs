use encoding_rs::SHIFT_JIS;
use hamu::write::le::*;
use std::ops::*;

use super::{Backtrace, ensure};

#[derive(Debug, thiserror::Error)]
pub enum WriteError {
	#[error("{source}")]
	Io { #[from] source: std::io::Error, backtrace: Backtrace },

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
	if text.contains('\0') {
		return Err(EncodeError { text: text.to_owned() });
	}
	let (bytes, _, error) = SHIFT_JIS.encode(text);
	if error {
		return Err(EncodeError { text: text.to_owned() });
	}
	Ok(bytes.into_owned())
}

pub trait OutExt: Out {
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

	fn sized_string<const N: usize>(&mut self, s: &str) -> Result<(), WriteError> {
		let s = encode(s)?;
		// Not using multiple() here to include the string in the error
		ensure!(s.len() <= N, super::cast_error::<[u8; N]>(format!("{s:?}"), "too large").into());
		let mut buf = [0; N];
		buf[..s.len()].copy_from_slice(&s);
		self.array::<N>(buf);
		Ok(())
	}

	fn name_desc(&mut self, nd: &super::NameDesc) -> Result<(), WriteError> where Self: OutDelay {
		let super::NameDesc { name, desc } = nd;
		let l1 = Label::new();
		let l2 = Label::new();
		self.delay_u16(l1);
		self.delay_u16(l2);
		self.label(l1);
		self.string(name)?;
		self.label(l2);
		self.string(desc)?;
		Ok(())
	}
}
impl<T: Out + ?Sized> OutExt for T {}
