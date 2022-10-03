use std::{
	hash::Hash,
	collections::{HashMap, hash_map::Entry},
	fmt::Debug,
	rc::Rc,
	ops::Range,
	num::TryFromIntError,
};

pub mod prelude {
	pub use super::{OutBase, Label, OutDelay, Out, OutBytes, Count, Unique};
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("undefined label '{label}'")]
	Undefined { label: String },
	#[error("duplicate label '{label}': {v1} → {v2}")]
	Duplicate { label: String, v1: usize, v2: usize },
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

pub trait Label: Eq + Hash + Clone + Debug + 'static {}
impl<T: Eq + Hash + Clone + Debug + 'static> Label for T {}

pub trait OutDelay<L: Label>: OutBase {
	fn label(&mut self, label: L);
	fn delay<const N: usize, F>(&mut self, cb: F) where
		F: FnOnce(&dyn Fn(L) -> Result<usize>) -> Result<[u8; N]> + 'static;
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

type Delayed<L> = Box<dyn FnOnce(&dyn Fn(L) -> Result<usize>, &mut [u8]) -> Result<()>>;

#[derive(Default)]
#[must_use]
pub struct OutBytes<L: Label> {
	data: Vec<u8>,
	delays: Vec<(Range<usize>, Delayed<L>)>,
	labels: HashMap<L, usize>,
}

impl<L: Label> OutBytes<L> {
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
					self.labels.get(&k)
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

	fn set_label(&mut self, label: L, val: usize) {
		match self.labels.entry(label) {
			Entry::Vacant(entry) => entry.insert(val),
			Entry::Occupied(entry) => {
				panic!("Duplicate label {:?} (prev: {:#X}, new: {:#X})", entry.key(), entry.get(), val)
			}
		};
	}

	pub fn concat(self, other: Self) -> Self {
		self.concat_with(other, |a| a)
	}

	pub fn concat_with<M: Label>(
		mut self,
		mut other: OutBytes<M>,
		f: impl Fn(M) -> L + 'static,
	) -> Self {
		let shift = self.len();
		self.data.append(&mut other.data);

		let f = Rc::new(f);
		for (range, cb) in other.delays {
			let range = range.start+shift..range.end+shift;
			self.delays.push((range, Box::new({
				let f = f.clone();
				move |lookup, slice| cb(&|k| lookup(f(k)), slice)
			})))
		}

		for (k, v) in other.labels {
			self.set_label(f(k), v+shift);
		}

		self
	}

	pub fn map<M: Label>(
		self,
		f: impl Fn(L) -> M + 'static,
	) -> OutBytes<M> {
		OutBytes::new().concat_with(self, f)
	}
}

impl<L: Label> OutBase for OutBytes<L> {
	fn len(&self) -> usize {
		self.data.len()
	}

	fn slice(&mut self, data: &[u8]) {
		self.data.extend(data)
	}
}

impl<L: Label> OutDelay<L> for OutBytes<L> {
	fn label(&mut self, label: L) {
		self.set_label(label, self.len());
	}

	fn delay<const N: usize, F>(&mut self, cb: F) where
		F: FnOnce(&dyn Fn(L) -> Result<usize>) -> Result<[u8; N]> + 'static,
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
				fn [<delay_ $utype>]<L: Label>(&mut self, k: L) where Self: OutDelay<L> {
					self.[<delay_ $utype _ $suf>](k);
				}

				fn [<delay_ $utype _ $suf>]<L: Label>(&mut self, k: L) where Self: OutDelay<L> {
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

#[deprecated]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Count { value: usize }

#[allow(deprecated)]
impl Count {
	pub fn new() -> Self { Self::default() }

	#[allow(clippy::should_implement_trait)]
	pub fn next(&mut self) -> usize {
		let v = self.value;
		self.value += 1;
		v
	}
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Unique(usize);

impl Debug for Unique {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Unique(0x{:04X})", self.0)
	}
}

impl Unique {
	#[allow(clippy::new_without_default)]
	pub fn new() -> Unique {
		use std::sync::atomic::{AtomicUsize, Ordering};
		static COUNT: AtomicUsize = AtomicUsize::new(0);
		Unique(COUNT.fetch_add(1, Ordering::Relaxed))
	}
}
