use std::collections::BTreeMap;

use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::util::*;

newtype!(BgmId, u16);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bgm {
	pub name: String,
	pub loops: bool,
}

// I'm throwing away the record order in the file here, hope that doesn't matter.
pub fn read(t_town: &[u8]) -> Result<BTreeMap<BgmId, Bgm>, ReadError> {
	let mut f = Coverage::new(Bytes::new(t_town));
	let mut table = BTreeMap::new();
	while f.remaining() > 0 {
		let id = BgmId(f.u16()?);
		f.check_u16(0)?;
		let name = f.sized_string::<8>()?;
		let loops = cast_bool(f.u32()?)?;
		table.insert(id, Bgm { name, loops });
	}

	f.assert_covered()?;
	Ok(table)
}

pub fn write(table: &BTreeMap<BgmId, Bgm>) -> Result<Vec<u8>, WriteError> {
	let mut f = OutBytes::new();
	for (&id, &Bgm { ref name, loops }) in table {
		f.u16(id.0);
		f.u16(0);
		f.sized_string::<8>(name)?;
		f.u32(loops.into());
	}
	Ok(f.finish()?)
}

#[cfg(test)]
mod test {
	use crate::util::test::*;

	#[test_case::test_case(&FC; "fc")]
	fn roundtrip(arc: &crate::archive::Archives) -> Result<(), Error> {
		check_roundtrip(&arc.get_decomp("t_bgmtbl._dt").unwrap(), super::read, super::write)?;
		Ok(())
	}
}
