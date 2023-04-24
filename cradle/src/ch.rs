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

macro_rules! guess {
	($($prefix:literal, $mode:ident, $w:literal, $h:literal;)*) => {
		pub fn guess_from_byte_size(name: &str, bytes: usize) -> Option<(Mode, usize, usize)> {
			$(if name.starts_with($prefix) && bytes == $w * $h * Mode::$mode.bytes_per() {
				return Some((Mode::$mode, $w, $h))
			})*
			None
		}
		pub fn guess_from_image_size(name: &str, w: usize, h: usize) -> Option<Mode> {
			$(if name.starts_with($prefix) && w == $w && h == $h {
				return Some(Mode::$mode)
			})*
			None
		}
	}
}

guess! {
	"c_ka",     Argb1555,  128,  128; // dialogue face
	"h_ka",     Argb1555,  256,  256;
	"c_stch",   Argb8888,  512,  512; // menu portrait
	"h_stch",   Argb8888, 1024, 1024;
	"cti",      Argb1555,  256,  256; // s-craft cut-in
	"bface",    Argb1555,  256,  256; // battle face
	"hface",    Argb1555,  512,  512;
	"m",        Argb4444, 1024, 1024; // minimap
	"ca",       Argb1555,  128,  128; // bestiary image
	"ca",       Argb1555,  256,  256;
	"c_note",   Argb1555,  768,  512; // notebook
	"h_note",   Argb1555, 1536, 1024;
	"c_epi",    Argb4444,  208,  176; // door thumbnails
	"h_epi",    Argb4444,  416,  352;
	"c_orb",    Argb1555,  512,  512; // character orbment
	"c_subti",  Argb8888,  256,  256; // misc
	"h_subti",  Argb8888,  512,  512;
	"c_mnbg01", Argb4444,  128,  128; // impossible to tell if 4444 or 1555
	"c_tuto20", Argb4444,  768,  512;
	"c_map00",  Argb1555, 1024,  512;
	"c_map00",  Argb1555,  768,  512;
	"c_map01",  Argb4444, 1024,  512;
	"h_map01",  Argb4444, 2048, 1024;

	"c_camp01", Argb4444,  256,  256; // menu textures
	"h_camp01", Argb4444,  512,  512;
	"c_camp02", Argb1555,  256,  256;
	"h_camp02", Argb1555,  512,  512;
	"c_camp03", Argb1555,  256,  256;
	"h_camp03", Argb1555,  512,  512;
	"c_camp04", Argb4444,  256,  256;
	"h_camp04", Argb4444,  512,  512;
	"c_camp05", Argb4444,  256,  256;
	"h_camp05", Argb4444,  512,  512;

	"c_back",   Argb1555,  768,  512; // main menu bg (02 is for orbment menu)
	"w_back",   Argb1555, 1024,  512;
	"c_title0", Argb1555, 1024,  512;
	"c_title1", Argb4444,  512,  512;
	"h_title1", Argb4444, 1024, 1024;
	"c_title2", Argb4444, 1024,  512;
	"c_title3", Argb4444,  512,  512;
	"c_title4", Argb4444, 1024,  512;
	"c_title5", Argb1555, 1024,  512;
	"c_title6", Argb1555, 1024,  512;

	"c_vis419", Argb4444,  768,  512;
	"c_vis438", Argb4444,  768,  512;
	"c_vis439", Argb4444,  768,  512;
	"h_vis419", Argb4444, 1536, 1024;
	"h_vis438", Argb4444, 1536, 1024;
	"h_vis439", Argb4444, 1536, 1024;
	"c_vis448", Argb4444,  768,  512;
	"c_vis478", Argb4444,  768,  512;
	"c_vis53",  Argb4444,  768,  512;
	"c_vis54",  Argb4444,  768,  512;

	"c_vis",    Argb1555,  768,  512;
	"c_vis",    Argb1555,  256,  256;
	"c_vis",    Argb1555,  512,  256;
	"c_vis",    Argb1555,  640,  304;
	"c_vis",    Argb1555,  128,   64;
	"c_vis",    Argb1555, 1024, 1024;
	"h_vis",    Argb1555, 1536, 1024;
	"w_vis",    Argb1555, 2048, 1024;

	"",         Argb4444,  256,  256;
	"",         Argb4444,  512,  512;
	"",         Argb4444,  768,  512;
	"",         Argb4444, 1024, 1024;
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
