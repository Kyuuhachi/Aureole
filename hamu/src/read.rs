use std::{
	rc::Rc,
	cell::RefCell,
	ops::Range,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Out-of-bounds seek to {pos:#X} (size {size:#X})")]
	Seek { pos: usize, size: usize },
	#[error("Out-of-bounds read of {pos:#X}+{len} (size {size:#X})")]
	Read { pos: usize, len: usize, size: usize },
	#[error("Mismatched {type_} at {pos:#X}\n  got:      {got}\n  expected: {expected}")]
	Check { pos: usize, type_: String, got: String, expected: String },
	#[error("Uncovered data at {uncovered:X?}")]
	Uncovered { uncovered: Vec<Range<usize>> },
}
pub type Result<T, E=Error> = std::result::Result<T, E>;

#[allow(clippy::len_without_is_empty)]
pub trait In<'a> {
	fn pos(&self) -> usize;
	fn len(&self) -> usize;
	fn seek(&mut self, pos: usize) -> Result<()>;
	fn slice(&mut self, len: usize) -> Result<&'a [u8]>;

	fn remaining(&self) -> usize {
		self.len() - self.pos()
	}

	fn at(mut self, pos: usize) -> Result<Self> where Self: Sized {
		self.seek(pos)?;
		Ok(self)
	}

	fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
		Ok(self.slice(N)?.try_into().unwrap())
	}

	fn align(&mut self, size: usize) -> Result<()> {
		self.slice((size-(self.pos()%size))%size)?;
		Ok(())
	}

	fn check(&mut self, v: &[u8]) -> Result<()> {
		let pos = self.pos();
		let u = self.slice(v.len())?;
		if u != v {
			self.seek(pos)?;
			let mut got = Vec::new();
			let mut exp = Vec::new();
			for (&g, &e) in std::iter::zip(u, v) {
				got.extend(std::ascii::escape_default(g).map(char::from));
				exp.extend(std::ascii::escape_default(e).map(char::from));
				while got.len() < exp.len() { got.push('░') }
				while exp.len() < got.len() { exp.push('░') }
			}
			return Err(Error::Check {
				pos,
				type_:    format!("[u8; {}]", v.len()),
				got:      format!("b\"{}\"", got.into_iter().collect::<String>()),
				expected: format!("b\"{}\"", exp.into_iter().collect::<String>()),
			})
		}
		Ok(())
	}
}

pub trait Dump<'a>: In<'a> {
	fn dump(&self) -> beryl::Dump;
}

macro_rules! primitives {
	($name:ident, $conv:ident; $($type:ident),*) => { paste::paste! {
		pub trait $name<'a>: In<'a> {
			$(fn $type(&mut self) -> Result<$type> {
				Ok($type::$conv(self.array()?))
			})*

			$(fn [<check_ $type>](&mut self, v: $type) -> Result<()> {
				let pos = self.pos();
				let u = $name::$type(self)?;
				if u != v {
					self.seek(pos)?;
					return Err(Error::Check {
						pos,
						type_: stringify!($type).to_owned(),
						got: u.to_string(),
						expected: v.to_string(),
					})
				}
				Ok(())
			})*
		}
		impl<'a, T: In<'a>> $name<'a> for T {}
	} }
}

primitives!(Le, from_le_bytes; u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);
primitives!(Be, from_be_bytes; u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

#[derive(Clone)]
pub struct Bytes<'a> {
	data: &'a [u8],
	pos: usize,
}

impl<'a> Bytes<'a> {
	pub fn new(data: &'a [u8]) -> Self {
		Self {
			data,
			pos: 0,
		}
	}
}

impl<'a> In<'a> for Bytes<'a> {
	fn pos(&self) -> usize {
		self.pos
	}

	fn len(&self) -> usize {
		self.data.len()
	}

	fn seek(&mut self, pos: usize) -> Result<()> {
		if pos > self.len() {
			return Err(Error::Seek { pos, size: self.len() });
		}
		self.pos = pos;
		Ok(())
	}

	fn slice(&mut self, len: usize) -> Result<&'a [u8]> {
		if len > self.len() - self.pos() {
			return Err(Error::Read { pos: self.pos(), len, size: self.len() });
		}

		let pos = self.pos;
		self.pos += len;
		Ok(&self.data[pos..pos+len])
	}
}

impl<'a> Dump<'a> for Bytes<'a> {
	fn dump(&self) -> beryl::Dump {
		let mut cursor = std::io::Cursor::new(&self.data);
		cursor.set_position(self.pos as u64);
		beryl::Dump::new(cursor, self.pos)
			.num_width_from(self.len())
	}
}

#[derive(Clone)]
pub struct Coverage<'a, T: In<'a>> {
	inner: T,
	coverage: Rc<RefCell<Vec<Range<usize>>>>,
	last_coverage: usize,
	_p: std::marker::PhantomData<&'a ()>
}

impl<'a, T: In<'a>> Coverage<'a, T> {
	pub fn new(inner: T) -> Self {
		Self {
			inner,
			coverage: Rc::new(RefCell::new(vec![0..0])),
			last_coverage: 0,
			_p: std::marker::PhantomData,
		}
	}
}

impl<'a, T: In<'a>> In<'a> for Coverage<'a, T> {
	fn pos(&self) -> usize {
		self.inner.pos()
	}

	fn len(&self) -> usize {
		self.inner.len()
	}

	fn seek(&mut self, pos: usize) -> Result<()> {
		self.inner.seek(pos)?;
		self.find_coverage(pos);
		Ok(())
	}

	fn slice(&mut self, len: usize) -> Result<&'a [u8]> {
		let pos = self.pos();
		let data = self.inner.slice(len)?;
		self.insert_coverage(pos..pos+len);
		Ok(data)
	}
}

impl<'a, T: Dump<'a>> Dump<'a> for Coverage<'a, T> {
	fn dump(&self) -> beryl::Dump {
		let mut d = self.inner.dump();
		for r in self.coverage.borrow().iter() {
			d = d.mark(r.start, "\x1B[7m[\x1B[m");
			d = d.mark(r.end+1, "\x1B[7m]\x1B[m");
		}
		d
	}
}

impl<'a, T: In<'a>> Coverage<'a, T> {
	pub fn coverage(&self) -> Vec<Range<usize>> {
		// Cloning isn't strictly necessary here, but it makes a better interface and isn't used in
		// hot paths
		self.coverage.borrow().clone()
	}

	pub fn uncovered(&self) -> Vec<Range<usize>> {
		let mut uncovered = Vec::new();
		let mut last = 0;
		for range in self.coverage.borrow().iter() {
			if range.start != last {
				uncovered.push(last..range.start);
			}
			last = range.end;
		}
		if last != self.len() {
			uncovered.push(last..self.len());
		}
		uncovered
	}

	pub fn assert_covered(&self) -> Result<()> {
		let uncovered = self.uncovered();
		if uncovered.is_empty() {
			Ok(())
		} else {
			Err(Error::Uncovered { uncovered })
		}
	}

	pub fn dump_uncovered(&self, mut f: impl FnMut(beryl::Dump)) -> Result<()> where Self: Dump<'a> + Clone {
		let uncovered = self.uncovered();
		if uncovered.is_empty() {
			Ok(())
		} else {
			for r in uncovered.iter() {
				f(self.clone().at(r.start).unwrap().dump().end(r.end))
			}
			Err(Error::Uncovered { uncovered })
		}
	}

	fn find_coverage(&mut self, pos: usize) {
		let mut coverage = self.coverage.borrow_mut();
		use std::cmp::Ordering;
		match coverage.binary_search_by(|a| {
			if a.start > pos { Ordering::Greater }
			else if a.end < pos { Ordering::Less }
			else { Ordering::Equal }
		}) {
			Ok(index) => self.last_coverage = index,
			Err(index) => {
				coverage.insert(index, pos..pos);
				self.last_coverage = index
			},
		}
	}

	fn insert_coverage(&mut self, range: Range<usize>) {
		let mut coverage = self.coverage.borrow_mut();
		let mut i = self.last_coverage.min(coverage.len()-1);

		while coverage[i].start > range.start {
			i -= 1;
		}
		while coverage[i].end < range.start {
			i += 1;
		}

		while let Some(j) = coverage.get(i+1).filter(|a| range.end >= a.start) {
			coverage[i].end = j.end;
			coverage.remove(i+1);
		}

		coverage[i].end = coverage[i].end.max(range.end);
		self.last_coverage = i;
	}
}
