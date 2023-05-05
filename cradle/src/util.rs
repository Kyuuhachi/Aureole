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
pub fn swizzle<A: Clone, const N: usize>(data: &[A], out: &mut [A], shape: [usize; N], order: [usize; N]) {
	assert_eq!(out.len(), data.len());
	assert_eq!(shape.iter().copied().product::<usize>(), data.len());
	let stride = order.map(|i| shape[i+1..].iter().copied().product::<usize>());
	for (i, p) in iter_shape(shape, order).enumerate() {
		let mut n = 0;
		for j in 0..N {
			n += stride[j] * p[order[j]];
		}
		out[i] = data[n].clone();
	}
}

#[inline(always)]
pub fn iter_shape<const N: usize>(shape: [usize; N], order: [usize; N]) -> impl Iterator<Item=[usize; N]> {
	let mut n = shape.map(|_| 0);
	let mut done = false;
	std::iter::from_fn(move || {
		if done { return None }
		let v = n;
		for k in order.into_iter().rev() {
			n[k] += 1;
			if n[k] == shape[k] {
				n[k] = 0;
			} else {
				return Some(v);
			}
		}
		done = true;
		Some(v)
	})
}

#[test]
fn test_iter_shape() {
	assert_eq!(iter_shape([2,3,2],[0,1,2]).collect::<Vec<_>>(), vec![
		[0,0,0], [0,0,1], [0,1,0], [0,1,1], [0,2,0], [0,2,1],
		[1,0,0], [1,0,1], [1,1,0], [1,1,1], [1,2,0], [1,2,1],
	]);

	assert_eq!(iter_shape([2,3,2],[0,2,1]).collect::<Vec<_>>(), vec![
		[0,0,0], [0,1,0], [0,2,0], [0,0,1], [0,1,1], [0,2,1],
		[1,0,0], [1,1,0], [1,2,0], [1,0,1], [1,1,1], [1,2,1]
	]);
}
