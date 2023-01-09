use std::{backtrace::Backtrace, borrow::Cow};

use hamu::read::le::*;
use ndarray::{Array2, Array, Axis};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("{source}")]
	Read { #[from] source: hamu::read::Error, backtrace: Backtrace },
	#[error("bad itp type {val:?}")]
	BadType { val: u32, backtrace: Backtrace },
}

#[derive(Debug, thiserror::Error)]
#[error("error in decompression {0}")]
pub struct DecompressError(u32);

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Itp {
	Palette(PaletteImage),
}

#[derive(Clone, PartialEq)]
pub struct PaletteImage {
	pub palette: Vec<u32>,
	pub pixels: Array2<u8>,
}

impl std::fmt::Debug for PaletteImage {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("PaletteImage")
			.field("width", &self.pixels.dim().0)
			.field("height", &self.pixels.dim().1)
			.finish_non_exhaustive()
	}
}

impl PaletteImage {
	pub fn save(&self, f: impl std::io::Write) -> Result<(), png::EncodingError> {
		let mut png = png::Encoder::new(f, self.pixels.dim().1 as u32, self.pixels.dim().0 as u32);
		let mut pal = Vec::with_capacity(3*self.palette.len());
		let mut alp = Vec::with_capacity(self.palette.len());
		for i in &self.palette {
			let [r,g,b,a] = i.to_le_bytes();
			pal.push(r);
			pal.push(g);
			pal.push(b);
			alp.push(a);
		}
		png.set_color(png::ColorType::Indexed);
		png.set_depth(png::BitDepth::Eight);
		png.set_palette(pal);
		png.set_trns(alp);
		let mut w = png.write_header()?;
		w.write_image_data(&self.pixels.iter().copied().collect::<Vec<u8>>())?;
		w.finish()?;
		Ok(())
	}
}

// TODO move this to decompress module
fn decompress(f: &mut Reader) -> Result<Vec<u8>, hamu::read::Error> {
	let csize = f.u32()? as usize;
	let start = f.pos();
	let usize = f.u32()? as usize;
	let mut out = Vec::with_capacity(usize);
	for _ in 0..f.u32()?-1 {
		let len = f.u16()? as usize;
		out.extend(decompress::decompress_chunk(f.slice(len - 2)?)?);
		f.check_u8(1)?;
	}

	// println!("{:?}", (csize, usize));
	// f.dump().oneline().to_stdout();

	f.check_u32(0x06000006)?;
	f.u8()?; // unknown
	f.u8()?;
	f.u8()?;

	assert_eq!(f.pos(), csize+start);
	assert_eq!(out.len(), usize);
	Ok(out)
}

pub fn read(data: &[u8]) -> Result<Itp, Error> {
	read0(&mut Reader::new(data))
}

pub(crate) fn read0(f: &mut Reader) -> Result<Itp, Error> {
	match f.u32()? {
		1000 => Ok(Itp::Palette(read1000(f)?)),
		1002 => Ok(Itp::Palette(read1002(f)?)),
		1004 => Ok(Itp::Palette(read1004(f)?)),
		1006 => Ok(Itp::Palette(read1006(f)?)),

		i => Err(Error::BadType { val: i, backtrace: Backtrace::capture() })
	}
}

fn read1000(f: &mut Reader) -> Result<PaletteImage, Error> {
	let w = f.u32()? as usize;
	let h = f.u32()? as usize;
	let palette = read_palette(256, f)?;
	let pixels = f.slice(w * h)?.to_owned();
	let pixels = Array::from_vec(pixels).into_shape((h, w)).unwrap();
	Ok(PaletteImage { palette, pixels })
}

fn read1002(f: &mut Reader) -> Result<PaletteImage, Error> {
	let w = f.u32()? as usize;
	let h = f.u32()? as usize;
	let palette = read_palette(256, &mut Reader::new(&decompress(f)?))?;
	let pixels = decompress(f)?;
	let pixels = Array::from_vec(pixels).into_shape((h, w)).unwrap();
	Ok(PaletteImage { palette, pixels })
}

fn read1004(f: &mut Reader) -> Result<PaletteImage, Error> {
	let w = f.u32()? as usize;
	let h = f.u32()? as usize;
	let palette = read_palette(f.u32()?, &mut Reader::new(&decompress(f)?))?;
	let data = decompress(f)?;

	let mut chunks = Array::from_vec(data).into_shape((h/8, w/16, 8, 16)).unwrap();
	chunks.swap_axes(1, 2);
	let pixels = chunks.as_standard_layout().into_owned().into_shape((h, w)).unwrap();

	Ok(PaletteImage { palette, pixels })
}

fn read1006(f: &mut Reader) -> Result<PaletteImage, Error> {
	f.dump().oneline().to_stdout();
	let size = f.u32()? as usize;
	f.check(b"CCPI")?;
	f.check_u16(7)?;
	let pal_size = f.u16()?;
	let cw = 1 << f.u8()? as usize;
	let ch = 1 << f.u8()? as usize;
	let w = f.u16()? as usize;
	let h = f.u16()? as usize;
	let flags = f.u16()?;

	let data = if flags & 0x8000 != 0 {
		let d = decompress(f)?;
		assert_eq!(d.len(), size-16);
		Cow::Owned(d)
	} else {
		Cow::Borrowed(f.slice(size-16)?)
	};
	let mut g = Reader::new(&data);

	let palette = if flags & 0x0300 == 0 {
		read_palette(pal_size as u32, &mut g)?
	} else {
		while g.u8()? != 0 { }
		Vec::new()
	};

	// print!("{w}×{h} {cw}×{ch} {wh} {size} {flags:04X} ", wh=w*h);
	// g.dump().oneline().to_stdout();

	let mut pixels = vec![0; w*h];

	let mut xs = Vec::with_capacity(256);
	let mut chunk = Vec::with_capacity(cw*ch/4);
	for cx in 0..w/cw {
		for cy in 0..h/ch {
			for _ in 0..g.u8()? {
				xs.push(g.array::<4>()?);
			}
			for i in 0..256-xs.len() {
				let [a,b,c,d] = xs[i];
				xs.push([c,d,a,b])
			}
			for i in 0..256-xs.len() {
				let [a,b,c,d] = xs[i];
				xs.push([b,a,d,c])
			}

			while chunk.len() < cw*ch/4 {
				match g.u8()? {
					0xFF => for _ in 0..g.u8()? {
						chunk.push(*chunk.last().unwrap());
					},
					v => chunk.push(v),
				}
			}

			for y in 0..cw/2 {
				for x in 0..ch/2 {
					let c = chunk[y*2+x];
					let [a,b,c,d] = xs[c as usize];
					let x = cx*cw+x;
					let y = cy*ch+y;
					let p = y*w+x;
					pixels[p] = a;
					pixels[p+1] = b;
					pixels[p+w] = c;
					pixels[p+1+w] = d;
				}
			}

			xs.clear();
			chunk.clear();
		}
	}

	todo!()

	// Ok(PaletteImage { width: w as u32, height: h as u32, palette, pixels })
}

pub(crate) fn read_palette(pal_size: u32, g: &mut Reader) -> Result<Vec<u32>, Error> {
	let mut palette = Vec::with_capacity(pal_size as usize);
	for _ in 0..pal_size {
		palette.push(g.u32()?);
	}
	Ok(palette)
}
