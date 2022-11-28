use std::collections::HashMap;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::gamedata::Lookup;
use crate::tables::bgmtbl::BgmId;
use crate::tables::btlset::BattleId;
use crate::tables::town::TownId;
use crate::util::*;

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Scena {
	pub name1: String,
	pub name2: String,
	pub filename: String, // first string in string table (always @FileName in ed6, but valid here)
	pub town: TownId,
	pub bgm: BgmId,
	pub flags: u32,
	pub includes: [Option<String>; 6],

	pub chcp: Vec<Option<String>>,
	pub labels: Option<Vec<Label>>,
	pub npcs: Vec<Npc>,
	pub monsters: Vec<Monster>,
	pub triggers: Vec<Trigger>,
	pub look_points: Vec<LookPoint>,
	pub animations: Vec<Animation>,
	pub entry: Option<Entry>,
	pub functions: Vec<Vec<code::FlatInsn>>,

	/// The first five, if present, are always the same nonsensical values.
	pub field_sepith: Vec<[u8; 8]>,
	pub at_rolls: Vec<[u8; 16]>,
	pub placements: Vec<[(u8,u8,u16); 8]>,
	pub battles: Vec<Battle>,

	pub unk1: u8,
	pub unk2: u16,
}

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

pub fn read(iset: code::InstructionSet, lookup: &dyn Lookup, data: &[u8]) -> Result<Scena, ReadError> {
	let mut f = Bytes::new(data);

	let name1 = f.sized_string::<10>()?;
	let name2 = f.sized_string::<10>()?;
	let town = TownId(f.u16()?);
	let bgm = BgmId(f.u16()?);
	let flags = f.u32()?;
	let includes = f.multiple_loose::<6, _>(&[0xFF;4], |g| Ok(lookup.name(g.u32()?)?))?;


	let mut strings = f.ptr32()?;
	let strings_start = strings.pos();
	let filename = strings.string()?;

	let chcp     = f.ptr()?;
	let npcs     = f.ptr()?;
	let monsters = f.ptr()?;
	let triggers = f.ptr()?;
	let look_points = f.ptr()?;

	let func_table = f.ptr()?;
	let func_count = f.u16()? / 4;
	let animations = f.ptr()?;

	let labels = f.ptr()?;
	let n_labels = f.u16()?;

	let mut g = chcp;
	let chcp = list(f.u8()? as usize, || Ok(match g.u32()? {
		0 => None,
		n => Some(lookup.name(n)?)
	})).strict()?;

	let mut g = npcs;
	let npcs = list(f.u8()? as usize, || Ok(Npc {
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

	let mut g = monsters;
	let mut monsters = list(f.u8()? as usize, || Ok(Monster {
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
	let battle_end = animations.pos();

	let mut g = triggers;
	let triggers = list(f.u8()? as usize, || Ok(Trigger {
		pos: (g.f32()?, g.f32()?, g.f32()?),
		radius: g.f32()?,
		transform: array(|| {
			array(|| Ok(g.f32()?))
		}).strict()?,
		unk1: g.u8()?,
		unk2: g.u16()?,
		function: FuncRef(g.u8()? as u16, g.u8()? as u16),
		unk3: g.u8()?,
		unk4: g.u16()?,
		unk5: g.u32()?,
		unk6: g.u32()?,
	})).strict()?;

	let mut g = look_points;
	let look_points = list(f.u8()? as usize, || Ok(LookPoint {
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

	let anim_count = (func_table.pos()-animations.pos())/12;
	let mut g = animations;
	let animations = list(anim_count, || {
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

	let mut g = func_table;
	let func_table = list(func_count as usize, || Ok(g.u32()? as usize)).strict()?;

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
	let sepith_start = strings_start - (strings_start - code_end) / 8 * 8;
	let mut g = f.clone().at(sepith_start)?;
	while g.pos() < strings_start {
		field_sepith_pos.insert(g.pos() as u32, field_sepith.len() as u16);
		field_sepith.push(g.array::<8>()?);
	}

	// The battle-related structs (including sepith above) are not as well-delineated as most other
	// chunks, so I can't do anything other than simple heuristics for parsing those. Which sucks,
	// but there's nothing I can do about it.
	let mut g = f.clone().at(battle_start)?;

	let mut at_rolls = Vec::new();
	let mut at_roll_pos = HashMap::new();
	while g.pos() < battle_end {
		// Heuristic: first field of AT rolls is 100
		if g.clone().u8()? != 100 {
			break
		}
		at_roll_pos.insert(g.pos() as u32, at_rolls.len() as u16);
		at_rolls.push(g.array::<16>()?);
	}

	let mut placements = Vec::new();
	let mut placement_pos = HashMap::new();
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

	let mut battles = Vec::new();
	let mut battle_pos = HashMap::new();
	while g.pos() < battle_end {
		battle_pos.insert(g.pos() as u32, battles.len() as u32);
		battles.push(Battle {
			flags: g.u16()?,
			level: g.u16()?,
			unk1: g.u8()?,
			vision_range: g.u8()?,
			move_range: g.u8()?,
			can_move: g.u8()?,
			move_speed: g.u16()?,
			unk2: g.u16()?,
			battlefield: g.ptr32()?.string()?,
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

	let labels = if labels.pos() == 0 {
		None
	} else {
		let mut g = labels;
		Some(list(n_labels as usize, || Ok(Label {
			pos: (g.f32()?, g.f32()?, g.f32()?),
			flags: g.u32()?,
			name: g.ptr32()?.string()?,
		})).strict()?)
	};

	// Fill in battles
	for mons in &mut monsters {
		mons.battle.0 = *battle_pos.get(&mons.battle.0).ok_or("invalid battle ptr")?;
	}
	for func in &mut functions {
		for insn in func {
			if let code::FlatInsn::Insn(code::Insn::ED7Battle { 0: btl, .. }) = insn {
				btl.0 = *battle_pos.get(&btl.0).ok_or("invalid battle ptr")?;
			}
		}
	}

	Ok(Scena {
		name1,
		name2,
		filename,
		town,
		bgm,
		flags,
		includes,
		chcp,
		labels,
		npcs,
		monsters,
		triggers,
		look_points,
		animations,
		entry,
		functions,
		field_sepith,
		at_rolls,
		placements,
		battles,
		unk1,
		unk2,
	})
}

pub fn write(iset: code::InstructionSet, lookup: &dyn Lookup, scena: &Scena) -> Result<Vec<u8>, WriteError> {
	let mut f = OutBytes::new();
	f.sized_string::<10>(&scena.name1)?;
	f.sized_string::<10>(&scena.name2)?;
	f.u16(scena.town.0);
	f.u16(scena.bgm.0);
	f.u32(scena.flags);
	f.multiple_loose::<6, _>(&[0xFF; 4], &scena.includes, |g, a| { g.u32(lookup.index(a)?); Ok(()) }).strict()?;

	let mut strings = f.ptr32();
	strings.string(&scena.filename)?;

	let mut chcp = f.ptr();
	let mut npcs = f.ptr();
	let mut monsters = f.ptr();
	let mut triggers = f.ptr();
	let mut look_points = f.ptr();

	let mut func_table = f.ptr();
	f.u16(cast(scena.functions.len() * 4)?);
	let mut animations = f.ptr();

	let mut labels = OutBytes::new();
	if let Some(l) = &scena.labels {
		f.delay_u16(labels.here());
		f.u16(cast(l.len())?);
	} else {
		f.u16(0);
		f.u16(0);
	}

	let mut entry = OutBytes::new();
	let mut functions = OutBytes::new();
	let mut field_sepith = OutBytes::new();
	let mut at_rolls = OutBytes::new();
	let mut placements = OutBytes::new();
	let mut battles = OutBytes::new();

	f.u8(cast(scena.chcp.len())?);
	let g = &mut chcp;
	for chcp in &scena.chcp {
		g.u32(chcp.as_ref().map_or(Ok(0), |a| lookup.index(a))?);
	}

	f.u8(cast(scena.npcs.len())?);
	let g = &mut npcs;
	for npc in &scena.npcs {
		strings.string(&npc.name)?;
		g.pos3(npc.pos);
		g.i16(npc.angle);
		g.u16(npc.unk1);
		g.u16(npc.unk2);
		g.u16(npc.unk3);
		g.u16(npc.unk4);
		g.u8(cast(npc.init.0)?);
		g.u8(cast(npc.init.1)?);
		g.u32(npc.unk5);
	}

	f.u8(cast(scena.monsters.len())?);
	let g = &mut monsters;
	for monster in &scena.monsters {
		g.pos3(monster.pos);
		g.i16(monster.angle);
		g.u16(monster.unk1);
		g.delay_u16(hamu::write::Label::known(monster.battle.0).0);
		g.u16(monster.flag.0);
		g.u16(monster.chcp);
		g.u16(monster.unk2);
		g.u32(monster.stand_anim);
		g.u32(monster.walk_anim);
	}

	f.u8(cast(scena.triggers.len())?);
	let g = &mut triggers;
	for trigger in &scena.triggers {
		g.f32(trigger.pos.0);
		g.f32(trigger.pos.1);
		g.f32(trigger.pos.2);
		g.f32(trigger.radius);
		for row in trigger.transform {
			for col in row {
				g.f32(col)
			}
		}
		g.u8(trigger.unk1);
		g.u16(trigger.unk2);
		g.u8(cast(trigger.function.0)?);
		g.u8(cast(trigger.function.1)?);
		g.u8(trigger.unk3);
		g.u16(trigger.unk4);
		g.u32(trigger.unk5);
		g.u32(trigger.unk6);
	}

	f.u8(cast(scena.look_points.len())?);
	let g = &mut look_points;
	for lp in &scena.look_points {
		g.pos3(lp.pos);
		g.u32(lp.radius);
		g.pos3(lp.bubble_pos);
		g.u8(lp.unk1);
		g.u16(lp.unk2);
		g.u8(cast(lp.function.0)?);
		g.u8(cast(lp.function.1)?);
		g.u8(lp.unk3);
		g.u16(lp.unk4);
	}

	f.u8(scena.unk1);
	f.u16(scena.unk2);

	let g = &mut entry;
	for entry in &scena.entry {
		g.pos3(entry.pos);
		g.u32(entry.unk1);
		g.pos3(entry.cam_from);
		g.u32(entry.cam_pers);
		g.u16(entry.unk2);
		g.u16(entry.cam_deg);
		g.u16(entry.cam_limit1);
		g.u16(entry.cam_limit2);
		g.pos3(entry.cam_at);
		g.u16(entry.unk3);
		g.u16(entry.unk4);
		g.u16(entry.flags);
		g.u16(entry.town.0);
		g.u8(cast(entry.init.0)?);
		g.u8(cast(entry.init.1)?);
		g.u8(cast(entry.reinit.0)?);
		g.u8(cast(entry.reinit.1)?);
	}

	let g = &mut animations;
	for anim in &scena.animations {
		let count = anim.frames.len();
		ensure!(count <= 8, "too many frames: {count}");
		let mut frames = [0; 8];
		frames[..count].copy_from_slice(&anim.frames);
		g.u16(anim.speed);
		g.u8(anim.unk);
		g.u8(count as u8);
		g.slice(&frames);
	}

	for func in &scena.functions {
		func_table.delay_u32(functions.here());
		code::write(&mut functions, iset, lookup, func)?;
	}

	let mut field_sepith_pos = Vec::new();
	for sep in &scena.field_sepith {
		field_sepith_pos.push(field_sepith.here());
		field_sepith.slice(sep);
	}

	let mut at_roll_pos = Vec::new();
	for roll in &scena.at_rolls {
		at_roll_pos.push(at_rolls.here());
		at_rolls.slice(roll);
	}

	let g = &mut placements;
	let mut placement_pos = Vec::new();
	for plac in &scena.placements {
		placement_pos.push(g.here());
		for p in plac {
			g.u8(p.0);
			g.u8(p.1);
			g.u16(p.2);
		}
	}

	let g = &mut battles;
	for (idx, battle) in scena.battles.iter().enumerate() {
		g.label(hamu::write::Label::known(idx as u32).1);
		g.u16(battle.flags);
		g.u16(battle.level);
		g.u8(battle.unk1);
		g.u8(battle.vision_range);
		g.u8(battle.move_range);
		g.u8(battle.can_move);
		g.u16(battle.move_speed);
		g.u16(battle.unk2);
		g.delay_u32(strings.here());
		strings.string(&battle.battlefield)?;
		if let Some(s) = battle.sepith {
			g.delay_u32(field_sepith_pos.get(s as usize).cloned()
				.ok_or_else(|| "field sepith out of bounds".to_owned())?);
		} else {
			g.u32(0);
		}
		let mut weights = [0u8; 4];
		let mut h = OutBytes::new();
		ensure!(battle.setups.len() <= 4, "too many setups");
		for (i, setup) in battle.setups.iter().enumerate() {
			weights[i] = setup.weight;
			for ms in &setup.enemies {
				h.u32(ms.as_ref().map_or(Ok(0), |a| lookup.index(a))?);
			}
			h.delay_u16(placement_pos.get(setup.placement as usize).cloned()
				.ok_or_else(|| "placement out of bounds".to_owned())?);
			h.delay_u16(placement_pos.get(setup.placement_ambush as usize).cloned()
				.ok_or_else(|| "placement out of bounds".to_owned())?);
			h.u16(setup.bgm.0);
			h.u16(setup.bgm_ambush.0);
			h.delay_u32(at_roll_pos.get(setup.at_roll as usize).cloned()
				.ok_or_else(|| "at roll out of bounds".to_owned())?);
		}
		g.array(weights);
		g.append(h);
	}

	if let Some(l) = &scena.labels {
		let g = &mut labels;
		for l in l {
			g.f32(l.pos.0);
			g.f32(l.pos.1);
			g.f32(l.pos.2);
			g.u32(l.flags);
			g.delay_u32(strings.here());
			strings.string(&l.name)?;
		}
	}

	f.append(entry);
	f.append(labels);
	f.append(triggers);
	f.append(look_points);
	f.append(chcp);
	f.append(npcs);
	f.append(monsters);
	f.append(at_rolls);
	f.append(placements);
	f.append(battles);
	f.append(animations);
	f.append(func_table);
	f.append(functions);
	f.append(field_sepith);
	f.append(strings);
	Ok(f.finish()?)
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
			
			if let Err(err) = check_roundtrip_strict(
				&data,
				|a| super::read(iset, &ED7Lookup, a),
				|a| super::write(iset, &ED7Lookup, a),
			) {
				println!("{name}: {err:?}");
				failed = true;
			};
		}

		assert!(!failed);
		Ok(())
	}
	}
}