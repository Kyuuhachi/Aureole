use image::RgbaImage;
use hamu::read::{In, Le, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
	Rgba4444,
	Rgba1555,
}

pub fn read(data: &[u8], width: u32, height: u32, format: Format) -> Result<RgbaImage> {
	let mut i = In::new(data);
	let img = read_buf(&mut i, width, height, format)?;
	i.dump_uncovered(|a| a.to_stderr())?;
	Ok(img)
}

#[inline]
fn part(x: u16, mask: u16, mul: u32, shift: u32) -> u8 {
	(((x&mask) as u32 * mul) >> shift) as u8
}

#[tracing::instrument(skip(i))]
fn read_buf(i: &mut In, width: u32, height: u32, format: Format) -> Result<RgbaImage> {
	let mut img = RgbaImage::new(width, height);
	for y in 0..height {
		for x in 0..width {
			let px = match format {
				Format::Rgba4444 => {
					let px = i.u16()?;
					[
						part(px, 0x0F00, 0x11, 8),
						part(px, 0x00F0, 0x11, 4),
						part(px, 0x000F, 0x11, 0),
						part(px, 0xF000, 0x11, 12),
					]
				}
				Format::Rgba1555 => {
					let px = i.u16()?;
					[
						part(px, 0x7C00, 0x21, 12),
						part(px, 0x03E0, 0x21, 7),
						part(px, 0x001F, 0x21, 2),
						part(px, 0x8000, 0xFF, 15),
					]
				}
			};
			img.put_pixel(x, y, image::Rgba(px));
		}
	}
	Ok(img)
}
