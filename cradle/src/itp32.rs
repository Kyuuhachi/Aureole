use hamu::read::le::*;
use hamu::write::le::*;

use image::{Rgba, RgbaImage};

use crate::util::*;

pub struct Itp32 {
	pub width: usize,
	pub height: usize,
	pub levels: Vec<Vec<u128>>,
}

impl Itp32 {
	pub fn to_rgba(&self, level: usize) -> RgbaImage {
		let width = self.width >> level;
		let height = self.height >> level;
		let mut pixels = Vec::with_capacity(width * height);
		for p in &self.levels[level] {
			pixels.extend(bc7::decode(*p).flatten().flatten())
		}
		let mut c = pixels.clone();
		swizzle(&mut pixels, &mut c, width*4, 4*4, 4);
		image(width, height, pixels).unwrap()
	}

	pub fn levels(&self) -> usize {
		self.levels.len()
	}

	pub fn has_mipmaps(&self) -> bool {
		self.levels() > 1
	}
}

pub fn read(data: &[u8]) -> Result<Itp32, Error> {
	let mut f = Reader::new(data);
	f.check(b"ITP\xFF")?;

	let mut width = 0;
	let mut height = 0;
	let mut n_mips = 0;
	let mut minor = 0;
	let mut levels = Vec::new();

	loop {
		let fourcc = f.array::<4>()?;
		let size = f.u32()?;
		let mut f = Reader::new(f.slice(size as usize)?);
		match &fourcc {
			b"IHDR" => {
				f.check_u32(32)?; // chunk size
				width = f.u32()? as usize;
				height = f.u32()? as usize;
				let file_size = f.u32()? as usize;
				let major = f.u16()?;
				minor = f.u16()?;

				f.check_u16(0)?; // swizzle; see https://imgur.com/a/E5YnYXN
				f.check_u16(6)?; // rest are unknown
				f.check_u16(3)?;
				f.check_u16(0)?;
				f.check_u16(0)?;

				ensure!(file_size == data.len(), "itp32: invalid file size");
				ensure!(major == 3, "itp32: invalid major {major}, only 3 supported");
			}

			b"IMIP" => {
				f.check_u32(12)?; // chunk size
				let has_mip = f.u16()? != 0;
				n_mips = f.u16()? as usize;
				f.check_u32(0)?;
				ensure!(has_mip == (n_mips > 0), "itp32: invalid mipmap spec");
				levels.reserve(n_mips + 1);
			}

			b"IHAS" => {
				// ignored
			}

			b"IALP" => {
				f.check_u32(8)?; // chunk size
				let _has_alpha = f.u32()? != 0;
			}

			b"IDAT" => {
				let n = levels.len();
				f.check_u32(8)?; // chunk size (except actual data)
				f.check_u16(0)?;
				f.check_u16(n as u16)?;

				let capacity = (width >> n) * (height >> n);
				let mut data = Vec::with_capacity(capacity);
				match minor {
					5 => {
						while f.remaining() > 0 {
							decompress(&mut f, &mut data)?;
						}
					}

					10 => {
						f.check_u32(0x80000001)?;
						let n_chunks = f.u32()? as usize;
						let total_csize = f.u32()? as usize;
						let largest_csize = f.u32()? as usize;
						let total_usize = f.u32()? as usize;
						ensure!(f.pos() + total_csize == f.len(), "itp32: invalid compressed size");
						ensure!(total_usize == capacity, "itp32: invalid total uncompressed size");

						let mut max_csize = 0;
						for _ in 0..n_chunks {
							let start = f.pos();
							decompress(&mut f, &mut data)?;
							max_csize = max_csize.max(f.pos() - start);
						}
						ensure!(max_csize == largest_csize, "itp32: incorrect largest_csize");
					}

					_ => bail!("itp32: invalid minor {minor}, only 5 or 10 supported")
				}
				ensure!(data.len() == capacity, "itp32: not enough data");
				let data = data.chunks(16)
					.map(|a| a.try_into().unwrap())
					.map(u128::from_le_bytes)
					.collect();
				levels.push(data);
			}

			b"IEND" => {
				break
			}

			_ => bail!("itp32: invalid chunk {:?}", String::from_utf8_lossy(&fourcc))
		}
	}

	ensure!(levels.len() == n_mips + 1, "itp32: expected {n_mips} levels, got {}", levels.len());

	Ok(Itp32 {
		width,
		height,
		levels,
	})
}

fn decompress(f: &mut Reader, out: &mut Vec<u8>) -> Result<(), Error> {
	let csize = f.u32()? as usize;
	let usize = f.u32()? as usize;
	let data = f.slice(csize)?;

	let mut f = Reader::new(data);
	let start = out.len();
	let mode = f.u32()?;
	if mode == 0 {
		out.extend_from_slice(&data[4..]);
	} else {
		while f.remaining() > 0 {
			let x = f.u16()? as usize;
			let op = x & !(!0 << mode);
			let num = x >> mode;
			if op == 0 {
				out.extend(f.slice(num)?);
			} else {
				ensure!(num <= out.len() - start, "itp32: repeat offset too large ({num} > {})", out.len() - start);
				for _ in 0..op {
					out.push(out[out.len() - num - 1])
				}
				out.push(f.u8()?);
			}
		}
	}

	let written = out.len() - start;
	ensure!(written == usize, "itp32: invalid decompressed size");
	Ok(())
}

#[test]
fn test() -> Result<(), Box<dyn std::error::Error>> {
	// let d = read(&std::fs::read("../data/zero/data/visual/title.itp")?)?;
	let d = read(&std::fs::read("../data/zero/data/visual/bu00000.itp")?)?;
	// assert!(read(&write1000(&d)?)? == d);
	d.to_rgba(0).save("/tmp/itp32_0.png")?;

	Ok(())
}
