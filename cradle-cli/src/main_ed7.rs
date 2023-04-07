use std::fs::File;
use std::io::{Cursor, Read, Seek, Write, SeekFrom, BufRead};
use std::path::{PathBuf, Path};

use clap::{Parser, ValueHint};
use cradle::{itp::Itp, itp32::Itp32, itc::Itc};
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

	let path = |ext: &str| {
		cli.output.clone()
			.unwrap_or_else(|| cli.file.with_extension(ext))
	};
	let file = |ext: &str| {
		File::create(path(ext))
	};

	let data = std::fs::read(&cli.file)?;

	if name.ends_with(".itp") {
		if data.starts_with(b"ITP\xFF") {
			let itp = cradle::itp32::read(&data)?;
			if itp.has_mipmaps() {
				itp.to_bc7_dds().write(&mut file("dds")?)?;
			} else {
				itp.to_rgba(0).write(None, file("png")?)?;
			}
		} else {
			let itp = cradle::itp::read(&data)?;
			itp.to_rgba().write(Some(&itp.palette), file("png")?)?;
		}

	} else if name.ends_with(".png") {
		let (img, pal) = load_png(Cursor::new(&data))?;
		if let Some(pal) = pal {
			Itp::from_rgba(&img, pal).unwrap().write(file("itp")?)?;
		} else {
			Itp32::from_rgba(&img).write(file("itp")?)?;
		}

	} else if name.ends_with(".dds") {
		let dds = ddsfile::Dds::read(Cursor::new(&data))?;
		if let Some(itp) = Itp32::from_bc7_dds(&dds) {
			itp.write(file("itp")?)?;
		} else {
			let img = image::load(Cursor::new(&data), IF::Dds)?.to_rgba8();
			Itp32::from_rgba(&img).write(file("itp")?)?;
		}

	} else if name.ends_with(".itc") {
		let itc = cradle::itc::read(&data)?;
		let outdir = path("");
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

fn convert_itc(itc: &Itc, outdir: &Path) -> Result<()> {
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

				img.write(pal.as_deref(), File::create(&path)?)?;

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
		out.write(i.pal.as_deref(), File::create(&i.path)?)?;
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

#[extend::ext]
impl Itp {
	fn write(&self, mut w: impl Write) -> Result<()> {
		Ok(w.write_all(&cradle::itp::write1004(self)?)?)
	}
}

#[extend::ext]
impl Itp32 {
	fn write(&self, mut w: impl Write) -> Result<()> {
		Ok(w.write_all(&cradle::itp32::write(self)?)?)
	}
}

#[extend::ext]
impl RgbaImage {
	fn write(&self, pal: Option<&[Rgba<u8>]>, mut w: impl Write + Seek) -> Result<()> {
		if let Some(pal) = &pal {
			let itp = Itp::from_rgba(self, pal.to_vec()).unwrap();
			write_indexed_png(w, &itp)?;
		} else {
			self.write_to(&mut w, IF::Png)?;
		}
		Ok(())
	}
}

fn load_png(mut r: impl BufRead + Seek) -> Result<(RgbaImage, Option<Vec<Rgba<u8>>>)> {
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

fn write_indexed_png(mut w: impl Write, itp: &Itp) -> Result<()> {
	let mut png = png::Encoder::new(&mut w, itp.image.width(), itp.image.height());
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
	Ok(())
}
