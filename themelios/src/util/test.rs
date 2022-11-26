use crate::archive::Archives;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("{source}")]
	Io { #[from] source: std::io::Error, backtrace: std::backtrace::Backtrace },

	#[error("{source}")]
	Lookup { #[from] source: crate::gamedata::LookupError, backtrace: std::backtrace::Backtrace },

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
	pub static ref SC: Archives = Archives::new("../data/sc").unwrap();
	pub static ref TC: Archives = Archives::new("../data/3rd").unwrap();
}

pub fn check_equal<T: PartialEq + std::fmt::Debug>(a: &T, b: &T) -> Result<(), Error> {
	if a != b {
		let a = format!("{:#?}", a);
		let b = format!("{:#?}", b);
		let diff = similar::TextDiff::configure().diff_lines(&a, &b);

		for (i, hunk) in diff.unified_diff().iter_hunks().enumerate() {
			if i > 0 {
				println!("\x1B[34m…\x1B[39m");
			}
			for change in hunk.iter_changes() {
				match change.tag() {
					similar::ChangeTag::Delete => print!("\x1B[31m-{change}\x1B[39m"),
					similar::ChangeTag::Insert => print!("\x1B[32m+{change}\x1B[39m"),
					similar::ChangeTag::Equal => print!(" {change}"),
				};
			}
		}
		return Err(format!("{} differs", std::any::type_name::<T>()).into())
	}
	Ok(())
}

pub fn check_roundtrip<T>(
	data: &[u8],
	read: impl Fn(&[u8]) -> Result<T, super::ReadError>,
	write: impl Fn(&T) -> Result<Vec<u8>, super::WriteError>,
) -> Result<T, Error> where
	T: PartialEq + std::fmt::Debug,
{
	let val = read(data)?;
	let data2 = write(&val)?;
	let val2 = read(&data2)?;
	check_equal(&val, &val2)?;
	Ok(val)
}

pub fn check_roundtrip_strict<T>(
	data: &[u8],
	read: impl Fn(&[u8]) -> Result<T, super::ReadError>,
	write: impl Fn(&T) -> Result<Vec<u8>, super::WriteError>,
) -> Result<T, Error> where
	T: PartialEq + std::fmt::Debug,
{
	let val = read(data)?;
	let data2 = write(&val)?;
	if data != data2 {
		println!("differs! rereading");
		let val2 = read(&data2)?;
		check_equal(&val, &val2)?;

		let diff = similar::capture_diff_slices(similar::Algorithm::Patience, data, &data2);

		for chunk in diff {
			match chunk {
				similar::DiffOp::Equal { old_index, new_index, len } => {
					println!(
						"{:?} = {:?}",
						old_index..old_index+len,
						new_index..new_index+len,
					);
				}
				similar::DiffOp::Delete { old_index, old_len, new_index } => {
					println!(
						"{:?} ⇒ {} ({:02X?} ⇒ [])",
						old_index..old_index+old_len,
						new_index,
						&data[old_index..old_index+old_len],
					);
				}
				similar::DiffOp::Insert { old_index, new_index, new_len } => {
					println!(
						"{} ⇐ {:?} ([] ⇐ {:02X?})",
						old_index,
						new_index..new_index+new_len,
						&data2[new_index..new_index+new_len],
					);
				}
				similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
					println!(
						"{:?} ≠ {:?} ({:02X?} ≠ {:02X?})",
						old_index..old_index+old_len,
						new_index..new_index+new_len,
						&data[old_index..old_index+old_len],
						&data2[new_index..new_index+new_len],
					);
				}
			}
		}
		return Err(format!("{} bytes differ", std::any::type_name::<T>()).into())
	}
	Ok(val)
}

#[macro_export]
macro_rules! __simple_roundtrip {
	($name:literal) => {
		#[test_case::test_case(&$crate::util::test::FC; "fc")]
		fn roundtrip(arc: &$crate::archive::Archives) -> Result<(), $crate::util::test::Error> {
			$crate::util::test::check_roundtrip_strict(
				&arc.get_decomp($name).unwrap(),
				super::read,
				|a| super::write(a),
			)?;
			Ok(())
		}
	};
}
pub use __simple_roundtrip as simple_roundtrip;

#[macro_export]
macro_rules! __simple_roundtrip_arc {
	($name:literal) => {
		#[test_case::test_case(&$crate::util::test::FC; "fc")]
		fn roundtrip(arc: &$crate::archive::Archives) -> Result<(), $crate::util::test::Error> {
			$crate::util::test::check_roundtrip_strict(
				&arc.get_decomp($name).unwrap(),
				|a| super::read(arc, a),
				|a| super::write(arc, a),
			)?;
			Ok(())
		}
	};
}
pub use __simple_roundtrip_arc as simple_roundtrip_arc;
