use gospel::write::{Writer, Le as _};
use super::ensure;

use glam::Vec3;
use crate::types::{Pos2, Pos3};

type Backtrace = std::backtrace::Backtrace;

#[derive(Debug, thiserror::Error)]
pub enum WriteError {
	#[error("{source}")]
	Write { #[from] source: gospel::write::Error, backtrace: Backtrace },

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
			backtrace: std::backtrace::Backtrace::capture(),
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

#[extend::ext(name = WriterExt)]
pub impl Writer {
	fn string(&mut self, s: &str) -> Result<(), WriteError> {
		let s = encode(s)?;
		self.slice(&s);
		self.array([0]);
		Ok(())
	}

	fn sized_string<const N: usize>(&mut self, s: &str) -> Result<(), WriteError> {
		let s = encode(s)?;
		ensure!(s.len() <= N, super::cast_error::<[u8; N]>(format!("{s:?}"), "too large").into());
		let mut buf = [0; N];
		buf[..s.len()].copy_from_slice(&s);
		self.array::<N>(buf);
		Ok(())
	}

	fn pos2(&mut self, p: Pos2) {
		self.i32(p.x);
		self.i32(p.z);
	}

	fn pos3(&mut self, p: Pos3) {
		self.i32(p.x);
		self.i32(p.y);
		self.i32(p.z);
	}


	fn vec3(&mut self, p: Vec3) {
		self.f32(p.x);
		self.f32(p.y);
		self.f32(p.z);
	}
}
