use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::archive::Archives;
use crate::util::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
	pub level: u16,
	pub hp: u32,
	pub atk: u16,
	pub def: u16,
	pub ats: u16,
	pub adf: u16,
	pub dex: u16,
	pub agl: u16,
	pub mov: u16,
	pub spd: u16,
}

pub fn read(_arcs: &Archives, data: &[u8]) -> Result<Vec<Vec<Status>>, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));
	let n = f.clone().u16()? / 2;
	let m = (f.clone().at(2)?.u16()? - f.clone().u16()?)/22;
	let mut list = Vec::with_capacity(n as usize);
	for _ in 0..n {
		let mut char = Vec::new();
		let pos = f.u16()? as usize;
		let mut g = f.clone().at(pos)?;
		for _ in 0..m {
			char.push(Status {
				level: g.u16()?,
				hp: g.u32()?,
				atk: g.u16()?,
				def: g.u16()?,
				ats: g.u16()?,
				adf: g.u16()?,
				dex: g.u16()?,
				agl: g.u16()?,
				mov: g.u16()?,
				spd: g.u16()?,
			});
		}
		list.push(char);
	}
	f.assert_covered()?;
	Ok(list)
}

pub fn write(_arcs: &Archives, list: &[Vec<Status>]) -> Result<Vec<u8>, WriteError> {
	let mut head = Out::new();
	let mut body = Out::new();
	let mut count = Count::new();
	for char in list {
		let l = count.next();
		head.delay_u16(l);
		body.label(l);
		for status in char {
			body.u16(status.level);
			body.u32(status.hp);
			body.u16(status.atk);
			body.u16(status.def);
			body.u16(status.ats);
			body.u16(status.adf);
			body.u16(status.dex);
			body.u16(status.agl);
			body.u16(status.mov);
			body.u16(status.spd);
		}
	}
	head.concat(body);
	Ok(head.finish()?)
}

#[cfg(test)]
mod test {
	use crate::archive::Archives;
	use crate::util::test::*;

	#[test_case::test_case(&FC; "fc")]
	fn roundtrip(arc: &Archives) -> Result<(), Error> {
		check_roundtrip(arc, "t_status._dt", super::read, super::write)?;
		Ok(())
	}
}
