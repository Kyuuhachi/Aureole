use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _};
use crate::util::*;
use crate::types::ItemId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
	pub pages: Vec<Vec<TextSegment>>
}

#[derive(Clone, PartialEq, Eq)]
pub enum TextSegment {
	String(String),
	Line,
	Wait,
	Color(u8),
	Item(ItemId),
	Byte(u8), // other byte of unknown meaning
}

impl std::fmt::Debug for TextSegment {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::String(v) => v.fmt(f),
			Self::Line => write!(f, "Line"),
			Self::Wait => write!(f, "Wait"),
			Self::Color(v) => f.debug_tuple("Color").field(v).finish(),
			Self::Item(v) => f.debug_tuple("Item").field(v).finish(),
			Self::Byte(v) => f.debug_tuple("Byte").field(v).finish(),
		}
	}
}

impl Text {
	pub fn read(f: &mut Reader) -> Result<Text, ReadError> {
		let mut pages = vec![Vec::new()];
		let mut items = pages.last_mut().unwrap();
		loop {
			items.push(match f.u8()? {
				0x00 => break,
				0x01 => TextSegment::Line,
				0x02 => TextSegment::Wait,
				0x03 => {
					pages.push(Vec::new());
					items = pages.last_mut().unwrap();
					continue;
				}
				0x07 => TextSegment::Color(f.u8()?),
				0x1F => TextSegment::Item(ItemId(f.u16()?)),
				ch@(0x05 | 0x06 | 0x09 | 0x0D | 0x18) => TextSegment::Byte(ch),
				ch@(0x0A | 0x0C) => TextSegment::Byte(ch), // Geofront Azure only
				ch@(0x00..=0x1F) => bail!("b{:?}", char::from(ch)),
				0x20.. => {
					let start = f.pos() - 1;
					while f.u8()? >= 0x20 { }
					let len = f.pos() - start - 1;
					f.seek(start)?;
					TextSegment::String(decode(f.slice(len)?)?)
				}
			})
		}
		Ok(Text { pages })
	}

	pub fn write(f: &mut Writer, v: &Text) -> Result<(), WriteError> {
		for (i, page) in v.pages.iter().enumerate() {
			if i != 0 {
				f.u8(0x03); // page
			}
			for item in page {
				match &item {
					TextSegment::String(ref s) => f.slice(&encode(s)?),
					TextSegment::Line => f.u8(0x01),
					TextSegment::Wait => f.u8(0x02),
					TextSegment::Color(n) => { f.u8(0x07); f.u8(*n); }
					TextSegment::Item(n) => { f.u8(0x1F); f.u16(n.0); }
					TextSegment::Byte(n) => f.u8(*n),
				}
			}
		}
		f.u8(0);
		Ok(())
	}
}
