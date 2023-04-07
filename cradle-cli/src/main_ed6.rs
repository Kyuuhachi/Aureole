#![feature(try_blocks)]

use std::fs::File;
use std::path::{PathBuf, Path};

use clap::{Parser, ValueHint};
use cradle::{ch, chcp};
use eyre::{Result, Context, ContextCompat};
use image::{RgbaImage, ImageFormat as IF};

#[derive(Debug, Clone, Parser)]
struct Cli {
	/// Where to place the output.
	///
	/// If unspecified, output will be placed next to the input file.
	///
	/// For chcp files, the names of the individual frames, or the ch file when recompiling, can currently not be controlled.
	#[clap(long, short, value_hint = ValueHint::FilePath)]
	output: Option<PathBuf>,

	/// The file to be processed. Should be a ._ch, ._cp, ._ds, .png, .dds, or .json, or a directory containing a .json.
	#[clap(required = true, value_hint = ValueHint::FilePath)]
	file: PathBuf,

	#[command(flatten)]
	mode: Mode,

	/// Read ch as the specified width, rather than guessing based on filename.
	///
	/// Has no effect when writing.
	#[clap(long, short)]
	width: Option<usize>,
}

#[derive(Debug, Clone, clap::Args)]
#[group(multiple = false)]
struct Mode {
	/// Read/write ch in argb1555 format, rather than guessing based on filename.
	#[clap(long="1555", short='1')]
	argb1555: bool,
	/// Read/write ch in argb4444 format, rather than guessing based on filename.
	#[clap(long="4444", short='4')]
	argb4444: bool,
	/// Read/write ch in argb8888 format, rather than guessing based on filename.
	#[clap(long="8888", short='8')]
	argb8888: bool,
}

impl Mode {
	fn get(&self) -> Option<ch::Mode> {
		match self {
			Mode { argb1555: true, .. } => Some(ch::Mode::Argb1555),
			Mode { argb4444: true, .. } => Some(ch::Mode::Argb4444),
			Mode { argb8888: true, .. } => Some(ch::Mode::Argb8888),
			_ => None
		}
	}
}

fn main() -> Result<()> {
	let cli = Cli::parse();

	let mut infile = cli.file.clone();
	if infile.join("chip.json").is_file() {
		infile = infile.join("chip.json");
	}

	let Some(name) = infile.file_name().and_then(|a| a.to_str()) else {
		eyre::bail!("file has no name");
	};
	let name = name.to_lowercase();

	let path = |ext: &str| {
		cli.output.clone()
			.unwrap_or_else(|| cli.file.with_extension(ext))
	};

	let data = std::fs::read(&infile)?;

	if name.ends_with("._ch") {
		let img = if name.starts_with("ch") && data.len() % (2*16*16) == 2 {
			println!("this is a chcp; you likely want to convert the ._cp file instead");
			ch::read(ch::Mode::Argb4444, 16, &data[2..])?
		} else {
			let p = path("");
			let basename = p.file_name().unwrap().to_str().unwrap();
			let guess = ch::guess_from_byte_size(basename, data.len());
			let mode = cli.mode.get().or(guess.map(|a| a.0)).context("could not guess format")?;
			let width = cli.width.or(guess.map(|a| a.1)).context("could not guess format")?;
			ch::read(mode, width, &data)?
		};
		img.write_to(&mut File::create(path("png"))?, IF::Png)?;

	} else if name.ends_with(".png") {
		let img = image::open(&infile)?.to_rgba8();
		let p = path("");
		let basename = p.file_name().unwrap().to_str().unwrap();
		let guess = ch::guess_from_image_size(basename, img.width() as usize, img.height() as usize);
		let mode = cli.mode.get().or(guess).context("could not guess format")?;
		let data = ch::write(mode, &img)?;
		std::fs::write(path("_ch"), data)?;

	} else if name.ends_with(".dds") {
		std::fs::write(path("_ds"), &data)?;

	} else if name.ends_with("._ds") {
		std::fs::write(path("dds"), &data)?;

	} else if name.ends_with("._cp") {
		let basename = name.strip_suffix("._cp").unwrap();
		let basename = basename.strip_suffix('p').unwrap_or(basename);
		let ch_path = infile.with_file_name(format!("{basename}._ch"));
		let ch_data = std::fs::read(&ch_path)
			.with_context(|| format!("could not find corresponding ._ch file: {}", ch_path.display()))?;
		let chcp = chcp::read(&ch_data, &data)?;
		let outdir = cli.output.unwrap_or_else(|| cli.file.with_file_name(basename));
		std::fs::create_dir_all(&outdir)?;

		convert_chcp(&chcp, &outdir)?;

	} else if name == "chip.json" || name.ends_with(".chip.json") {
		let (ch, cp) = convert_to_chcp(&infile)?;
		let (ch_out, cp_out) = if let Some(out) = &cli.output {
			let outname = out.file_name().unwrap().to_str().unwrap().to_lowercase();
			let ch_out = if let Some(o) = outname.strip_suffix("p._cp") {
				out.with_file_name(format!("{o}._ch"))
			} else {
				out.with_extension("_ch")
			};
			(ch_out, out.to_path_buf())
		} else {
			(path("_ch"), path("_cp"))
		};
		std::fs::write(ch_out, ch)?;
		std::fs::write(cp_out, cp)?;

	} else if name.ends_with(".itp") || name.ends_with(".itc") {
		eyre::bail!("this looks like an ed7 file, try cradle-ed7");

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

fn convert_chcp(chcp: &[RgbaImage], outdir: &Path) -> Result<()> {
	let mut imgdata = Vec::new();
	for (frame_id, img) in chcp.iter().enumerate() {
		if img.pixels().all(|a| a.0[3] == 0) {
			continue
		}
		let path = outdir.join(format!("{frame_id}.png"));
		img.write_to(&mut File::create(&path)?, IF::Png)?;
		imgdata.push(ItcImage {
			path: path.strip_prefix(outdir).unwrap().to_path_buf(),
			frame: frame_id,
			offset: None,
			scale: (1., 1.),
		});
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

fn convert_to_chcp(jsonpath: &Path) -> Result<(Vec<u8>, Vec<u8>)> {
	let spec: Vec<ItcImage> = serde_json::from_reader(File::open(jsonpath)?)?;
	let mut chcp = Vec::new();
	for i in spec {
		let img = image::open(jsonpath.parent().unwrap().join(&i.path))?.to_rgba8();
		eyre::ensure!(i.offset.is_none(), "i.offset.is_none()");
		eyre::ensure!(i.scale == (1., 1.), "i.scale == (1., 1.)");
		chcp.push(img);
	}
	Ok(cradle::chcp::write(&chcp)?)
}
