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
	pub id: NameId,
	pub ch1: String,
	pub ch2: String,
	pub cp1: String,
	pub cp2: String,
	pub ms1: Option<String>,
	pub ms2: Option<String>,
	pub name: String,
}

pub fn read(arc: &Archives, data: &[u8]) -> Result<Vec<Name>, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));
	let n = f.clone().u16()? / 2;
	let mut list = Vec::with_capacity(n as usize);
	let fileref = |a| if a == [0; 4] { Ok(None) } else { arc.name(a).map(|a| Some(a.to_owned())) };

	for _ in 0..n-1 {
		let mut g = f.clone().at(f.u16()? as usize)?;
		let id = g.u32()?.into();
		let ch1 = arc.name(g.array()?)?.to_owned();
		let ch2 = arc.name(g.array()?)?.to_owned();
		let cp1 = arc.name(g.array()?)?.to_owned();
		let cp2 = arc.name(g.array()?)?.to_owned();
		let ms1 = fileref(g.array()?)?;
		let ms2 = fileref(g.array()?)?;
		let name = g.clone().at(g.u16()? as usize)?.string()?;
		list.push(Name { id, ch1, ch2, cp1, cp2, ms1, ms2, name });
	}

	let mut g = f.clone().at(f.u16()? as usize)?;
	g.check_u32(999)?;
	g.check(&[0; 4*6])?;
	let name = g.clone().at(g.u16()? as usize)?.string()?;
	if name != " " {
		return Err(format!("last name should be blank, was {name:?}").into())
	}

	f.assert_covered()?;
	Ok(list)
}

pub fn write(arc: &Archives, list: &[Name]) -> Result<Vec<u8>, WriteError> {
	let mut head = Out::new();
	let mut body = Out::new();
	let mut count = Count::new();
	let fileref = |a| Option::map_or(a, Ok([0; 4]), |a| arc.index(a));
	for Name { id, ch1, ch2, cp1, cp2, ms1, ms2, name } in list {
		let l = count.next();
		head.delay_u16(l);
		body.label(l);
		body.u32((*id).into());
		body.array(arc.index(ch1)?);
		body.array(arc.index(ch2)?);
		body.array(arc.index(cp1)?);
		body.array(arc.index(cp2)?);
		body.array(fileref(ms1.as_deref())?);
		body.array(fileref(ms2.as_deref())?);
		let l = count.next();
		body.delay_u16(l);
		body.label(l);
		body.string(name)?;
	}

	let l = count.next();
	head.delay_u16(l);
	body.label(l);
	body.u32(999);
	body.array([0; 6*4]);
	let l = count.next();
	body.delay_u16(l);
	body.label(l);
	body.string(" ")?;

	head.concat(body);
	Ok(head.finish()?)
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
