use std::ops::*;

#[cfg(test)]
pub mod test;

pub mod read;
pub mod write;

pub use read::*;
pub use write::*;

pub type Backtrace = Box<std::backtrace::Backtrace>;

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

#[macro_export]
macro_rules! __ensure {
	($cond:expr, $($t:tt)*) => {
		if !($cond) {
			$crate::util::bail!($($t)*)
		}
	};
}
pub use __ensure as ensure;

#[macro_export]
macro_rules! __bail {
	($str:literal $($arg:tt)*) => {
		return Err(format!($str $($arg)*).into())
	};
	($e:expr) => {
		return Err($e)
	}
}
pub use __bail as bail;

#[macro_export]
macro_rules! __newtype {
	($name:ident, $ty:ty) => {
		#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
		#[derive(derive_more::From, derive_more::Into)]
		pub struct $name(pub $ty);

		// For some reason DebugCustom doesn't work, probably because I want to include $name
		impl std::fmt::Debug for $name where $ty: std::fmt::Debug {
			fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(f, "{}({:?})", stringify!($name), &self.0)
			}
		}
	};
}
pub use __newtype as newtype;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameDesc {
	pub name: String,
	pub desc: String,
}


#[repr(transparent)]
pub struct StrictResult<A, B>(Result<A, B>);

pub trait ResultExt<A, B> {
	fn strict(self) -> StrictResult<A, B>;
}

impl<A, B> ResultExt<A, B> for Result<A, B> {
	fn strict(self) -> StrictResult<A, B> {
		StrictResult(self)
	}
}

impl<A, B> FromResidual<StrictResult<!, B>> for StrictResult<A, B> {
	fn from_residual(r: StrictResult<!, B>) -> Self {
		match r {
			StrictResult(Ok(v)) => match v {},
			StrictResult(Err(v)) => StrictResult(Err(v))
		}
	}
}
impl<A, B> FromResidual<StrictResult<!, B>> for Result<A, B> {
	fn from_residual(r: StrictResult<!, B>) -> Self {
		match r {
			StrictResult(Ok(v)) => match v {},
			StrictResult(Err(r)) => Err(r)
		}
	}
}

impl<A, B> Try for StrictResult<A, B> {
	type Output = A;
	type Residual = StrictResult<!, B>;

	fn from_output(r: A) -> Self {
		StrictResult(Ok(r))
	}

	fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
		match self {
			StrictResult(Ok(v)) => ControlFlow::Continue(v),
			StrictResult(Err(e)) => ControlFlow::Break(StrictResult(Err(e))),
		}
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
