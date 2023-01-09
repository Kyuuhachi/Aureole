use std::{backtrace::Backtrace, borrow::Cow};

use hamu::read::le::*;
use ndarray::prelude::*;

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
		1005 => Ok(Itp::Palette(read1005(f)?)),
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

	let pixels = Array::from_vec(data)
		.into_shape((h/8, w/16, 8, 16)).unwrap()
		.permuted_axes((0,2,1,3))
		.as_standard_layout().into_owned()
		.into_shape((h, w)).unwrap();

	Ok(PaletteImage { palette, pixels })
}

fn read1005(f: &mut Reader) -> Result<PaletteImage, Error> {
	fn nibbles(f: &mut Reader, n: usize) -> Result<Array1<u8>, hamu::read::Error> {
		let mut b = Array::zeros((n/2, 2));
		for mut b in b.outer_iter_mut() {
			let x = f.u8()?;
			b[0] = x >> 4;
			b[1] = x & 15;
		}
		Ok(b.into_shape(n).unwrap())
	}

	let w = f.u32()? as usize;
	let h = f.u32()? as usize;
	let mut palette = read_palette(f.u32()?, &mut Reader::new(&decompress(f)?))?;
	for i in 1..palette.len() {
		palette[i] = palette[i].wrapping_add(palette[i-1]);
	}

	let size = f.u32()? as usize;
	let d = decompress(f)?;
	assert_eq!(d.len(), size);
	let mut g = Reader::new(&d);

	let mut ncolors = nibbles(&mut g, (h/8)*(w/16))?;
	ncolors.map_inplace(|a| if *a != 0 { *a += 1 });

	let totalcolors = ncolors.fold(1, |a, b| a + *b as usize);
	let mut c = Reader::new(g.slice(totalcolors)?);

	let mut chunks = Array::zeros(((h/8)*(w/16), 8*16));
	for (mut chunk, ncolors) in chunks.outer_iter_mut().zip(ncolors) {
		if ncolors != 0 {
			let colors = c.slice(ncolors as usize)?;
			let idx = nibbles(&mut g, 8*16)?;
			chunk.zip_mut_with(&idx, |a, b| *a = colors[*b as usize]);
		}
	}

	let pixels = chunks
		.into_shape((h/8, w/16, 8, 16)).unwrap()
		.permuted_axes((0,2,1,3))
		.as_standard_layout().into_owned()
		.into_shape((h, w)).unwrap();

	Ok(PaletteImage { palette, pixels })
}

fn read1006(f: &mut Reader) -> Result<PaletteImage, Error> {
	let size = f.u32()? as usize;
	f.check(b"CCPI")?;
	f.check_u16(7)?;
	let pal_size = f.u16()?;
	let cw = 1 << f.u8()? as usize;
	let ch = 1 << f.u8()? as usize;
	let w = f.u16()? as usize;
	let h = f.u16()? as usize;
	let flags = f.u16()?;

	assert_eq!(w%cw, 0);
	assert_eq!(h%ch, 0);

	let data = if flags & 0x8000 != 0 {
		let d = decompress(f)?;
		assert_eq!(d.len(), size-16);
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

	let mut chunks = Array::zeros(((h/ch)*(w/cw), (ch/2)*(cw/2), 2*2));

	let mut tileset = Vec::with_capacity(256);
	let mut tiles = Vec::with_capacity(chunks.dim().1);
	for mut chunk in chunks.outer_iter_mut() {
		tileset.clear();
		for _ in 0..g.u8()? {
			tileset.push(g.array::<4>()?);
		}
		for i in 0..256-tileset.len() {
			let [a,b,c,d] = tileset[i];
			tileset.push([b,a,d,c])
		}
		for i in 0..256-tileset.len() {
			let [a,b,c,d] = tileset[i];
			tileset.push([c,d,a,b])
		}

		tiles.clear();
		while tiles.len() < chunk.dim().0 {
			match g.u8()? {
				0xFF => for _ in 0..g.u8()? {
					tiles.push(*tiles.last().unwrap());
				},
				v => tiles.push(v),
			}
		}
		assert_eq!(tiles.len(), chunk.dim().0);

		for (mut c, t) in chunk.outer_iter_mut().zip(&tiles) {
			c.assign(&ArrayView::from(&tileset[*t as usize]));
		}
	}

	let pixels = chunks
		.into_shape((h/ch, w/cw, ch/2, cw/2, 2, 2)).unwrap()
		.permuted_axes((0,2,4,1,3,5))
		.as_standard_layout().into_owned()
		.into_shape((h, w)).unwrap();

	Ok(PaletteImage { palette, pixels })
}

pub(crate) fn read_palette(pal_size: u32, g: &mut Reader) -> Result<Vec<u32>, Error> {
	let mut palette = Vec::with_capacity(pal_size as usize);
	for _ in 0..pal_size {
		palette.push(g.u32()?);
	}
	Ok(palette)
}
