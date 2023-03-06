use std::cell::RefCell;

use crate::span::{Span, Spanned};

thread_local! {
	pub static DIAGNOSTICS: RefCell<Vec<Diag>> = RefCell::default();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
	Error,
	Warning,
	Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct Diag {
	pub level: Level,
	pub text: Spanned<String>,
	pub notes: Vec<Spanned<String>>,
}

impl Diag {
	pub fn new(level: Level, span: Span, text: impl ToString) -> Diag {
		Diag { level, text: Spanned(span, text.to_string()), notes: Vec::new() }
	}

	pub fn error(span: Span, text: impl ToString) -> Diag {
		Self::new(Level::Error, span, text)
	}

	pub fn warn(span: Span, text: impl ToString) -> Diag {
		Self::new(Level::Warning, span, text)
	}

	pub fn info(span: Span, text: impl ToString) -> Diag {
		Self::new(Level::Info, span, text)
	}

	pub fn note(mut self, span: Span, text: impl ToString) -> Diag {
		self.notes.push(Spanned(span, text.to_string()));
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

pub fn print_diags(filename: &str, source: &str, diags: &[Diag]) {
	use codespan_reporting::diagnostic::{Diagnostic, Label};
	use codespan_reporting::files::SimpleFiles;
	use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

	let writer = StandardStream::stderr(ColorChoice::Always);
	let config = codespan_reporting::term::Config::default();
	let mut files = SimpleFiles::new();
	let file_id = files.add(filename, source);

	for d in diags {
		let mut l = vec![
			Label::primary(file_id, d.text.0.as_range()).with_message(&d.text.1),
		];
		for n in &d.notes {
			l.push(Label::secondary(file_id, n.0.as_range()).with_message(&n.1));
		}
		let d = match d.level {
			Level::Error => Diagnostic::error(),
			Level::Warning => Diagnostic::warning(),
			Level::Info => Diagnostic::help(),
		};
		let d = d.with_labels(l);
		codespan_reporting::term::emit(&mut writer.lock(), &config, &files, &d).unwrap();
	}
}
