use std::collections::HashMap;
use glam::{Vec3, Mat4};
use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _};
use crate::types::*;
use themelios_common::util::*;
use super::code::{Code, FlatInsn, Insn};

#[derive(Debug, Clone, PartialEq)]
pub struct Scena {
	pub name1: String,
	pub name2: String,
	pub filename: String, // first string in string table (always @FileName in ed6, but valid here)
	pub town: TownId,
	pub bgm: BgmId,
	pub flags: u32,
	pub includes: [FileId; 6],

	pub chips: Vec<FileId>,
	pub labels: Option<Vec<Label>>,
	pub npcs: Vec<Npc>,
	pub monsters: Vec<Monster>,
	pub triggers: Vec<Trigger>,
	pub look_points: Vec<LookPoint>,
	pub animations: Vec<Animation>,
	pub entry: Option<Entry>,
	pub functions: Vec<Code>,

	/// The first five, if present, are always the same nonsensical values.
	pub sepith: Vec<[u8; 8]>,
	pub at_rolls: Vec<[u8; 16]>,
	pub placements: Vec<[(u8,u8,Angle); 8]>,
	pub battles: Vec<Battle>,

	pub item_use: FuncId,
	pub unk2: u8,
	pub unk3: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Label {
	pub name: TString,
	pub pos: Vec3,
	pub unk1: u16,
	pub unk2: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Npc {
	pub name: TString,
	pub pos: Pos3,
	pub angle: Angle,
	pub flags: CharFlags,
	pub unk2: u16,
	pub chip: ChipId,
	pub init: FuncId,
	pub talk: FuncId,
	pub unk4: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monster {
	pub pos: Pos3,
	pub angle: i16,
	pub flags: CharFlags,
	pub battle: BattleId,
	pub flag: Flag,
	pub chip: ChipId,
	pub unk2: u16,
	pub stand_anim: AnimId,
	pub walk_anim: AnimId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Trigger {
	pub pos: Vec3,
	pub radius: f32,
	pub transform: Mat4,
	pub unk1: u8,
	pub unk2: u16,
	pub function: FuncId,
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
	pub function: FuncId,
	pub unk3: u8,
	pub unk4: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
	pub pos: Pos3,
	pub unk1: u32,

	pub cam_from: Pos3,
	pub cam_pers: i32,
	pub unk2: u16,
	pub cam_deg: Angle,
	pub cam_limit: (Angle, Angle),
	pub cam_at: Pos3,
	pub unk3: u16,
	pub unk4: u16,

	pub flags: EntryFlags,
	pub town: TownId,
	pub init: FuncId,
	pub reinit: FuncId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Animation {
	pub speed: Time,
	pub frames: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Battle {
	pub flags: u16,
	pub level: u16,
	pub unk1: u8,
	pub vision_range: u8,
	pub move_range: u8,
	pub can_move: u8,
	pub move_speed: u16,
	pub unk2: u16,
	pub battlefield: String,
	pub sepith: Option<SepithId>,
	pub setups: Vec<BattleSetup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleSetup {
	pub weight: u8,
	pub enemies: [FileId; 8],
	pub placement: PlacementId,
	pub placement_ambush: PlacementId,
	pub bgm: BgmId,
	pub bgm_ambush: BgmId, // not entirely sure if this is what it is
	pub at_roll: AtRollId,
}

impl Scena {
	pub fn read(game: Game, data: &[u8]) -> Result<Scena, ReadError> {
		let mut f = Reader::new(data);

		let name1 = f.sized_string::<10>()?;
		let name2 = f.sized_string::<10>()?;
		let town = TownId(f.u16()?);
		let bgm = BgmId(f.u16()?);
		let flags = f.u32()?;
		let includes = array(|| Ok(FileId(match f.u32()? { 0xFFFFFFFF => 0, a => a}))).strict()?;

		let mut strings = f.ptr32()?;
		let strings_start = strings.pos();
		let filename = strings.string()?;

		let p_chips    = f.u16()? as usize;
		let p_npcs     = f.u16()? as usize;
		let p_monsters = f.u16()? as usize;
		let p_triggers = f.u16()? as usize;
		let p_look_points = f.u16()? as usize;

		let p_func_table = f.u16()? as usize;
		let func_count = (f.u16()? / 4) as usize;
		let p_animations = f.u16()? as usize;

		let p_labels = f.u16()? as usize;
		let n_labels = f.u8()? as usize;
		let unk3 = f.u8()?;

		let n_chips    = f.u8()? as usize;
		let n_npcs     = f.u8()? as usize;
		let n_monsters = f.u8()? as usize;
		let n_triggers = f.u8()? as usize;
		let n_look_points = f.u8()? as usize;

		let item_use = FuncId(f.u8()? as u16, f.u8()? as u16);
		let unk2 = f.u8()?;

		let entry = if f.pos() != p_triggers {
			Some(Entry {
				pos: f.pos3()?,
				unk1: f.u32()?,
				cam_from: f.pos3()?,
				cam_pers: f.i32()?,
				unk2: f.u16()?,
				cam_deg: Angle(f.i16()?),
				cam_limit: (Angle(f.i16()?), Angle(f.i16()?)),
				cam_at: f.pos3()?,
				unk3: f.u16()?,
				unk4: f.u16()?,
				flags: EntryFlags(f.u16()?),
				town: TownId(f.u16()?),
				init: FuncId(f.u8()? as u16, f.u8()? as u16),
				reinit: FuncId(f.u8()? as u16, f.u8()? as u16),
			})
		} else {
			None
		};

		let data_chunks = [
			(p_chips,       n_chips       != 0),
			(p_npcs,        n_npcs        != 0),
			(p_monsters,    n_monsters    != 0),
			(p_triggers,    n_triggers    != 0),
			(p_look_points, n_look_points != 0),
			(p_labels,      n_labels      != 0),
			(p_animations,  p_animations != f.pos()),
			(p_func_table,  true),
		];
		let first_data_chunk = data_chunks.into_iter()
			.filter_map(|(a, b)| b.then_some(a))
			.min().unwrap();
		let is_vanilla =
			p_labels == 0
			|| first_data_chunk == f.pos()
			&& p_labels <= f.pos()
			&& p_labels + n_labels*20 == p_triggers
			&& p_triggers + n_triggers*96 == p_look_points
			&& p_look_points + n_look_points*36 == p_chips
			&& p_chips + n_chips*4 == p_npcs
			&& p_npcs + n_npcs*28 == p_monsters
			&& p_monsters + n_monsters*32 <= p_animations
			&& p_animations <= p_func_table
		;
		// This misidentifies a few eddec scenas as vanilla, but the battles seem to be in the right
		// position in those anyway.

		let battle_chunk = if is_vanilla {
			Some((p_monsters + n_monsters * 32, p_animations))
		} else {
			None
		};

		let mut g = f.clone().at(p_chips)?;
		let chips = list(n_chips, || Ok(FileId(g.u32()?))).strict()?;

		let mut g = f.clone().at(p_npcs)?;
		let npcs = list(n_npcs, || Ok(Npc {
			name: TString(strings.string()?),
			pos: g.pos3()?,
			angle: Angle(g.i16()?),
			flags: CharFlags(g.u16()?),
			unk2: g.u16()?,
			chip: ChipId(g.u16()?),
			init: FuncId(g.u8()? as u16, g.u8()? as u16),
			talk: FuncId(g.u8()? as u16, g.u8()? as u16),
			unk4: g.u32()?,
		})).strict()?;

		let mut g = f.clone().at(p_monsters)?;
		let mut monsters = list(n_monsters, || Ok(Monster {
			pos: g.pos3()?,
			angle: g.i16()?,
			flags: CharFlags(g.u16()?),
			battle: BattleId(cast(g.u16()?)?),
			flag: Flag(g.u16()?),
			chip: ChipId(g.u16()?),
			unk2: g.u16()?,
			stand_anim: AnimId(cast(g.u32()?)?),
			walk_anim: AnimId(cast(g.u32()?)?),
		})).strict()?;

		let mut g = f.clone().at(p_triggers)?;
		let triggers = list(n_triggers, || Ok(Trigger {
			pos: g.vec3()?,
			radius: g.f32()?,
			transform: Mat4::from_cols_array(&array(|| Ok(g.f32()?)).strict()?),
			unk1: g.u8()?,
			unk2: g.u16()?,
			function: FuncId(g.u8()? as u16, g.u8()? as u16),
			unk3: g.u8()?,
			unk4: g.u16()?,
			unk5: g.u32()?,
			unk6: g.u32()?,
		})).strict()?;

		let mut g = f.clone().at(p_look_points)?;
		let look_points = list(n_look_points, || Ok(LookPoint {
			pos: g.pos3()?,
			radius: g.u32()?,
			bubble_pos: g.pos3()?,
			unk1: g.u8()?,
			unk2: g.u16()?,
			function: FuncId(g.u8()? as u16, g.u8()? as u16),
			unk3: g.u8()?,
			unk4: g.u16()?,
		})).strict()?;

		let anim_count = (p_func_table-p_animations)/12;
		let mut g = f.clone().at(p_animations)?;
		let animations = list(anim_count, || {
			let speed = Time(g.u16()? as u32);
			g.check_u8(0)?;
			let count = g.u8()? as usize;
			let frames = array::<8, _>(|| Ok(g.u8()?)).strict()?;
			ensure!(count <= 8, "too many frames: {count}");
			let frames = frames[..count].to_owned();
			Ok(Animation {
				speed,
				frames,
			})
		}).strict()?;

		let mut g = f.clone().at(p_func_table)?;
		let func_table = list(func_count, || Ok(g.u32()? as usize)).strict()?;

		let mut functions = Vec::with_capacity(func_table.len());
		let starts = func_table.iter().copied();
		let ends = func_table.iter().copied().skip(1).map(Some).chain(Some(None));

		let mut code_end = strings_start;
		for (start, end) in starts.zip(ends) {
			let mut g = f.clone().at(start)?;
			let mut func = Code::read(&mut g, game, end)?;

			// Sometimes there's an extra return statement after what the control flow analysis gives.
			// Probably if they end the function with an explicit return.
			if end.is_none() && g.pos() != strings_start && (strings_start - g.pos()) % 8 == 1 && g.clone().u8()? == 0x01 {
				g.check_u8(0x01)?;
				func.push(FlatInsn::Insn(Insn::Return()))
			}

			functions.push(func);
			code_end = g.pos();
		}

		let labels = if p_labels == 0 || !is_vanilla && n_labels == 0 {
			None
		} else {
			let mut g = f.clone().at(p_labels)?;
			Some(list(n_labels, || Ok(Label {
				pos: g.vec3()?,
				unk1: g.u16()?,
				unk2: g.u16()?,
				name: TString(g.ptr32()?.string()?),
			})).strict()?)
		};

		let mut btl = BattleRead::default();

		let sepith_start = strings_start - (strings_start - code_end) / 8 * 8;
		let mut g = f.clone().at(sepith_start)?;
		while g.pos() < strings_start {
			btl.get_sepith(&mut g)?;
		}

		// Load all battle parts in order, to be able to roundtrip them
		if let Some((battle_start, battle_end)) = battle_chunk {
			// The battle-related structs (including sepith above) are not as well-delineated as most other
			// chunks, so I can't do anything other than simple heuristics for parsing those. Which sucks,
			// but there's nothing I can do about it.
			let mut g = f.clone().at(battle_start)?;

			while g.pos() < battle_end {
				// Heuristic: first field of AT rolls is 100
				if g.clone().u8()? != 100 {
					break
				}
				btl.get_at_roll(&mut g)?;
			}

			while g.pos() < battle_end {
				// if both alternatives and field sepith is zero, it's not a placement
				if g.pos() + 16+8 <= battle_end && g.clone().at(g.pos()+16)?.u64()? == 0 {
					break
				}
				// if there's a valid AT roll pointer for the first alternative, it's probably not a placement
				if g.pos() + 64+4 <= battle_end && btl.at_roll_pos.contains_key(&(g.clone().at(g.pos()+64)?.u32()? as usize)) {
					break
				}
				btl.get_placement(&mut g)?;
			}

			while g.pos() < battle_end {
				btl.get_battle(&mut g)?;
			}
		}

		// Fill in battles
		for mons in &mut monsters {
			mons.battle = btl.get_battle(&mut f.clone().at(mons.battle.0 as usize)?)?;
		}
		for func in &mut functions {
			for insn in &mut func.0 {
				if let FlatInsn::Insn(Insn::ED7Battle { 0: battle, .. }) = insn {
					*battle = btl.get_battle(&mut f.clone().at(battle.0 as usize)?)?;
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
			chips,
			labels,
			npcs,
			monsters,
			triggers,
			look_points,
			animations,
			entry,
			functions,
			sepith: btl.sepith,
			at_rolls: btl.at_rolls,
			placements: btl.placements,
			battles: btl.battles,
			item_use,
			unk2,
			unk3,
		})
	}

	pub fn write(game: Game, scena: &Scena) -> Result<Vec<u8>, WriteError> {
		let mut f = Writer::new();
		f.sized_string::<10>(&scena.name1)?;
		f.sized_string::<10>(&scena.name2)?;
		f.u16(scena.town.0);
		f.u16(scena.bgm.0);
		f.u32(scena.flags);
		for i in scena.includes {
			f.u32(match i.0 { 0 => 0xFFFFFFFF, a => a });
		}

		let mut strings = f.ptr32();
		strings.string(&scena.filename)?;

		let mut chips = f.ptr16();
		let mut npcs = f.ptr16();
		let mut monsters = f.ptr16();
		let mut triggers = f.ptr16();
		let mut look_points = f.ptr16();

		let mut func_table = f.ptr16();
		f.u16(cast(scena.functions.len() * 4)?);
		let mut animations = f.ptr16();

		let mut labels = Writer::new();
		if let Some(l) = &scena.labels {
			f.delay16(labels.here());
			f.u8(cast(l.len())?);
		} else {
			f.u16(0);
			f.u8(0);
		}
		f.u8(scena.unk3);

		f.u8(cast(scena.chips.len())?);
		f.u8(cast(scena.npcs.len())?);
		f.u8(cast(scena.monsters.len())?);
		f.u8(cast(scena.triggers.len())?);
		f.u8(cast(scena.look_points.len())?);
		f.u8(cast(scena.item_use.0)?);
		f.u8(cast(scena.item_use.1)?);
		f.u8(scena.unk2);

		let mut entry = Writer::new();
		let mut functions = Writer::new();
		let mut sepith = Writer::new();
		let mut at_rolls = Writer::new();
		let mut placements = Writer::new();
		let mut battles = Writer::new();

		let g = &mut chips;
		for chip in &scena.chips {
			g.u32(chip.0);
		}

		let g = &mut npcs;
		for npc in &scena.npcs {
			strings.string(npc.name.as_str())?;
			g.pos3(npc.pos);
			g.i16(npc.angle.0);
			g.u16(npc.flags.0);
			g.u16(npc.unk2);
			g.u16(npc.chip.0);
			g.u8(cast(npc.init.0)?);
			g.u8(cast(npc.init.1)?);
			g.u8(cast(npc.talk.0)?);
			g.u8(cast(npc.talk.1)?);
			g.u32(npc.unk4);
		}

		let g = &mut monsters;
		for monster in &scena.monsters {
			g.pos3(monster.pos);
			g.i16(monster.angle);
			g.u16(monster.flags.0);
			g.delay16(gospel::write::Label::known(monster.battle.0));
			g.u16(monster.flag.0);
			g.u16(monster.chip.0);
			g.u16(monster.unk2);
			g.u32(monster.stand_anim.0 as u32);
			g.u32(monster.walk_anim.0 as u32);
		}

		let g = &mut triggers;
		for trigger in &scena.triggers {
			g.f32(trigger.pos.x);
			g.f32(trigger.pos.y);
			g.f32(trigger.pos.z);
			g.f32(trigger.radius);
			for v in trigger.transform.to_cols_array() {
				g.f32(v)
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

		let g = &mut entry;
		for entry in &scena.entry {
			g.pos3(entry.pos);
			g.u32(entry.unk1);
			g.pos3(entry.cam_from);
			g.i32(entry.cam_pers);
			g.u16(entry.unk2);
			g.i16(entry.cam_deg.0);
			g.i16(entry.cam_limit.0.0);
			g.i16(entry.cam_limit.1.0);
			g.pos3(entry.cam_at);
			g.u16(entry.unk3);
			g.u16(entry.unk4);
			g.u16(entry.flags.0);
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
			g.u16(cast(anim.speed.0)?);
			g.u8(0);
			g.u8(count as u8);
			g.slice(&frames);
		}

		for func in &scena.functions {
			func_table.delay32(functions.here());
			Code::write(&mut functions, game, func)?;
		}

		let mut sepith_pos = Vec::new();
		for sep in &scena.sepith {
			sepith_pos.push(sepith.here());
			sepith.slice(sep);
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
				g.i16(p.2.0);
			}
		}

		let g = &mut battles;
		for (idx, battle) in scena.battles.iter().enumerate() {
			g.label(gospel::write::Label::known(idx as u32));
			g.u16(battle.flags);
			g.u16(battle.level);
			g.u8(battle.unk1);
			g.u8(battle.vision_range);
			g.u8(battle.move_range);
			g.u8(battle.can_move);
			g.u16(battle.move_speed);
			g.u16(battle.unk2);
			g.delay32(strings.here());
			strings.string(&battle.battlefield)?;
			if let Some(s) = battle.sepith {
				g.delay32(*sepith_pos.get(s.0 as usize).ok_or_else(|| "field sepith out of bounds".to_owned())?);
			} else {
				g.u32(0);
			}
			let mut weights = [0u8; 4];
			let mut h = Writer::new();
			ensure!(battle.setups.len() <= 4, "too many setups");
			for (i, setup) in battle.setups.iter().enumerate() {
				weights[i] = setup.weight;
				for ms in &setup.enemies {
					h.u32(ms.0);
				}
				h.delay16(*placement_pos.get(setup.placement.0 as usize).ok_or_else(|| "placement out of bounds".to_owned())?);
				h.delay16(*placement_pos.get(setup.placement_ambush.0 as usize).ok_or_else(|| "placement out of bounds".to_owned())?);
				h.u16(setup.bgm.0);
				h.u16(setup.bgm_ambush.0);
				h.delay32(*at_roll_pos.get(setup.at_roll.0 as usize).ok_or_else(|| "at roll out of bounds".to_owned())?);
			}
			g.array(weights);
			g.append(h);
		}

		if let Some(l) = &scena.labels {
			let g = &mut labels;
			for l in l {
				g.f32(l.pos.x);
				g.f32(l.pos.y);
				g.f32(l.pos.z);
				g.u16(l.unk1);
				g.u16(l.unk2);
				g.delay32(strings.here());
				strings.string(l.name.as_str())?;
			}
		}

		f.append(entry);
		f.append(labels);
		f.append(triggers);
		f.append(look_points);
		f.append(chips);
		f.append(npcs);
		f.append(monsters);
		f.append(at_rolls);
		f.append(placements);
		f.append(battles);
		f.append(animations);
		f.append(func_table);
		f.append(functions);
		f.append(sepith);
		f.append(strings);
		// EDDec has order
		//   header, entry, at_rolls, sepith, placements, battles,
		//   chips, npcs, monsters, triggers, look_points, labels,
		//   animations, func_table, functions, strings
		Ok(f.finish()?)
	}
}

#[derive(Default)]
struct BattleRead {
	sepith: Vec<[u8;8]>,
	sepith_pos: HashMap<usize, SepithId>,
	at_rolls: Vec<[u8;16]>,
	at_roll_pos: HashMap<usize, AtRollId>,
	placements: Vec<[(u8,u8,Angle);8]>,
	placement_pos: HashMap<usize, PlacementId>,
	battles: Vec<Battle>,
	battle_pos: HashMap<usize, BattleId>,
}

impl BattleRead {
	fn get_sepith(&mut self, f: &mut Reader) -> Result<SepithId, ReadError> {
		match self.sepith_pos.entry(f.pos()) {
			std::collections::hash_map::Entry::Occupied(e) => Ok(*e.get()),
			std::collections::hash_map::Entry::Vacant(e) => {
				let v = *e.insert(SepithId(self.sepith.len() as u16));
				self.sepith.push(f.array::<8>()?);
				Ok(v)
			}
		}
	}

	fn get_at_roll(&mut self, f: &mut Reader) -> Result<AtRollId, ReadError> {
		match self.at_roll_pos.entry(f.pos()) {
			std::collections::hash_map::Entry::Occupied(e) => Ok(*e.get()),
			std::collections::hash_map::Entry::Vacant(e) => {
				let v = *e.insert(AtRollId(self.at_rolls.len() as u16));
				self.at_rolls.push(f.array::<16>()?);
				Ok(v)
			}
		}
	}

	fn get_placement(&mut self, f: &mut Reader) -> Result<PlacementId, ReadError> {
		match self.placement_pos.entry(f.pos()) {
			std::collections::hash_map::Entry::Occupied(e) => Ok(*e.get()),
			std::collections::hash_map::Entry::Vacant(e) => {
				let v = *e.insert(PlacementId(self.placements.len() as u16));
				self.placements.push(array::<8, _>(|| Ok((f.u8()?, f.u8()?, Angle(f.i16()?)))).strict()?);
				Ok(v)
			}
		}
	}

	fn get_battle(&mut self, f: &mut Reader) -> Result<BattleId, ReadError> {
		match self.battle_pos.entry(f.pos()) {
			std::collections::hash_map::Entry::Occupied(e) => Ok(*e.get()),
			std::collections::hash_map::Entry::Vacant(e) => {
				let v = *e.insert(BattleId(self.battles.len() as u32));
				let battle = Battle {
					flags: f.u16()?,
					level: f.u16()?,
					unk1: f.u8()?,
					vision_range: f.u8()?,
					move_range: f.u8()?,
					can_move: f.u8()?,
					move_speed: f.u16()?,
					unk2: f.u16()?,
					battlefield: f.ptr32()?.string()?,
					sepith: match f.u32()? {
						0 => None,
						n => Some(self.get_sepith(&mut f.clone().at(n as usize)?)?)
					},
					setups: {
						let mut setups = Vec::new();
						for weight in f.array::<4>()? {
							if weight == 0 {
								continue
							}
							setups.push(BattleSetup {
								weight,
								enemies: array(|| Ok(FileId(f.u32()?))).strict()?,
								placement: self.get_placement(&mut f.ptr16()?)?,
								placement_ambush: self.get_placement(&mut f.ptr16()?)?,
								bgm: BgmId(f.u16()?),
								bgm_ambush: BgmId(f.u16()?),
								at_roll: self.get_at_roll(&mut f.ptr32()?)?,
							});
						}
						setups
					},
				};
				self.battles.push(battle);
				Ok(v)
			}
		}
	}
}
