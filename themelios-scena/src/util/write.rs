use gospel::write::Writer;
use super::ensure;

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

#[extend::ext(name = WriterExtU)]
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
}
