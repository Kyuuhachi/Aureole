use encoding_rs::SHIFT_JIS;
use hamu::read::prelude::*;
use hamu::write::prelude::*;

#[derive(Debug, snafu::Snafu)]
pub enum ReadError {
	#[snafu(display("{source}"), context(false))]
	Io { source: std::io::Error, backtrace: snafu::Backtrace },

	#[snafu(display("{source}"), context(false))]
	Read { source: hamu::read::Error, backtrace: snafu::Backtrace },

	#[snafu(display("{source}"), context(false))]
	Coverage { source: hamu::read::coverage::Error, backtrace: snafu::Backtrace },

	#[snafu(display("{source}"), context(false))]
	Encoding { source: crate::util::DecodeError, backtrace: snafu::Backtrace },

	#[snafu(display("{source}"), context(false))]
	Cast { source: CastError, backtrace: snafu::Backtrace },

	#[snafu(whatever, display("{}", source.as_ref().map_or(message.into(), |source| format!("{message}\n{source}"))))]
	Whatever {
		#[snafu(source(from(Box<dyn std::error::Error>, Some)))]
		source: Option<Box<dyn std::error::Error>>,
		message: String,
		backtrace: snafu::Backtrace,
	},
}

#[derive(Debug, snafu::Snafu)]
pub enum WriteError {
	#[snafu(display("{source}"), context(false))]
	Io { source: std::io::Error, backtrace: snafu::Backtrace },

	#[snafu(display("{source}"), context(false))]
	Write { source: hamu::write::Error, backtrace: snafu::Backtrace },

	#[snafu(display("{source}"), context(false))]
	Encoding { source: crate::util::EncodeError, backtrace: snafu::Backtrace },

	#[snafu(display("{source}"), context(false))]
	Cast { source: CastError, backtrace: snafu::Backtrace },

	#[snafu(whatever, display("{}", source.as_ref().map_or(message.into(), |source| format!("{message}\n{source}"))))]
	Whatever {
		#[snafu(source(from(Box<dyn std::error::Error>, Some)))]
		source: Option<Box<dyn std::error::Error>>,
		message: String,
		backtrace: snafu::Backtrace,
	},
}

#[derive(Debug, snafu::Snafu)]
#[snafu(display("Invalid SJIS string {text:?}"))]
pub struct DecodeError { text: String }

pub fn decode(bytes: &[u8]) -> Result<String, DecodeError> {
	let (text, _, error) = SHIFT_JIS.decode(bytes);
	snafu::ensure!(!error, DecodeSnafu { text });
	snafu::ensure!(!text.contains('\0'), DecodeSnafu { text });
	Ok(text.into_owned())
}

#[derive(Debug, snafu::Snafu)]
#[snafu(display("Cannot encode {text:?} as SJIS"))]
pub struct EncodeError { text: String }

pub fn encode(text: &str) -> Result<Vec<u8>, EncodeError> {
	snafu::ensure!(!text.contains('\0'), EncodeSnafu { text });
	let (bytes, _, error) = SHIFT_JIS.encode(text);
	snafu::ensure!(!error, EncodeSnafu { text });
	Ok(bytes.into_owned())
}

#[derive(Debug, snafu::Snafu)]
#[snafu(display("cannot convert {value} into {type_}\n{source}"))]
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

	fn sized_string<const N: usize>(&mut self) -> Result<String, ReadError> {
		let buf = self.array::<N>()?;
		let buf = buf.splitn(2, |&a| a==b'\0').next().unwrap();
		Ok(decode(buf)?)
	}
}
impl<'a, T: In<'a>> InExt<'a> for T {}

pub trait OutExt<L: Eq + std::hash::Hash + std::fmt::Debug> {
	fn string(&mut self, s: &str) -> Result<(), WriteError>;
	fn sized_string<const N: usize>(&mut self, s: &str) -> Result<(), WriteError>;
}
impl<L: Eq + std::hash::Hash + std::fmt::Debug> OutExt<L> for Out<'_, L> {
	fn string(&mut self, s: &str) -> Result<(), WriteError> {
		let s = encode(s)?;
		self.slice(&s);
		self.array([0]);
		Ok(())
	}

	fn sized_string<const N: usize>(&mut self, s: &str) -> Result<(), WriteError> {
		let s = encode(s)?;
		if s.len() > N {
			return Err(cast_error::<[u8; N]>(s, "too large").into());
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

	#[derive(Debug, snafu::Snafu)]
	pub enum Error {
		#[snafu(display("{source}"), context(false))]
		Io { source: std::io::Error, backtrace: snafu::Backtrace },

		#[snafu(display("{source}"), context(false))]
		Read { #[snafu(backtrace)] source: crate::util::ReadError },

		#[snafu(display("{source}"), context(false))]
		Write { #[snafu(backtrace)] source: crate::util::WriteError },
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

	pub fn check_roundtrip<T, T2>(
		arc: &Archives,
		name: &str,
		read: fn(&Archives, &[u8]) -> Result<T, super::ReadError>,
		write: fn(&Archives, &T2) -> Result<Vec<u8>, super::WriteError>,
	) -> Result<(), Error> where
		T: PartialEq + std::fmt::Debug,
		T: AsRef<T2>,
		T2: ?Sized,
	{
		let data = arc.get_decomp(name)?;
		let parsed = read(arc, &data)?;
		let data2 = write(arc, parsed.as_ref())?;
		let parsed2 = read(arc, &data2)?;
		check_equal(&parsed, &parsed2)
	}
}
