use std::borrow::Cow;
use std::collections::HashMap;

use image::{GrayImage, Rgba, RgbaImage};

use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _};
use decompress::{decompress_ed7 as decompress, compress_ed7 as compress};
use crate::util::*;

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

	pub fn from_rgba(image: &RgbaImage, palette: Vec<Rgba<u8>>) -> Result<Self, Rgba<u8>> {
		let map = palette.iter().enumerate().map(|a| (a.1, a.0)).collect::<HashMap<_, _>>();
		let mut out = GrayImage::new(image.width(), image.height());
		for (a, b) in out.pixels_mut().zip(image.pixels()) {
			a.0[0] = *map.get(b).ok_or(*b)? as u8;
		}
		Ok(Itp {
			image: out,
			palette,
		})
	}
}

pub fn read(data: &[u8]) -> Result<Itp, Error> {
	match Reader::new(data).u32()? {
		1000 => read1000(data),
		1002 => read1002(data),
		1004 => read1004(data),
		1005 => read1005(data),
		1006 => read1006(data),
		_ => bail!("invalid itp type")
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

pub fn write1000(itp: &Itp) -> Result<Vec<u8>, Error> {
	let mut f = Writer::new();
	f.u32(1000);
	f.u32(itp.image.width());
	f.u32(itp.image.height());
	write_palette(&itp.palette, &mut f);
	f.slice(itp.image.as_raw());
	Ok(f.finish()?)
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

pub fn write1002(itp: &Itp) -> Result<Vec<u8>, Error> {
	let mut f = Writer::new();
	f.u32(1002);
	f.u32(itp.image.width());
	f.u32(itp.image.height());
	compress(&mut f, &{
		let mut g = Writer::new();
		write_palette(&itp.palette, &mut g);
		g.finish()?
	});
	compress(&mut f, itp.image.as_raw());
	Ok(f.finish()?)
}

pub fn read1004(data: &[u8]) -> Result<Itp, Error> {
	let mut f = Reader::new(data);
	f.check_u32(1004)?;
	let w = f.u32()? as usize;
	let h = f.u32()? as usize;
	let palette = read_palette(f.u32()?, &mut Reader::new(&decompress(&mut f)?))?;
	let mut pixels = decompress(&mut f)?;
	swizzle(&pixels.clone(), &mut pixels, [h/8, w/16, 8, 16], [0,2,1,3]);
	Ok(Itp { palette, image: image(w, h, pixels)? })
}

pub fn write1004(itp: &Itp) -> Result<Vec<u8>, Error> {
	let mut f = Writer::new();
	f.u32(1004);
	f.u32(itp.image.width());
	f.u32(itp.image.height());
	f.u32(itp.palette.len() as u32);
	compress(&mut f, &{
		let mut g = Writer::new();
		write_palette(&itp.palette, &mut g);
		g.finish()?
	});
	let mut pixels = itp.image.as_raw().clone();
	let (w, h) = (itp.image.width() as usize, itp.image.height() as usize);
	swizzle(&pixels.clone(), &mut pixels, [h/8, 8, w/16, 16], [0,2,1,3]);
	compress(&mut f, &pixels);
	Ok(f.finish()?)
}

pub fn read1005(data: &[u8]) -> Result<Itp, Error> {
	fn nibbles(f: &mut Reader, out: &mut [u8]) -> Result<(), Error> {
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
	ensure!(d.len() == size, "wrong decompressed size");
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

	swizzle(&pixels.clone(), &mut pixels, [h/8, w/16, 8, 16], [0,2,1,3]);
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

	ensure!(w%cw == 0 && h%ch == 0, "invalid chunk size");

	let data = if flags & 0x8000 != 0 {
		let d = decompress(&mut f)?;
		ensure!(d.len() == size - 16, "invalid ccpi size");
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
		ensure!(pixels.len() == end, "overshot when reading tile");
	}

	swizzle(&pixels.clone(), &mut pixels, [h/ch, w/cw, ch/2, cw/2, 2, 2], [0,2,4,1,3,5]);
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
	let d = read1000(&std::fs::read("../data/ao-evo/data/minigame/m02_0001.itp")?)?;
	assert!(read1000(&write1000(&d)?)? == d);
	d.to_rgba().save("/tmp/itp0.png")?;

	let d = read1002(&std::fs::read("../data/zero-gf/data/minigame/m42_0002.itp")?)?;
	assert!(read1002(&write1002(&d)?)? == d);
	d.to_rgba().save("/tmp/itp2.png")?;

	let d = read1004(&std::fs::read("../data/ao-evo/data/visual/c_vis600.itp")?)?;
	assert!(read1004(&write1004(&d)?)? == d);
	d.to_rgba().save("/tmp/itp4.png")?;

	let d = read1005(&std::fs::read("../data/zero-gf/data/minigame/m02_0002.itp")?)?;
	d.to_rgba().save("/tmp/itp5.png")?;

	let d = read1006(&std::fs::read("../data/zero-gf/data/cooking/cook04.itp")?)?;
	d.to_rgba().save("/tmp/itp6.png")?;

	Ok(())
}
