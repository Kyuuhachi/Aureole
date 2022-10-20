mod enc;
mod dec;

pub fn decode(bytes: &[u8]) -> Result<String, usize> {
	let mut out = String::with_capacity(bytes.len());
	let mut pos = 0;
	while pos < bytes.len() {
		let (ch, len) = decode_char(&bytes[pos..]);
		let ch = ch.ok_or(pos)?;
		out.push(ch);
		pos += len;
	}
	Ok(out)
}

pub fn decode_lossy(bytes: &[u8]) -> String {
	let mut out = String::with_capacity(bytes.len());
	let mut pos = 0;
	while pos < bytes.len() {
		let (ch, len) = decode_char(&bytes[pos..]);
		let ch = ch.unwrap_or('�');
		out.push(ch);
		pos += len;
	}
	out
}

fn decode_char(bytes: &[u8]) -> (Option<char>, usize) {
	use std::char::from_u32 as ch;
	let c = match bytes.first() {
		None => return (None, 1),
		Some(c) => *c,
	};
	match c {
		0x00..=0x80 => (ch(c as u32), 1),
		0xA0        => (ch(0xF8F0), 1), // half-width katakana
		0xA1..=0xDF => (ch(0xFEC0 + c as u32), 1),
		0xFD..=0xFF => (ch(0xF8F1 - 0xFD + c as u32), 1), // Windows compatibility
		_ => {
			let c2 = match bytes.get(1) {
				None => return (None, 2),
				Some(c2) => *c2,
			};

			if let Some(ch) = dec::cp932ext(c, c2) {
				return (Some(ch), 2)
			}

			let c2 = match c2 {
				0x40..=0x7E => c2 - 0x40,
				0x80..=0xFC => c2 - 0x41,
				_ => return (None, 2),
			};
			match c {
				0x81..=0x9F | 0xE0..=0xEA => {
					let c1 = (c - 1) % 0x40;
					let c1 = (c1 << 1) + c2 / 0x5E;
					let c2 = c2 % 0x5E;
					let (c1, c2) = (c1 + 0x21, c2 + 0x21);
					(dec::jisx0208(c1, c2), 2)
				}
				0xF0..=0xF9 => {
					(ch(0xE000 + 188 * (c - 0xF0) as u32 + c2 as u32), 2)
				}
				_ => (None, 2)
			}
		}
	}
}

pub fn encode(text: &str) -> Result<Vec<u8>, usize> {
	let mut out = Vec::new();
	for (pos, ch) in text.char_indices() {
		let ch = ch as u32;
		match ch {
			0x00..=0x80 => out.push(ch as u8),
			0xFF61..=0xFF9F => out.push((ch-0xFEC0) as u8),
			0xF8F0 => out.push(0xA0),
			0xF8F1..=0xF8F3 => out.push((ch - 0xF8F1 + 0xFD) as u8),
			0xFFFF.. => return Err(pos),
			_ => {
				if let Some([c1, c2]) = enc::cp932ext(ch as u16) {
					out.push(c1);
					out.push(c2);
					continue
				}

				if let Some([c1, c2]) = enc::jisxcommon(ch as u16) {
					if c2 & 0x80 != 0 {
						return Err(pos) // MSB set: JIS X 0212
					}

					let (c1, c2) = (c1 - 0x21, c2 - 0x21);
					let c2 = (c1 & 1) * 0x5E + c2;
					let c1 = (c1 >> 1) + 1;
					out.push(if c1 < 0x20 { c1 + 0x80 } else { c1 + 0xC0 });
					out.push(if c2 < 0x3F { c2 + 0x40 } else { c2 + 0x41 });
				} else if (0xE000..0xE000+1880).contains(&ch) {
					let c1 = ((ch - 0xE000) / 188) as u8;
					let c2 = ((ch - 0xE000) % 188) as u8;
					out.push(c1 + 0xF0);
					out.push(if c2 < 0x3F { c2 + 0x40 } else { c2 + 0x41 });
				} else {
					return Err(pos)
				}
			}
		}
	}
	Ok(out)
}
