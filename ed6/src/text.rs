use hamu::read::le::*;
use crate::util::*;
use crate::tables::{face::FaceId, item::ItemId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text(pub Vec<TextSegment>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextSegment {
	String(String),
	Wait,
	Page,
	_05,
	_06,
	_09,
	Color(u8),
	Item(ItemId),
	A(Option<u16>),
	Face(Option<FaceId>),
	K(u16),
	Pos(u16),
	Ruby(u16, String),
	Size(u16),
	Speed(Option<u16>),
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
				0x01 => { curr.push(b'\n') }
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
					let n = std::str::from_utf8(&n).unwrap();
					let n2: Result<u16, ReadError> = n.parse::<u16>().map_err(|e| e.to_string().into());
					let n3: Result<Option<u16>, ReadError> = if n.is_empty() {
						Ok(None)
					} else {
						n.parse::<u16>().map_err(|e| e.to_string().into()).map(Some)
					};

					segments.push(match ch {
						0x02 if n.is_empty() => TextSegment::BrokenNoop,
						b'A' => TextSegment::A(n3?),
						b'F' => TextSegment::Face(n3?.map(FaceId)),
						b'K' => TextSegment::K(n2?),
						b'P' => TextSegment::Pos(n2?),
						b'R' => {
							let mut ruby = Vec::new();
							loop { match f.u8()? {
								b'#' => break,
								ch => ruby.push(ch),
							} }
							TextSegment::Ruby(n2?, decode(&ruby)?)
						},
						b'S' => TextSegment::Size(n2?),
						b'W' => TextSegment::Speed(n3?),
						ch => bail!("#{}{}", n, char::from(ch))
					});
				}
				ch => curr.push(ch)
			}
		}
		Ok(Text(segments))
	}
}
