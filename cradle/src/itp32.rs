use image::RgbaImage;
use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _, Label};
use crate::util::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Itp32 {
	pub width: usize,
	pub height: usize,
	pub levels: Vec<Vec<u128>>,
}

impl Itp32 {
	pub fn to_rgba(&self, level: usize) -> RgbaImage {
		let width = self.width >> level;
		let height = self.height >> level;
		let mut pixels = Vec::with_capacity(width * height * 4);
		for p in &self.levels[level] {
			pixels.extend(bc7::decode(*p).flatten().flatten())
		}
		swizzle(&pixels.clone(), &mut pixels, [height/4, width/4, 4, 4, 4], [0, 2, 1, 3, 4]);
		image(width, height, pixels).unwrap()
	}

	#[cfg(feature = "intel_tex_2")]
	pub fn from_rgba(img: &RgbaImage) -> Itp32 {
		let a = intel_tex_2::bc7::compress_blocks(
			&intel_tex_2::bc7::alpha_very_fast_settings(),
			&intel_tex_2::RgbaSurface {
				width: img.width(),
				height: img.height(),
				stride: img.width() * 4,
				data: img,
			}
		);
		let data = a
			.chunks_exact(16)
			.map(|a| u128::from_le_bytes(a.try_into().unwrap()))
			.collect();
		Itp32 {
			width: img.width() as usize,
			height: img.height() as usize,
			levels: vec![data],
		}
	}

	pub fn levels(&self) -> usize {
		self.levels.len()
	}

	pub fn has_mipmaps(&self) -> bool {
		self.levels() > 1
	}

	#[cfg(feature = "ddsfile")]
	pub fn from_bc7_dds(dds: &ddsfile::Dds) -> Option<Itp32> {
		match dds.get_dxgi_format() {
			Some(ddsfile::DxgiFormat::BC7_Typeless)
			| Some(ddsfile::DxgiFormat::BC7_UNorm)
			| Some(ddsfile::DxgiFormat::BC7_UNorm_sRGB)
			=> {}
			_ => return None,
		}

		let width = dds.get_width() as usize;
		let height = dds.get_height() as usize;

		let mut it = dds.data
			.chunks_exact(16)
			.map(|a| u128::from_le_bytes(a.try_into().unwrap()));

		let levels = (0..dds.get_num_mipmap_levels() as u16).map(|level| {
			let level_size = (width >> level) * (height >> level);
			it.by_ref().take(level_size >> 4).collect()
		}).collect();
		Some(Itp32 { width, height, levels })
	}

	#[cfg(feature = "ddsfile")]
	pub fn to_bc7_dds(self) -> ddsfile::Dds {
		let mut dds = ddsfile::Dds::new_dxgi(ddsfile::NewDxgiParams {
			height: self.width as u32,
			width: self.height as u32,
			depth: None,
			format: ddsfile::DxgiFormat::BC7_UNorm,
			mipmap_levels: self.has_mipmaps().then_some(self.levels() as u32),
			array_layers: None,
			caps2: None,
			is_cubemap: false,
			resource_dimension: ddsfile::D3D10ResourceDimension::Texture2D,
			alpha_mode: ddsfile::AlphaMode::Unknown,
		}).unwrap();
		dds.data = self.levels.iter().flatten().copied().flat_map(u128::to_le_bytes).collect();
		dds
	}
}

pub fn read(data: &[u8]) -> Result<Itp32, Error> {
	let mut f = Reader::new(data);
	f.check(b"ITP\xFF")?;
	let file_len = f.data().len();

	let mut width = 0;
	let mut height = 0;
	let mut n_mips = 0;
	let mut minor = 0;
	let mut swizzle = 0; // see https://imgur.com/a/E5YnYXN
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
				f.check_u32(file_len as u32)?;

				let major = f.u16()?;
				minor = f.u16()?;
				swizzle = f.u16()?;
				f.check_u16(6)?; // rest are unknown
				f.check_u16(3)?;
				f.check_u16(0)?;
				f.check_u16(0)?;

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

				let w = width >> n;
				let h = height >> n;
				let capacity = w * h;
				let mut data = Vec::with_capacity(capacity);
				match minor {
					5 => {
						while !f.is_empty() {
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

				let mut data: Vec<u128> = data
					.chunks_exact(16)
					.map(|a| u128::from_le_bytes(a.try_into().unwrap()))
					.collect();

				match swizzle {
					0 | 5.. => {},
					4 => crate::util::swizzle(&data.clone(), &mut data, [h/32, w/32, 2, 2, 2, 2, 2, 2], [0,2,4,6,1,3,5,7]),
					_ => bail!("unsupported swizzle mode {swizzle}")
				};

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

pub fn write(itp: &Itp32) -> Result<Vec<u8>, Error> {
	let mut f = Writer::new();
	let len = Label::new();
	f.slice(b"ITP\xFF");

	f.slice(b"IHDR");
	f.u32(32);
	f.u32(32);
	f.u32(itp.width as u32);
	f.u32(itp.height as u32);
	f.delay32(len);
	f.u16(3); // major
	f.u16(10); // minor
	f.u16(0); // swizzle
	f.u16(6);
	f.u16(3);
	f.u16(0);
	f.u16(0);
	f.u16(0);

	f.slice(b"IMIP");
	f.u32(12);
	f.u32(12);
	f.u16(u16::from(itp.has_mipmaps()));
	f.u16((itp.levels() - 1) as u16);
	f.u32(0);

	// IHAS - hash. ignored intentionally
	// IALP - has-alpha flag. Not sure if it has any effect.

	for (n, l) in itp.levels.iter().enumerate() {
		let l = l.iter().copied().flat_map(u128::to_le_bytes).collect::<Vec<_>>();
		const CHUNK_SIZE: usize = 0x40000;

		let mut g = Writer::new();
		let mut max_chunk = 0;
		for uchunk in l.chunks(CHUNK_SIZE) {
			let p = g.len();
			compress(&mut g, uchunk);
			max_chunk = max_chunk.max(g.len() - p);
		}

		f.slice(b"IDAT");
		f.u32(28 + g.len() as u32);
		f.u32(8);
		f.u16(0);
		f.u16(n as u16);

		f.u32(0x80000001);
		f.u32(l.chunks(CHUNK_SIZE).count() as u32);
		f.u32(g.len() as u32);
		f.u32(max_chunk as u32);
		f.u32(l.len() as u32);
		f.append(g);
	}

	f.slice(b"IEND");
	f.u32(0);

	f.label(len);
	Ok(f.finish()?)
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
		while !f.is_empty() {
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

fn compress(f: &mut Writer, data: &[u8]) {
	f.u32(4 + data.len() as u32);
	f.u32(data.len() as u32);
	f.u32(0);
	f.slice(data);
}

#[test]
fn test() -> Result<(), Box<dyn std::error::Error>> {
	// let d = read(&std::fs::read("../data/zero/data/visual/title.itp")?)?;
	let d = read(&std::fs::read("../data/zero/data/visual/bu00000.itp")?)?;
	assert!(read(&write(&d)?)? == d); // can't really guarantee this one since bc7 is lossy :(
	d.to_rgba(0).save("/tmp/itp32_0.png")?;

	Ok(())
}
