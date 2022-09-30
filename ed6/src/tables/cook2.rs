use std::collections::BTreeMap;

use enumflags2::*;
use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;

use crate::archive::Archives;
use crate::util::*;
use super::item::ItemId;

newtype!(RecipeId, u16);

#[bitflags]
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeFlag {
	ToGo   = 0x01,
	Revive = 0x02,
	Doom   = 0x04,
	Cp     = 0x08,
	Dummy  = 0x80,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipe {
	pub name_desc: NameDesc,
	pub ingredients: Vec<(ItemId, u16)>,
	pub flags: BitFlags<RecipeFlag>,
	pub result: ItemId,
	pub heal: u16,
}

pub fn read(_arcs: &Archives, data: &[u8]) -> Result<BTreeMap<RecipeId, Recipe>, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));
	let n = f.clone().u16()? / 2;
	let mut table = BTreeMap::new();

	for _ in 0..n {
		let mut g = f.ptr()?;

		let id = RecipeId(g.u16()?);
		let ingredients = g.multiple::<8, _>(&[0;4], |g| Ok((ItemId(g.u16()?), g.u16()?)))?;
		let flags = cast(g.u16()?)?;
		let result = ItemId(g.u16()?);
		g.check_u16(0)?;
		let heal = g.u16()?;
		let name_desc = g.name_desc()?;

		table.insert(id, Recipe { name_desc, ingredients, flags, result, heal });
	}

	f.assert_covered()?;
	Ok(table)
}

pub fn write(_arcs: &Archives, table: &BTreeMap<RecipeId, Recipe>) -> Result<Vec<u8>, WriteError> {
	let mut f = OutBytes::new();
	let mut g = OutBytes::new();
	let mut count = Count::new();

	for (&id, &Recipe { ref name_desc, ref ingredients, flags, result, heal }) in table {
		let l = count.next();
		f.delay_u16(l);
		g.label(l);

		g.u16(id.0);
		g.multiple::<8, _>(&[0;4], ingredients, |g, &i| { g.u16(i.0.0); g.u16(i.1); Ok(()) })?;
		g.u16(flags.bits());
		g.u16(result.0);
		g.u16(0);
		g.u16(heal);
		g.name_desc(count.next(), count.next(), name_desc)?;
	}
	Ok(f.concat(g).finish()?)
}

#[cfg(test)]
mod test {
	use crate::archive::Archives;
	use crate::util::test::*;

	#[test_case::test_case(&FC; "fc")]
	fn roundtrip(arc: &Archives) -> Result<(), Error> {
		check_roundtrip_strict(arc, "t_cook2._dt", super::read, super::write)?;
		Ok(())
	}
}
