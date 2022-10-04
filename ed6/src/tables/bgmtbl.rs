use std::collections::BTreeMap;

use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::archive::Archives;
use crate::util::*;

newtype!(BgmId, u8);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bgm {
	pub name: String,
	pub loops: bool,
}

// I'm throwing away the record order in the file here, hope that doesn't matter.
pub fn read(_arcs: &Archives, t_town: &[u8]) -> Result<BTreeMap<BgmId, Bgm>, ReadError> {
	let mut f = Coverage::new(Bytes::new(t_town));
	let mut table = BTreeMap::new();
	while f.remaining() > 16 {
		let id = BgmId(f.u8()?);
		f.check(&[0; 3])?;
		let name = f.sized_string::<8>()?;
		let loops = cast_bool(f.u32()?)?;
		table.insert(id, Bgm { name, loops });
	}

	f.check_u32(0)?;
	f.check(b"ED6999\0\0")?;
	f.check_u32(0)?;

	f.assert_covered()?;
	Ok(table)
}

pub fn write(_arcs: &Archives, table: &BTreeMap<BgmId, Bgm>) -> Result<Vec<u8>, WriteError> {
	let mut f = OutBytes::new();
	for (&id, &Bgm { ref name, loops }) in table {
		f.u8(id.0);
		f.slice(&[0; 3]);
		f.sized_string::<8>(name)?;
		f.u32(loops.into());
	}
	f.u32(0);
	f.slice(b"ED6999\0\0");
	f.u32(0);
	Ok(f.finish()?)
}

#[cfg(test)]
mod test {
	use crate::archive::Archives;
	use crate::util::test::*;

	#[test_case::test_case(&FC; "fc")]
	fn roundtrip(arc: &Archives) -> Result<(), Error> {
		check_roundtrip(arc, "t_bgmtbl._dt", super::read, super::write)?;
		Ok(())
	}
}
