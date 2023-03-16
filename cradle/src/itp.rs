use std::borrow::Cow;

use hamu::read::le::*;
use hamu::write::le::*;
use image::{GrayImage, Rgba, RgbaImage};
use crate::util::{Error, swizzle, decompress};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Itp {
	pub palette: Vec<Rgba<u8>>,
	pub image: GrayImage,
}

impl Itp {
	pub fn to_rgba(&self) -> RgbaImage {
		let mut image = RgbaImage::new(self.image.width(), self.image.height());
		for (p1, p2) in image.pixels_mut().zip(self.image.pixels()) {
			*p1 = self.palette[p2.0[0] as usize];
		}
		image
	}
}

fn image(w: usize, h: usize, pixels: Vec<u8>) -> Result<GrayImage, Error> {
	GrayImage::from_vec(w as u32, h as u32, pixels).ok_or(Error::Invalid("wrong number of pixels".to_owned()))
}

pub fn read(data: &[u8]) -> Result<Itp, Error> {
	match Reader::new(data).u32()? {
		1000 => read1000(data),
		1002 => read1002(data),
		1004 => read1004(data),
		1005 => read1005(data),
		1006 => read1006(data),
		_ => Err(Error::Invalid("invalid itp type".to_owned()))
	}
}

pub fn read1000(data: &[u8]) -> Result<Itp, Error> {
	let mut f = Reader::new(data);
	f.check_u32(1000)?;
	let w = f.u32()? as usize;
	let h = f.u32()? as usize;
	let palette = read_palette(256, &mut f)?;
	let pixels = f.slice(w * h)?.to_owned();
	Ok(Itp { palette, image: image(w, h, pixels)? })
}

pub fn read1002(data: &[u8]) -> Result<Itp, Error> {
	let mut f = Reader::new(data);
	f.check_u32(1002)?;
	let w = f.u32()? as usize;
	let h = f.u32()? as usize;
	let palette = read_palette(256, &mut Reader::new(&decompress(&mut f)?))?;
	let pixels = decompress(&mut f)?;
	Ok(Itp { palette, image: image(w, h, pixels)? })
}

pub fn read1004(data: &[u8]) -> Result<Itp, Error> {
	let mut f = Reader::new(data);
	f.check_u32(1004)?;
	let w = f.u32()? as usize;
	let h = f.u32()? as usize;
	let palette = read_palette(f.u32()?, &mut Reader::new(&decompress(&mut f)?))?;
	let mut pixels = decompress(&mut f)?;

	let mut c = pixels.clone();
	swizzle(&mut pixels, &mut c, w, 16, 8);
	Ok(Itp { palette, image: image(w, h, pixels)? })
}

pub fn read1005(data: &[u8]) -> Result<Itp, Error> {
	fn nibbles(f: &mut Reader, out: &mut [u8]) -> Result<(), hamu::read::Error> {
		for i in 0..out.len()/2 {
			let x = f.u8()?;
			out[2*i] = x >> 4;
			out[2*i+1] = x & 15;
		}
		Ok(())
	}

	let mut f = Reader::new(data);
	f.check_u32(1005)?;

	let w = f.u32()? as usize;
	let h = f.u32()? as usize;

	let palette = {
		let pal_size = f.u32()?;
		let g = decompress(&mut f)?;
		let mut g = Reader::new(&g);
		let mut palette = Vec::with_capacity(pal_size as usize);
		let mut val = 0u32;
		for _ in 0..pal_size {
			val = val.wrapping_add(g.u32()?);
			palette.push(Rgba(u32::to_le_bytes(val)));
		}
		palette
	};

	let size = f.u32()? as usize;
	let d = decompress(&mut f)?;
	if d.len() != size {
		return Err(Error::Invalid("wrong decompressed size".to_owned()))
	}
	let mut g = Reader::new(&d);

	let mut ncolors = vec![0; (h/8)*(w/16)];
	nibbles(&mut g, &mut ncolors)?;
	for a in &mut ncolors {
		if *a != 0 {
			*a += 1;
		}
	}

	let totalcolors = 1 + ncolors.iter().map(|a| *a as usize).sum::<usize>();
	let mut c = Reader::new(g.slice(totalcolors)?);

	let mut pixels = Vec::with_capacity(h*w);
	for ncolors in ncolors {
		let mut chunk = [0; 8*16];
		if ncolors != 0 {
			let colors = c.slice(ncolors as usize)?;
			nibbles(&mut g, &mut chunk)?;
			chunk = chunk.map(|a| colors[a as usize]);
		}
		pixels.extend(chunk);
	}

	let mut c = pixels.clone();
	swizzle(&mut pixels, &mut c, w, 16, 8);
	Ok(Itp { palette, image: image(w, h, pixels)? })
}

pub fn read1006(data: &[u8]) -> Result<Itp, Error> {
	let mut f = Reader::new(data);
	f.check_u32(1006)?;
	let size = f.u32()? as usize;
	f.check(b"CCPI")?;
	// usually 7, but 6 in a few of 3rd Evo's.
	// Those files all seem to be single-colored squares, so they're not very interesting.
	let _unk1 = f.u16()?;
	let pal_size = f.u16()?;
	let cw = 1 << f.u8()? as usize;
	let ch = 1 << f.u8()? as usize;
	let w = f.u16()? as usize;
	let h = f.u16()? as usize;
	let flags = f.u16()?;

	if w%cw != 0 || h%ch != 0 {
		return Err(Error::Invalid("invalid chunk size".to_owned()))
	}

	let data = if flags & 0x8000 != 0 {
		let d = decompress(&mut f)?;
		if d.len() != size-16 {
			return Err(Error::Invalid("invalid ccpi size".to_owned()))
		}
		Cow::Owned(d)
	} else {
		Cow::Borrowed(f.slice(size-16)?)
	};
	let mut g = Reader::new(&data);

	// These two bits seem to always be used together
	let palette = if flags & 0x0300 == 0 {
		read_palette(pal_size as u32, &mut g)?
	} else {
		while g.u8()? != 0 { } // String containing the containing itc's filename
		Vec::new()
	};

	let mut pixels = Vec::with_capacity(w*h);
	let mut tiles = Vec::with_capacity(256);
	for _ in 0..(h/ch)*(w/cw) {
		tiles.clear();
		let n = g.u8()? as usize;
		for _ in 0..n {
			tiles.push(g.array::<4>()?);
		}
		for i in n..(n*2).min(256) {
			let [a,b,c,d] = tiles[i-n];
			tiles.push([b,a,d,c]); // x-flip
		}
		for i in n*2..(n*4).min(256) {
			let [a,b,c,d] = tiles[i-n*2];
			tiles.push([c,d,a,b]); // y-flip
		}

		let end = pixels.len() + ch*cw;
		let mut last = 0;
		while pixels.len() < end {
			match g.u8()? {
				0xFF => for _ in 0..g.u8()? {
					pixels.extend(tiles[last]);
				},
				v => {
					last = v as usize;
					pixels.extend(tiles[last])
				}
			}
		}
		if pixels.len() > end {
			return Err(Error::Invalid("overshot when reading tile".to_owned()))
		}
	}

	// I do not understand why this swizzle sequence works. But it does.
	let mut c = pixels.clone();
	swizzle(&mut c, &mut pixels, cw, 2, 2);
	swizzle(&mut pixels, &mut c, w, cw, ch);
	Ok(Itp { palette, image: image(w, h, pixels)? })
}

pub(crate) fn read_palette(pal_size: u32, g: &mut Reader) -> Result<Vec<Rgba<u8>>, Error> {
	let mut palette = Vec::with_capacity(pal_size as usize);
	for _ in 0..pal_size {
		palette.push(Rgba(g.array()?));
	}
	Ok(palette)
}

pub(crate) fn write_palette(pal: &[Rgba<u8>], g: &mut Writer) {
	for p in pal {
		g.array(p.0);
	}
}

#[test]
fn test() -> Result<(), Box<dyn std::error::Error>> {
	let d = std::fs::read("../data/ao-evo/data/minigame/m02_0001.itp")?; // 1000
	read(&d)?.to_rgba().save("/tmp/itp0.png")?;

	let d = std::fs::read("../data/zero-gf/data/minigame/m42_0002.itp")?; // 1002
	read(&d)?.to_rgba().save("/tmp/itp2.png")?;

	let d = std::fs::read("../data/ao-evo/data/visual/c_vis600.itp")?; // 1004
	read(&d)?.to_rgba().save("/tmp/itp4.png")?;

	let d = std::fs::read("../data/zero-gf/data/minigame/m02_0002.itp")?; // 1005
	read(&d)?.to_rgba().save("/tmp/itp5.png")?;

	let d = std::fs::read("../data/zero-gf/data/cooking/cook04.itp")?; // 1006
	read(&d)?.to_rgba().save("/tmp/itp6.png")?;

	Ok(())
}
