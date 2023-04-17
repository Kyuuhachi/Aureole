#![feature(try_blocks)]

use std::fs::File;
use std::io::{Cursor, Seek, Write, SeekFrom, BufRead, BufReader};
use std::path::{PathBuf, Path};

use clap::{Parser, ValueHint};
use cradle::{itp::Itp, itp32::Itp32, itc::Itc};
use anyhow::Result;
use image::{RgbaImage, ImageFormat as IF, Rgba, GenericImage, GenericImageView};

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
	let cli = Cli::parse();

	let mut infile = cli.file.clone();
	if infile.join("chip.json").is_file() {
		infile = infile.join("chip.json");
	}

	let Some(name) = infile.file_name().and_then(|a| a.to_str()) else {
		anyhow::bail!("file has no name");
	};
	let name = name.to_lowercase();

	let path = |ext: &str| {
		cli.output.clone()
			.unwrap_or_else(|| cli.file.with_extension(ext))
	};
	let file = |ext: &str| {
		File::create(path(ext))
	};

	let data = std::fs::read(&infile)?;

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
		img.write_itp(pal.as_deref(), file("itp")?)?;

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

	} else if name == "chip.json" || name.ends_with(".chip.json") {
		convert_to_itc(&infile)?.write(file("itc")?)?;

	} else if name.ends_with("._ch") || name.ends_with("._cp") {
		anyhow::bail!("this looks like an ed7 file, try cradle-ed7");

	} else {
		anyhow::bail!("could not infer file type");
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
			let ys = frame.y_scale.recip();
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
					path: path.strip_prefix(outdir).unwrap().to_path_buf(),
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

fn convert_to_itc(jsonpath: &Path) -> Result<Itc> {
	let spec: Vec<ItcImage> = serde_json::from_reader(File::open(jsonpath)?)?;
	let mut itc = Itc::default();
	for i in spec {
		let (img, pal) = load_png(BufReader::new(File::open(jsonpath.parent().unwrap().join(&i.path))?))?;
		let (img, (xo, yo)) = if let Some(o) = i.offset {
			(img, o)
		} else {
			let (img, (xo, yo)) = crop(&img);
			(img.to_image(), (xo as f32, yo as f32))
		};
		anyhow::ensure!(i.frame < 128);
		itc.frames[i.frame] = cradle::itc::Frame {
			index: Some(itc.content.len()),
			unknown: 0,
			x_offset: xo / i.scale.0 / img.width() as f32,
			y_offset: yo / i.scale.1 / img.height() as f32,
			x_scale: i.scale.0.recip(),
			y_scale: i.scale.1.recip(),
		};
		let mut c = Cursor::new(Vec::<u8>::new());
		img.write_itp(pal.as_deref(), &mut c)?;
		itc.content.push(c.into_inner().into());
	}
	Ok(itc)
}

fn crop(img: &RgbaImage) -> (image::SubImage<&RgbaImage>, (i32, i32)) {
	Option::unwrap_or_else(try {
		let w = img.width();
		let h = img.height();
		let l = (0..w). find(|&x| (0..h).any(|y| img.get_pixel(x, y).0[3] != 0))?;
		let r = (0..w).rfind(|&x| (0..h).any(|y| img.get_pixel(x, y).0[3] != 0))?;
		let u = (0..h). find(|&y| (0..w).any(|x| img.get_pixel(x, y).0[3] != 0))?;
		let d = (0..h).rfind(|&y| (0..w).any(|x| img.get_pixel(x, y).0[3] != 0))?;

		let cx = w as i32 / 2 - (r+l) as i32 / 2;
		let cy = h as i32 / 2 - (d+u) as i32 / 2;

		let ow = (r - l + 2).next_power_of_two().max(4); // I don't know why the +2
		let oh = (d - u + 2).next_power_of_two().max(4);
		let ox = (w as i32 / 2 - cx) as u32 - ow / 2;
		let oy = (h as i32 / 2 - cy) as u32 - oh / 2;

		(img.view(ox, oy, ow, oh), (cx, cy))
	}, || (img.view(0, 0, img.width(), img.height()), (0, 0)))
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
impl Itc<'_> {
	fn write(&self, mut w: impl Write) -> Result<()> {
		Ok(w.write_all(&cradle::itc::write(self)?)?)
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

	fn write_itp(&self, pal: Option<&[Rgba<u8>]>, w: impl Write) -> Result<()> {
		if let Some(pal) = pal {
			Itp::from_rgba(self, pal.to_vec()).unwrap().write(w)
		} else {
			Itp32::from_rgba(self).write(w)
		}
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
