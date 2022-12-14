use std::collections::BTreeMap;

use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::gamedata::Lookup;
use crate::util::*;

newtype!(NameId, u16);

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

pub fn read(lookup: &dyn Lookup, data: &[u8]) -> Result<BTreeMap<NameId, Name>, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));
	let n = f.clone().u16()? / 2;
	let mut table = BTreeMap::new();
	let fileref = |a| if a == 0 { Ok(None) } else { lookup.name(a).map(Some) };

	for _ in 0..n-1 {
		let mut g = f.ptr()?;
		let id = NameId(g.u16()?);
		g.check_u16(0)?;
		let ch1 = lookup.name(g.u32()?)?.to_owned();
		let ch2 = lookup.name(g.u32()?)?.to_owned();
		let cp1 = lookup.name(g.u32()?)?.to_owned();
		let cp2 = lookup.name(g.u32()?)?.to_owned();
		let ms1 = fileref(g.u32()?)?;
		let ms2 = fileref(g.u32()?)?;
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

pub fn write(lookup: &dyn Lookup, table: &BTreeMap<NameId, Name>) -> Result<Vec<u8>, WriteError> {
	let mut f = OutBytes::new();
	let mut g = OutBytes::new();
	let fileref = |a| Option::map_or(a, Ok(0), |a| lookup.index(a));
	for (&id, Name { ch1, ch2, cp1, cp2, ms1, ms2, name }) in table {
		f.delay_u16(g.here());
		g.u16(id.0);
		g.u16(0);
		g.u32(lookup.index(ch1)?);
		g.u32(lookup.index(ch2)?);
		g.u32(lookup.index(cp1)?);
		g.u32(lookup.index(cp2)?);
		g.u32(fileref(ms1.as_deref())?);
		g.u32(fileref(ms2.as_deref())?);
		let (l, l_) = Label::new();
		g.delay_u16(l);
		g.label(l_);
		g.string(name)?;
	}

	f.delay_u16(g.here());
	g.u32(999);
	g.array([0; 6*4]);
	let (l, l_) = Label::new();
	g.delay_u16(l);
	g.label(l_);
	g.string(" ")?;

	Ok(f.concat(g).finish()?)
}
