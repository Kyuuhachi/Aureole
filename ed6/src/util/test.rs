use crate::archive::Archives;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("{source}")]
	Io { #[from] source: std::io::Error, backtrace: std::backtrace::Backtrace },

	#[error(transparent)]
	Read { #[from] #[backtrace] source: crate::util::ReadError },

	#[error(transparent)]
	Write { #[from] #[backtrace] source: crate::util::WriteError },

	#[error("{assertion}")]
	Assert { assertion: Box<str>, backtrace: Box<std::backtrace::Backtrace> },
}

impl std::convert::From<String> for Error {
	fn from(assertion: String) -> Self {
		Self::Assert {
			assertion: assertion.into(),
			backtrace: std::backtrace::Backtrace::capture().into(),
		}
	}
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
		return Err(format!("{} differs", std::any::type_name::<T>()).into())
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
