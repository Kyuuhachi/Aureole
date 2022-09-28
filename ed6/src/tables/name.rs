use std::collections::BTreeMap;

use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::archive::Archives;
use crate::util::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[derive(derive_more::From, derive_more::Into)]
pub struct NameId(u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
	pub ch1: String,
	pub ch2: String,
	pub cp1: String,
	pub cp2: String,
	pub ms1: Option<String>,
	pub ms2: Option<String>,
	pub name: String,
}

pub fn read(arc: &Archives, data: &[u8]) -> Result<BTreeMap<NameId, Name>, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));
	let n = f.clone().u16()? / 2;
	let mut table = BTreeMap::new();
	let fileref = |a| if a == [0; 4] { Ok(None) } else { arc.name(a).map(|a| Some(a.to_owned())) };

	for _ in 0..n-1 {
		let mut g = f.ptr()?;
		let id = g.u32()?.into();
		let ch1 = arc.name(g.array()?)?.to_owned();
		let ch2 = arc.name(g.array()?)?.to_owned();
		let cp1 = arc.name(g.array()?)?.to_owned();
		let cp2 = arc.name(g.array()?)?.to_owned();
		let ms1 = fileref(g.array()?)?;
		let ms2 = fileref(g.array()?)?;
		let name = g.ptr()?.string()?;
		table.insert(id, Name { ch1, ch2, cp1, cp2, ms1, ms2, name });
	}

	let mut g = f.ptr()?;
	g.check_u32(999)?;
	g.check(&[0; 4*6])?;
	let name = g.ptr()?.string()?;
	ensure!(name == " ", "last name should be blank, was {name:?}");

	f.assert_covered()?;
	Ok(table)
}

pub fn write(arc: &Archives, table: &BTreeMap<NameId, Name>) -> Result<Vec<u8>, WriteError> {
	let mut f = Out::new();
	let mut g = Out::new();
	let mut count = Count::new();
	let fileref = |a| Option::map_or(a, Ok([0; 4]), |a| arc.index(a));
	for (&id, Name { ch1, ch2, cp1, cp2, ms1, ms2, name }) in table {
		let l = count.next();
		f.delay_u16(l);
		g.label(l);
		g.u32(id.into());
		g.array(arc.index(ch1)?);
		g.array(arc.index(ch2)?);
		g.array(arc.index(cp1)?);
		g.array(arc.index(cp2)?);
		g.array(fileref(ms1.as_deref())?);
		g.array(fileref(ms2.as_deref())?);
		let l = count.next();
		g.delay_u16(l);
		g.label(l);
		g.string(name)?;
	}

	let l = count.next();
	f.delay_u16(l);
	g.label(l);
	g.u32(999);
	g.array([0; 6*4]);
	let l = count.next();
	g.delay_u16(l);
	g.label(l);
	g.string(" ")?;

	Ok(f.concat(g).finish()?)
}

#[cfg(test)]
mod test {
	use crate::archive::Archives;
	use crate::util::test::*;

	#[test_case::test_case(&FC; "fc")]
	fn roundtrip(arc: &Archives) -> Result<(), Error> {
		check_roundtrip(arc, "t_name._dt", super::read, super::write)?;
		Ok(())
	}
}
