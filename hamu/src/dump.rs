use std::{
	collections::BTreeMap,
	fmt::{Formatter, Result},
};
use crate::read::In;

pub mod preview {
	use super::{Formatter, Result};

	pub fn ascii(f: &mut Formatter, data: &[u8]) -> Result {
		for b in data {
			match b {
				0x20..=0x7E => write!(f, "{}", std::str::from_utf8(&[*b]).unwrap())?,
				_           => write!(f, "\x1B[2m·\x1B[22m")?,
			};
		}
		Ok(())
	}
}

pub mod color {
	use super::{Formatter, Result};

	pub fn ascii(f: &mut Formatter, byte: u8) -> Result {
		match byte {
			0x00        => write!(f, "\x1B[0;2m")?,
			0xFF        => write!(f, "\x1B[0;38;5;9m")?,
			0x20..=0x7E => write!(f, "\x1B[0;38;5;10m")?,
			_           => write!(f, "\x1B[0m")?,
		};
		Ok(())
	}

	pub fn gray(f: &mut Formatter, byte: u8) -> Result {
		write!(f, "\x1B[0;48;5;{};38;5;{}m",
			if byte == 0x00 { 237 } else { 238 + byte / 16 },
			if byte < 0x30 { 245 } else { 236 },
		)?;
		Ok(())
	}

	pub fn none(_buf: &mut Formatter, _byte: u8) -> Result { Ok(()) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DumpLength {
	Bytes(usize),
	Lines(usize),
}

#[must_use]
#[derive(Clone)]
pub struct Dump<'a> {
	i: &'a In<'a>,
	length: DumpLength,
	width: usize,
	color: &'a dyn Fn(&mut Formatter, u8) -> Result,
	#[allow(clippy::type_complexity)]
	preview: Option<&'a dyn Fn(&mut Formatter, &[u8]) -> Result>,
	number_width: Option<usize>,
	newline: bool,
	marks: BTreeMap<usize, String>,
}

impl<'a> Dump<'a> {
	pub fn new(i: &'a In<'a>) -> Self {
		Self {
			i,
			width: 0,
			length: DumpLength::Bytes(usize::MAX),
			color: &color::ascii,
			preview: Some(&preview::ascii),
			number_width: Some(0),
			newline: true,
			marks: BTreeMap::new(),
		}
	}

	pub fn oneline(self) -> Self {
		self.lines(1).newline(false)
	}

	pub fn lines(self, lines: usize) -> Self {
		Dump { length: DumpLength::Lines(lines), ..self }
	}

	pub fn bytes(self, bytes: usize) -> Self {
		Dump { length: DumpLength::Bytes(bytes), ..self }
	}

	pub fn end(self, end: usize) -> Self {
		Dump { length: DumpLength::Bytes(end - self.i.pos()), ..self }
	}

	pub fn width(self, width: usize) -> Self {
		assert!(width > 0);
		Dump { width, ..self }
	}

	pub fn color(self, color: &'a dyn Fn(&mut Formatter, u8) -> Result) -> Self {
		Dump { color, ..self }
	}

	pub fn preview(self, preview: &'a dyn Fn(&mut Formatter, &[u8]) -> Result) -> Self {
		Dump { preview: Some(preview), ..self }
	}

	// This is different from a no-op preview function, since it affects the
	// default width and trailing space on the last line.
	pub fn no_preview(self) -> Self {
		Dump { preview: None, ..self }
	}

	pub fn number_width(self, size: usize) -> Self {
		Dump { number_width: Some(size), ..self }
	}

	pub fn no_number(self) -> Self {
		Dump { number_width: None, ..self }
	}

	pub fn newline(self, newline: bool) -> Self {
		Dump { newline, ..self }
	}

	pub fn mark(mut self, pos: usize, mark: impl AsRef<str>) -> Self {
		self.marks.insert(pos, mark.as_ref().to_owned());
		self
	}

	pub fn marks(mut self, iter: impl Iterator<Item=(impl std::ops::Deref<Target=usize>, impl AsRef<str>)>) -> Self {
		for (k, v) in iter {
			self = self.mark(*k, v);
		}
		self
	}
}

impl std::fmt::Display for Dump<'_> {
	fn fmt(&self, f: &mut Formatter) -> Result {
		let width = match self.width {
			0 if self.preview.is_some() => 48,
			0 if self.preview.is_none() => 72,
			w => w,
		};

		let start = self.i.pos();
		let end = start + match self.length {
			DumpLength::Bytes(b) => b,
			DumpLength::Lines(l) => l * width,
		}.min(self.i.remaining());
		let mut pos = start;

		let mut marks = self.marks.range(start..=end).peekable();
		let number_width = self.number_width.map(|a| a.max(format!("{:X}", self.i.len()).len()));

		loop {
			if let Some(number_width) = number_width {
				let mut s = format!("{:X}", pos);
				if s.len() < number_width {
					s = format!("\x1B[2m{}\x1B[22m{}", "0".repeat(number_width - s.len()), s);
				}
				write!(f, "\x1B[33m{}\x1B[m", s)?;
			}

			let data = &self.i.data()[pos..pos+width.min(end - pos)];
			for &b in data {
				match marks.next_if(|&(&a, _)| a <= pos) {
					Some((_, mark)) => write!(f, "{}", mark)?,
					_ => write!(f, " ")?,
				}
				(self.color)(f, b)?;
				write!(f, "{:02X}", b)?;
				pos += 1;
			}
			// If a mark lands on a line break, we'll print it both before and after because why not.
			match marks.peek() {
				Some(&(&a, mark)) if a <= pos => write!(f, "{}", mark)?,
				_ => write!(f, " ")?,
			}
			write!(f, "\x1B[m")?;

			if let Some(preview) = self.preview {
				if data.len() < width {
					write!(f, "{}", "   ".repeat(width - data.len()))?;
				}
				write!(f, "▏")?;
				(preview)(f, data)?;
			}

			writeln!(f)?;
			if pos == end { break; }
		}

		if self.newline {
			writeln!(f)?;
		}

		Ok(())
	}
}

impl Dump<'_> {
	pub fn to_stdout(&self) {
		println!("{}", self);
	}

	pub fn to_stderr(&self) {
		eprintln!("{}", self);
	}
}
