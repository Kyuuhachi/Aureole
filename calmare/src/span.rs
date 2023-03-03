use std::ops::Range;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Span {
	pub start: usize,
	pub end: usize,
}

impl std::fmt::Debug for Span {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}+{}", self.start, self.end-self.start)
	}
}

impl Span {
	pub fn new(start: usize, end: usize) -> Self {
		assert!(start <= end);
		Span { start, end }
	}

	pub fn new_at(pos: usize) -> Self {
		Span { start: pos, end: pos }
	}

	pub fn at_start(&self) -> Self {
		Span::new_at(self.start)
	}

	pub fn at_end(&self) -> Self {
		Span::new_at(self.end)
	}

	pub fn join(self, b: Span) -> Self {
		Span::new(
			self.start.min(b.start),
			self.end.max(b.end)
		)
	}

	pub fn connects(self, b: Span) -> bool {
		let a = self;
		a.end == b.start
	}

	pub fn as_range(self) -> Range<usize> {
		self.start..self.end
	}
}

impl std::ops::BitOr for Span {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output {
		self.join(rhs)
	}
}

impl std::ops::BitOrAssign for Span {
	fn bitor_assign(&mut self, rhs: Self) {
		*self = *self | rhs;
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Spanned<T>(pub Span, pub T);

impl<T: std::fmt::Debug> std::fmt::Debug for Spanned<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if f.alternate() {
			self.0.fmt(f)?;
		}
		f.write_str("@")?;
		self.1.fmt(f)
	}
}

impl<A> Spanned<A> {
	pub fn map<B>(self, f: impl FnOnce(A) -> B) -> Spanned<B> {
		Spanned(self.0, f(self.1))
	}
}
