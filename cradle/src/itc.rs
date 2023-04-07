use std::borrow::Cow;
use std::collections::BTreeSet;

use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _, Label};
use image::Rgba;
use crate::util::*;

#[derive(Clone, PartialEq)]
pub struct Frame {
	pub index: Option<usize>,
	pub unknown: u16,
	pub x_offset: f32,
	pub y_offset: f32,
	pub x_scale: f32,
	pub y_scale: f32,
}

impl std::fmt::Debug for Frame {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if self == &Frame::default() {
			write!(f, "Frame::default()")
		} else {
			f.debug_struct("Frame")
				.field("index", &self.index)
				.field("unknown", &self.unknown)
				.field("x_offset", &self.x_offset)
				.field("y_offset", &self.y_offset)
				.field("x_scale", &self.x_scale)
				.field("y_scale", &self.y_scale)
				.finish()
		}
	}
}

impl Default for Frame {
	fn default() -> Self {
		Self {
			index: None,
			unknown: 0,
			x_offset: 0.0,
			y_offset: 0.0,
			x_scale: 1.0,
			y_scale: 1.0,
		}
	}
}

#[derive(Clone, PartialEq)]
pub struct Itc<'a> {
	pub frames: [Frame; 128],
	pub content: Vec<Cow<'a, [u8]>>,
	pub palette: Option<Vec<Rgba<u8>>>,
}

impl<'a> Default for Itc<'a> {
	fn default() -> Self {
		Self {
			frames: std::array::from_fn(|_| Default::default()),
			content: Default::default(),
			palette: Default::default()
		}
	}
}

impl<'a> std::fmt::Debug for Itc<'a> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		use std::fmt::*;
		struct _D<F: Fn(&mut Formatter) -> Result>(F);
		impl<F: Fn(&mut Formatter) -> Result> Debug for _D<F> {
			fn fmt(&self, f: &mut Formatter) -> Result {
				self.0(f)
			}
		}
		f.debug_struct("Itc")
			.field("frames", &self.frames)
			.field("content", &_D(|f| {
				let mut x = f.debug_list();
				for a in &self.content {
					x.entry(&format_args!("[_; {}]", a.len()));
				}
				x.finish()
			}))
			.field("palette", &self.palette)
			.finish()
	}
}

#[allow(clippy::type_complexity)]
pub fn read(data: &[u8]) -> Result<Itc, Error> {
	let mut f = Reader::new(data);

	let has_palette = match &f.array()? {
		b"V101" => false,
		b"V102" => true,
		_ => bail!("itc: invalid header")
	};

	let mut frames = std::array::from_fn(|_| Frame::default());
	let mut slices = [(0, 0); 128];

	for i in &mut slices {
		*i = (f.u32()? as usize, f.u32()? as usize);
	}

	for k in &mut frames { k.unknown  = f.u16()?; }
	for k in &mut frames { k.x_offset = f.f32()?; }
	for k in &mut frames { k.y_offset = f.f32()?; }
	for k in &mut frames { k.x_scale  = f.f32()?; }
	for k in &mut frames { k.y_scale  = f.f32()?; }

	let palette = if has_palette {
		Some(crate::itp::read_palette(f.u32()?, &mut f)?)
	} else {
		None
	};

	let points = {
		let mut points = BTreeSet::new();
		for p in slices {
			if p != (0, 0) {
				points.insert(p.0);
			}
		}
		points.insert(f.len());
		Vec::from_iter(points)
	};

	let mut content = Vec::with_capacity(points.len()-1);
	for (&start, &end) in points.iter().zip(points.iter().skip(1)) {
		content.push(f.at(start)?.slice(end-start)?.into())
	}

	for (k, (off, len)) in frames.iter_mut().zip(slices.into_iter()) {
		if (off, len) != (0, 0) {
			let n = points.binary_search(&off).unwrap();
			ensure!(n < points.len() - 1);
			ensure!(points[n+1] == off + len);
			k.index = Some(n);
		}
	}

	Ok(Itc {
		frames,
		content,
		palette,
	})
}

pub fn write(itc: &Itc) -> Result<Vec<u8>, Error> {
	let mut f = Writer::new();
	let mut slice = Writer::new();
	let mut unknown = Writer::new();
	let mut x_offset = Writer::new();
	let mut y_offset = Writer::new();
	let mut x_scale = Writer::new();
	let mut y_scale = Writer::new();
	let mut palette = Writer::new();

	if let Some(pal) = &itc.palette {
		f.slice(b"V102");
		palette.u32(pal.len() as u32);
		crate::itp::write_palette(pal, &mut palette);
	} else {
		f.slice(b"V101");
	}

	for fr in &itc.frames {
		if let Some(index) = fr.index {
			slice.delay32(Label::known(index as u32));
			slice.u32(itc.content[index].len() as u32);
		} else {
			slice.u32(0);
			slice.u32(0);
		}
		unknown.u16(fr.unknown);
		x_offset.f32(fr.x_offset);
		y_offset.f32(fr.y_offset);
		x_scale.f32(fr.x_scale);
		y_scale.f32(fr.y_scale);
	}

	f.append(slice);
	f.append(unknown);
	f.append(x_offset);
	f.append(y_offset);
	f.append(x_scale);
	f.append(y_scale);
	f.append(palette);

	for (i, img) in itc.content.iter().enumerate() {
		f.label(Label::known(i as u32));
		f.slice(img);
	}

	Ok(f.finish()?)
}

#[test]
fn test() -> Result<(), Box<dyn std::error::Error>> {
	use std::path::Path;
	fn dir(p: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
		let mut i = std::fs::read_dir(p.as_ref())?.collect::<Result<Vec<_>, _>>()?;
		i.sort_by_key(|a| a.path());
		for file in i {
			if file.path().extension() == Some(std::ffi::OsStr::new("itc")) {
				let data = std::fs::read(file.path())?;
				let img = read(&data)?;
				let y = write(&img)?;
				let mut xs = img.frames.iter().filter_map(|a| a.index).collect::<Vec<_>>();
				xs.sort();
				assert_eq!(xs, (0..xs.len()).collect::<Vec<_>>());
				if data != y {
					println!("{}", file.path().display());
					std::fs::write(Path::new("/tmp").join(file.path().file_name().unwrap()), &y)?;
				}
			}
		}
		Ok(())
	}

	dir("../data/fc-evo/data/chr/")?;
	dir("../data/sc-evo/data_sc/chr/")?;
	dir("../data/3rd-evo/data_3rd/chr/")?;
	dir("../data/zero/data/chr/")?;
	dir("../data/zero/data/apl/")?;
	dir("../data/zero/data/monster/")?;
	dir("../data/zero-evo/data/chr/")?;
	dir("../data/zero-evo/data/apl/")?;
	dir("../data/zero-evo/data/monster/")?;
	dir("../data/ao-evo/data/chr/")?;
	dir("../data/ao-evo/data/apl/")?;
	dir("../data/ao-evo/data/monster/")?;
	Ok(())
}

#[test]
fn test2() -> Result<(), Box<dyn std::error::Error>> {
	fn f(path: &str) -> Result<(), Box<dyn std::error::Error>> {
		let d = std::fs::read(path)?;
		let img = read(&d)?;
		for (i, f) in img.frames.iter().enumerate() {
			if let Some(e) = f.index {
				print!("{:02}", e);
			} else {
				print!("--");
			}
			if i % 8 == 7 {
				println!();
			} else {
				print!(" ");
			}
		}
		let xs = (0..img.content.len()).map(|i| img.frames.iter().position(|j| j.index == Some(i)).unwrap()).collect::<Vec<_>>();
		let mut last = 0;
		for i in xs {
			if i <= last {
				println!();
			} else {
				print!(" ");
			}
			last = i;
			print!("{:02}", i);
		}
		println!();
		println!();
		Ok(())
	}
	f("../data/fc-evo/data/chr/ch00100.itc")?;
	f("../data/fc-evo/data/chr/ch00108.itc")?;

	panic!()
}

