use either::*;
use encoding_rs::SHIFT_JIS;
use hamu::read::prelude::*;

#[derive(Debug, snafu::Snafu)]
#[snafu(display("Invalid SJIS string {text:?}"))]
pub struct DecodeError { text: String }

pub fn decode(s: &[u8]) -> Result<String, DecodeError> {
	let (text, _, error) = SHIFT_JIS.decode(s);
	snafu::ensure!(!error, DecodeSnafu { text });
	Ok(text.into_owned())
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
