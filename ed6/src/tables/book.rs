use std::collections::BTreeMap;
use hamu::read::coverage::Coverage;
use hamu::read::le::*;

use crate::archive::Archives;
use crate::util::*;
use super::item::ItemId;

// TODO I don't like that this one calls .get_decomp. I prefer for the parsers to be pure.
pub fn read(arc: &Archives, data: &[u8]) -> Result<BTreeMap<ItemId, NameDesc>, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));
	let mut table = BTreeMap::new();

	let mut chunks = BTreeMap::new();

	while f.remaining() > 12 {
		let id = f.u16()?.into();
		f.check_u16(0)?;
		let file = arc.name(f.array()?)?;
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
	use crate::archive::Archives;
	use crate::util::test::*;

	#[test_case::test_case(&FC; "fc")]
	fn parse(arc: &Archives) -> Result<(), Error> {
		let data = arc.get_decomp("t_book00._dt")?;
		let _parsed = super::read(arc, &data)?;
		Ok(())
	}
}
