use std::fs::File;
use std::io::{Cursor, Read, Seek, Write, SeekFrom, BufRead};
use std::path::{PathBuf, Path};

use clap::{Parser, ValueHint};
use cradle::{itp::Itp, itp32::Itp32, itc::Itc};
use ddsfile::DxgiFormat as Dxgi;
use eyre::Result;
use image::{RgbaImage, ImageFormat as IF, Rgba, GenericImage};

#[derive(Debug, Clone, Parser)]
struct Cli {
	/// Where to place the output.
	///
	/// If unspecified, output will be placed next to the input file.
	///
	/// For itc files, the names of the individual frames can currently not be controlled.
	#[clap(long, short, value_hint = ValueHint::FilePath)]
	output: Option<PathBuf>,

	/// The file to be processed. Should be a .itp, .itc, .png, .dds, or .json, or a directory containing a .json.
	#[clap(required = true, value_hint = ValueHint::FilePath)]
	file: PathBuf,
}

fn main() -> Result<()> {
	let mut cli = Cli::parse();

	if cli.file.join("chip.json").is_file() {
		cli.file = cli.file.join("chip.json");
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
			w("png", &to_indexed_png(&itp)?)?
		}

	} else if name.ends_with(".png") {
		let (img, pal) = load_png(Cursor::new(&data))?;
		if let Some(pal) = pal {
			w("itp", &write_itp(&Itp::from_rgba(&img, pal).unwrap())?)?
		} else {
			w("itp", &write_itp32(&rgba_to_itp32(img))?)?
		}

	} else if name.ends_with(".dds") {
		let dds = ddsfile::Dds::read(Cursor::new(&data))?;
		if let Some(Dxgi::BC7_Typeless|Dxgi::BC7_UNorm|Dxgi::BC7_UNorm_sRGB) = dds.get_dxgi_format() {
			w("itp", &write_itp32(&dds_to_itp32(&dds))?)?
		} else {
			let img = image::load(Cursor::new(&data), IF::Dds)?.to_rgba8();
			w("itp", &write_itp32(&rgba_to_itp32(img))?)?
		}

	} else if name.ends_with(".itc") {
		let itc = cradle::itc::read(&data)?;
		let outdir = cli.output.clone().unwrap_or_else(|| cli.file.with_extension(""));
		std::fs::create_dir_all(&outdir)?;

		convert_itc(&itc, &outdir)?;

	// } else if name == "chip.json" || name.ends_with(".chip.json") {
	// 	let mut rdr = csv::Reader::from_path(&cli.file)?;
	// 	let frames: Vec<ItcFrame> = rdr.deserialize().collect::<Result<_, _>>()?;
	// 	let mut images = BTreeMap::new();
	// 	for f in &frames {
	// 		if !images.contains_key(&f.filename) {
	// 			let path = cli.file.parent().unwrap().join(&f.filename);
	// 			let data = std::fs::read(&path)?;
	// 			images.insert(f.filename.clone(), convert_image(&data)?);
	// 		}
	// 	}
	//
	// 	let itc = Itc {
	// 		frames: frames.iter().map(|f| {
	// 			let (index, (_, &(_, w, h))) = images.iter().enumerate().find(|a| a.1.0 == &f.filename).unwrap();
	// 			cradle::itc::Frame {
	// 				index,
	// 				unknown: f.unknown,
	// 				x_offset: f.x_offset / w as f32,
	// 				y_offset: f.y_offset / h as f32,
	// 				x_scale: f.x_scale.recip(),
	// 				y_scale: f.y_scale.recip(),
	// 			}
	// 		}).collect(),
	// 		content: images.values().map(|a| a.0.as_slice()).collect(),
	// 		palette: None,
	// 	};
	//
	// 	let out = if let Some(i) = cli.output.clone() {
	// 		i
	// 	} else if name == "chip.json" {
	// 		PathBuf::from(cli.file.parent().unwrap().to_str().unwrap().to_owned() + ".itc")
	// 	} else { // *.chip.json
	// 		cli.file.with_extension("")
	// 	};
	//
	// 	let data = cradle::itc::write(&itc)?;
	// 	std::fs::write(out, data)?;

	} else {
		eyre::bail!("could not infer file type");
	}

	Ok(())
}

#[derive(Debug, Clone, PartialEq)]
#[derive(serde::Serialize, serde::Deserialize)]
struct ItcImage {
	frame: usize,
	path: PathBuf,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	offset: Option<(f32, f32)>,
	#[serde(default = "unit_scale", skip_serializing_if = "is_unit_scale")]
	scale: (f32, f32),
}

fn unit_scale() -> (f32, f32) { (1.0, 1.0) }
fn is_unit_scale(a: &(f32, f32)) -> bool { *a == unit_scale() }

fn convert_itc(itc: &Itc, outdir: &Path) -> eyre::Result<()> {
	struct Img {
		path: PathBuf,
		img: RgbaImage,
		pal: Option<Vec<Rgba<u8>>>,
		offset: (i32, i32),
	}
	let mut images = Vec::new();
	let mut imgdata = Vec::new();
	for (i, data) in itc.content.iter().enumerate() {
		if let Some((frame_id, frame)) = itc.frames.iter().enumerate().find(|a| a.1.index == Some(i)) {
			let (pal, img) = if data.starts_with(b"ITP\xFF") {
				let itp = cradle::itp32::read(data)?;
				(None, itp.to_rgba(0))
			} else {
				let itp = cradle::itp::read(data)?;
				let pal = itc.palette.as_ref().unwrap_or(&itp.palette).to_owned();
				(Some(pal), itp.to_rgba())
			};

			let path = outdir.join(format!("{frame_id}.png"));
			let xs = frame.x_scale.recip();
			let ys = frame.x_scale.recip();
			let xo = frame.x_offset * img.width() as f32 * xs;
			let yo = frame.y_offset * img.height() as f32 * ys;

			if (xo-xo.round()).abs() < f32::EPSILON && (yo-yo.round()).abs() < f32::EPSILON {
				let xo = xo.round() as i32;
				let yo = yo.round() as i32;

				images.push(Img {
					path: path.clone(),
					img,
					pal,
					offset: (xo, yo)
				});

				imgdata.push(ItcImage {
					path,
					frame: frame_id,
					offset: None,
					scale: (xs, ys),
				})
			} else {
				println!("couldn't deduce transform for {}", path.display());

				save_png(File::create(&path)?, &img, pal.as_deref())?;

				imgdata.push(ItcImage {
					path,
					frame: frame_id,
					offset: Some((xo, yo)),
					scale: (xs, ys),
				})
			}
		}
	}

	let w = images.iter()
		.map(|i| 2 * (i.img.width() / 2 + i.offset.0.unsigned_abs()))
		.max().unwrap_or(0).next_power_of_two();
	let h = images.iter()
		.map(|i| 2 * (i.img.height() / 2 + i.offset.1.unsigned_abs()))
		.max().unwrap_or(0).next_power_of_two();

	for i in &images {
		let mut out = RgbaImage::new(w, h);
		let x = w / 2 - (i.img.width() as i32 / 2 + i.offset.0) as u32;
		let y = h / 2 - (i.img.height() as i32 / 2 + i.offset.1) as u32;
		out.pixels_mut().for_each(|p| *p = *i.img.get_pixel(0, 0));
		out.copy_from(&i.img, x, y)?;
		save_png(File::create(&i.path)?, &out, i.pal.as_deref())?;
	}

	let mut f = std::fs::File::create(outdir.join("chip.json"))?;
	{
		use serde_json::ser::Formatter;
		let mut pf = serde_json::ser::PrettyFormatter::with_indent(b"\t");
		pf.begin_array(&mut f)?;
		for (i, v) in imgdata.iter().enumerate() {
			pf.begin_array_value(&mut f, i == 0)?;
			serde_json::to_writer(&mut f, v)?;
			pf.end_array_value(&mut f)?;
		}
		pf.end_array(&mut f)?;
	}

	Ok(())
}

fn save_png(mut w: impl Write + Seek, img: &RgbaImage, pal: Option<&[Rgba<u8>]>) -> eyre::Result<()> {
	if let Some(pal) = &pal {
		let itp = Itp::from_rgba(img, pal.to_vec()).unwrap();
		let data = to_indexed_png(&itp)?;
		w.write_all(&data)?;
	} else {
		img.write_to(&mut w, IF::Png)?;
	}
	Ok(())
}

fn load_png(mut r: impl BufRead + Seek) -> eyre::Result<(RgbaImage, Option<Vec<Rgba<u8>>>)> {
	let pos = r.stream_position()?;
	let png = png::Decoder::new(&mut r).read_info()?;
	let info = png.info();
	let pal = info.palette.as_ref().map(|pal| {
		if let Some(trns) = &info.trns {
			pal.chunks_exact(3).zip(trns.iter())
				.map(|(a, b)| Rgba([a[0], a[1], a[2], *b]))
				.collect()
		} else {
			pal.chunks_exact(3)
				.map(|a| Rgba([a[0], a[1], a[2], 0xFF]))
				.collect()
		}
	});
	r.seek(SeekFrom::Start(pos))?;
	let img = image::load(r, IF::Png)?.to_rgba8();
	Ok((img, pal))
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
	Ok(cradle::itp32::write(itp)?)
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
