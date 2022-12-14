use hamu::read::le::*;
use hamu::write::le::*;
use crate::util::*;
use crate::tables::item::ItemId;

#[derive(Clone, PartialEq, Eq, derive_more::Deref, derive_more::DerefMut)]
pub struct Text(#[deref] #[deref_mut] pub Vec<TextSegment>);

impl std::fmt::Debug for Text {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("Text(")?;
		self.0.fmt(f)?;
		f.write_str(")")?;
		Ok(())
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextSegment {
	String(String),
	Line,
	Line2,
	Wait,
	Page,
	Color(u8),
	Item(ItemId),
	Byte(u8), // other byte of unknown meaning
}

impl Text {
	pub fn read<'a>(f: &mut impl In<'a>) -> Result<Text, ReadError> {
		let mut items = Vec::new();
		loop {
			items.push(match f.u8()? {
				0x00 => break,
				0x01 => TextSegment::Line,
				0x02 => TextSegment::Wait,
				0x03 => TextSegment::Page,
				0x07 => TextSegment::Color(f.u8()?),
				0x0D => TextSegment::Line2,
				0x1F => TextSegment::Item(ItemId(f.u16()?)),
				ch@(0x05 | 0x06 | 0x09 | 0x18) => TextSegment::Byte(ch),
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
		Ok(Text(items))
	}

	pub fn write(f: &mut impl Out, v: &Text) -> Result<(), WriteError> {
		for item in v.iter() {
			match &item {
				TextSegment::String(ref s) => f.slice(&encode(s)?),
				TextSegment::Line => f.u8(0x01),
				TextSegment::Wait => f.u8(0x02),
				TextSegment::Page => f.u8(0x03),
				TextSegment::Color(n) => { f.u8(0x07); f.u8(*n); }
				TextSegment::Line2 => f.u8(0x0D),
				TextSegment::Item(n) => { f.u8(0x1F); f.u16(n.0); }
				TextSegment::Byte(n) => f.u8(*n),
			}
		}
		f.u8(0);
		Ok(())
	}
}
