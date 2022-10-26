use std::{io, ops::Range };
pub use ansi_term::{Color::Fixed as Color, Style, ANSIString as Ansi};
use rangemap::RangeMap;

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
	pub fn encoding(encoding: &'static encoding_rs::Encoding) -> impl Fn(&mut Vec<Ansi>, &[u8]) {
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
	}
}

pub mod color {
	use super::*;

	pub fn ascii(byte: u8, mark: Option<u8>) -> Style {
		let mut s = Style::new();
		s = match byte {
			0x00        => s.dimmed(),
			0xFF        => s.fg(Color(9)), // bright red
			0x20..=0x7E => s.fg(Color(10)), // bright green
			_           => s,
		};
		if let Some(c) = mark {
			s = s.on(Color(c))
		}
		s
	}

	pub fn gray(byte: u8, mark: Option<u8>) -> Style {
		let mut s = Style::new();
		if let Some(c) = mark {
			s = s.fg(Color(c));
		} else {
			s = s.fg(Color(if byte <  0x30 { 245 } else { 236 }));
		}
		s = s.on(Color(if byte == 0x00 { 237 } else { 238 + byte / 16 }));
		s
	}

	pub fn none(_byte: u8, mark: Option<u8>) -> Style {
		let mut s = Style::new();
		if let Some(c) = mark {
			s = s.on(Color(c))
		}
		s
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DumpLength {
	None,
	Bytes(usize),
	Lines(usize),
	End(usize),
}

pub type ColorFn = dyn Fn(u8, Option<u8>) -> Style;
pub type PreviewFn = dyn Fn(&mut Vec<Ansi>, &[u8]);

#[must_use]
pub struct Dump<'a> {
	reader: Box<dyn io::Read + 'a>,
	start: usize,
	length: DumpLength,
	width: usize,
	color: Box<dyn Fn(u8, Option<u8>) -> Style + 'a>,
	#[allow(clippy::type_complexity)]
	preview: Option<Box<dyn Fn(&mut Vec<Ansi>, &[u8]) + 'a>>,
	num_width: usize,
	newline: bool,
	marks: RangeMap<usize, u8>,
}

impl<'a> Dump<'a> {
	pub fn new(reader: impl io::Read + 'a, start: usize) -> Self {
		Self {
			reader: Box::new(reader),
			start,
			width: 0,
			length: DumpLength::None,
			color: Box::new(color::ascii),
			preview: Some(Box::new(preview::ascii)),
			num_width: 1,
			newline: true,
			marks: RangeMap::new(),
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
		Dump { length: DumpLength::End(end), ..self }
	}

	pub fn width(self, width: usize) -> Self {
		assert!(width > 0);
		Dump { width, ..self }
	}

	pub fn color(self, color: impl Fn(u8, Option<u8>) -> Style + 'a) -> Self {
		Dump { color: Box::new(color), ..self }
	}

	pub fn preview(self, preview: impl Fn(&mut Vec<Ansi>, &[u8]) + 'a) -> Self {
		self.preview_option(Some(preview))
	}

	// This is different from a no-op preview function, since it affects the
	// default width and trailing space on the last line.
	pub fn no_preview(self) -> Self {
		Dump { preview: None, ..self }
	}

	#[cfg(feature = "encoding_rs")]
	pub fn preview_encoding(self, enc: &str) -> Self {
		let encoding = encoding_rs::Encoding::for_label_no_replacement(enc.as_bytes())
			.expect("invalid encoding");
		self.preview(preview::encoding(encoding))
	}

	pub fn preview_option(self, preview: Option<impl Fn(&mut Vec<Ansi>, &[u8]) + 'a>) -> Self {
		if let Some(a) = preview {
			Dump { preview: Some(Box::new(a)), ..self }
		} else {
			Dump { preview: None, ..self }
		}
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

	pub fn mark(self, pos: usize, color: u8) -> Self {
		self.mark_range(pos..pos+1, color)
	}

	pub fn mark_range(mut self, range: Range<usize>, color: u8) -> Self {
		if !range.is_empty() {
			self.marks.insert(range, color);
		}
		self
	}

	pub fn clear_marks(mut self) -> Self {
		self.marks = RangeMap::new(); // there is no RangeMap::clear, I posted #53
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
			DumpLength::End(l)   => Some(l.saturating_sub(self.start)),
		};

		let mut pos = self.start;
		let mut buf = vec![0; width];

		let num_width = if let Some(len) = len {
			self.num_width.max(format!("{:X}", self.start + len).len())
		} else {
			self.num_width
		};

		let mut first = true;

		let mut marks = self.marks.iter().peekable();

		loop {
			let buf = if let Some(len) = len {
				&mut buf[..width.min(self.start + len - pos)]
			} else {
				&mut buf[..width]
			};
			let nread = self.reader.read(buf)?;
			let buf = &buf[..nread];

			if buf.is_empty() && !first { break; }
			first = false;

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
				while marks.next_if(|(r, _)| pos >= r.end).is_some() {}
				let mark = marks.peek().and_then(|(r, &c)| r.contains(&pos).then_some(c));
				let style = (self.color)(b, mark);
				line.push(style.paint(format!(" {:02X}", b)));
				pos += 1;
			}

			line.push(" ".into());

			if let Some(preview) = &self.preview {
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

			if Some(pos) == len.map(|a| self.start+a) { break; }
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
