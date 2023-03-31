use std::fmt;
use gospel::read::Reader;

#[derive(Clone, Copy)]
pub struct Dump<'a> {
	start: usize,
	end: usize,
	data: &'a [u8],
	length_as: usize,
}

pub fn dump<'a>(f: &Reader<'a>) -> Dump<'a> {
	Dump {
		start: f.pos(),
		end: f.len(),
		data: f.data(),
		length_as: f.len(),
	}
}

impl Dump<'_> {
	pub fn start(self, start: usize) -> Self { Self { start, ..self } }
	pub fn end(self, end: usize) -> Self { Self { end, ..self } }
	pub fn length_as(self, length_as: usize) -> Self { Self { length_as, ..self } }
}

impl fmt::UpperHex for Dump<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.print(f, |x, f| write!(f, "{x:02X}"), 2)
	}
}

impl fmt::LowerHex for Dump<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.print(f, |x, f| write!(f, "{x:02x}"), 2)
	}
}

impl fmt::Binary for Dump<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.print(f, |x, f| write!(f, "{x:08b}"), 8)
	}
}

impl Dump<'_> {
	fn print(
		self,
		f: &mut fmt::Formatter,
		write: impl Fn(u8, &mut fmt::Formatter) -> fmt::Result,
		cell_width: usize,
	) -> fmt::Result {
		const SCREEN_WIDTH: usize = 240;

		let num_width = if self.length_as == 0 {
			0
		} else {
			format!("{:X}", self.length_as).len()
		};

		let has_text = !f.sign_minus();
		let lines = f.width().unwrap_or(usize::MAX);
		let width = f.precision().unwrap_or_else(|| {
			let c = cell_width + 1 + usize::from(has_text);
			let w = num_width + usize::from(num_width != 0) + usize::from(has_text);
			(SCREEN_WIDTH - w) / c / 4 * 4
		}).max(1);

		for (i, chunk) in self.data[self.start..self.end].chunks(width).take(lines).enumerate() {
			let pos = self.start + i * width;

			if num_width > 0 {
				let s = format!("{:X}", pos);
				if s.len() < num_width {
					sgr(f, "2;33")?;
					for _ in s.len()..num_width {
						f.write_str("0")?;
					}
				}
				sgr(f, "33")?;
				f.write_str(&s)?;
				sgr(f, "")?;
				f.write_str(" ")?;
			}

			let mut prev_c = "";
			for (i, &b) in chunk.iter().enumerate() {
				if i != 0 {
					f.write_str(" ")?;
				}

				let c = match b {
					0x00        => "2",
					0xFF        => "38;5;9",
					0x20..=0x7E => "38;5;10",
					_           => "",
				};

				if prev_c != c {
					sgr(f, c)?;
					prev_c = c;
				}
				write(b, f)?;
			}
			sgr(f, "")?;

			if has_text {
				for _ in chunk.len()..width {
					f.write_str("   ")?;
				}
				f.write_str(" ▏")?;

				let mut chunk = chunk;
				let mut prev_c = "";
				while !chunk.is_empty() {
					let (a, n) = cp932::decode_char(chunk).unwrap_or(('\0', 1));
					let (c, ch) = match chunk[0] {
						0x00 => ("38;5;8", '·'),
						0xFF => ("31", '·'),
						0x20..=0x7E|0x81..=0xEA if !a.is_control() => ("", a),
						_ => ("2", '·'),
					};
					if prev_c != c {
						sgr(f, c)?;
						prev_c = c;
					}
					write!(f, "{ch}")?;
					chunk = &chunk[n..];
				}
				sgr(f, "")?;
			}
			f.write_str("\n")?;
		}

		Ok(())
	}
}

fn sgr(f: &mut fmt::Formatter, arg: &str) -> fmt::Result {
	if f.alternate() {
		write!(f, "\x1B[0;{arg}m")
	} else {
		Ok(())
	}
}
