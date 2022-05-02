use std::str::FromStr;

use image::{RgbaImage, GenericImage, GenericImageView};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageEdit {
	ReGrid { w1: u32, h1: u32, w2: u32, h2: u32, },
	Crop { x: u32, y: u32, w: u32, h: u32, imgw: u32, imgh: u32 },
}

impl FromStr for ImageEdit {
	type Err = eyre::Report;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let (command, args) = s.split_once(':').ok_or_else(|| eyre::eyre!("no command"))?;
		let mut args = args.split(',');
		match command {
			"regrid" => Ok(Self::ReGrid {
				w1: args.next().ok_or_else(|| eyre::eyre!("no w1"))?.parse()?,
				h1: args.next().ok_or_else(|| eyre::eyre!("no h1"))?.parse()?,
				w2: args.next().ok_or_else(|| eyre::eyre!("no w2"))?.parse()?,
				h2: args.next().ok_or_else(|| eyre::eyre!("no h2"))?.parse()?,
			}),
			"crop" => Ok(Self::Crop {
				x: args.next().ok_or_else(|| eyre::eyre!("no x"))?.parse()?,
				y: args.next().ok_or_else(|| eyre::eyre!("no y"))?.parse()?,
				w: args.next().ok_or_else(|| eyre::eyre!("no w"))?.parse()?,
				h: args.next().ok_or_else(|| eyre::eyre!("no h"))?.parse()?,
				imgw: args.next().ok_or_else(|| eyre::eyre!("no imgw"))?.parse()?,
				imgh: args.next().ok_or_else(|| eyre::eyre!("no imgh"))?.parse()?,
			}),
			_ => eyre::bail!("invalid command"),
		}
	}
}

impl ImageEdit {
	pub fn perform(&self, src: RgbaImage) -> eyre::Result<RgbaImage> {
		match *self {
			ImageEdit::ReGrid { w1, h1, w2, h2 } => {
				let (w, h) = src.dimensions();
				eyre::ensure!(w % w1 == 0, "invalid w1");
				eyre::ensure!(h % h1 == 0, "invalid h1");
				eyre::ensure!(w1*h1 == w2*h2, "mismatch");
				let cw = w/w1;
				let ch = h/h1;
				let mut dst = RgbaImage::new(w / w1 * w2, h * w1 / w2);
				for i in 0..w1*h1 {
					let src = src.view(i%w1*cw, i/w1*ch, cw, ch);
					dst.copy_from(&*src, i%w2*cw, i/w2*ch).unwrap();
				}
				Ok(dst)
			}
			ImageEdit::Crop { x, y, w, h, imgw, imgh } => {
				let (imgw_, imgh_) = src.dimensions();
				eyre::ensure!(imgw_ % imgw == 0, "invalid imgw");
				eyre::ensure!(imgh_ % imgh == 0, "invalid imgh");
				eyre::ensure!(imgh_ % imgh == 0, "invalid imgh");
				// XXX this can panic
				Ok(src.view(x*(imgw_/imgw), y*(imgh_/imgh), w*(imgw_/imgw), h*(imgh_/imgh)).to_image())
			}
		}
	}
}
