use std::{ops::Range, cell::RefCell};

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

thread_local! {
	pub static DIAGNOSTICS: RefCell<Vec<Diag>> = RefCell::default();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
	Error,
	Warning,
	Info,
	Note,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct Diag {
	pub level: Level,
	pub span: Span,
	pub text: String,
	pub notes: Vec<(Span, String)>,
}

impl Diag {
	pub fn new(level: Level, span: Span, text: impl ToString) -> Diag {
		Diag { level, span, text: text.to_string(), notes: Vec::new() }
	}

	pub fn error(span: Span, text: impl ToString) -> Diag {
		Self::new(Level::Error, span, text)
	}

	pub fn warn(span: Span, text: impl ToString) -> Diag {
		Self::new(Level::Error, span, text)
	}

	pub fn info(span: Span, text: impl ToString) -> Diag {
		Self::new(Level::Error, span, text)
	}

	pub fn note(mut self, span: Span, text: impl ToString) -> Diag {
		self.notes.push((span, text.to_string()));
		self
	}

	pub fn emit(self) {
		DIAGNOSTICS.with(|d| d.borrow_mut().push(self));
	}
}

// Note that calling [`Diag::emit`] outside of [`diagnose`] will cause the diagnostic to be
// leaked until the thread terminates.
pub fn diagnose<A>(f: impl FnOnce() -> A) -> (A, Vec<Diag>) {
	let prev = DIAGNOSTICS.with(|a| a.take());
	let v = f();
	let diag = DIAGNOSTICS.with(|a| a.replace(prev));
	(v, diag)
}

#[extend::ext]
pub impl<A, B> Result<A, B> {
	fn consume_err(self, f: impl FnOnce(B)) -> Option<A> {
		match self {
			Ok(a) => Some(a),
			Err(e) => {
				f(e);
				None
			}
		}
	}
}
