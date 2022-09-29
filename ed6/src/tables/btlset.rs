use std::collections::BTreeMap;
use std::rc::Rc;

use enumflags2::*;
use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use crate::archive::Archives;
use crate::tables::bgmtbl::BgmId;
use crate::util::*;

newtype!(BattleId, u16);

#[bitflags]
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleFlag {
	_01 = 0x01,
	_02 = 0x02,
	_04 = 0x04,
	_08 = 0x08,
	_10 = 0x10,
	_20 = 0x20,
	_40 = 0x40,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement([Position; 8]);

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Position(u8, u8, u16);

impl std::fmt::Debug for Position {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Position({}, {}, {})", self.0, self.1, self.2)
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Battlefield(u16, String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtRoll {
	pub unknown: u8,
	pub hp10: u8,
	pub hp50: u8,
	pub ep10: u8,
	pub ep50: u8,
	pub cp10: u8,
	pub cp50: u8,
	pub atk10: u8,
	pub atk50: u8,
	pub sepith: u8,
	pub crit: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Battle {
	pub flags: BitFlags<BattleFlag>,
	pub unk1: u16,
	pub unk2: u16,
	pub unk3: u16,
	pub unk4: u16,
	pub enemies: [Option<String>; 8],
	pub placement1: Rc<Placement>,
	pub placement2: Rc<Placement>,
	pub battlefield: Rc<Battlefield>,
	pub bgm: BgmId,
	pub at_roll: Rc<AtRoll>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoBattle {
	pub unk1: u16,
	pub battlefield: Rc<Battlefield>,
	pub side1: [Option<String>; 8],
	pub side2: [Option<String>; 8],
	pub bgm: BgmId,
}

fn read_rc<K: Ord + PartialEq, T>(
	cache: &mut BTreeMap<K, Rc<T>>,
	k: K,
	f: impl FnOnce(&K) -> Result<T, ReadError>,
) -> Result<Rc<T>, ReadError> {
	Ok(match cache.entry(k) {
		std::collections::btree_map::Entry::Vacant(e) => {
			let v = f(e.key())?;
			e.insert(Rc::new(v)).clone()
		}
		std::collections::btree_map::Entry::Occupied(e) => e.get().clone(),
	})
}

fn read_placement<'a>(
	f: &mut (impl In<'a> + Clone),
	placements: &mut BTreeMap<u16, Rc<Placement>>,
) -> Result<Rc<Placement>, ReadError> {
	read_rc(placements, f.u16()?, |&o| {
		let mut g = f.clone().at(o as usize)?;
		let p = array(|| Ok(Position(g.u8()?, g.u8()?, g.u16()?))).strict()?;
		Ok(Placement(p))
	})
}

fn read_battlefield<'a>(
	f: &mut (impl In<'a> + Clone),
	battlefields: &mut BTreeMap<u16, Rc<Battlefield>>,
) -> Result<Rc<Battlefield>, ReadError> {
	read_rc(battlefields, f.u16()?, |&o| {
		let mut g = f.clone().at(o as usize)?;
		let unk1 = g.u16()?;
		let l1 = g.u16()? as usize;
		ensure!(g.pos() == l1, "invalid battlefield");
		let battlefield = g.string()?;
		Ok(Battlefield(unk1, battlefield))
	})
}

fn read_battles<'a>(
	arc: &Archives,
	mut f: impl In<'a> + Clone,
) -> Result<BTreeMap<BattleId, Battle>, ReadError> {
	let mut placements = BTreeMap::new();
	let mut battlefields = BTreeMap::new();
	let mut at_rolls = BTreeMap::new();

	let fileref = |a| if a == [0; 4] { Ok(None) } else { arc.name(a).map(|a| Some(a.to_owned())) };

	let mut table = BTreeMap::new();

	loop {
		let mut g = f.ptr()?;

		let id = BattleId(g.u16()?);
		let flags: BitFlags<BattleFlag> = cast(g.u16()?)?;

		let unk1 = g.u16()?;
		let unk2 = g.u16()?;
		let unk3 = g.u16()?;
		let unk4 = g.u16()?;

		let enemies: [_; 8] = array(|| Ok(fileref(g.array()?)?)).strict()?;

		let placement1 = read_placement(&mut g, &mut placements)?;
		let placement2 = read_placement(&mut g, &mut placements)?;
		let battlefield = read_battlefield(&mut g, &mut battlefields)?;
		g.check_u16(0)?;

		let bgm: BgmId = BgmId(g.u16()?);
		g.check_u16(0)?;

		let at_roll = read_rc(&mut at_rolls, g.u16()?, |&o| {
			let mut g = f.clone().at(o as usize)?;
			let unknown = g.u8()?;
			let hp10 = g.u8()?;
			let hp50 = g.u8()?;
			let ep10 = g.u8()?;
			let ep50 = g.u8()?;
			let cp10 = g.u8()?;
			let cp50 = g.u8()?;
			let atk10 = g.u8()?;
			let atk50 = g.u8()?;
			g.check_u8(0)?;
			g.check_u8(0)?;
			let sepith = g.u8()?;
			let crit = g.u8()?;
			g.check_u8(0)?;
			g.check_u8(0)?;
			g.check_u8(0)?;
			Ok(AtRoll { unknown, hp10, hp50, ep10, ep50, cp10, cp50, atk10, atk50, sepith, crit })
		})?;
		g.check_u16(0)?;

		table.insert(id, Battle {
			flags,
			unk1,
			unk2,
			unk3,
			unk4,
			enemies,
			placement1,
			placement2,
			battlefield,
			bgm,
			at_roll,
		});

		if id == BattleId(0xFFFF) {
			break
		}
	}

	Ok(table)
}

fn read_auto_battles<'a>(
	arc: &Archives,
	mut f: impl In<'a> + Clone,
) -> Result<BTreeMap<BattleId, AutoBattle>, ReadError> {
	let mut battlefields = BTreeMap::new();

	let fileref = |a| if a == [0; 4] { Ok(None) } else { arc.name(a).map(|a| Some(a.to_owned())) };

	let mut table = BTreeMap::new();

	loop {
		let mut g = f.ptr()?;

		let id = BattleId(g.u16()?);

		let unk1 = g.u16()?;
		let battlefield = read_battlefield(&mut g, &mut battlefields)?;
		g.check_u16(0)?;

		let side1: [_; 8] = array(|| Ok(fileref(g.array()?)?)).strict()?;
		let side2: [_; 8] = array(|| Ok(fileref(g.array()?)?)).strict()?;

		let bgm: BgmId = BgmId(g.u16()?);
		g.check_u16(0)?;

		table.insert(id, AutoBattle {
			unk1,
			battlefield,
			side1,
			side2,
			bgm,
		});

		if id == BattleId(0xFFFF) {
			break
		}
	}

	Ok(table)
}

#[allow(clippy::type_complexity)]
pub fn read(arc: &Archives, data: &[u8]) -> Result<(BTreeMap<BattleId, Battle>, BTreeMap<BattleId, AutoBattle>), ReadError> {
	let mut f = Coverage::new(Bytes::new(data));

	let battles = read_battles(arc, f.ptr()?)?;
	let auto_battles = read_auto_battles(arc, f.ptr()?)?;

	// f.assert_covered()?; // Does not have full coverage
	Ok((battles, auto_battles))
}

// I'll skip the writing for now.

#[cfg(test)]
mod test {
	use crate::archive::Archives;
	use crate::util::test::*;

	#[test_case::test_case(&FC; "fc")]
	fn parse(arc: &Archives) -> Result<(), Error> {
		let data = arc.get_decomp("t_btlset._dt")?;
		let _parsed = super::read(arc, &data)?;
		Ok(())
	}
}
