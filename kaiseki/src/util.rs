use std::{ops::Range, fmt::Display};

use encoding_rs::SHIFT_JIS;
use itermore::Itermore;
use derive_more::*;
use hamu::read::{In, Le};

#[derive(Debug, thiserror::Error)]
pub enum StringError {
	#[error("read error")]
	Read(#[from] hamu::read::Error),
	#[error("decode error")]
	Decode(#[from] DecodeError),
}

#[extend::ext(name=InExt)]
pub impl In<'_> where Self: Sized {
	fn string(&mut self) -> Result<String, StringError> {
		let mut s = Vec::new();
		loop {
			match self.u8()? {
				0 => break,
				b => s.push(b),
			}
		}
		Ok(decode(&s)?)
	}

	fn ptr_u16(&mut self) -> hamu::read::Result<Self> {
		Ok(self.clone().at(self.u16()? as usize)?)
	}

	fn bytestring<const N: usize>(&mut self) -> hamu::read::Result<ByteString<N>> {
		Ok(ByteString(self.array()?))
	}
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid SJIS string {0:?}")]
pub struct DecodeError(String);

pub fn decode(s: &[u8]) -> Result<String, DecodeError> {
	let (out, _, error) = SHIFT_JIS.decode(s);
	if error {
		Err(DecodeError(out.into_owned()))
	} else {
		Ok(out.into_owned())
	}
}

#[derive(Debug, thiserror::Error)]
pub enum MultiError<E: Display> {
	#[error("read error")]
	Read(#[from] hamu::read::Error),
	#[error("{}", _0.iter().map(|(i,e)| format!("{i}: {e}")).collect::<Vec<_>>().join("\n"))] // TODO
	Multiple(Vec<(usize, E)>),
}

pub fn toc<A, E, F>(i: &[u8], f: F) -> Result<Vec<A>, MultiError<E>> where
	E: Display,
	F: FnMut(&mut In, usize) -> Result<A, E>,
{
	let mut i = In::new(i);
	let start = i.clone().u16()? as usize;
	let mut pos = Vec::with_capacity(start/2);
	for _ in 0..start/2 {
		pos.push(i.u16()? as usize);
	}
	let len = i.len();
	let out = multiple(&i, &pos, len, f)?;
	i.dump_uncovered(|a| a.to_stderr())?;
	Ok(out)
}

pub fn multiple<A, E, F>(i: &In, pos: &[usize], end: usize, mut f: F) -> Result<Vec<A>, MultiError<E>> where
	E: Display,
	F: FnMut(&mut In, usize) -> Result<A, E>,
{
	let mut out = Vec::with_capacity(pos.len());
	let mut errors = Vec::new();
	for (idx, range) in ranges(pos.iter().copied(), end).enumerate() {
		match f(&mut i.clone().at(range.start)?, range.end-range.start) {
			Ok(v) => out.push(v),
			Err(e) => errors.push((idx, e)),
		}
	}

	match errors.len() {
		0 => Ok(out),
		_ => Err(MultiError::Multiple(errors)),
	}
}

pub fn ranges<A: Clone>(items: impl Iterator<Item=A>, end: A) -> impl Iterator<Item=Range<A>> {
	items.chain(std::iter::once(end)).array_windows().map(|[a,b]| a..b)
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Deref, DerefMut, From, Into)]
pub struct ByteString<const N: usize>(pub [u8; N]);

impl<const N: usize> std::fmt::Debug for ByteString<N> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "b\"{}\"",
			self.0.into_iter()
				.flat_map(std::ascii::escape_default)
				.map(|a| a as char)
				.collect::<String>()
		)
	}
}

impl<const N: usize> PartialEq<&ByteString<N>> for ByteString<N> {
	fn eq(&self, other: &&ByteString<N>) -> bool {
		self.0 == other.0
	}
}

impl<const N: usize> PartialEq<[u8;N]> for ByteString<N> {
	fn eq(&self, other: &[u8;N]) -> bool {
		&self.0 == other
	}
}

impl<const N: usize> PartialEq<&[u8]> for ByteString<N> {
	fn eq(&self, other: &&[u8]) -> bool {
		&self.0 == other
	}
}

impl<const N: usize> AsRef<ByteString<N>> for ByteString<N> {
	fn as_ref(&self) -> &ByteString<N> {
		self
	}
}

impl<const N: usize> AsRef<[u8;N]> for ByteString<N> {
	fn as_ref(&self) -> &[u8;N] {
		&self.0
	}
}

impl<const N: usize> AsRef<ByteString<N>> for [u8;N] {
	fn as_ref(&self) -> &ByteString<N> {
		// SAFETY: it's repr(transparent)
		unsafe { std::mem::transmute::<&[u8;N], &ByteString<N>>(self) }
	}
}

impl<const N: usize> ByteString<N> {
	pub fn decode(&self) -> String {
		let len = self.iter().position(|&a| a == 0).unwrap_or(N);
		let trimmed = self.split_at(len).0;
		decode(trimmed).unwrap_or_else(|a| a.0)
	}
}

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
	Item(u16 /*Item*/),
	A(u16),
	Face(u16 /*Face*/),
	K(u16),
	Pos(u16),
	Ruby(u16, String),
	Size(u16),
	Speed(u16),
}

#[derive(Debug, thiserror::Error)]
pub enum TextError {
	#[error("read error")]
	Read(#[from] hamu::read::Error),
	#[error("decode error")]
	Decode(#[from] DecodeError),
	#[error("Unknown TextSegment at {pos}: {text}")]
	Unknown {
		pos: usize,
		text: String,
	},
}

impl Text {
	pub fn read(i: &mut In) -> Result<Text, TextError> {
		let mut segments = Vec::new();
		let mut curr = Vec::new();
		fn drain(segments: &mut Vec<TextSegment>, curr: &mut Vec<u8>) -> Result<(), DecodeError> {
			if !curr.is_empty() {
				segments.push(TextSegment::String(decode(curr)?));
			}
			curr.clear();
			Ok(())
		}
		loop { match i.u8()? {
			0x00 => { drain(&mut segments, &mut curr)?; break }
			0x01 => { curr.push(b'\n') }
			0x02 => { drain(&mut segments, &mut curr)?; segments.push(TextSegment::Wait) }
			0x03 => { drain(&mut segments, &mut curr)?; segments.push(TextSegment::Page) }
			0x05 => { drain(&mut segments, &mut curr)?; segments.push(TextSegment::_05) }
			0x06 => { drain(&mut segments, &mut curr)?; segments.push(TextSegment::_06) }
			0x07 => { drain(&mut segments, &mut curr)?; segments.push(TextSegment::Color(i.u8()?)) }
			0x09 => { drain(&mut segments, &mut curr)?; segments.push(TextSegment::_09) }
			// 0x18 =>
			0x1F => { drain(&mut segments, &mut curr)?; segments.push(TextSegment::Item(i.u16()?)) }
			op@(0x00..=0x1F) => return Err(TextError::Unknown { pos: i.pos(), text: format!("b{:?}", char::from(op)) }),
			b'#' => {
				drain(&mut segments, &mut curr)?;
				let start = i.pos() - 1;
				let mut n = 0;
				segments.push(loop { match i.u8()? {
					// XXX this can panic on overflow
					ch@(b'0'..=b'9') => n = n * 10 + (ch - b'0') as u16,
					b'A' => break TextSegment::A(n),
					b'F' => break TextSegment::Face(n),
					b'K' => break TextSegment::K(n),
					b'P' => break TextSegment::Pos(n),
					b'R' => break TextSegment::Ruby(n, {
						let mut ruby = Vec::new();
						loop { match i.u8()? {
							b'#' => break,
							ch => ruby.push(ch),
						} }
						decode(&ruby)?
					}),
					b'S' => break TextSegment::Size(n),
					b'W' => break TextSegment::Speed(n),
					ch => return Err(TextError::Unknown { pos: start, text: format!("#{}{}", n, char::from(ch)) }),
				} })
			}
			ch => curr.push(ch)
		} }
		Ok(Text(segments))
	}
}
