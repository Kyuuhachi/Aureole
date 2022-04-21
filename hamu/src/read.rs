use std::{
	rc::Rc,
	cell::RefCell,
	ops::Range,
};

use crate::dump::Dump;

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

#[derive(Clone)]
pub struct In<'a> {
	data: &'a [u8],
	pos: usize,
	coverage: Rc<RefCell<Vec<Range<usize>>>>,
	last_coverage: usize,
}

impl<'a> In<'a> {
	pub fn new(data: &'a [u8]) -> Self {
		Self {
			data,
			pos: 0,
			coverage: Rc::new(RefCell::new(vec![0..0])),
			last_coverage: 0,
		}
	}

	pub fn data(&self) -> &[u8] {
		self.data
	}

	pub fn pos(&self) -> usize {
		self.pos
	}

	#[allow(clippy::len_without_is_empty)]
	pub fn len(&self) -> usize {
		self.data.len()
	}

	pub fn remaining(&self) -> usize {
		self.len() - self.pos()
	}

	pub fn seek(&mut self, pos: usize) -> Result<()> {
		if pos > self.len() {
			return Err(Error::Seek { pos, size: self.len() });
		}
		self.pos = pos;
		self.last_coverage = find_coverage(&mut self.coverage.borrow_mut(), pos);
		Ok(())
	}

	pub fn at(mut self, pos: usize) -> Result<Self> {
		self.seek(pos)?;
		Ok(self)
	}

	pub fn slice(&mut self, len: usize) -> Result<&'a [u8]> {
		if len > self.len() - self.pos() {
			return Err(Error::Read { pos: self.pos(), len, size: self.len() });
		}

		let pos = self.pos;
		self.pos += len;
		insert_coverage(&mut self.coverage.borrow_mut(), &mut self.last_coverage, pos..pos+len);
		Ok(&self.data[pos..pos+len])
	}

	pub fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
		Ok(self.slice(N)?.try_into().unwrap())
	}

	pub fn align(&mut self, size: usize) -> Result<()> {
		self.slice((size-(self.pos()%size))%size)?;
		Ok(())
	}

	pub fn check(&mut self, v: &[u8]) -> Result<()> {
		let pos = self.pos();
		let u = self.slice(v.len())?;
		if u != v {
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

	pub fn dump(&self) -> Dump {
		Dump::new(self)
	}
}

macro_rules! primitives {
	($name:ident, $conv:ident; $($type:ident),*) => { paste::paste! {
		#[extend::ext(name=$name)]
		pub impl In<'_> {
			$(fn $type(&mut self) -> Result<$type> {
				Ok($type::$conv(self.array()?))
			})*

			$(fn [<check_ $type>](&mut self, v: $type) -> Result<()> {
				let pos = self.pos();
				let u = $name::$type(self)?;
				if u != v {
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
	} }
}

primitives!(Le, from_le_bytes; u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);
primitives!(Be, from_be_bytes; u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

impl<'a> In<'a> {
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

	pub fn dump_uncovered(&self, mut f: impl FnMut(Dump)) -> Result<()> {
		let uncovered = self.uncovered();
		if uncovered.is_empty() {
			Ok(())
		} else {
			uncovered.iter().for_each(|r| {
				f(self.clone()
					.at(r.start).unwrap()
					.dump()
					.end(r.end)
				)
			});
			Err(Error::Uncovered { uncovered })
		}
	}
}

fn find_coverage(coverage: &mut Vec<Range<usize>>, pos: usize) -> usize {
	use std::cmp::Ordering;
	match coverage.binary_search_by(|a| {
		if a.start > pos { Ordering::Greater }
		else if a.end < pos { Ordering::Less }
		else { Ordering::Equal }
	}) {
		Ok(index) => index,
		Err(index) => {
			coverage.insert(index, pos..pos);
			index
		},
	}
}

fn insert_coverage(coverage: &mut Vec<Range<usize>>, last: &mut usize, range: Range<usize>) {
	*last = usize::min(*last, coverage.len()-1);
	while coverage[*last].start > range.start {
		*last -= 1;
	}
	while coverage[*last].end < range.start {
		*last += 1;
	}

	while let Some(j) = coverage.get(*last+1).filter(|a| range.end >= a.start) {
		coverage[*last].end = j.end;
		coverage.remove(*last+1);
	}

	coverage[*last].end = coverage[*last].end.max(range.end);
}
