use std::collections::BTreeMap;

use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::archive::Archives;
use crate::scena::Flag;
use crate::util::*;

newtype!(QuestId, u16);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quest {
	pub section: u16,
	pub index: u16,
	pub bp: u16,
	pub mira: u16,
	pub flags: [Flag; 3],
	pub name: String,
	pub desc: String,
	pub extra_desc: Option<String>,
	pub steps: [String; 16],
}

pub fn read(_arc: &Archives, data: &[u8]) -> Result<BTreeMap<QuestId, Quest>, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));
	let n = f.clone().u16()? / 2;
	let mut table = BTreeMap::new();

	for _ in 0..n {
		let mut g = f.ptr()?;

		let id = QuestId(g.u16()?);
		g.check_u16(0)?;

		let section = g.u16()?;
		let index = g.u16()?;
		let bp = g.u16()?;
		let mira = g.u16()?;
		let flags = array(|| Ok(Flag(g.u16()?))).strict()?;

		let namep = g.u16()? as usize;
		let descp = g.u16()? as usize;
		let stepp = array::<16, _>(|| Ok(g.u16()? as usize)).strict()?;
		ensure!(g.pos() == namep, "{} != {}", g.pos(), namep);
		let name = g.string()?;
		ensure!(g.pos() == descp, "{} != {}", g.pos(), descp);
		let desc = g.string()?;

		let extra_desc = (g.pos() != stepp[0]).then(|| g.string()).transpose()?;

		let steps = stepp.try_map(|p| {
			while g.pos() < p {
				g.check_u8(0)?;
			}
			ensure!(g.pos() == p, "{} != {}", g.pos(), p);
			g.string()
		})?;

		table.insert(id, Quest { section, index, bp, mira, flags, name, desc, extra_desc, steps });
	}

	f.assert_covered()?;
	Ok(table)
}

pub fn write(_arc: &Archives, table: &BTreeMap<QuestId, Quest>) -> Result<Vec<u8>, WriteError> {
	let mut f = OutBytes::new();
	let mut g = OutBytes::new();
	let mut count = Count::new();

	for (&id, &Quest { section, index, bp, mira, flags, ref name, ref desc, ref extra_desc, ref steps }) in table {
		let l = count.next();
		f.delay_u16(l);
		g.label(l);

		g.u16(id.0);
		g.u16(0);
		g.u16(section);
		g.u16(index);
		g.u16(bp);
		g.u16(mira);
		g.u16(flags[0].0);
		g.u16(flags[1].0);
		g.u16(flags[2].0);

		let mut h = OutBytes::new();

		let l = count.next();
		g.delay_u16(l);
		h.label(l);
		h.string(name)?;

		let l = count.next();
		g.delay_u16(l);
		h.label(l);
		h.string(desc)?;
		if let Some(extra_desc) = extra_desc {
			h.string(extra_desc)?;
		}

		for step in steps {
			let l = count.next();
			g.delay_u16(l);
			h.label(l);
			h.string(step)?;
		}

		g = g.concat(h);
	}

	Ok(f.concat(g).finish()?)
}

#[cfg(test)]
mod test {
	use crate::archive::Archives;
	use crate::util::test::*;

	#[test_case::test_case(&FC; "fc")]
	fn roundtrip(arc: &Archives) -> Result<(), Error> {
		check_roundtrip(arc, "t_quest._dt", super::read, super::write)?;
		Ok(())
	}
}

