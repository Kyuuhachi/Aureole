use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::archive::Archives;
use crate::util::*;
use super::Element;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orbment {
	pub slots: Vec<Option<Element>>, // 6 in FC, 7 in SC/3rd
	pub lines: Vec<Vec<u8>>,
}

pub fn read(_arcs: &Archives, data: &[u8]) -> Result<Vec<Orbment>, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));
	let n = f.clone().u16()? / 2;
	let mut list = Vec::with_capacity(n as usize);

	let nslots = 6; // 7 in sc/3rd
	let npad = 1; // 2 in sc/3rd

	for _ in 0..n {
		let mut g = f.clone().at(f.u16()? as usize)?;

		let mut slots = Vec::with_capacity(nslots);
		for _ in 0..nslots {
			slots.push(Element::from_u8_opt(g.u8()?)?);
		}
		g.check(&[0;2][..npad])?;

		let nlines = g.u8()?;
		let mut lines = Vec::with_capacity(nlines as usize);
		for _ in 0..nlines {
			lines.push(g.multiple::<8, _>(&[0xFF], |a| Ok(a.u8()?))?);
		}
		g.check(&[0xFF; 2])?;

		list.push(Orbment { slots, lines });
	}

	f.assert_covered()?;
	Ok(list)
}

pub fn write(_arcs: &Archives, list: &Vec<Orbment>) -> Result<Vec<u8>, WriteError> {
	let mut head = Out::new();
	let mut body = Out::new();
	let mut count = Count::new();

	let nslots = 6; // 7 in sc/3rd
	let npad = 1; // 2 in sc/3rd

	for Orbment { slots, lines } in list {
		let l = count.next();
		head.delay_u16(l);
		body.label(l);

		if slots.len() != nslots {
			return Err(format!("must be {nslots}").into())
		}
		for s in slots {
			body.u8(Element::to_u8_opt(*s));
		}
		body.slice(&[0;2][..npad]);

		body.u8(cast(lines.len())?);
		for line in lines {
			if line.len() > 8 {
				return Err("line cannot be longer than 8".to_owned().into())
			}
			let mut buf = [0xFF; 8];
			buf[..line.len()].copy_from_slice(line);
			body.array(buf);
		}
		body.array([0xFF; 2]);
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
		check_roundtrip(arc, "t_orb._dt", super::read, super::write)?;
		Ok(())
	}
}
