use gospel::read::Reader;
use std::ops::*;

use super::{Backtrace, ensure};

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
	#[error("{source}")]
	Read { #[from] source: gospel::read::Error, backtrace: Backtrace },

	#[error("{source}")]
	Encoding { #[from] source: DecodeError, backtrace: Backtrace },

	#[error("{source}")]
	Cast { #[from] source: super::CastError, backtrace: Backtrace },

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

impl std::convert::From<&str> for ReadError {
	fn from(assertion: &str) -> Self {
		assertion.to_owned().into()
	}
}

impl std::convert::From<std::convert::Infallible> for ReadError {
	fn from(v: std::convert::Infallible) -> Self {
		match v {}
	}
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid SJIS string {text:?}")]
pub struct DecodeError { text: String }

pub fn decode(bytes: &[u8]) -> Result<String, DecodeError> {
	cp932::decode(bytes).map_err(|_| DecodeError { text: cp932::decode_lossy(bytes) })
}


#[extend::ext(name = ReaderExtU)]
pub impl Reader<'_> {
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

				ensure!(self.pos() == j, "inconsistent position: {i} != {j}");
				ensure!(!has_junk, "junk after end: {v:?}");

				out.push(v);
			}
		}
		Ok(out)
	}

	fn multiple_loose<const N: usize, A: PartialEq + std::fmt::Debug>(
		&mut self,
		nil: &[u8],
		mut f: impl FnMut(&mut Self) -> Result<A, ReadError>,
	) -> Result<[Option<A>; N], ReadError> {
		super::array(|| {
			let i = self.pos();
			if self.slice(nil.len())? == nil {
				Ok(None)
			} else {
				let j = self.pos();
				self.seek(i)?;
				let v = f(self)?;
				ensure!(self.pos() == j, "inconsistent position: {i} != {j}");
				Ok(Some(v))
			}
		})
	}

	fn sized_string<const N: usize>(&mut self) -> Result<String, ReadError> {
		let buf = self.multiple::<N, _>(&[0], |a| Ok(a.slice(1)?[0]))?;
		Ok(decode(&buf)?)
	}
}
