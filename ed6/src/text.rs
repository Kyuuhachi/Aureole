use hamu::read::le::*;
use hamu::write::le::*;
use crate::util::*;
use crate::tables::{face::FaceId, item::ItemId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text(pub Vec<TextSegment>);

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
	// Can't store only the parsed number, that makes roundtripping lossy
	A(String), // nullable
	Face(String), // nullable
	K(String),
	Pos(String),
	Ruby(String, String),
	Size(String),
	Speed(String), // nullable
	BrokenNoop, // Used in FC t2410:9, I'm pretty sure it's a bug
}

impl Text {
	pub fn read<'a>(f: &mut impl In<'a>) -> Result<Text, ReadError> {
		let mut segments = Vec::new();
		let mut curr = Vec::new();
		fn drain(segments: &mut Vec<TextSegment>, curr: &mut Vec<u8>) -> Result<(), DecodeError> {
			if !curr.is_empty() {
				segments.push(TextSegment::String(decode(curr)?));
			}
			curr.clear();
			Ok(())
		}

		loop {
			match f.u8()? {
				0x00 => { drain(&mut segments, &mut curr)?; break }
				0x01 => { drain(&mut segments, &mut curr)?; segments.push(TextSegment::Line) }
				0x02 => { drain(&mut segments, &mut curr)?; segments.push(TextSegment::Wait) }
				0x03 => { drain(&mut segments, &mut curr)?; segments.push(TextSegment::Page) }
				0x05 => { drain(&mut segments, &mut curr)?; segments.push(TextSegment::_05) }
				0x06 => { drain(&mut segments, &mut curr)?; segments.push(TextSegment::_06) }
				0x07 => { drain(&mut segments, &mut curr)?; segments.push(TextSegment::Color(f.u8()?)) }
				0x09 => { drain(&mut segments, &mut curr)?; segments.push(TextSegment::_09) }
				// 0x18 =>
				0x1F => { drain(&mut segments, &mut curr)?; segments.push(TextSegment::Item(ItemId(f.u16()?))) }
				ch@(0x00..=0x1F) => bail!("b{:?}", char::from(ch)),
				b'#' => {
					drain(&mut segments, &mut curr)?;

					let mut n = Vec::new();
					let ch = loop {
						match f.u8()? {
							ch@(b'0'..=b'9') => n.push(ch),
							ch => break ch,
						}
					};
					let n = std::str::from_utf8(&n).unwrap().to_owned();

					segments.push(match ch {
						0x02 if n.is_empty() => TextSegment::BrokenNoop,
						b'A' => TextSegment::A(n),
						b'F' => TextSegment::Face(n),
						b'K' => TextSegment::K(n),
						b'P' => TextSegment::Pos(n),
						b'R' => {
							let mut ruby = Vec::new();
							loop { match f.u8()? {
								b'#' => break,
								ch => ruby.push(ch),
							} }
							TextSegment::Ruby(n, decode(&ruby)?)
						},
						b'S' => TextSegment::Size(n),
						b'W' => TextSegment::Speed(n),
						ch => bail!("#{}{}", n, char::from(ch))
					});
				}
				ch => curr.push(ch)
			}
		}
		Ok(Text(segments))
	}

	pub fn write(f: &mut impl Out, v: &Text) -> Result<(), WriteError> {
		for seg in &v.0 {
			match seg {
				TextSegment::String(ref s) => f.slice(&encode(s)?),
				TextSegment::Line => f.u8(0x01),
				TextSegment::Wait => f.u8(0x02),
				TextSegment::Page => f.u8(0x03),
				TextSegment::_05 => f.u8(0x05),
				TextSegment::_06 => f.u8(0x06),
				TextSegment::Color(n) => {
					f.u8(0x07);
					f.u8(*n);
				},
				TextSegment::_09 => f.u8(0x09),
				TextSegment::Item(n) => {
					f.u8(0x1F);
					f.u16(n.0);
				},
				TextSegment::A(n)       => f.slice(format!("#{n}A").as_bytes()),
				TextSegment::Face(n)    => f.slice(format!("#{n}F").as_bytes()),
				TextSegment::K(n)       => f.slice(format!("#{n}K").as_bytes()),
				TextSegment::Pos(n)     => f.slice(format!("#{n}P").as_bytes()),
				TextSegment::Ruby(n, s) => {
					f.slice(format!("#{n}R").as_bytes());
					f.slice(&encode(s)?);
					f.u8(b'#');
				}
				TextSegment::Size(n)    => f.slice(format!("#{n}S").as_bytes()),
				TextSegment::Speed(n)   => f.slice(format!("#{n}W").as_bytes()),
				TextSegment::BrokenNoop => f.slice("#\x02".as_bytes())
					,
			}
		}
		f.u8(0);
		Ok(())
	}
}
