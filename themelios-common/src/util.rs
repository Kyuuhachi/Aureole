pub mod read;
pub mod write;

use std::ops::{Residual, Try};

pub use read::*;
pub use write::*;

pub use strict_result::*;

#[derive(Debug, thiserror::Error)]
#[error("cannot convert {value} into {type_}\n{source}")]
pub struct CastError {
	source: Box<dyn std::error::Error + Sync + Send>,
	type_: &'static str,
	value: String,
}

pub fn cast<A, B>(a: A) -> Result<B, CastError> where
	A: std::fmt::Debug + Clone,
	B: TryFrom<A>,
	B::Error: std::error::Error + Sync + Send + 'static,
{
	a.clone().try_into().map_err(|e| cast_error::<B>(a, e))
}

pub fn cast_bool(a: impl Into<u64> + std::fmt::Debug + Clone) -> Result<bool, CastError> {
	match a.into() {
		0 => Ok(false),
		1 => Ok(true),
		n => Err(cast_error::<bool>(n, "out of range integral type conversion attempted")),
	}
}

pub fn cast_error<T>(
	val: impl std::fmt::Debug,
	cause: impl Into<Box<dyn std::error::Error + Sync + Send>>,
) -> CastError {
	CastError {
		type_: std::any::type_name::<T>(),
		value: format!("{:?}", val),
		source: cause.into()
	}
}

pub macro ensure {
	($cond:expr, $($t:tt)*) => {
		if !($cond) {
			$crate::util::bail!($($t)*)
		}
	},
	($cond:expr) => {
		if !($cond) {
			bail!(stringify!($cond).into())
		}
	}
}

pub macro bail {
	($str:literal $($arg:tt)*) => {
		bail!(format!($str $($arg)*).into())
	},
	($e:expr) => {
		Err($e).strict()?
	}
}

pub fn array<const N: usize, R: Try>(
	mut f: impl FnMut() -> R,
) -> <R::Residual as Residual<[R::Output; N]>>::TryType where
	R::Residual: Residual<[R::Output; N]>,
{
	[(); N].try_map(move |()| f())
}

pub fn list<V, E>(
	n: usize,
	mut f: impl FnMut() -> Result<V, E>,
) -> Result<Vec<V>, E> {
	let mut a = Vec::with_capacity(n);
	for _ in 0..n {
		a.push(f()?);
	}
	Ok(a)
}
