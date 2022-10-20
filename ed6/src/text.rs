use hamu::read::le::*;
use hamu::write::le::*;
use crate::util::*;
use crate::tables::{face::FaceId, item::ItemId};

#[derive(Clone, PartialEq, Eq)]
pub struct Text(Box<[u8]>);

impl std::fmt::Debug for Text {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("Text::from_iter(")?;
		f.debug_list().entries(self.iter()).finish()?;
		f.write_str(")")?;
		Ok(())
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextSegment {
	String(String),
	Line,
	Wait,
	Page,
	_05,
	_06,
	Color(u8),
	_09,
	Item(ItemId),
	_18,
	Hash(HashSegment),

	// Invalid sjis, or hash sequence that did not parse correctly.
	// The one known instance is in FC t2410:9, where a line contains #\x02.
	Error(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HashSegment {
	NoA,
	A(u16),
	NoFace,
	Face(FaceId),
	K(u16),
	Pos(u16),
	Ruby(u16, String),
	Size(u16),
	NoSpeed,
	Speed(u16),
}

impl Text {
	pub fn read<'a>(f: &mut impl In<'a>) -> Result<Text, ReadError> {
		let pos = f.pos();
		loop {
			match f.u8()? {
				0x00 => break,
				0x01 | 0x02 | 0x03 | 0x05 | 0x06 | 0x09 => {}
				0x07 => { f.u8()?; }
				0x18 => {}
				0x1F => { f.u16()?; }
				ch@(0x00..=0x1F) => bail!("b{:?}", char::from(ch)),
				0x20.. => {}
			}
		}
		let end = f.pos();
		f.seek(pos)?;
		Ok(Text(Box::from(f.slice(end-pos)?)))
	}

	pub fn write(f: &mut impl Out, v: &Text) -> Result<(), WriteError> {
		f.slice(&v.0);
		Ok(())
	}

	pub fn iter(&self) -> Iter {
		Iter { data: &self.0, pos: 0 }
	}
}

impl<'a> IntoIterator for &'a Text {
	type Item = TextSegment;
	type IntoIter = Iter<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl std::iter::FromIterator<TextSegment> for Text {
	fn from_iter<T: IntoIterator<Item = TextSegment>>(iter: T) -> Self {
		let mut f = OutBytes::new();
		for item in iter {
			item.write_to(&mut f);
		}
		f.u8(0);
		Text(f.finish().unwrap().into_boxed_slice())
	}
}

impl<'a> std::iter::FromIterator<&'a TextSegment> for Text {
	fn from_iter<T: IntoIterator<Item = &'a TextSegment>>(iter: T) -> Self {
		let mut f = OutBytes::new();
		for item in iter {
			item.write_to(&mut f);
		}
		f.u8(0);
		Text(f.finish().unwrap().into_boxed_slice())
	}
}

pub struct Iter<'a> {
	data: &'a [u8],
	pos: usize,
}

impl Iter<'_> {
	fn parse_hash(&mut self) -> Option<HashSegment> {
		let start = self.pos;
		while (b'0'..=b'9').contains(&self.data[self.pos]) {
			self.pos += 1;
		}
		let n = std::str::from_utf8(&self.data[start..self.pos]).unwrap();
		let ch = self.data[self.pos];
		self.pos += 1;
		Some(match ch {
			b'A' if n.is_empty() => HashSegment::NoA,
			b'A' => HashSegment::A(n.parse().ok()?),
			b'F' if n.is_empty() => HashSegment::NoFace,
			b'F' => HashSegment::Face(FaceId(n.parse().ok()?)),
			b'K' => HashSegment::K(n.parse().ok()?),
			b'P' => HashSegment::Pos(n.parse().ok()?),
			b'R' => {
				let s = self.parse_string();
				let ch = self.data[self.pos];
				self.pos += 1;
				if ch != b'#' {
					return None
				}
				HashSegment::Ruby(n.parse().ok()?, s?)
			},
			b'S' => HashSegment::Size(n.parse().ok()?),
			b'W' => HashSegment::Speed(n.parse().ok()?),
			_ => return None
		})
	}

	fn parse_string(&mut self) -> Option<String> {
		let start = self.pos;
		while self.data[self.pos] >= 0x20 && self.data[self.pos] != b'#' {
			self.pos += 1;
		}
		let bytes = &self.data[start..self.pos];
		cp932::decode(bytes).ok()
	}
}

impl Iterator for Iter<'_> {
	type Item = TextSegment;

	fn next(&mut self) -> Option<Self::Item> {
		let start = self.pos;
		#[allow(clippy::match_overlapping_arm)]
		Some(match self.data[self.pos] {
			0x00 => return None,
			0x01 => { self.pos += 1; TextSegment::Line }
			0x02 => { self.pos += 1; TextSegment::Wait }
			0x03 => { self.pos += 1; TextSegment::Page }
			0x05 => { self.pos += 1; TextSegment::_05 }
			0x06 => { self.pos += 1; TextSegment::_06 }
			0x07 => {
				let n = self.data[self.pos+1];
				self.pos += 2;
				TextSegment::Color(n)
			}
			0x09 => { self.pos += 1; TextSegment::_09 }
			0x18 => { self.pos += 1; TextSegment::_18 }
			0x1F => {
				let n = u16::from_le_bytes([self.data[self.pos+1], self.data[self.pos+2]]);
				self.pos += 3;
				TextSegment::Item(ItemId(n))
			}
			a@(0x00..=0x1F) => unreachable!("{a:02X} {:02X?}", &self.data),
			b'#' => {
				self.pos += 1;
				if let Some(seg) = self.parse_hash() {
					TextSegment::Hash(seg)
				} else {
					if self.pos == self.data.len() {
						self.pos -= 1;
					}
					TextSegment::Error(self.data[start..self.pos].to_owned())
				}
			}
			0x20.. => {
				if let Some(s) = self.parse_string() {
					TextSegment::String(s)
				} else {
					TextSegment::Error(self.data[start..self.pos].to_owned())
				}
			}
		})
	}
}

impl TextSegment {
	fn write_to(&self, f: &mut OutBytes) {
		match self {
			TextSegment::String(ref s) => {
				f.slice(&cp932::encode(s).unwrap());
			}
			TextSegment::Line => f.u8(0x01),
			TextSegment::Wait => f.u8(0x02),
			TextSegment::Page => f.u8(0x03),
			TextSegment::_05 => f.u8(0x05),
			TextSegment::_06 => f.u8(0x06),
			TextSegment::Color(n) => {
				f.u8(0x07);
				f.u8(*n);
			}
			TextSegment::_09 => f.u8(0x09),
			TextSegment::_18 => f.u8(0x18),
			TextSegment::Item(n) => {
				f.u8(0x1F);
				f.u16(n.0);
			}

			TextSegment::Hash(a) => {
				let s = a.to_hash();
				f.slice(&cp932::encode(&s).unwrap());
			}

			TextSegment::Error(ref s) => f.slice(s),
		}
	}
}

impl HashSegment {
	pub fn to_hash(&self) -> std::borrow::Cow<str> {
		match self {
			HashSegment::NoA             => "#A".into(),
			HashSegment::A(n)            => format!("#{n}A").into(),
			HashSegment::NoFace          => "#F".into(),
			HashSegment::Face(FaceId(n)) => format!("#{n}F").into(),
			HashSegment::K(n)            => format!("#{n}K").into(),
			HashSegment::Pos(n)          => format!("#{n}P").into(),
			HashSegment::Ruby(n, s)      => format!("#{n}R{s}#").into(),
			HashSegment::Size(n)         => format!("#{n}S").into(),
			HashSegment::NoSpeed         => "#W".into(),
			HashSegment::Speed(n)        => format!("#{n}W").into(),
		}
	}
}
