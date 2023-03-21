use image::{GenericImage, GenericImageView, Rgba, RgbaImage};
use gospel::read::{Reader, Le as _};
use hamu::write::le::*;
use crate::ch;
use crate::util::*;

pub fn read(ch: &[u8], cp: &[u8]) -> Result<Vec<RgbaImage>, Error> {
	let mut ch = Reader::new(ch);
	let n_tiles = ch.u16()? as usize;
	let mut base = Vec::with_capacity(n_tiles);
	for _ in 0..n_tiles {
		let mut k = RgbaImage::new(16, 16);
		for k in k.pixels_mut() {
			*k = ch::from4444(ch.u16()?);
		}
		base.push(k);
	}

	let mut cp = Reader::new(cp);
	let n_frames = cp.u16()? as usize;
	let mut frames = Vec::with_capacity(n_frames);
	for _ in 0..n_frames {
		let mut frame = RgbaImage::new(256, 256);
		for y in 0..16 {
			for x in 0..16 {
				let ix = cp.u16()? as usize;
				if ix != 0xFFFF {
					ensure!(ix < n_tiles, "chcp: invalid tile id");
					frame.copy_from(&base[ix], x * 16, y * 16).unwrap()
				}
			}
		}
		frames.push(frame);
	}

	Ok(frames)
}

pub fn write<I>(frames: &[I]) -> Result<(Vec<u8>, Vec<u8>), Error> where
	I: GenericImageView<Pixel=Rgba<u8>>
{
	let mut base = Vec::<[[u16; 16]; 16]>::new();
	let mut pat = vec![[[0xFFFF; 16]; 16]; frames.len()];

	for f in frames {
		ensure!(f.dimensions() == (256, 256), "chcp: must be 256x256");
	}

	for (p, f) in pat.iter_mut().zip(frames.iter()) {
		for y in 0..16 {
			for x in 0..16 {
				let sub = f.view(x*16, y*16, 16, 16);
				let mut c = [[0; 16]; 16];
				for (c, (_, _, p)) in c.flatten_mut().iter_mut().zip(sub.pixels()) {
					*c = ch::to4444(p);
				}
				let p = &mut p[y as usize][x as usize];
				if c != [[0; 16]; 16] {
					if let Some(n) = base.iter().position(|a| *a == c) {
						*p = n as u16;
					} else {
						*p = base.len() as u16;
						base.push(c);
					}
				}
			}
		}
	}

	let mut ch = Writer::new();
	ch.u16(base.len() as u16);
	for a in base.iter().flatten().flatten() {
		ch.u16(*a);
	}

	let mut cp = Writer::new();
	cp.u16(frames.len() as u16);
	for a in pat.iter().flatten().flatten() {
		cp.u16(*a);
	}

	Ok((ch.finish()?, cp.finish()?))
}

#[test]
fn test() -> Result<(), Box<dyn std::error::Error>> {
	let mut paths = std::fs::read_dir("../data/fc.extract/07")?.collect::<Result<Vec<_>, _>>()?;
	paths.sort_by_key(|a| a.path());
	for f in paths.iter() {
		let p = f.path();
		if p.extension().map_or(true, |a| a != "_ch") {
			continue
		}
		println!("{}", p.display());
		let p2 = p.with_file_name(format!("{}p._cp", p.with_extension("").file_name().unwrap().to_str().unwrap()));
		let ch = std::fs::read(&p)?;
		let cp = std::fs::read(&p2)?;
		let frames = read(&ch, &cp)?;
		let (ch2, cp2) = write(&frames)?;
		let frames2 = read(&ch2, &cp2)?;
		assert!(frames == frames2);
	}
	Ok(())
}
