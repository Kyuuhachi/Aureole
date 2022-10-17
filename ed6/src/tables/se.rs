use std::collections::BTreeMap;

use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::gamedata::Lookup;
use crate::util::*;

newtype!(SoundId, u16);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sound {
	pub unk: u16,
	pub file: String,
	pub flag1: bool,
	pub flag2: bool,
}

pub fn read(lookup: &dyn Lookup, data: &[u8]) -> Result<BTreeMap<SoundId, Sound>, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));
	let mut table = BTreeMap::new();
	while f.remaining() > 12 {
		let id = SoundId(f.u16()?);
		let unk = f.u16()?;
		let file = lookup.name(f.u32()?)?.to_owned();
		let flag1 = cast_bool(f.u16()?)?;
		let flag2 = cast_bool(f.u16()?)?;
		table.insert(id, Sound { unk, file, flag1, flag2 });
	}

	f.check_u16(0xFFFF)?;
	f.check_u16(0x0001)?;
	f.check_u32(0)?;
	f.check_u16(0)?;
	f.check_u16(0)?;

	f.assert_covered()?;
	Ok(table)
}

pub fn write(lookup: &dyn Lookup, table: &BTreeMap<SoundId, Sound>) -> Result<Vec<u8>, WriteError> {
	let mut out = OutBytes::new();
	for (&id, &Sound { unk, ref file, flag1, flag2 }) in table {
		out.u16(id.0);
		out.u16(unk);
		out.u32(lookup.index(file)?);
		out.u16(flag1.into());
		out.u16(flag2.into());
	}

	out.u16(0xFFFF);
	out.u16(0x0001);
	out.u32(0);
	out.u16(0);
	out.u16(0);

	Ok(out.finish()?)
}

#[cfg(test)]
mod test {
	crate::util::test::simple_roundtrip_arc!("t_se._dt");
}
