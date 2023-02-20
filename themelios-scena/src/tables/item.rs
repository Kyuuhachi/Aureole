use std::collections::BTreeMap;

use enumflags2::*;
use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::util::*;
use crate::types::ItemId;

#[bitflags]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemFlag {
	Battle      = 0x01,
	Use         = 0x02, // on battle items, whether they can target allies. Otherwise, equal to Book
	Sell        = 0x04,
	Discard     = 0x08,
	TargetEnemy = 0x10,
	TargetDead  = 0x20,
	Book        = 0x40, // never used together with Battle
	_80         = 0x80,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
	pub name_desc: NameDesc,
	pub flags: BitFlags<ItemFlag>,
	pub usable_by: u8,
	pub ty: [u8; 4],
	pub _unk1: u8, // 0 on quartz and key items, 2 on cannons, 1 otherwise
	pub stats: [i16; 10],
	pub limit: u16,
	pub price: u32,
}

pub fn read(t_item: &[u8], t_item2: &[u8]) -> Result<BTreeMap<ItemId, Item>, ReadError> {
	let mut f1 = Coverage::new(Reader::new(t_item));
	let mut f2 = Coverage::new(Reader::new(t_item2));
	let n = f1.clone().u16()? / 2;
	let n2 = f2.clone().u16()? / 2;
	ensure!(n == n2, "mismatched item/item2");

	let mut table = BTreeMap::new();

	for _ in 0..n {
		let mut g1 = f1.ptr()?;
		let mut g2 = f2.ptr()?;

		let id = ItemId(g1.u16()?);
		let flags = cast(g1.u8()?)?;
		let usable_by = g1.u8()?; // TODO Flags in FC, enum in others
		let ty = [ g1.u8()?, g1.u8()?, g1.u8()?, g1.u8()? ];
		g1.check_u8(0)?;
		let _unk1 = g1.u8()?;
		let stats = [
			g1.i16()?, g1.i16()?, g1.i16()?, g1.i16()?, g1.i16()?,
			g1.i16()?, g1.i16()?, g1.i16()?, g1.i16()?, g1.i16()?,
		];
		let limit = g1.u16()?;
		let price = g1.u32()?;

		let name_desc = g2.name_desc()?;

		table.insert(id, Item { name_desc, flags, usable_by, ty, _unk1, stats, limit, price });
	}

	f1.assert_covered()?;
	f2.assert_covered()?;
	Ok(table)
}

pub fn write(table: &BTreeMap<ItemId, Item>) -> Result<(Vec<u8>, Vec<u8>), WriteError> {
	let mut f1 = Writer::new();
	let mut g1 = Writer::new();
	let mut f2 = Writer::new();
	let mut g2 = Writer::new();

	for (&id, &Item { ref name_desc, flags, usable_by, ty, _unk1, stats, limit, price }) in table {
		f1.delay_u16(g1.here());
		f2.delay_u16(g2.here());

		g1.u16(id.0);
		g1.u8(flags.bits());
		g1.u8(usable_by);
		g1.array(ty);
		g1.u8(0);
		g1.u8(_unk1);
		for s in stats { g1.i16(s) }
		g1.u16(limit);
		g1.u32(price);

		g2.name_desc(name_desc)?;
	}
	f1.append(g1);
	f2.append(g2);
	Ok((f1.finish()?, f2.finish()?))
}
