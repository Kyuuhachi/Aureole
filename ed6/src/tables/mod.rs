// t_bgmtbl
// t_book{00..=07}
// t_btlset
// t_cook2
// t_cook
// t_crfget
// t_exp
// t_face
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
// t_town
// t_world

pub mod town;

#[derive(Debug, snafu::Snafu)]
pub enum Error {
	#[snafu(display("{source}"), context(false))]
	Archive { source: crate::archive::Error, backtrace: snafu::Backtrace },

	#[snafu(display("{source}"), context(false))]
	Decompress { source: crate::decompress::Error, backtrace: snafu::Backtrace },

	#[snafu(display("{source}"), context(false))]
	Read { source: hamu::read::Error, backtrace: snafu::Backtrace },

	#[snafu(display("{source}"), context(false))]
	Coverage { source: hamu::read::coverage::Error, backtrace: snafu::Backtrace },

	#[snafu(display("{source}"), context(false))]
	Encoding { source: crate::util::DecodeError, backtrace: snafu::Backtrace },

	#[snafu(display("invalid value for enum {type_}: {value}"))]
	Enum { value: String, type_: String, backtrace: snafu::Backtrace },
}

impl<T> From<num_enum::TryFromPrimitiveError<T>> for Error where
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
