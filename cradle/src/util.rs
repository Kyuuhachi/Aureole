use image::{ImageBuffer, GenericImageView, GenericImage};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("{error}")]
	Invalid { error: String, backtrace: std::backtrace::Backtrace },
	#[error("{source}")]
	Decompress{ #[from] source: decompress::Error, backtrace: std::backtrace::Backtrace },
	#[error("{source}")]
	Read { #[from] source: gospel::read::Error, backtrace: std::backtrace::Backtrace },
	#[error("{source}")]
	Write { #[from] source: gospel::write::Error, backtrace: std::backtrace::Backtrace },
}

pub macro ensure {
	($cond:expr, $($t:tt)*) => {
		if !($cond) {
			bail!($($t)*)
		}
	},
	($cond:expr) => {
		ensure!($cond, stringify!($cond).into())
	}
}

pub macro bail {
	($str:literal $($arg:tt)*) => {
		bail!(format!($str $($arg)*).into())
	},
	($e:expr) => {
		Err(Error::Invalid { error: $e, backtrace: std::backtrace::Backtrace::capture() })?
	}
}

pub fn image<P: image::Pixel>(w: usize, h: usize, pixels: Vec<P::Subpixel>) -> Result<ImageBuffer<P, Vec<P::Subpixel>>, Error> {
	let Some(img) = ImageBuffer::from_vec(w as u32, h as u32, pixels) else {
		bail!("wrong number of pixels")
	};
	Ok(img)
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
