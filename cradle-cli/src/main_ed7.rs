use std::{path::{PathBuf, Path}, io::Cursor};

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
	filename: String,
	unknown: u16,
	x_offset: f32,
	y_offset: f32,
	x_scale: f32,
	y_scale: f32,
}

fn main() -> Result<()> {
	let mut cli = Cli::parse();

	if cli.file.join("itc.csv").is_file() {
		cli.file = cli.file.join("itc.csv");
	}

	let Some(name) = cli.file.file_name().and_then(|a| a.to_str()) else {
		eyre::bail!("file has no name");
	};
	let name = name.to_lowercase();

	let w = |ext: &str, val: &[u8]| {
		let out = cli.output.clone()
			.unwrap_or_else(|| cli.file.with_extension(ext));
		std::fs::write(out, val)
	};

	let data = std::fs::read(&cli.file)?;

	if name.ends_with(".itp") {
		if data.starts_with(b"ITP\xFF") {
			let itp = cradle::itp32::read(&data)?;
			if itp.has_mipmaps() {
				w("dds", &to_raw_dds(&itp)?)?
			} else {
				w("png", &to_image(IF::Png, itp.to_rgba(0))?)?
			}
		} else {
			let itp = cradle::itp::read(&data)?;
			w("png", &to_indexed_png(&itp)?)?;
		}

	} else if name.ends_with(".png") {
		let png = png::Decoder::new(Cursor::new(&data)).read_info()?;
		if png.info().color_type == png::ColorType::Indexed {
			w("itp", &write_itp(&indexed_png_to_itp(png)?)?)?;
		} else {
			let img = image::load(Cursor::new(&data), IF::Png)?.to_rgba8();
			w("itp", &write_itp32(&rgba_to_itp32(img))?)?;
		}

	} else if name.ends_with(".dds") {
		let dds = ddsfile::Dds::read(Cursor::new(&data))?;
		if let Some(Dxgi::BC7_Typeless|Dxgi::BC7_UNorm|Dxgi::BC7_UNorm_sRGB) = dds.get_dxgi_format() {
			w("itp", &write_itp32(&dds_to_itp32(&dds))?)?;
		} else {
			let img = image::load(Cursor::new(&data), IF::Dds)?.to_rgba8();
			w("itp", &write_itp32(&rgba_to_itp32(img))?)?;
		}

	} else if name.ends_with(".itc") {
		let itc = cradle::itc::read(&data)?;
		let outdir = cli.output.clone().unwrap_or_else(|| cli.file.with_extension(""));
		std::fs::create_dir_all(&outdir)?;

		let nd = itc.content.len().to_string().len();
		let mut images = Vec::with_capacity(itc.content.len());
		for (i, data) in itc.content.iter().enumerate() {
			let (ext, data, w, h) = if data.starts_with(b"ITP\xFF") {
				let itp = cradle::itp32::read(data)?;
				if itp.has_mipmaps() {
					("dds", to_raw_dds(&itp)?, itp.width as u32, itp.height as u32)
				} else {
					("png", to_image(IF::Png, itp.to_rgba(0))?, itp.width as u32, itp.height as u32)
				}
			} else {
				let mut itp = cradle::itp::read(data)?;
				if let Some(pal) = &itc.palette {
					itp.palette = pal.clone();
				}
				("png", to_indexed_png(&itp)?, itp.image.width(), itp.image.height())
			};
			let name = format!("{i:0nd$}.{ext}");
			std::fs::write(outdir.join(&name), &data)?;
			images.push((name, w, h));
		}

		let f = std::fs::File::create(outdir.join("itc.csv"))?;
		let mut wtr = csv::Writer::from_writer(f);
		for frame in &itc.frames {
			let (name, w, h) = images[frame.index].clone();
			wtr.serialize(ItcFrame {
				filename: name,
				unknown: frame.unknown,
				x_offset: frame.x_offset*w as f32,
				y_offset: frame.y_offset*h as f32,
				x_scale: frame.x_scale.recip(),
				y_scale: frame.y_scale.recip(),
			})?;
		}

	} else if name == "itc.csv" || name.ends_with(".itc.csv") {
		todo!()

	} else {
		eyre::bail!("could not infer file type");
	}

	Ok(())
}

fn indexed_png_to_itp<T: std::io::Read>(mut png: png::Reader<T>) -> Result<Itp> {
	let Some(pal) = &png.info().palette else {
		eyre::bail!("no palette?")
	};
	eyre::ensure!(png.info().bit_depth == png::BitDepth::Eight, "only 8-bit palette supported");
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
	Ok(Itp { palette, image })
}

fn dds_to_itp32(dds: &ddsfile::Dds) -> Itp32 {
	let width = dds.get_width() as usize;
	let height = dds.get_height() as usize;

	let mut it = dds.data
		.chunks_exact(16)
		.map(|a| u128::from_le_bytes(a.try_into().unwrap()));

	let levels = (0..dds.get_num_mipmap_levels() as u16).map(|level| {
		let level_size = (width >> level) * (height >> level);
		it.by_ref().take(level_size >> 4).collect()
	}).collect();
	Itp32 { width, height, levels }
}

fn rgba_to_itp32(img: RgbaImage) -> Itp32 {
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
	Itp32 {
		width: img.width() as usize,
		height: img.height() as usize,
		levels: vec![data],
	}
}

fn to_image(format: IF, img: RgbaImage) -> Result<Vec<u8>> {
	let mut f = Cursor::new(Vec::new());
	img.write_to(&mut f, format)?;
	Ok(f.into_inner())
}

fn write_itp(itp: &Itp) -> Result<Vec<u8>> {
	Ok(cradle::itp::write1004(itp)?)
}

fn write_itp32(itp: &Itp32) -> Result<Vec<u8>> {
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
