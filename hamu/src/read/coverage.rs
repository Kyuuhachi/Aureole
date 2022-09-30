use std::{
	rc::Rc,
	cell::RefCell,
	ops::Range,
};
use super::prelude::*;

#[derive(Debug, thiserror::Error)]
#[error("uncovered data at {uncovered:X?}")]
pub struct Error {
	uncovered: Vec<Range<usize>>,
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

impl<'a, T: In<'a>> InBase<'a> for Coverage<'a, T> {
	fn pos(&self) -> usize {
		self.inner.pos()
	}

	fn len(&self) -> usize {
		self.inner.len()
	}

	fn seek(&mut self, pos: usize) -> Result<(), super::Error> {
		self.inner.seek(pos)?;
		self.find_coverage(pos);
		Ok(())
	}

	fn slice(&mut self, len: usize) -> Result<&'a [u8], super::Error> {
		let pos = self.pos();
		let data = self.inner.slice(len)?;
		self.insert_coverage(pos..pos+len);
		Ok(data)
	}

	fn dump(&self) -> beryl::Dump {
		let mut d = self.inner.dump();
		for r in self.coverage.borrow().iter() {
			d = d.mark_range(r.clone(), 6);
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

	pub fn assert_covered(&self) -> Result<(), Error> {
		let uncovered = self.uncovered();
		if !uncovered.is_empty() {
			return Err(Error { uncovered })
		}
		Ok(())
	}

	pub fn dump_uncovered(&self, mut f: impl FnMut(beryl::Dump)) -> Result<(), Error> where Self: Clone {
		let uncovered = self.uncovered();
		if !uncovered.is_empty() {
			for r in uncovered.iter() {
				f(self.clone().at(r.start).unwrap().dump().end(r.end))
			}
			return Err(Error { uncovered });
		}
		Ok(())
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
