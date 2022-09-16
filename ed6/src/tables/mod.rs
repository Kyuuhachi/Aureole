// t_bgmtbl
// t_book{00..=07}
// t_btlset
// t_cook2
// t_cook
// t_crfget
// t_exp
// t_item2
// t_item
// t_magget
// t_magic
// t_magqrt
// t_memo
// t_name
// t_orb
// t_quartz
// t_quest
// t_se
// t_shop
// t_sltget
// t_status
// t_world

// t_town
pub mod town;

// t_face
pub mod face;

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

	#[snafu(display("invalid value for enum {type_}: {value}"))]
	Enum { value: String, type_: String, backtrace: snafu::Backtrace },
}

impl<T> From<num_enum::TryFromPrimitiveError<T>> for ReadError where
	T: num_enum::TryFromPrimitive,
	T::Primitive: std::fmt::Display,
{
	fn from(e: num_enum::TryFromPrimitiveError<T>) -> Self {
		EnumSnafu {
			value: e.number.to_string(),
			type_: std::any::type_name::<T>(),
		}.build()
	}
}

#[derive(Debug, snafu::Snafu)]
pub enum WriteError {
	#[snafu(display("{source}"), context(false))]
	Io { source: std::io::Error, backtrace: snafu::Backtrace },

	#[snafu(display("{source}"), context(false))]
	Write { source: hamu::write::Error, backtrace: snafu::Backtrace },

	#[snafu(display("{source}"), context(false))]
	Encoding { source: crate::util::EncodeError, backtrace: snafu::Backtrace },
}

#[cfg(test)]
mod test {
	use crate::archive::Archives;

	#[derive(Debug, snafu::Snafu)]
	pub enum Error {
		#[snafu(display("{source}"), context(false))]
		Io { source: std::io::Error, backtrace: snafu::Backtrace },

		#[snafu(display("{source}"), context(false))]
		Read { #[snafu(backtrace)] source: super::ReadError },

		#[snafu(display("{source}"), context(false))]
		Write { #[snafu(backtrace)] source: super::WriteError },
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
}
