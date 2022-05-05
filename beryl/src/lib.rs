use std::{
	collections::BTreeMap,
	io,
};
pub use ansi_term::{Color::Fixed as Color, Style, ANSIString as Ansi};

pub mod preview {
	use super::*;

	pub fn ascii(f: &mut Vec<Ansi>, data: &[u8]) {
		for b in data {
			f.push(match b {
				0x20..=0x7E => (*b as char).to_string().into(),
				_           => Style::new().dimmed().paint("·"),
			})
		}
	}

	#[cfg(feature = "encoding_rs")]
	pub fn encoding(label: &[u8]) -> Option<impl Fn(&mut Vec<Ansi>, &[u8])> {
		encoding_rs::Encoding::for_label_no_replacement(label)
		.map(|encoding| {
			move |f: &mut Vec<Ansi>, data: &[u8]| {
				for c in encoding.decode_without_bom_handling(data).0.chars() {
					if c.is_control() {
						f.push(Style::new().dimmed().paint("·"))
					} else if c == '�' {
						f.push(Style::new().dimmed().paint("�"))
					} else {
						f.push(c.to_string().into())
					}
				}
			}
		})
	}
}

pub mod color {
	use super::*;

	pub fn ascii(byte: u8) -> Style {
		match byte {
			0x00        => Style::new().dimmed(),
			0xFF        => Style::new().fg(Color(9)), // bright red
			0x20..=0x7E => Style::new().fg(Color(10)), // bright green
			_           => Style::new(),
		}
	}

	pub fn gray(byte: u8) -> Style {
		Style::new()
			.fg(Color(if byte <  0x30 { 245 } else { 236 }))
			.on(Color(if byte == 0x00 { 237 } else { 238 + byte / 16 }))
	}

	pub fn none(_byte: u8) -> Style { Style::new() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DumpLength {
	None,
	Bytes(usize),
	Lines(usize),
}

pub type ColorFn = dyn Fn(u8) -> Style;
pub type PreviewFn = dyn Fn(&mut Vec<Ansi>, &[u8]);

#[must_use]
pub struct Dump<'a> {
	reader: Box<dyn io::Read + 'a>,
	start: usize,
	length: DumpLength,
	width: usize,
	color: &'a ColorFn,
	preview: Option<&'a PreviewFn>,
	num_width: usize,
	newline: bool,
	marks: BTreeMap<usize, Ansi<'a>>,
}

impl<'a> Dump<'a> {
	pub fn new(reader: impl io::Read + 'a, start: usize) -> Self {
		Self {
			reader: Box::new(reader),
			start,
			width: 0,
			length: DumpLength::None,
			color: &color::ascii,
			preview: Some(&preview::ascii),
			num_width: 1,
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
		Dump { length: DumpLength::Bytes(end - self.start), ..self }
	}

	pub fn width(self, width: usize) -> Self {
		assert!(width > 0);
		Dump { width, ..self }
	}

	pub fn color(self, color: &'a ColorFn) -> Self {
		Dump { color, ..self }
	}

	pub fn preview(self, preview: &'a PreviewFn) -> Self {
		self.preview_option(Some(preview))
	}

	// This is different from a no-op preview function, since it affects the
	// default width and trailing space on the last line.
	pub fn no_preview(self) -> Self {
		self.preview_option(None)
	}

	pub fn preview_option(self, preview: Option<&'a PreviewFn>) -> Self {
		Dump { preview, ..self }
	}

	pub fn num_width(self, num_width: usize) -> Self {
		Dump { num_width, ..self }
	}

	pub fn num_width_from(self, max: usize) -> Self {
		self.num_width(format!("{max:X}").len())
	}

	pub fn newline(self, newline: bool) -> Self {
		Dump { newline, ..self }
	}

	pub fn mark(mut self, pos: usize, mark: impl Into<Ansi<'a>>) -> Self {
		self.marks.insert(pos, mark.into());
		self
	}

	pub fn marks(mut self, iter: impl Iterator<Item=(usize, impl Into<Ansi<'a>>)>) -> Self {
		for (k, v) in iter {
			self = self.mark(k, v);
		}
		self
	}

	pub fn write_to(mut self, out: &mut impl io::Write, ansi: bool) -> io::Result<()> {
		let width = match self.width {
			0 if self.preview.is_some() => 48,
			0 if self.preview.is_none() => 72,
			w => w,
		};

		let len = match self.length {
			DumpLength::None => None,
			DumpLength::Bytes(b) => Some(b),
			DumpLength::Lines(l) => Some(l * width),
		};

		let mut marks = self.marks.range(self.start..).peekable();
		let mut pos = self.start;
		let mut buf = vec![0; width];

		let num_width = if let Some(len) = len {
			self.num_width.max(format!("{:X}", self.start + len).len())
		} else {
			self.num_width
		};

		loop {
			let buf = if let Some(len) = len {
				&mut buf[..width.min(self.start + len - pos)]
			} else {
				&mut buf[..width]
			};
			let nread = self.reader.read(buf)?;
			let buf = &buf[..nread];


			let mut line = Vec::new();
			if self.num_width > 0 {
				let s = format!("{:X}", pos);
				let style = Style::new().fg(Color(3));
				if s.len() < num_width {
					line.push(style.dimmed().paint("0".repeat(num_width - s.len())));
				}
				line.push(style.paint(s));
			}

			for &b in buf {
				match marks.next_if(|&(&a, _)| a <= pos) {
					Some((_, mark)) => line.push(mark.clone()), // cloning here is a little wasteful
					_ => line.push(" ".into()),
				}
				let style = (self.color)(b);
				line.push(style.paint(format!("{:02X}", b)));
				pos += 1;
			}
			// If a mark lands on a line break, we'll print it both before and after because why not.
			match marks.peek() {
				Some(&(&a, mark)) if a <= pos => line.push(mark.clone()),
				_ => line.push(" ".into()),
			}

			if let Some(preview) = self.preview {
				if buf.len() < width {
					line.push("   ".repeat(width - buf.len()).into());
				}
				line.push("▏".into());
				(preview)(&mut line, buf);
			}

			if ansi {
				write!(out, "{}", ansi_term::ANSIStrings(&line))?;
			} else {
				for seg in &line {
					write!(out, "{}", &**seg)?;
				}
			}
			writeln!(out)?;

			if Some(pos) == len.map(|a| self.start+a) || buf.is_empty() { break; }
		}

		if self.newline {
			writeln!(out)?;
		}

		Ok(())
	}

	pub fn to_string(self, ansi: bool) -> String {
		let mut out = Vec::new();
		self.write_to(&mut out, ansi).unwrap();
		String::from_utf8(out).unwrap()
	}

	pub fn to_stdout(self) {
		let mut out = io::stdout().lock();
		let mut ansi = std::env::var_os("NO_COLOR").is_none();
		#[cfg(feature="atty")]
		{ ansi &= atty::is(atty::Stream::Stdout); }
		self.write_to(&mut out, ansi).unwrap()
	}

	pub fn to_stderr(self) {
		let mut out = io::stderr().lock();
		let mut ansi = std::env::var_os("NO_COLOR").is_none();
		#[cfg(feature="atty")]
		{ ansi &= atty::is(atty::Stream::Stderr); }
		self.write_to(&mut out, ansi).unwrap()
	}
}
