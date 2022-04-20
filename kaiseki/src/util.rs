use anyhow::Result;
use encoding_rs::SHIFT_JIS;
use itermore::Itermore;
use hamu::read::{In, Le};

#[extend::ext]
pub impl In<'_> where Self: Sized {
	fn str(&mut self) -> Result<String> {
		let mut s = Vec::new();
		loop {
			match self.u8()? {
				0 => break,
				b => s.push(b),
			}
		}
		let (out, _, error) = SHIFT_JIS.decode(&s);
		anyhow::ensure!(!error, "Invalid string: {:?}", out);
		Ok(out.into_owned())
	}
}

pub fn toc<'a>(i: &mut In<'a>) -> Result<impl Iterator<Item=(In<'a>, usize)>> {
	assert_eq!(i.pos(), 0);
	let mut i = i.clone();
	let start = i.clone().u16()? as usize;
	let mut v = Vec::with_capacity(start/2+1);
	for _ in 0..start/2 {
		let p = i.u16()? as usize;
		i.clone().seek(p)?;
		v.push(p);
	}
	v.push(i.len());
	Ok(
		v.into_iter()
		.array_windows()
		.map(move |[a, b]| (i.clone().at(a).unwrap(), b-a))
	)
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ByteString<const N: usize>(pub [u8; N]);

impl<const N: usize> std::fmt::Debug for ByteString<N> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "b\"{}\"",
			self.0.into_iter()
				.flat_map(std::ascii::escape_default)
				.map(|a| a as char)
				.collect::<String>()
		)
	}
}

impl<const N: usize> std::ops::Deref for ByteString<N> {
	type Target = [u8; N];

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<const N: usize> std::ops::DerefMut for ByteString<N> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<const N: usize> PartialEq<&ByteString<N>> for ByteString<N> {
	fn eq(&self, other: &&ByteString<N>) -> bool {
		self.0 == other.0
	}
}

impl<const N: usize> PartialEq<[u8;N]> for ByteString<N> {
	fn eq(&self, other: &[u8;N]) -> bool {
		&self.0 == other
	}
}

impl<const N: usize> PartialEq<&[u8]> for ByteString<N> {
	fn eq(&self, other: &&[u8]) -> bool {
		&&self.0 == other
	}
}

impl<const N: usize> AsRef<[u8;N]> for ByteString<N> {
	fn as_ref(&self) -> &[u8;N] {
		&self.0
	}
}

impl<const N: usize> AsRef<ByteString<N>> for [u8;N] {
	fn as_ref(&self) -> &ByteString<N> {
		// SAFETY: it's repr(transparent)
		unsafe { std::mem::transmute::<&[u8;N], &ByteString<N>>(self) }
	}
}
