use hamu::read::le::*;
use hamu::write::le::*;
use image::{ImageBuffer, GenericImageView, GenericImage};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("{0}")]
	Invalid(String),
	#[error(transparent)]
	Decompress{ #[from] source: decompress::Error },
	#[error("{source}")]
	Read { #[from] source: hamu::read::Error, backtrace: std::backtrace::Backtrace },
	#[error("{source}")]
	Write { #[from] source: hamu::write::Error, backtrace: std::backtrace::Backtrace },
}

pub macro ensure($cond:expr, $($t:tt)*) {
	if !($cond) {
		bail!($($t)*)
	}
}

pub macro bail($str:literal $($arg:tt)*) {
	Err(Error::Invalid(format!($str $($arg)*)))?
}

pub fn image<P: image::Pixel>(w: usize, h: usize, pixels: Vec<P::Subpixel>) -> Result<ImageBuffer<P, Vec<P::Subpixel>>, Error> {
	ImageBuffer::from_vec(w as u32, h as u32, pixels)
		.ok_or(Error::Invalid("wrong number of pixels".to_owned()))
}

pub fn tile<I>(
	frames: &[I],
	width: u32,
) -> ImageBuffer<I::Pixel, Vec<<I::Pixel as image::Pixel>::Subpixel>> where
	I: GenericImageView
{
	assert_ne!(width, 0);
	if frames.is_empty() {
		return ImageBuffer::new(0, 0)
	}
	let height = (frames.len() as u32 + width-1) / width;
	let (w, h) = frames[0].dimensions();
	let mut img = ImageBuffer::new(width * w, height * h);
	for (i, f) in frames.iter().enumerate() {
		assert_eq!(f.dimensions(), (w, h));
		img.copy_from(f, i as u32 % width * w, i as u32 / width * h).unwrap();
	}
	img
}

#[inline(always)]
pub fn swizzle<A>(data: &mut [A], residual: &mut [A], w: usize, cw: usize, ch: usize) {
	assert_eq!(data.len(), residual.len());
	for (ci, r) in residual.chunks_mut(cw*ch).enumerate() {
		let cx = ci % (w / cw) * cw;
		let cy = ci / (w / cw) * ch;
		for (y, r) in r.chunks_mut(cw).enumerate() {
			for (x, r) in r.iter_mut().enumerate() {
				let x = cx+x;
				let y = cy+y;
				std::mem::swap(&mut data[y*w+x], r);
			}
		}
	}
}

pub fn decompress(f: &mut Reader) -> Result<Vec<u8>, Error> {
	let csize = f.u32()? as usize;
	let start = f.pos();
	let usize = f.u32()? as usize;
	let mut out = Vec::with_capacity(usize);
	for _ in 1..f.u32()? {
		let Some(chunklen) = (f.u16()? as usize).checked_sub(2) else {
			return Err(Error::Invalid("bad chunk length".to_owned()))
		};
		decompress::decompress(f.slice(chunklen)?, &mut out)?;
		f.check_u8(1)?;
	}

	f.check_u32(0x06000006)?;
	f.slice(3)?; // unknown

	if f.pos()-start != csize {
		return Err(Error::Invalid(format!("wrong compressed length: expected {}, got {}", csize, f.pos()-start)))
	}

	if out.len() != usize {
		return Err(Error::Invalid(format!("wrong uncompressed length: expected {}, got {}", usize, out.len())))
	}

	Ok(out)
}

pub fn compress(f: &mut Writer, data: &[u8]) {
	let (csize_r, csize_w) = Label::new();
	f.delay(|l| Ok(u32::to_le_bytes(hamu::write::cast_usize::<u32>(l(csize_r)?)? - 4)));
	f.u32(data.len() as u32);
	f.u32(1+data.chunks(0xFFF0).count() as u32);
	for chunk in data.chunks(0xFFF0) {
		let data = decompress::compress(chunk);
		f.u16(data.len() as u16 + 2);
		f.slice(&data);
		f.u8(1);
	}
	f.u32(0x06000006);
	f.slice(&[0,0,0]);
	f.label(csize_w);
}
