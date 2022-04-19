use std::{
	io::{self, Write},
	collections::BTreeMap,
};
use crate::read;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Read error")]
	Read(#[from] read::Error),
	#[error("Write error")]
	Write(#[from] io::Error),
}
pub type Result<T, E=Error> = std::result::Result<T, E>;

pub mod preview {
	use std::io::Write;
	use super::Result;

	pub fn ascii(buf: &mut dyn Write, data: &[u8]) -> Result<()> {
		for b in data {
			match b {
				0x20..=0x7E => write!(buf, "{}", std::str::from_utf8(&[*b]).unwrap())?,
				_           => write!(buf, "\x1B[2m·\x1B[22m")?,
			};
		}
		Ok(())
	}
}

pub mod color {
	use std::io::Write;
	use super::Result;

	pub fn ascii(buf: &mut dyn Write, byte: u8) -> Result<()> {
		match byte {
			0x00        => write!(buf, "\x1B[0;2m")?,
			0xFF        => write!(buf, "\x1B[0;38;5;9m")?,
			0x20..=0x7E => write!(buf, "\x1B[0;38;5;10m")?,
			_           => write!(buf, "\x1B[0m")?,
		};
		Ok(())
	}

	pub fn gray(buf: &mut dyn Write, byte: u8) -> Result<()> {
		write!(buf, "\x1B[0;48;5;{};38;5;{}m",
			if byte == 0x00 { 237 } else { 238 + byte / 16 },
			if byte < 0x30 { 245 } else { 236 },
		)?;
		Ok(())
	}

	pub fn none(_buf: &mut dyn Write, _byte: u8) -> Result<()> { Ok(()) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DumpLength {
	Bytes(usize),
	Lines(usize),
}

#[derive(Clone)]
pub struct DumpSpec<'a> {
	length: DumpLength,
	width: usize,
	color: &'a dyn Fn(&mut dyn Write, u8) -> Result<()>,
	#[allow(clippy::type_complexity)]
	preview: Option<&'a dyn Fn(&mut dyn Write, &[u8]) -> Result<()>>,
	number_width: Option<usize>,
	newline: bool,
	marks: BTreeMap<usize, String>,
}

impl<'a> DumpSpec<'a> {
	pub fn new() -> Self {
		Self {
			width: 0,
			length: DumpLength::Bytes(usize::MAX),
			color: &color::ascii,
			preview: Some(&preview::ascii),
			number_width: Some(0),
			newline: true,
			marks: BTreeMap::new(),
		}
	}

	pub fn oneline() -> Self {
		Self::new().lines(1).newline(false)
	}

	pub fn lines(self, lines: usize) -> Self {
		DumpSpec { length: DumpLength::Lines(lines), ..self }
	}

	pub fn bytes(self, bytes: usize) -> Self {
		DumpSpec { length: DumpLength::Bytes(bytes), ..self }
	}

	pub fn width(self, width: usize) -> Self {
		assert!(width > 0);
		DumpSpec { width, ..self }
	}

	pub fn color(self, color: &'a dyn Fn(&mut dyn Write, u8) -> Result<()>) -> Self {
		DumpSpec { color, ..self }
	}

	pub fn preview(self, preview: &'a dyn Fn(&mut dyn Write, &[u8]) -> Result<()>) -> Self {
		DumpSpec { preview: Some(preview), ..self }
	}

	// This is different from a no-op preview function, since it affects the
	// default width and trailing space on the last line.
	pub fn no_preview(self) -> Self {
		DumpSpec { preview: None, ..self }
	}

	pub fn number_width(self, size: usize) -> Self {
		DumpSpec { number_width: Some(size), ..self }
	}

	pub fn no_number(self) -> Self {
		DumpSpec { number_width: None, ..self }
	}

	pub fn newline(self, newline: bool) -> Self {
		DumpSpec { newline, ..self }
	}

	pub fn mark(mut self, pos: usize, mark: String) -> Self {
		self.marks.insert(pos, mark);
		self
	}

	pub fn marks<T>(mut self, iter: T) -> Self where T: IntoIterator, BTreeMap<usize, String>: Extend<T::Item> {
		self.marks.extend(iter);
		self
	}
}

impl Default for DumpSpec<'_> {
	fn default() -> Self {
		Self::new()
	}
}

#[extend::ext(name=Dump)]
pub impl read::In<'_> {
	fn dump<W: Write>(&mut self, out: W, spec: &DumpSpec) -> Result<()> {
		let mut out = io::BufWriter::new(out);
		let width = match spec.width {
			0 if spec.preview.is_some() => 48,
			0 if spec.preview.is_none() => 72,
			w => w,
		};

		let start = self.pos();
		let end = start + match spec.length {
			DumpLength::Bytes(b) => b,
			DumpLength::Lines(l) => l * width,
		}.min(self.remaining());
		let mut pos = start;

		let mut marks = spec.marks.range(start..=end).peekable();
		let number_width = spec.number_width.map(|a| a.max(format!("{:X}", self.len()).len()));

		loop {
			if let Some(number_width) = number_width {
				let mut s = format!("{:X}", pos);
				if s.len() < number_width {
					s = format!("\x1B[2m{}\x1B[22m{}", "0".repeat(number_width - s.len()), s);
				}
				write!(out, "\x1B[33m{}\x1B[m", s)?;
			}

			let data = self.slice(width.min(end - pos))?;
			for &b in data {
				match marks.next_if(|&(&a, _)| a <= pos) {
					Some((_, mark)) => write!(out, "{}", mark)?,
					_ => write!(out, " ")?,
				}
				(spec.color)(&mut out, b)?;
				write!(out, "{:02X}", b)?;
				pos += 1;
			}
			// If a mark lands on a line break, we'll print it both before and after because why not.
			match marks.peek() {
				Some(&(&a, mark)) if a <= pos => write!(out, "{}", mark)?,
				_ => write!(out, " ")?,
			}
			write!(out, "\x1B[m")?;

			if let Some(preview) = spec.preview {
				if data.len() < width {
					write!(out, "{}", "   ".repeat(width - data.len()))?;
				}
				write!(out, "▏")?;
				(preview)(&mut out, data)?;
			}

			writeln!(out)?;
			if pos == end { break; }
		}

		if spec.newline {
			writeln!(out)?;
		}

		Ok(())
	}

	fn edump(&mut self, spec: &DumpSpec) -> Result<()> {
		self.dump(std::io::stderr(), spec)
	}
}
