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
