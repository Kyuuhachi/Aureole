use either::*;
use encoding_rs::SHIFT_JIS;
use hamu::read::prelude::*;
use hamu::write::prelude::*;

#[derive(Debug, snafu::Snafu)]
#[snafu(display("Invalid SJIS string {text:?}"))]
pub struct DecodeError { text: String }

pub fn decode(bytes: &[u8]) -> Result<String, DecodeError> {
	let (text, _, error) = SHIFT_JIS.decode(bytes);
	snafu::ensure!(!error, DecodeSnafu { text });
	Ok(text.into_owned())
}

#[derive(Debug, snafu::Snafu)]
#[snafu(display("Cannot encode {text:?} as SJIS"))]
pub struct EncodeError { text: String }

pub fn encode(text: &str) -> Result<Vec<u8>, EncodeError> {
	let (bytes, _, error) = SHIFT_JIS.encode(text);
	snafu::ensure!(!error, EncodeSnafu { text });
	Ok(bytes.into_owned())
}

pub trait InExt<'a>: In<'a> {
	fn string(&mut self) -> Result<String, Either<hamu::read::Error, DecodeError>> {
		let mut buf = Vec::new();
		loop {
			match self.array().map_err(Left)? {
				[0] => break,
				[n] => buf.push(n),
			}
		}
		decode(&buf).map_err(Right)
	}
}
impl<'a, T: In<'a>> InExt<'a> for T {}

pub trait OutExt<L: Eq + std::hash::Hash + std::fmt::Debug> {
	fn string(&mut self, s: &str);
}
impl<L: Eq + std::hash::Hash + std::fmt::Debug> OutExt<L> for Out<'_, L> {
	fn string(&mut self, s: &str) {
		assert!(!s.contains('\0'), "{s:?} contains NUL");
		self.slice(&encode(s).unwrap());
		self.array([0]);
	}
}