use eyre::Result;
use image::RgbaImage;
use hamu::read::{In, Le};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
	Rgba4444,
}

pub fn read(data: &[u8], width: u32, height: u32, format: Format) -> Result<RgbaImage> {
	let mut i = In::new(data);
	let img = read_buf(&mut i, width, height, format)?;
	i.dump_uncovered(|a| a.to_stderr())?;
	Ok(img)
}

fn read_buf(i: &mut In, width: u32, height: u32, format: Format) -> Result<RgbaImage> {
	let mut img = RgbaImage::new(width, height);
	for y in 0..height {
		for x in 0..width {
			#[allow(clippy::identity_op)]
			let px = match format {
				Format::Rgba4444 => {
					let px = i.u16()?;
					[
						((px & 0x0F00) >> 8) as u8 * 0x11,
						((px & 0x00F0) >> 4) as u8 * 0x11,
						((px & 0x000F) >> 0) as u8 * 0x11,
						((px & 0xF000) >> 12) as u8 * 0x11,
					]
				}
			};
			img.put_pixel(x, y, image::Rgba(px));
		}
	}
	Ok(img)
}
