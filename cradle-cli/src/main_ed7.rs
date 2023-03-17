#![feature(decl_macro, let_chains)]

use std::{path::PathBuf, io::Cursor};

use clap::{Parser, ValueHint};
use cradle::{itp::Itp, itp32::Itp32};
use ddsfile::DxgiFormat as Dxgi;
use eyre::Result;
use image::{RgbaImage, ImageFormat as IF, Rgba};

#[derive(Debug, Clone, Parser)]
struct Cli {
	/// Where to place the output.
	///
	/// If unspecified, output will be placed next to the input file.
	///
	/// For itc files, the names of the individual frames can currently not be controlled.
	#[clap(long, short, value_hint = ValueHint::FilePath)]
	output: Option<PathBuf>,

	/// The file to process. Should be a .itp, .itc, .png, .dds, or .csv.
	#[clap(required = true, value_hint = ValueHint::FilePath)]
	file: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
#[derive(serde::Serialize, serde::Deserialize)]
struct ItcFrame {
	filename: PathBuf,
	unknown: u16,
	x_offset: f32,
	y_offset: f32,
	x_scale: f32,
	y_scale: f32,
}

fn main() -> Result<()> {
	let cli = Cli::parse();
	let data = std::fs::read(&cli.file)?;

	macro w($ext:literal, $e:expr) {
		{
			let out = cli.output.clone()
				.unwrap_or_else(|| cli.file.with_extension($ext));
			std::fs::write(out, $e)?
		}
	}

	match data.get(..4).unwrap_or_default() {
		&[a,b,c,d] if (1000..=1006).contains(&u32::from_le_bytes([a,b,c,d])) => {
			let itp = cradle::itp::read(&data)?;
			w!("png", to_indexed_png(&itp)?);
		}

		b"ITP\xFF" => {
			let itp = cradle::itp32::read(&data)?;
			match () {
				() if itp.has_mipmaps() => w!("dds", to_raw_dds(&itp)?),
				()                      => w!("png", to_image(IF::Png, itp.to_rgba(0))?),
			}
		}

		b"DDS " => {
			let dds = ddsfile::Dds::read(Cursor::new(&data))?;
			if let Some(Dxgi::BC7_Typeless|Dxgi::BC7_UNorm|Dxgi::BC7_UNorm_sRGB) = dds.get_dxgi_format() {
				let width = dds.get_width() as usize;
				let height = dds.get_height() as usize;

				let mut it = dds.data
					.chunks_exact(16)
					.map(|a| u128::from_le_bytes(a.try_into().unwrap()));

				let levels = (0..dds.get_num_mipmap_levels() as u16).map(|level| {
					let level_size = (width >> level) * (height >> level);
					it.by_ref().take(level_size >> 4).collect()
				}).collect();
				w!("itp", to_raw_itp32(&Itp32 { width, height, levels })?);
			} else {
				let img = image::load(Cursor::new(&data), IF::Dds)?.to_rgba8();
				w!("itp", to_itp32(img)?);
			}
		}

		b"\x89PNG" => {
			let mut png = png::Decoder::new(Cursor::new(&data))
				.read_info()?;
			println!("{:?}", png.info());
			if let Some(pal) = &png.info().palette {
				let width = png.info().width as usize;
				let height = png.info().height as usize;
				let palette = if let Some(trns) = &png.info().trns {
					pal.chunks_exact(3).zip(trns.iter())
						.map(|(a, b)| Rgba([a[0], a[1], a[2], *b]))
						.collect()
				} else {
					pal.chunks_exact(3)
						.map(|a| Rgba([a[0], a[1], a[2], 0xFF]))
						.collect()
				};
				let mut data = vec![0; width * height];
				png.next_frame(&mut data)?;
				let image = cradle::util::image(width, height, data)?;
				w!("itp", to_indexed_itp(&Itp { palette, image })?);
			} else {
				let img = image::load(Cursor::new(&data), IF::Png)?.to_rgba8();
				w!("itp", to_itp32(img)?);
			}
		}

		b"V101"|b"V102" => {
			todo!()
		}
		b"file" => {
			todo!()
		}

		_ => eyre::bail!("could not identify input file"),
	}
	Ok(())
}

fn to_image(format: IF, img: RgbaImage) -> Result<Vec<u8>> {
	let mut f = Cursor::new(Vec::new());
	img.write_to(&mut f, format)?;
	Ok(f.into_inner())
}

fn to_indexed_itp(itp: &Itp) -> Result<Vec<u8>> {
	Ok(cradle::itp::write1004(itp)?)
}

fn to_itp32(img: RgbaImage) -> Result<Vec<u8>> {
	let a = intel_tex_2::bc7::compress_blocks(
		&intel_tex_2::bc7::alpha_very_fast_settings(),
		&intel_tex_2::RgbaSurface {
			width: img.width(),
			height: img.width(),
			stride: img.width() * 4,
			data: &img,
		}
	);
	let data = a
		.chunks_exact(16)
		.map(|a| u128::from_le_bytes(a.try_into().unwrap()))
		.collect();
	to_raw_itp32(&Itp32 {
		width: img.width() as usize,
		height: img.height() as usize,
		levels: vec![data],
	})
}

fn to_raw_itp32(itp: &Itp32) -> Result<Vec<u8>> {
	todo!()
}

fn to_raw_dds(itp: &Itp32) -> Result<Vec<u8>> {
	let mut f = Cursor::new(Vec::new());
	let mut dds = ddsfile::Dds::new_dxgi(ddsfile::NewDxgiParams {
		height: itp.width as u32,
		width: itp.height as u32,
		depth: None,
		format: Dxgi::BC7_UNorm,
		mipmap_levels: itp.has_mipmaps().then_some(itp.levels() as u32),
		array_layers: None,
		caps2: None,
		is_cubemap: false,
		resource_dimension: ddsfile::D3D10ResourceDimension::Texture2D,
		alpha_mode: ddsfile::AlphaMode::Unknown,
	})?;
	dds.data = itp.levels.iter().flatten().copied().flat_map(u128::to_le_bytes).collect();
	dds.write(&mut f)?;
	Ok(f.into_inner())
}

fn to_indexed_png(itp: &Itp) -> Result<Vec<u8>> {
	let mut f = Cursor::new(Vec::new());
	let mut png = png::Encoder::new(&mut f, itp.image.width(), itp.image.height());
	let mut pal = Vec::with_capacity(3*itp.palette.len());
	let mut alp = Vec::with_capacity(itp.palette.len());
	for &Rgba([r,g,b,a]) in &itp.palette {
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
	w.write_image_data(&itp.image)?;
	w.finish()?;
	Ok(f.into_inner())
}
