use std::collections::BTreeSet;

use gospel::read::{Reader, Le as _};
use hamu::write::le::*;
use image::Rgba;
use crate::util::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
	pub index: usize,
	pub unknown: u16,
	pub x_offset: f32,
	pub y_offset: f32,
	pub x_scale: f32,
	pub y_scale: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Itc<'a> {
	pub frames: Vec<Frame>,
	pub content: Vec<&'a [u8]>,
	pub palette: Option<Vec<Rgba<u8>>>,
}

#[allow(clippy::type_complexity)]
pub fn read(data: &[u8]) -> Result<Itc, Error> {
	let mut f = Reader::new(data);

	let has_palette = match &f.array()? {
		b"V101" => false,
		b"V102" => true,
		_ => bail!("itc: invalid header")
	};

	let mut slice    = [(0, 0); 128];
	let mut unknown  = [0; 128];
	let mut x_offset = [0.; 128];
	let mut y_offset = [0.; 128];
	let mut x_scale  = [1.; 128];
	let mut y_scale  = [1.; 128];

	for i in &mut slice {
		*i = (f.u32()? as usize, f.u32()? as usize);
	}

	for i in &mut unknown { *i = f.u16()?; }
	for i in &mut x_offset { *i = f.f32()?; }
	for i in &mut y_offset { *i = f.f32()?; }
	for i in &mut x_scale { *i = f.f32()?; }
	for i in &mut y_scale { *i = f.f32()?; }

	let palette = if has_palette {
		Some(crate::itp::read_palette(f.u32()?, &mut f)?)
	} else {
		None
	};

	let points = {
		let mut points = BTreeSet::new();
		points.insert(f.pos());
		for p in slice {
			if p != (0, 0) {
				points.insert(p.0);
				points.insert(p.0+p.1);
			}
		}
		points.insert(f.len());
		Vec::from_iter(points)
	};

	let mut content = Vec::with_capacity(points.len()-1);
	for (&start, &end) in points.iter().zip(points.iter().skip(1)) {
		content.push(f.at(start)?.slice(end-start)?)
	}

	let mut frames = Vec::with_capacity(128);
	for i in 0..128 {
		if slice[i] == (0, 0) {
			break
		}
		let index = points.binary_search(&slice[i].0).unwrap();
		frames.push(Frame {
			index,
			unknown: unknown[i],
			x_offset: x_offset[i],
			y_offset: y_offset[i],
			x_scale: x_scale[i],
			y_scale: y_scale[i],
		});
	}

	Ok(Itc {
		frames,
		content,
		palette,
	})
}

pub fn write(itc: &Itc) -> Result<Vec<u8>, Error> {
	let mut f = Writer::new();
	let mut slice = Writer::new();
	let mut unknown = Writer::new();
	let mut x_offset = Writer::new();
	let mut y_offset = Writer::new();
	let mut x_scale = Writer::new();
	let mut y_scale = Writer::new();
	let mut palette = Writer::new();

	if let Some(pal) = &itc.palette {
		f.slice(b"V102");
		palette.u32(pal.len() as u32);
		crate::itp::write_palette(pal, &mut palette);
	} else {
		f.slice(b"V101");
	}

	for fr in &itc.frames {
		slice.delay_u32(Label::known(fr.index as u32).0);
		slice.u32(itc.content[fr.index].len() as u32);
		unknown.u16(fr.unknown);
		x_offset.f32(fr.x_offset);
		y_offset.f32(fr.y_offset);
		x_scale.f32(fr.x_scale);
		y_scale.f32(fr.y_scale);
	}

	for _ in itc.frames.len()..128 {
		slice.u32(0);
		slice.u32(0);
		unknown.u16(0);
		x_offset.f32(0.);
		y_offset.f32(0.);
		x_scale.f32(1.);
		y_scale.f32(1.);
	}

	f.append(slice);
	f.append(unknown);
	f.append(x_offset);
	f.append(y_offset);
	f.append(x_scale);
	f.append(y_scale);
	f.append(palette);

	for (i, img) in itc.content.iter().enumerate() {
		f.label(Label::known(i as u32).1);
		f.slice(img);
	}

	Ok(f.finish()?)
}

#[test]
fn test() -> Result<(), Box<dyn std::error::Error>> {
	fn f(x: &[u8]) -> Result<Itc, Box<dyn std::error::Error>> {
		let img = read(x)?;
		let y = &write(&img)?;
		assert!(x == y);
		Ok(img)
	}

	f(&std::fs::read("../data/ao-evo/data/apl/ch51211.itc")?)?;
	f(&std::fs::read("../data/ao-evo/data/chr/ch40004.itc")?)?;
	f(&std::fs::read("../data/ao-evo/data/monster/ch87953.itc")?)?;
	f(&std::fs::read("../data/ao-evo/data/apl/ch50005.itc")?)?;
	f(&std::fs::read("../data/3rd-evo/data_3rd/chr/chdummy.itc")?)?;
	f(&std::fs::read("../data/3rd-evo/data_3rd/chr/ch14570.itc")?)?;
	f(&std::fs::read("../data/zero/data/chr/ch00001.itc")?)?;
	f(&std::fs::read("../data/zero/data/apl/ch50112.itc")?)?;
	Ok(())
}

