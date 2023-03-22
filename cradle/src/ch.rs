#![allow(clippy::unusual_byte_groupings, clippy::identity_op)]

use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _};
use image::{RgbaImage, GenericImageView, Rgba};
use crate::util::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
	Argb1555,
	Argb4444,
	Argb8888,
}

impl Mode {
	fn bytes_per(self) -> usize {
		match self {
			Mode::Argb1555 => 2,
			Mode::Argb4444 => 2,
			Mode::Argb8888 => 4,
		}
	}
}

pub(crate) fn from1555(k: u16) -> Rgba<u8> {
	let a = ((k & 0b1_00000_00000_00000) >> 15) as u8;
	let r = ((k & 0b0_11111_00000_00000) >> 10) as u8;
	let g = ((k & 0b0_00000_11111_00000) >> 5) as u8;
	let b = ((k & 0b0_00000_00000_11111) >> 0) as u8;
	Rgba([
		r << 3 | r >> 2,
		g << 3 | g >> 2,
		b << 3 | b >> 2,
		a * 0xFF,
	])
}

pub(crate) fn to1555(Rgba([r, g, b, a]): Rgba<u8>) -> u16 {
	((a as u16 >> 7) << 15) |
	((r as u16 >> 3) << 10) |
	((g as u16 >> 3) << 5) |
	((b as u16 >> 3) << 0)
}

pub(crate) fn from4444(k: u16) -> Rgba<u8> {
	let a = ((k & 0b1111_0000_0000_0000) >> 12) as u8;
	let r = ((k & 0b0000_1111_0000_0000) >> 8) as u8;
	let g = ((k & 0b0000_0000_1111_0000) >> 4) as u8;
	let b = ((k & 0b0000_0000_0000_1111) >> 0) as u8;
	Rgba([ r * 0x11, g * 0x11, b * 0x11, a * 0x11 ])
}

pub(crate) fn to4444(Rgba([r, g, b, a]): Rgba<u8>) -> u16 {
	((a as u16 >> 4) << 12) |
	((r as u16 >> 4) << 8) |
	((g as u16 >> 4) << 4) |
	((b as u16 >> 4) << 0)
}

pub(crate) fn from8888(k: u32) -> Rgba<u8> {
	let [a, r, g, b] = u32::to_be_bytes(k);
	Rgba([ r, g, b, a ])
}

pub(crate) fn to8888(Rgba([r, g, b, a]): Rgba<u8>) -> u32 {
	u32::from_be_bytes([a, r, g, b])
}

pub fn read(mode: Mode, width: usize, ch: &[u8]) -> Result<RgbaImage, Error> {
	let stride = width * mode.bytes_per();
	ensure!(stride != 0 && ch.len() % stride == 0, "invalid width");

	let mut img = RgbaImage::new(width as u32, (ch.len() / stride) as u32);
	let mut ch = Reader::new(ch);
	for p in img.pixels_mut() {
		match mode {
			Mode::Argb1555 => *p = from1555(ch.u16()?),
			Mode::Argb4444 => *p = from4444(ch.u16()?),
			Mode::Argb8888 => *p = from8888(ch.u32()?),
		}
	}
	Ok(img)
}

// This is infallible, but it should give Result for consistency
pub fn write<I>(mode: Mode, img: &I) -> Result<Vec<u8>, Error> where
	I: GenericImageView<Pixel=Rgba<u8>>
{
	let mut ch = Writer::new();
	for (_, _, p) in img.pixels() {
		match mode {
			Mode::Argb1555 => ch.u16(to1555(p)),
			Mode::Argb4444 => ch.u16(to4444(p)),
			Mode::Argb8888 => ch.u32(to8888(p)),
		}
	}
	Ok(ch.finish()?)
}


#[test]
fn test() -> Result<(), Box<dyn std::error::Error>> {
	let d = std::fs::read("../data/fc.extract/03/mt4301._ch")?;
	read(Mode::Argb4444, 1024, &d)?.save("/tmp/ch0.png")?;

	let d = std::fs::read("../data/fc.extract/04/c_vis020._ch")?;
	read(Mode::Argb1555, 768, &d)?.save("/tmp/ch1.png")?;

	let d = std::fs::read("../data/fc.extract/04/h_vis020._ch")?;
	read(Mode::Argb1555, 1536, &d)?.save("/tmp/ch2.png")?;

	let d = std::fs::read("../data/fc.extract/04/w_vis020._ch")?;
	read(Mode::Argb1555, 2048, &d)?.save("/tmp/ch3.png")?;

	let d = std::fs::read("../data/fc.extract/00/c_stchr0._ch")?;
	read(Mode::Argb8888, 512, &d)?.save("/tmp/ch4.png")?;

	let d = std::fs::read("../data/fc.extract/00/h_stchr0._ch")?;
	read(Mode::Argb8888, 1024, &d)?.save("/tmp/ch5.png")?;

	Ok(())
}

