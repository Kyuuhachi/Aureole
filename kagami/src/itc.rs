use std::backtrace::Backtrace;

use hamu::read::le::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("{source}")]
	Read { #[from] source: hamu::read::Error, backtrace: Backtrace },
	#[error("bad itc header: expected V101 or V102, got {:?}", String::from_utf8_lossy(val))]
	BadHeader { val: [u8; 4], backtrace: Backtrace },
	#[error("no frame at {offset}")]
	InvalidFrame { offset: usize, backtrace: Backtrace },
	#[error("frame {index} is {got} bytes, expected {expected}")]
	InvalidFrameLength { index: usize, got: usize, expected: usize, backtrace: Backtrace },
	#[error(transparent)]
	Itp(#[from] crate::itp::Error),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Itc {
	pub frames: Vec<Frame>,
	pub images: Vec<crate::itp::Itp>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Frame {
	pub index: usize,
	pub x_offset: f32,
	pub y_offset: f32,
	pub x_scale: f32,
	pub y_scale: f32,
}

pub fn read(data: &[u8]) -> Result<Itc, Error> {
	let mut f = Reader::new(data);

	let v102 = match &f.array()? {
		b"V101" => false,
		b"V102" => true,
		&x => return Err(Error::BadHeader { val: x, backtrace: Backtrace::capture() })
	};

	let mut slice  = [(0, 0); 128];
	let mut xoff   = [0.; 128];
	let mut yoff   = [0.; 128];
	let mut xscale = [1.; 128];
	let mut yscale = [1.; 128];

	for i in &mut slice {
		*i = (f.u32()? as usize, f.u32()? as usize);
	}

	for _ in 0..128 { f.check_u16(0)?; }
	for i in &mut xoff { *i = f.f32()?; }
	for i in &mut yoff { *i = f.f32()?; }
	for i in &mut xscale { *i = f.f32()?; }
	for i in &mut yscale { *i = f.f32()?; }

	let palette = if v102 {
		Some(crate::itp::read_palette(f.u32()?, &mut f)?)
	} else {
		None
	};

	let mut image_pos = Vec::new();
	let mut images = Vec::new();
	while f.remaining() > 0 {
		let start = f.pos();
		let mut itp = crate::itp::read0(&mut f)?;
		let end = f.pos();
		image_pos.push(start..end);
		if let Some(palette) = &palette {
			let crate::itp::Itp::Palette(i) = &mut itp else { panic!() };
			assert!(i.palette.is_empty());
			i.palette = palette.to_owned();
		}
		images.push(itp)
	}

	let mut frames = Vec::with_capacity(128);
	for i in 0..128 {
		if slice[i] == (0, 0) {
			break
		}
		let (offset, len) = slice[i];
		let (index, range) = image_pos.iter().enumerate()
			.find(|a| a.1.start == offset)
			.ok_or_else(|| Error::InvalidFrame { offset, backtrace: Backtrace::capture() })?;
		if range.end-range.start != len {
			return Err(Error::InvalidFrameLength { index, expected: len, got: range.end-range.start, backtrace: Backtrace::capture() })?;
		}
		frames.push(Frame {
			index,
			x_offset: xoff[i],
			y_offset: yoff[i],
			x_scale: xscale[i],
			y_scale: yscale[i],
		});
	}

	Ok(Itc {
		frames,
		images,
	})
}

// pub fn write(itc: &Itc) -> Result<Vec<u8>, hamu::write::Error> {
// 	let mut f = Writer::new();
// 	let mut unk = Writer::new();
// 	let mut xoff = Writer::new();
// 	let mut yoff = Writer::new();
// 	let mut xscale = Writer::new();
// 	let mut yscale = Writer::new();
//
// 	let mut g = Writer::new();
//
// 	match itc.version {
// 		Version::V101 => f.slice(b"V101"),
// 		Version::V102 => f.slice(b"V102"),
// 	}
//
// 	for fr in &itc.frames {
// 		let start = g.len();
// 		f.delay_u32(g.here());
// 		f.u32(itc.images[fr.index].len() as u32);
// 		unk.u16(0);
// 		xoff.f32(fr.xoff);
// 		yoff.f32(fr.yoff);
// 		xscale.f32(fr.xscale);
// 		yscale.f32(fr.yscale);
// 	}
//
// 	for _ in itc.frames.len()..128 {
// 		f.u32(0);
// 		f.u32(0);
// 		unk.u16(0);
// 		xoff.f32(0.);
// 		yoff.f32(0.);
// 		xscale.f32(1.);
// 		yscale.f32(1.);
// 	}
//
//
// 	f.append(unk);
// 	f.append(xoff);
// 	f.append(yoff);
// 	f.append(xscale);
// 	f.append(yscale);
//
// 	for (i, img) in itc.images.iter().enumerate() {
// 		f.label(Label::known(i as u32).1);
// 		f.slice(img);
// 	}
//
// 	let data = f.finish()?;
// 	Reader::new(&data).dump().oneline().to_stdout();
// 	Ok(data)
// }

#[test]
fn test() -> Result<(), Box<dyn std::error::Error>>{
	use std::{fs, path::Path};

	let img = read(&fs::read("../data/ao-evo/data/apl/ch51211.itc")?)?;

	let outdir = Path::new("/tmp/ch40004");
	if outdir.exists() {
		fs::remove_dir_all(outdir)?;
	}
	fs::create_dir_all(outdir)?;

	let f = fs::File::create(outdir.join("frames.csv"))?;
	let mut wtr = csv::Writer::from_writer(f);
	for frame in &img.frames {
		wtr.serialize(frame)?;
	}

	for (i, img) in img.images.iter().enumerate() {
		let f = fs::File::create(outdir.join(format!("{i}.png")))?;
		match img {
			crate::itp::Itp::Palette(img) => img.save(f)?,
		}
	}
	Ok(())
}
