use gospel::read::{Reader, Le as _};

use glam::Vec3;
use crate::types::{Pos2, Pos3};

type Backtrace = std::backtrace::Backtrace;

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
			backtrace: std::backtrace::Backtrace::capture(),
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

#[extend::ext(name = ReaderExt)]
pub impl Reader<'_> {
	fn string(&mut self) -> Result<String, ReadError> {
		let mut s = self.clone();
		while self.array()? != [0] {}
		let data = s.slice(self.pos() - s.pos() - 1)?;
		Ok(decode(data)?)
	}

	fn sized_string<const N: usize>(&mut self) -> Result<String, ReadError> {
		let d = self.slice(N)?;
		let len = d.iter().position(|a| *a == 0).unwrap_or(d.len());
		Ok(decode(&d[..len])?)
	}

	fn pos2(&mut self) -> Result<Pos2, gospel::read::Error> {
		Ok(Pos2 { x: self.i32()?, z: self.i32()? })
	}

	fn pos3(&mut self) -> Result<Pos3, gospel::read::Error> {
		Ok(Pos3 { x: self.i32()?, y: self.i32()?, z: self.i32()? })
	}

	fn vec3(&mut self) -> Result<Vec3, gospel::read::Error> {
		Ok(Vec3 { x: self.f32()?, y: self.f32()?, z: self.f32()? })
	}
}
