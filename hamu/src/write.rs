use std::{
	hash::Hash,
	collections::HashMap,
	fmt::Debug,
	ops::Range,
	num::TryFromIntError,
};

pub mod prelude {
	pub use super::{OutBase, Label, OutDelay, Out, OutBytes};
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("undefined label '{label}'")]
	Undefined { label: String },
	#[error("failed to convert {value} to {type_}")]
	LabelSize {
		type_: &'static str,
		value: String,
	},
}
pub type Result<T, E=Error> = std::result::Result<T, E>;

#[allow(clippy::len_without_is_empty)]
pub trait OutBase {
	fn len(&self) -> usize;
	fn slice(&mut self, data: &[u8]);
}

pub trait OutDelay: OutBase {
	fn label(&mut self, label: LabelDef);
	fn delay<const N: usize, F>(&mut self, cb: F) where
		F: FnOnce(&dyn Fn(Label) -> Result<usize>) -> Result<[u8; N]> + 'static;

	fn here(&mut self) -> Label {
		let (l, l_) = Label::new();
		self.label(l_);
		l
	}
}

pub trait Out: OutBase {
	fn is_empty(&self) -> bool {
		self.len() == 0
	}

	fn array<const N: usize>(&mut self, data: [u8; N]) {
		self.slice(&data)
	}

	fn align(&mut self, size: usize) {
		self.slice(&vec![0;(size-(self.len()%size))%size]);
	}
}
impl<T> Out for T where T: OutBase + ?Sized {}

type Delayed = Box<dyn FnOnce(&dyn Fn(Label) -> Result<usize>, &mut [u8]) -> Result<()>>;

#[derive(Default)]
#[must_use]
pub struct OutBytes {
	data: Vec<u8>,
	delays: Vec<(Range<usize>, Delayed)>,
	labels: HashMap<LabelDef, usize>,
}

impl OutBytes {
	pub fn new() -> Self {
		Self {
			data: Vec::new(),
			delays: Vec::new(),
			labels: HashMap::new(),
		}
	}

	pub fn finish(mut self) -> Result<Vec<u8>> {
		for (range, cb) in self.delays {
			cb(
				&|k| {
					self.labels.get(&LabelDef(k.0))
						.copied()
						.ok_or_else(|| Error::Undefined {
							label: format!("{:?}", k),
						})
				},
				&mut self.data[range],
			)?;
		}
		Ok(self.data)
	}

	pub fn concat(mut self, mut other: OutBytes) -> Self {
		let shift = self.len();
		self.data.append(&mut other.data);

		for (range, cb) in other.delays {
			let range = range.start+shift..range.end+shift;
			self.delays.push((range, cb))
		}

		for (k, v) in other.labels {
			self.labels.insert(k, v+shift);
		}

		self
	}
}

impl OutBase for OutBytes {
	fn len(&self) -> usize {
		self.data.len()
	}

	fn slice(&mut self, data: &[u8]) {
		self.data.extend(data)
	}
}

impl OutDelay for OutBytes {
	fn label(&mut self, label: LabelDef) {
		self.labels.insert(label, self.len());
	}

	fn delay<const N: usize, F>(&mut self, cb: F) where
		F: FnOnce(&dyn Fn(Label) -> Result<usize>) -> Result<[u8; N]> + 'static,
	{
		let start = self.len();
		self.array([0; N]);
		let end = self.len();
		self.delays.push((start..end, Box::new(move |lookup, slice| {
			slice.copy_from_slice(&cb(lookup)?);
			Ok(())
		})));
	}
}

macro_rules! primitives {
	($name:ident, $suf: ident, $conv:ident; $($type:ident),*; $($utype:ident),*) => { paste::paste! {
		pub trait $name: Out {
			$(
				fn $type(&mut self, v: $type) {
					self.[<$type _ $suf>](v);
				}

				fn [<$type _ $suf>](&mut self, v: $type) {
					self.array(v.$conv());
				}
			)*

			$(
				fn [<delay_ $utype>](&mut self, k: Label) where Self: OutDelay {
					self.[<delay_ $utype _ $suf>](k);
				}

				fn [<delay_ $utype _ $suf>](&mut self, k: Label) where Self: OutDelay {
					self.delay(move |lookup| {
						let v = lookup(k.clone())?;
						let v = cast_usize::<$utype>(v)?;
						Ok(v.$conv())
					});
				}
			)*
		}
		impl<T: Out + ?Sized> $name for T {}

		pub mod $suf {
			pub use super::prelude::*;
			pub use super::$name;
		}
	} }
}

pub fn cast_usize<T: TryFrom<usize, Error=TryFromIntError>>(v: usize) -> Result<T> {
	T::try_from(v).map_err(|_| Error::LabelSize {
		type_: std::any::type_name::<T>(),
		value: format!("{:?}", v),
	})
}

primitives!(OutLe, le, to_le_bytes; u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64; u8, u16, u32, u64, u128);
primitives!(OutBe, be, to_be_bytes; u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64; u8, u16, u32, u64, u128);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Label(usize);
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LabelDef(usize);

impl Debug for Label {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Label(0x{:04X})", self.0)
	}
}

impl Debug for LabelDef {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "LabelDef(0x{:04X})", self.0)
	}
}

impl Label {
	#[allow(clippy::new_without_default)]
	pub fn new() -> (Label, LabelDef) {
		use std::sync::atomic::{AtomicUsize, Ordering};
		static COUNT: AtomicUsize = AtomicUsize::new(0);
		let n = COUNT.fetch_add(1, Ordering::Relaxed);
		(Label(n), LabelDef(n))
	}
}
