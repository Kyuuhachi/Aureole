use std::{
	hash::Hash,
	collections::{HashMap, hash_map::Entry},
	fmt::Debug,
	rc::Rc,
	ops::Range,
};

use snafu::prelude::*;

pub mod prelude {
	pub use super::{Out, Count, Error as WriteError};
}

#[derive(Debug, snafu::Snafu)]
pub enum Error {
	#[snafu(display("undefined label '{label}'"))]
	Undefined { label: String },
	#[snafu(display("duplicate label '{label}': {v1} → {v2}"))]
	Duplicate { label: String, v1: usize, v2: usize },
	#[snafu(display("failed to convert {label} ({value}) to {type_}: {source}"))]
	LabelSize {
		label: String,
		type_: &'static str,
		value: String,
		source: Box<dyn std::error::Error>
	},
}
pub type Result<T, E=Error> = std::result::Result<T, E>;

type Delayed<'a, L> = Box<dyn FnOnce(&dyn Fn(&L) -> Result<usize>, &mut [u8]) -> Result<()> + 'a>;

#[derive(Default)]
pub struct Out<'a, L: Eq + Hash + Debug + 'a> {
	data: Vec<u8>,
	delays: Vec<(Range<usize>, Delayed<'a, L>)>,
	labels: HashMap<L, usize>,
}

impl<'a, L: Eq + Hash + Debug> Out<'a, L> {
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
					self.labels.get(k)
						.copied()
						.with_context(|| UndefinedSnafu {
							label: format!("{:?}", k),
						})
				},
				&mut self.data[range],
			)?;
		}
		Ok(self.data)
	}

	pub fn len(&self) -> usize {
		self.data.len()
	}

	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	pub fn slice(&mut self, data: &[u8]) {
		self.data.extend(data)
	}

	pub fn array<const N: usize>(&mut self, data: [u8; N]) {
		self.slice(&data)
	}

	pub fn align(&mut self, size: usize) {
		self.slice(&vec![0;(size-(self.len()%size))%size]);
	}

	pub fn label(&mut self, label: L) {
		self.set_label(label, self.len());
	}

	fn set_label(&mut self, label: L, val: usize) {
		match self.labels.entry(label) {
			Entry::Vacant(entry) => entry.insert(val),
			Entry::Occupied(entry) => {
				panic!("Duplicate label {:?} (prev: {:#X}, new: {:#X})", entry.key(), entry.get(), val)
			}
		};
	}

	// It is unfortunate that I need Clone here.
	// But lookup must take the key by reference, and thus the mapping function must receive a reference.
	pub fn concat(&mut self, other: Self) where L: Clone {
		self.concat_with(other, |a| a.clone())
	}

	pub fn concat_with<M: Eq + Hash + Debug>(
		&mut self,
		mut other: Out<'a, M>,
		f: impl Fn(&M) -> L + 'a,
	) {
		let shift = self.len();
		self.data.append(&mut other.data);

		let f = Rc::new(f);
		for (range, cb) in other.delays {
			let range = range.start+shift..range.end+shift;
			self.delays.push((range, Box::new({
				let f = f.clone();
				move |lookup, slice| cb(&|k| lookup(&f(k)), slice)
			})))
		}

		for (k, v) in other.labels {
			self.set_label(f(&k), v+shift);
		}
	}

	pub fn map<M: Eq + Hash + Debug>(
		self,
		f: impl Fn(&L) -> M + 'a,
	) -> Out<'a, M> {
		let mut new = Out::new();
		new.concat_with(self, f);
		new
	}

	pub fn delay<const N: usize, F>(&mut self, cb: F) where
		F: FnOnce(&dyn Fn(&L) -> Result<usize>) -> Result<[u8; N]> + 'a,
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
		pub trait $name<L: Eq + Hash + Debug> {
			$(
				fn $type(&mut self, v: $type) {
					self.[<$type _ $suf>](v);
				}

				fn [<$type _ $suf>](&mut self, v: $type);
			)*

			$(
				fn [<delay_ $utype>](&mut self, k: L) {
					self.[<delay_ $utype _ $suf>](k);
				}

				fn [<delay_ $utype _ $suf>](&mut self, k: L);
			)*
		}

		impl<L: Eq + Hash + Debug> $name<L> for Out<'_, L> {
			$(
				fn [<$type _ $suf>](&mut self, v: $type) {
					self.array(v.$conv());
				}
			)*

			$(
				fn [<delay_ $utype _ $suf>](&mut self, k: L) {
					self.delay(move |lookup| {
						let v = lookup(&k)?;
						let v = $utype::try_from(v)
							.map_err(Box::from).with_context(|_| LabelSizeSnafu {
								label: format!("{:?}", k),
								type_: std::any::type_name::<$utype>(),
								value: format!("{:?}", v),
							})?;
						Ok(v.$conv())
					});
				}
			)*
		}

		pub mod $suf {
			pub use super::prelude::*;
			pub use super::$name;
		}
	} }
}

primitives!(OutLe, le, to_le_bytes; u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64; u8, u16, u32, u64, u128);
primitives!(OutBe, be, to_be_bytes; u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64; u8, u16, u32, u64, u128);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Count { value: usize }

impl Count {
	pub fn new() -> Self { Self::default() }

	#[allow(clippy::should_implement_trait)]
	pub fn next(&mut self) -> usize {
		let v = self.value;
		self.value += 1;
		v
	}
}
