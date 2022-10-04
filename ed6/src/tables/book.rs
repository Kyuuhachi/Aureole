use std::collections::BTreeMap;
use hamu::read::coverage::Coverage;
use hamu::read::le::*;

use crate::gamedata::GameData;
use crate::util::*;
use super::item::ItemId;

// TODO I don't like that this one calls .get_decomp. I prefer for the parsers to be pure.
pub fn read(arc: &GameData, data: &[u8]) -> Result<BTreeMap<ItemId, NameDesc>, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));
	let mut table = BTreeMap::new();

	let mut chunks = BTreeMap::new();

	while f.remaining() > 12 {
		let id = ItemId(f.u16()?);
		f.check_u16(0)?;
		let file = arc.name(f.u32()?)?;
		let index = f.u16()?;
		f.check_u16(0)?;

		if !chunks.contains_key(file) {
			chunks.insert(file, arc.get_decomp(file)?);
		}
		let chunkdata = chunks.get(file).unwrap();

		let mut h = Bytes::new(chunkdata).at((index as usize) * 4)?;
		let name = h.ptr()?.string()?;
		let desc = h.ptr()?.string()?;

		table.insert(id, NameDesc { name, desc });
	}

	f.check(&[0xFF; 12])?;

	f.assert_covered()?;
	Ok(table)
}

// TODO no write

#[cfg(test)]
mod test {
	use crate::gamedata::GameData;
	use crate::util::test::*;

	#[test_case::test_case(&FC; "fc")]
	fn parse(arc: &GameData) -> Result<(), Error> {
		let data = arc.get_decomp("t_book00._dt")?;
		let _parsed = super::read(arc, &data)?;
		Ok(())
	}
}
