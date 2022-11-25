use std::collections::HashMap;
use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::gamedata::Lookup;
use crate::tables::bgmtbl::BgmId;
use crate::tables::btlset::BattleId;
use crate::tables::town::TownId;
use crate::util::*;

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Label {
	pub name: String,
	pub pos: (f32, f32, f32),
	pub flags: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Npc {
	pub name: String,
	pub pos: Pos3,
	pub angle: i16,
	pub unk1: u16,
	pub unk2: u16,
	pub unk3: u16,
	pub unk4: u16,
	pub init: FuncRef,
	pub unk5: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monster {
	pub pos: Pos3,
	pub angle: i16,
	pub unk1: u16,
	pub battle: BattleId,
	pub flag: Flag,
	pub chcp: u16,
	pub unk2: u16,
	pub stand_anim: u32,
	pub walk_anim: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Trigger {
	pub pos: (f32, f32, f32),
	pub radius: f32,
	pub transform: [[f32; 4]; 4],
	pub unk1: u8,
	pub unk2: u16,
	pub function: FuncRef,
	pub unk3: u8,
	pub unk4: u16,
	pub unk5: u32,
	pub unk6: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookPoint {
	pub pos: Pos3,
	pub radius: u32,
	pub bubble_pos: Pos3,
	pub unk1: u8,
	pub unk2: u16,
	pub function: FuncRef,
	pub unk3: u8,
	pub unk4: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
	pub pos: Pos3,
	pub unk1: u32,

	pub cam_from: Pos3,
	pub cam_pers: u32,
	pub unk2: u16,
	pub cam_deg: u16,
	pub cam_limit1: u16,
	pub cam_limit2: u16,
	pub cam_at: Pos3,
	pub unk3: u16,
	pub unk4: u16,

	pub flags: u16,
	pub town: TownId,
	pub init: FuncRef,
	pub reinit: FuncRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Animation {
	pub speed: u16,
	pub unk: u8,
	pub frames: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Battle {
	flags: u16,
	level: u16,
	unk1: u8,
	vision_range: u8,
	move_range: u8,
	can_move: u8,
	move_speed: u16,
	unk2: u16,
	battlefield: String,
	sepith: Option<u16>, // index
	setups: Vec<BattleSetup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleSetup {
	weight: u8,
	enemies: [Option<String>; 8],
	placement: u16, // index
	placement_ambush: u16,
	bgm: BgmId,
	bgm_ambush: BgmId, // not entirely sure if this is what it is
	at_roll: u16, // index
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scena;

pub fn read(iset: code::InstructionSet, lookup: &dyn Lookup, data: &[u8]) -> Result<Scena, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));

	let name1 = f.sized_string::<10>()?;
	let name2 = f.sized_string::<10>()?;
	let town = TownId(f.u16()?);
	let bgm = BgmId(f.u16()?);
	let flags = f.u32()?;
	let includes = f.multiple_loose::<6, _>(&[0xFF;4], |g| Ok(lookup.name(g.u32()?)?))?;

	let strings_start = f.clone().u32()? as usize;
	let mut strings = f.ptr32()?;
	let filename = strings.string()?;

	let chcp     = f.ptr()?;
	let npcs     = f.ptr()?;
	let monsters = f.ptr()?;
	let triggers = f.ptr()?;
	let look_points = f.ptr()?;

	let func_table = f.ptr()?;
	let func_count = f.u16()? / 4;
	let anims = f.ptr()?;

	let labels = f.ptr()?;
	let n_labels = f.u16()?;

	let (mut g, n) = (chcp, f.u8()? as usize);
	let chcp = list(n, || Ok(match g.u32()? {
		0 => None,
		n => Some(lookup.name(n)?)
	})).strict()?;

	let (mut g, n) = (npcs, f.u8()? as usize);
	let npcs = list(n, || Ok(Npc {
		name: strings.string()?,
		pos: g.pos3()?,
		angle: g.i16()?,
		unk1: g.u16()?,
		unk2: g.u16()?,
		unk3: g.u16()?,
		unk4: g.u16()?,
		init: FuncRef(g.u8()? as u16, g.u8()? as u16),
		unk5: g.u32()?,
	})).strict()?;

	let (mut g, n) = (monsters, f.u8()? as usize);
	let monsters = list(n, || Ok(Monster {
		pos: g.pos3()?,
		angle: g.i16()?,
		unk1: g.u16()?,
		battle: BattleId(cast(g.u16()?)?),
		flag: Flag(g.u16()?),
		chcp: g.u16()?,
		unk2: g.u16()?,
		stand_anim: g.u32()?,
		walk_anim: g.u32()?,
	})).strict()?;

	let battle_start = g.pos();
	let battle_end = anims.pos();

	let (mut g, n) = (triggers, f.u8()? as usize);
	let triggers = list(n, || Ok(Trigger {
		pos: (g.f32()?, g.f32()?, g.f32()?),
		radius: g.f32()?,
		transform: array(|| array(|| Ok(g.f32()?))).strict()?,
		unk1: g.u8()?,
		unk2: g.u16()?,
		function: FuncRef(g.u8()? as u16, g.u8()? as u16),
		unk3: g.u8()?,
		unk4: g.u16()?,
		unk5: g.u32()?,
		unk6: g.u32()?,
	})).strict()?;

	let (mut g, n) = (look_points, f.u8()? as usize);
	let look_points = list(n, || Ok(LookPoint {
		pos: g.pos3()?,
		radius: g.u32()?,
		bubble_pos: g.pos3()?,
		unk1: g.u8()?,
		unk2: g.u16()?,
		function: FuncRef(g.u8()? as u16, g.u8()? as u16),
		unk3: g.u8()?,
		unk4: g.u16()?,
	})).strict()?;

	let unk1 = f.u8()?;
	let unk2 = f.u16()?;

	let entry = if f.pos() != func_table.pos() {
		Some(Entry {
			pos: f.pos3()?,
			unk1: f.u32()?,
			cam_from: f.pos3()?,
			cam_pers: f.u32()?,
			unk2: f.u16()?,
			cam_deg: f.u16()?,
			cam_limit1: f.u16()?,
			cam_limit2: f.u16()?,
			cam_at: f.pos3()?,
			unk3: f.u16()?,
			unk4: f.u16()?,
			flags: f.u16()?,
			town: TownId(f.u16()?),
			init: FuncRef(f.u8()? as u16, f.u8()? as u16),
			reinit: FuncRef(f.u8()? as u16, f.u8()? as u16),
		})
	} else {
		None
	};

	let anim_count = (func_table.pos()-anims.pos())/12;
	let (mut g, n) = (anims, anim_count);
	let anims = list(n, || {
		let speed = g.u16()?;
		let unk = g.u8()?;
		let count = g.u8()?;
		let frames = array::<8, _>(|| Ok(g.u8()?)).strict()?;
		ensure!(count <= 8, "too many frames: {count}");
		let frames = frames[..count as usize].to_owned();
		Ok(Animation {
			speed,
			unk,
			frames,
		})
	}).strict()?;

	let (mut g, n) = (func_table, func_count);
	let func_table = list(n as usize, || Ok(g.u32()? as usize)).strict()?;

	let mut functions = Vec::with_capacity(func_table.len());
	let starts = func_table.iter().copied();
	let ends = func_table.iter().copied().skip(1).map(Some).chain(Some(None));

	let mut code_end = strings_start;
	for (start, end) in starts.zip(ends) {
		let mut g = f.clone().at(start)?;
		let mut func = code::read(&mut g, iset, lookup, end)?;

		// Sometimes there's an extra return statement after what the control flow analysis gives.
		// Probably if they end the function with an explicit return.
		if end.is_none() && g.pos() != strings_start && (strings_start - g.pos()) % 8 == 1 && g.clone().u8()? == 0x01 {
			g.check_u8(0x01)?;
			func.push(code::FlatInsn::Insn(code::Insn::Return()))
		}

		functions.push(func);
		code_end = g.pos();
	}

	let mut field_sepith = Vec::new();
	let mut field_sepith_pos = HashMap::new();
	let mut at_rolls = Vec::new();
	let mut at_roll_pos = HashMap::new();
	let mut placements = Vec::new();
	let mut placement_pos = HashMap::new();
	let mut battles = Vec::new();
	let mut battle_pos = HashMap::new();

	let sepith_start = strings_start - (strings_start - code_end) / 8 * 8;
	let mut g = f.clone().at(sepith_start)?;
	while g.pos() < strings_start {
		field_sepith_pos.insert(g.pos() as u32, at_rolls.len() as u16);
		field_sepith.push(g.array::<8>()?);
	}

	if battle_start != battle_end {
		// I can't use anything but heuristics here. That sucks.
		let mut g = f.clone().at(battle_start)?;
		while g.pos() < battle_end {
			// Heuristic: first field of AT rolls is 100
			if g.clone().u8()? != 100 {
				break
			}
			at_roll_pos.insert(g.pos() as u32, at_rolls.len() as u16);
			at_rolls.push(g.array::<16>()?);
		}
		while g.pos() < battle_end {
			// if both alternatives and field sepith is zero, it's not a placement
			if g.pos() + 16+8 <= battle_end && g.clone().at(g.pos()+16)?.u64()? == 0 {
				break
			}
			// if there's a valid AT roll pointer for the first alternative, it's probably not a placement
			if g.pos() + 64+4 <= battle_end && at_roll_pos.contains_key(&g.clone().at(g.pos()+64)?.u32()?) {
				break
			}
			placement_pos.insert(g.pos() as u16, placements.len() as u16);
			placements.push(array::<8, _>(|| Ok((g.u8()?, g.u8()?, g.u16()?))).strict()?);
		}
		while g.pos() < battle_end {
			battle_pos.insert(g.pos() as u32, battles.len());
			battles.push(Battle {
				flags: g.u16()?,
				level: g.u16()?,
				unk1: g.u8()?,
				vision_range: g.u8()?,
				move_range: g.u8()?,
				can_move: g.u8()?,
				move_speed: g.u16()?,
				unk2: g.u16()?,
				battlefield: {
					ensure!(g.ptr32()?.pos() == strings.pos(), "unexpected battlefield ptr");
					strings.string()?
				},
				sepith: match g.u32()? {
					0 => None,
					n => Some(*field_sepith_pos.get(&n).ok_or("invalid field sepith ptr")?)
				},
				setups: {
					let mut setups = Vec::new();
					for weight in g.array::<4>()? {
						if weight == 0 {
							continue
						}
						setups.push(BattleSetup {
							weight,
							enemies: array(|| match g.u32()? {
								0 => Ok(None),
								n => Ok(Some(lookup.name(n)?))
							}).strict()?,
							placement: *placement_pos.get(&g.u16()?).ok_or("invalid placement ptr")?,
							placement_ambush: *placement_pos.get(&g.u16()?).ok_or("invalid placement ptr")?,
							bgm: BgmId(g.u16()?),
							bgm_ambush: BgmId(g.u16()?),
							at_roll: *at_roll_pos.get(&g.u32()?).ok_or("invalid at roll ptr")?,
						});
					}
					setups
				},
			});
		}
	}

	let (mut g, n) = (labels, n_labels as usize);
	let labels = list(n, || Ok(Label {
		pos: (g.f32()?, g.f32()?, g.f32()?),
		flags: g.u32()?,
		name: {
			ensure!(g.ptr32()?.pos() == strings.pos(), "unexpected battlefield ptr");
			strings.string()?
		}
	})).strict()?;

	let junk = match strings.remaining() {
		0 => None,
		1 => Some(strings.u8()?),
		n => bail!("unexpected {n} bytes of junk at end"),
	};

	println!("{name1} {name2} {filename}");
	f.dump_uncovered(|d| d.preview_encoding("sjis").to_stdout());

	Ok(Scena)
}

#[cfg(test)]
mod test {
	use super::code::InstructionSet;
	use crate::util::test::*;
	use crate::gamedata::ED7Lookup;

	macro_rules! test {
		($a:item) => {
			#[test_case::test_case(InstructionSet::Zero, true, "../data/zero/data/scena", ".bin"; "zero_nisa_jp")]
			#[test_case::test_case(InstructionSet::Zero, false, "../data/zero/data/scena_us", ".bin"; "zero_nisa_en")]
			$a
		}
	}

	test! {
	fn roundtrip(iset: InstructionSet, _decomp: bool, scenapath: &str, suffix: &str) -> Result<(), Error> {
		let mut failed = false;

		let mut paths = std::fs::read_dir(scenapath)?
			.map(|r| r.unwrap())
			.collect::<Vec<_>>();
		paths.sort_by_key(|dir| dir.path());

		for file in paths {
			let path = file.path();
			let name = path.file_name().unwrap().to_str().unwrap();
			if !name.ends_with(suffix) {
				continue
			}

			let data = std::fs::read(&path)?;
			
			if let Err(err) = super::read(iset, &ED7Lookup, &data) {
				println!("{name}: {err:?}");
				failed = true;
			};
		}

		assert!(!failed);
		Ok(())
	}
	}
}
