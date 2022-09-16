use std::{
	hash::Hash,
	collections::{HashMap, hash_map::Entry},
	fmt::Debug,
	rc::Rc,
	ops::Range,
};

pub mod prelude {
	pub use super::{Out, Count};
}

type Delayed<'a, L> = Box<dyn FnOnce(&dyn Fn(&L) -> usize, &mut [u8]) + 'a>;

pub struct Out<'a, L: Eq + Hash + Debug + 'a> {
	data: Vec<u8>,
	delays: Vec<(Range<usize>, Delayed<'a, L>)>,
	labels: HashMap<L, usize>,
}

impl<L: Eq + Hash + Debug> Default for Out<'_, L> {
	fn default() -> Self {
		Self::new()
	}
}

impl<'a, L: Eq + Hash + Debug> Out<'a, L> {
	pub fn new() -> Self {
		Self {
			data: Vec::new(),
			delays: Vec::new(),
			labels: HashMap::new(),
		}
	}

	pub fn finish(mut self) -> Vec<u8> {
		for (range, cb) in self.delays {
			cb(
				&|k| {
					*self.labels.get(k)
						.unwrap_or_else(|| panic!("Undefined label {:?}", k))
				},
				&mut self.data[range],
			);
		}
		self.data
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
		F: FnOnce(&dyn Fn(&L) -> usize) -> [u8; N] + 'a,
	{
		let start = self.len();
		self.array([0; N]);
		let end = self.len();
		self.delays.push((start..end, Box::new(move |lookup, slice| {
			slice.copy_from_slice(&cb(lookup))
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
						$utype::try_from(lookup(&k)).unwrap_or_else(|_| {
							panic!("{:?} is {:?}, which does not fit in a {}", &k, lookup(&k), stringify!($utype))
						}).$conv()
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
