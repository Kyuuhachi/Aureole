use image::GenericImage;
use image::GenericImageView;
use image::ImageBuffer;
use image::Rgba;
use image::RgbaImage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::ch;
use crate::error::Error;

pub fn read(ch: &[u8], cp: &[u8]) -> Result<Vec<RgbaImage>, Error> {
	let mut ch = Reader::new(ch);
	let n_chunks = ch.u16()? as usize;
	let mut base = Vec::with_capacity(n_chunks);
	for _ in 0..n_chunks {
		let mut k = RgbaImage::new(16, 16);
		for k in k.pixels_mut() {
			*k = ch::from4444(ch.u16()?);
		}
		base.push(k);
	}
	if ch.remaining() > 0 {
		return Err(Error::Size)
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
					if ix >= n_chunks {
						return Err(Error::Invalid)
					}
					frame.copy_from(&base[ix], x * 16, y * 16).unwrap()
				}
			}
		}
		frames.push(frame);
	}
	if cp.remaining() > 0 {
		return Err(Error::Size)
	}

	Ok(frames)
}

pub fn write<I>(frames: &[I]) -> Result<(Vec<u8>, Vec<u8>), Error> where
	I: GenericImageView<Pixel=Rgba<u8>>
{
	let mut base = Vec::<[[u16; 16]; 16]>::new();
	let mut pat = vec![[[0xFFFF; 16]; 16]; frames.len()];

	for f in frames {
		if f.dimensions() != (256, 256) {
			return Err(Error::Size);
		}
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

pub fn tile<I>(
	frames: &[I],
	width: u32,
) -> ImageBuffer<I::Pixel, Vec<<I::Pixel as image::Pixel>::Subpixel>> where
	I: GenericImageView
{
	assert_ne!(width, 0);
	if frames.is_empty() {
		return ImageBuffer::new(0, 0)
	}
	let height = (frames.len() as u32 + width-1) / width;
	let (w, h) = frames[0].dimensions();
	let mut img = ImageBuffer::new(width * w, height * h);
	for (i, f) in frames.iter().enumerate() {
		assert_eq!(f.dimensions(), (w, h));
		img.copy_from(f, i as u32 % width * w, i as u32 / width * h).unwrap();
	}
	img
}

#[test]
fn test() -> Result<(), Box<dyn std::error::Error>> {
	let o = std::path::Path::new("/tmp/chcp");
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
		// tile(&frames, 8).save(o.join(p.with_extension("png").file_name().unwrap()))?;
		let (ch2, cp2) = write(&frames)?;
		// let frames2 = read(&ch2, &cp2)?;
		// assert!(frames == frames2);
	}
	Ok(())
}


//
// #[test]
// fn test2() -> Result<(), Box<dyn std::error::Error>> {
// 	let cp = std::fs::read("../data/fc.extract/07/ch00001p._cp")?;
// 	let mut f = Reader::new(&cp);
// 	f.check_u16(64)?;
// 	let mut a = image::RgbaImage::new(16*8, 16*8);
// 	for i in 0..8 {
// 		for j in 0..8 {
// 			let mut su = image::RgbaImage::new(16, 16);
// 			for p in su.pixels_mut() {
// 				let v = f.u16()?;
// 				if v == 0x30 {
// 					*p = Rgba([0xFF,0,0,255]);
// 				} else {
// 					*p = ch::from4444(0xF000 | v);
// 				}
// 			}
// 			a.copy_from(&su, j*16, i*16)?;
// 		}
// 	}
// 	image::imageops::resize(&a, 256*8, 256*8, image::imageops::Nearest).save("/tmp/a.png")?;
// 	Ok(())
// }
//
// #[test]
// fn test3() -> Result<(), Box<dyn std::error::Error>> {
// 	let ch = std::fs::read("../data/fc.extract/07/ch00001._ch")?;
// 	let cp = std::fs::read("../data/fc.extract/07/ch00001p._cp")?;
// 	let frames = read(&ch, &cp)?;
// 	let mut a = image::RgbaImage::new(256*8, 256*8);
// 	for i in 0..8 {
// 		for j in 0..8 {
// 			a.copy_from(&frames[i*8+j], j as u32*256, i as u32*256)?;
// 		}
// 	}
//
// 	let mut x = Vec::<([Rgba<u8>;256], usize)>::new();
// 	for i in 0..128 {
// 		for j in 0..128 {
// 			let mut c = [Rgba([0,0,0,0]);256];
// 			c.iter_mut().zip(a.view(j as u32*16, i as u32*16, 16, 16).pixels())
// 				.for_each(|a| *a.0 = a.1.2);
// 			if c != [Rgba([0;4]);256] {
// 				if let Some(i) = x.iter_mut().find(|a| a.0 == c) {
// 					i.1 += 1;
// 				} else {
// 					x.push((c, 1));
// 				}
// 			}
// 		}
// 	}
// 	x.retain(|a| a.1 > 1);
//
// 	let mut b = image::RgbaImage::new(128, 128);
// 	for i in 0..128 {
// 		for j in 0..128 {
// 			let mut c = [Rgba([0,0,0,0]);256];
// 			c.iter_mut().zip(a.view(j as u32*16, i as u32*16, 16, 16).pixels())
// 				.for_each(|a| *a.0 = a.1.2);
// 			if c != [Rgba([0;4]);256] {
// 				if let Some(t) = x.iter_mut().position(|a| a.0 == c) {
// 					println!("{t}");
// 					let _r = ((t as u8 >> 4) & 3) * 0x55;
// 					let _g = ((t as u8 >> 2) & 3) * 0x55;
// 					let _b = ((t as u8 >> 0) & 3) * 0x55;
// 					b.put_pixel(j as u32, i as u32, Rgba([_r, _g, _b,0x7F]));
// 	 			}
// 			} else {
// 				b.put_pixel(j as u32, i as u32, Rgba([63,63,63,0x3F]));
// 			}
// 		}
// 	}
//
// 	let mut b = image::imageops::resize(&b, 256*8, 256*8, image::imageops::Nearest);
// 	image::imageops::overlay(&mut a, &b, 0, 0);
//
// 	a.save("/tmp/b.png")?;
// 	Ok(())
// }
