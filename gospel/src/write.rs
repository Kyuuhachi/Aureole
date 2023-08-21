use std::{
	hash::Hash,
	collections::HashMap,
	fmt::Debug,
	ops::Range,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("undefined label {label:?} referenced at {pos:#X}")]
	Label { pos: usize, label: Label },
	#[error("error at {pos:#X}: {source}")]
	Other { pos: usize, #[source] source: BoxError },
}

pub type Result<T, E=Error> = std::result::Result<T, E>;

impl Error {
	pub fn pos(&self) -> usize {
		match self {
			Error::Label { pos, .. } => *pos,
			Error::Other { pos, .. } => *pos,
		}
	}

	pub fn pos_mut(&mut self) -> &mut usize {
		match self {
			Error::Label { pos, .. } => pos,
			Error::Other { pos, .. } => pos,
		}
	}
}

#[derive(Clone, Debug, thiserror::Error)]
#[error("attempted to write {value:#X} as a u{size}")]
pub struct LabelSizeError {
	pub value: usize,
	pub size: usize,
}

type BoxError = Box<dyn std::error::Error + Send + Sync>;
type Delayed = Box<dyn FnOnce(&DelayContext, &mut [u8]) -> Result<(), BoxError>>;

/// An incremental writer to a `Vec<u8>`, with support for delayed labels.
#[derive(Default)]
#[must_use]
pub struct Writer {
	data: Vec<u8>,
	delays: Vec<(Range<usize>, Delayed)>,
	labels: HashMap<Label, usize>,
}

/// Context passed to the delay function.
pub struct DelayContext<'a> {
	pos: usize,
	labels: &'a HashMap<Label, usize>,
}

impl<'a> DelayContext<'a> {
	/// The position where the delayed data will be written.
	#[inline(always)]
	pub fn pos(&self) -> usize {
		self.pos
	}

	/// Look up an address.
	///
	/// Returns an error if the label does not exist.
	#[inline(always)]
	pub fn label(&self, label: Label) -> Result<usize> {
		self.labels.get(&label).copied()
			.ok_or(Error::Label { pos: self.pos(), label })
	}
}

impl Writer {
	/// Constructs a new `Writer`.
	#[inline(always)]
	pub fn new() -> Self {
		Self {
			data: Vec::new(),
			delays: Vec::new(),
			labels: HashMap::new(),
		}
	}

	/// Finalizes all delayed labels and returns the resulting `Vec<u8>`.
	///
	/// Returns any error returned by the delays, which is usually if a label is not defined or is
	/// too large to fit in its slot.
	pub fn finish(mut self) -> Result<Vec<u8>> {
		for (range, cb) in self.delays {
			let pos = range.start;
			let res = cb(
				&DelayContext { pos, labels: &self.labels},
				&mut self.data[range],
			);
			match res {
				Ok(()) => {},
				Err(e) => return match e.downcast() {
					Ok(e) => Err(*e),
					Err(e) => Err(Error::Other { pos, source: e })
				}
			}
		}
		Ok(self.data)
	}

	/// Writes some data.
	#[inline(always)]
	pub fn slice(&mut self, data: &[u8]) {
		self.data.extend_from_slice(data)
	}

	/// Writes some data.
	///
	/// This function is redundant, and exists only for symmetry.
	#[inline(always)]
	pub fn array<const N: usize>(&mut self, data: [u8; N]) {
		self.slice(&data)
	}

	/// Places a label at the current position, so it can be referenced with [`delay`](`Self::delay`).
	#[inline(always)]
	pub fn label(&mut self, label: Label) {
		self.put_label(label, self.len());
	}

	/// Creates and places a label at the current position.
	#[inline(always)]
	pub fn here(&mut self) -> Label {
		let l = Label::new();
		self.label(l);
		l
	}

	fn put_label(&mut self, label: Label, pos: usize) {
		if let Some(p) = self.labels.insert(label, pos) {
			panic!("label already defined at 0x{p:04X}")
		}
	}

	/// Writes some bytes to be filled in later.
	///
	/// The given closure is called with a function that allows looking up labels.
	/// Other kinds of state are not currently officially allowed.
	#[inline(always)]
	pub fn delay<const N: usize, F>(&mut self, cb: F) where
		F: FnOnce(&DelayContext) -> Result<[u8; N], BoxError> + 'static,
	{
		let start = self.len();
		self.array([0; N]);
		let end = self.len();
		self.delays.push((start..end, Box::new(move |ctx, slice| {
			slice.copy_from_slice(&cb(ctx)?);
			Ok(())
		})));
	}

	/// Concatenates two `Writer`s, including labels.
	#[inline]
	pub fn append(&mut self, mut other: Writer) {
		let shift = self.len();
		self.data.reserve(other.data.capacity());
		self.data.append(&mut other.data);

		for (range, cb) in other.delays {
			let range = range.start+shift..range.end+shift;
			self.delays.push((range, cb))
		}

		for (label, pos) in other.labels {
			self.put_label(label, pos+shift);
		}
	}

	/// Returns the number of bytes written so far, including delayed ones.
	#[must_use]
	#[inline(always)]
	pub fn len(&self) -> usize {
		self.data.len()
	}

	/// Returns whether any bytes have been written so far.
	#[must_use]
	#[inline(always)]
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Calls [`Vec::reserve`] on the underlying `Vec`.
	#[inline(always)]
	pub fn reserve(&mut self, size: usize) {
		self.data.reserve(size);
	}

	/// Returns the capacity of the underlying `Vec`.
	#[inline(always)]
	pub fn capacity(&self) -> usize {
		self.data.capacity()
	}

	/// Writes null bytes until the length is a multiple of `size`.
	#[inline(always)]
	pub fn align(&mut self, size: usize) {
		self.slice(&vec![0;(size-(self.len()%size))%size]);
	}
}

#[cfg(doc)]
#[doc(hidden)]
pub type T = ();

/// Functions for writing primitives to the stream. The underlying functions are hidden from
/// the docs for brevity.
///
/// Supported primitives are `u8..=u128`, `i8..=i128`, `f32`, `f64`.
///
/// The functions are suffixed with either `_le` or `_be`, for endianness. To use unsuffixed
/// versions, import either the [`Le`] or [`Be`] trait.
#[cfg(doc)]
impl Writer {
	// Could add a shitload of #[doc(alias = _)], but don't wanna.

	/// Write a primitive from the input.
	pub fn T(&mut self, val: T) {}

	/// Write the address of a label.
	///
	/// [`finish`](`Self::finish`) will throw an error if the resulting address does not fit in the type.
	pub fn delayN(&mut self, l: Label) {}

	/// Creates a new `Writer` and delays a pointer to it.
	///
	/// This is a shorthand for the common pattern of `{ let mut g = Writer::new(); f.delayN(g.here()); g }`.
	pub fn ptrN(&mut self) -> Writer {}
}

mod seal { pub trait Sealed: Sized {} }
impl seal::Sealed for Writer {}

macro_rules! primitives {
	(
		$(#[$trait_attrs:meta])* trait $trait:ident;
		$suf:ident, $conv:ident;
		{ $($type:ident),* }
		{ $($ptr:tt),* }
	) => { paste::paste! {
		// #[doc(hidden)]
		impl Writer {
			$(#[inline(always)] pub fn [<$type $suf>](&mut self, val: $type) {
				self.array($type::$conv(val));
			})*
			$(#[inline(always)] pub fn [<delay$ptr $suf>](&mut self, label: Label) {
				self.delay(move |ctx| {
					let value = ctx.label(label)?;
					let value = [<u$ptr>]::try_from(value).map_err(|_| LabelSizeError { value, size: $ptr })?;
					Ok([<u$ptr>]::$conv(value))
				});
			})*
			$(#[inline(always)] pub fn [<ptr$ptr $suf>](&mut self) -> Self {
				let mut g = Writer::new();
				self.[<delay$ptr $suf>](g.here());
				g
			})*
		}

		$(#[$trait_attrs])*
		pub trait $trait: seal::Sealed {
			$(#[doc(hidden)] fn $type(&mut self, val: $type);)*
			$(#[doc(hidden)] fn [<delay$ptr>](&mut self, label: Label);)*
			$(#[doc(hidden)] fn [<ptr$ptr>](&mut self) -> Self;)*
		}

		impl $trait for Writer {
			$(#[doc(hidden)] #[inline(always)] fn $type(&mut self, val: $type) {
				self.[<$type $suf>](val)
			})*
			$(#[doc(hidden)] #[inline(always)] fn [<delay$ptr>](&mut self, label: Label) {
				self.[<delay$ptr $suf>](label)
			})*
			$(#[doc(hidden)] #[inline(always)] fn [<ptr$ptr>](&mut self) -> Self {
				self.[<ptr$ptr $suf>]()
			})*
		}
	} }
}

primitives!(
	/// Allows writing little-endian primitives without `_le` suffix.
	///
	/// It is recommended to import this as `use gospel::read::Le as _;`.
	trait Le;
	_le, to_le_bytes;
	{
		u8, u16, u32, u64, u128,
		i8, i16, i32, i64, i128,
		f32, f64
	}
	{ 8, 16, 32, 64, 128 }
);
primitives!(
	/// Allows writing little-endian primitives without `_be` suffix.
	///
	/// It is recommended to import this as `use gospel::read::Be as _;`.
	trait Be;
	_be, to_be_bytes;
	{
		u8, u16, u32, u64, u128,
		i8, i16, i32, i64, i128,
		f32, f64
	}
	{ 8, 16, 32, 64, 128 }
);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Label(u64);

impl Debug for Label {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Label({:#04X})", self.0)
	}
}

impl Label {
	#[allow(clippy::new_without_default)]
	pub fn new() -> Label {
		use std::sync::atomic::{AtomicU64, Ordering};
		static COUNT: AtomicU64 = AtomicU64::new(0);
		let n = COUNT.fetch_add(1, Ordering::Relaxed);
		Label(n)
	}

	pub fn known(n: u32) -> Label {
		let n = n as u64 | 0xFFFFFFFF00000000;
		Label(n)
	}
}
