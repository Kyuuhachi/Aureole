use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _};
use crate::types::*;
use themelios_scena::util::*;
use super::code::{self, Code};
use super::{ReaderExt as _, WriterExt as _};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scena {
	pub path: String, // [Path; フォルダ]
	pub map: String, // [Map; マップファイル]
	pub town: TownId, // [Town; 町名]
	pub bgm: BgmId, // [BGM; BGM 番号]
	pub item: FuncId, // [Item; アイテム使用時イベント]
	pub includes: [FileId; 8], // [Scp0..7; スクリプト(１つだけは必須), これ以降は必要な場合のみ定義する]

	// The script puts cp before ch.
	pub ch: Vec<FileId>, // [Char_Data; キャラデータファイル]
	pub cp: Vec<FileId>, // [Char_Ptn; キャラパターンファイル]

	pub npcs: Vec<Npc>,
	pub monsters: Vec<Monster>,
	pub triggers: Vec<Trigger>,
	pub look_points: Vec<LookPoint>,
	pub entries: Vec<Entry>,
	pub functions: Vec<Code>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {  // [Entry]
	pub pos: Pos3,  // [PlyX, PlyY, PlyZ; Ｘ/Ｙ/Ｚ座標(1m単位)]
	pub chr: u16,   // [PlyChr; キャラパターン] Always 4
	pub angle: Angle, // [PlyVec; キャラ方角]

	pub cam_from: Pos3,  // [CameraFrom: カメラ位置(1m単位)]
	pub cam_at: Pos3,    // [CameraAt; 注目点⟩]
	pub cam_zoom: i32,   // [CameraZoom; ズーム(1mm単位)]
	pub cam_pers: i32,   // [CameraPers; パース]
	pub cam_deg: Angle,    // [CameraDeg; 角度(1度単位)]
	pub cam_limit: (Angle, Angle), // [CameraLimitDeg; カメラの回転可能角度]
	pub north: Angle,      // [NorthDeg; 北角度]

	pub flags: EntryFlags,   // [Flag]
	pub town: TownId, // [Place; 地名]
	pub init: FuncId, // [Init; 初期化用イベント]
	pub reinit: FuncId, // [ReInit; ロード後の再初期化用イベント]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Npc { // [Player]
	// They put name last, but that sucks
	pub name: TString, // [Name]
	pub pos: Pos3, // [X, Y, Z]
	pub angle: Angle, // [ANG]
	pub x: u16, // [X]
	pub cp: ChipId, // [Pt]
	pub frame: u16, // [No]
	pub ch: ChipId, // [Bs]
	pub flags: CharFlags, // [BXPNAWTDS]
	pub init: FuncId, // [MOVE_FUNC]
	pub talk: FuncId, // [EVENT_FUNC]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monster { // [Monster]
	pub name: TString,
	pub pos: Pos3,
	pub angle: Angle,
	pub chip: ChipId, // This looks like a chip index, but npcs have 4×u16 while this only has 1×u16?
	pub flags: CharFlags,
	pub unk2: i32, // Always -1
	pub battle: BattleId,
	pub flag: Flag, // set when defeated
	pub unk3: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trigger { // [Event]
	pub pos1: Pos3, // [X, Y, Z]
	pub pos2: Pos3, // [X, Y, Z]
	pub flags: TriggerFlags, // [  SN6428]
	pub func: FuncId, // [Scp:Func]
	pub unk1: u16, // (absent)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookPoint { // [LookPoint]
	pub pos: Pos3, // [X, Y, Z]
	pub radius: Length, // [R],
	pub bubble_pos: Pos3, // (absent)
	pub flags: LookPointFlags, // [_N____],
	pub func: FuncId, // [Scp:Func]
	pub unk1: u16, // (absent)
}

pub fn read(game: Game, data: &[u8]) -> Result<Scena, ReadError> {
	let mut f = Reader::new(data);

	let path = f.sized_string::<10>()?;
	let map = f.sized_string::<14>()?;
	let town = TownId(f.u16()?);
	let bgm = BgmId(f.u16()?);
	let item = FuncId(f.u16()?, f.u16()?);
	let includes = array(|| Ok(FileId(match f.u32()? { 0xFFFFFFFF => 0, a => a}))).strict()?;
	f.check_u16(0)?;

	let head_end = f.clone().u16()? as usize;

	let ch       = (f.ptr16()?, f.u16()?);
	let cp       = (f.ptr16()?, f.u16()?);
	let npcs     = (f.ptr16()?, f.u16()?);
	let monsters = (f.ptr16()?, f.u16()?);
	let triggers = (f.ptr16()?, f.u16()?);
	let look_points = (f.ptr16()?, f.u16()?);

	let mut strings = f.ptr16()?;

	let code_start = f.u16()? as usize;
	f.check_u16(0)?;
	let code_end = f.clone().u16()? as usize;
	let func_table = (f.ptr16()?, f.u16()? / 2);

	ensure!(strings.string()? == "@FileName", "expected @FileName");

	let (mut g, n) = ch;
	let ch = list(n as usize, || Ok(FileId(g.u32()?))).strict()?;

	let (mut g, n) = cp;
	let cp = list(n as usize, || Ok(FileId(g.u32()?))).strict()?;

	let (mut g, n) = npcs;
	let npcs = list(n as usize, || Ok(Npc {
		name: TString(strings.string()?),
		pos: g.pos3()?,
		angle: Angle(g.i16()?),
		x: g.u16()?,
		cp: ChipId(g.u16()?),
		frame: g.u16()?,
		ch: ChipId(g.u16()?),
		flags: CharFlags(g.u16()?),
		init: FuncId(g.u16()?, g.u16()?),
		talk: FuncId(g.u16()?, g.u16()?),
	})).strict()?;

	let (mut g, n) = monsters;
	let monsters = list(n as usize, || Ok(Monster {
		name: TString(strings.string()?),
		pos: g.pos3()?,
		angle: Angle(g.i16()?),
		chip: ChipId(g.u16()?),
		flags: CharFlags(g.u16()?),
		unk2: g.i32()?,
		battle: BattleId(cast(g.u16()?)?),
		flag: Flag(g.u16()?),
		unk3: g.u16()?,
	})).strict()?;

	let (mut g, n) = triggers;
	let triggers = list(n as usize, || Ok(Trigger {
		pos1: g.pos3()?,
		pos2: g.pos3()?,
		flags: TriggerFlags(g.u16()?),
		func: FuncId(g.u16()?, g.u16()?),
		unk1: g.u16()?,
	})).strict()?;

	let (mut g, n) = look_points;
	let look_points = list(n as usize, || Ok(LookPoint {
		pos: g.pos3()?,
		radius: Length(g.i32()?),
		bubble_pos: g.pos3()?,
		flags: LookPointFlags(cast(g.u16()?)?),
		func: FuncId(g.u16()?, g.u16()?),
		unk1: g.u16()?,
	})).strict()?;

	let (mut g, n) = func_table;
	let func_table = list(n as usize, || Ok(g.u16()? as usize)).strict()?;
	ensure!(func_table.is_empty() || func_table[0] == code_start,
		"Unexpected func table: {func_table:X?} does not start with {code_start:X?}"
	);

	let mut entries = Vec::new();
	while f.pos() < head_end {
		entries.push(Entry {
			pos: f.pos3()?,
			chr: f.u16()?,
			angle: Angle(f.i16()?),
			cam_from: f.pos3()?,
			cam_at: f.pos3()?,
			cam_zoom: f.i32()?,
			cam_pers: f.i32()?,
			cam_deg: Angle(f.i16()?),
			cam_limit: (Angle(f.i16()?), Angle(f.i16()?)),
			north: Angle(f.i16()?),
			flags: EntryFlags(f.u16()?),
			town: TownId(f.u16()?),
			init: FuncId(f.u16()?, f.u16()?),
			reinit: FuncId(f.u16()?, f.u16()?),
		});
	}
	ensure!(f.pos() == head_end, "overshot with entries");

	let mut functions = Vec::with_capacity(func_table.len());
	let starts = func_table.iter().copied();
	let ends = func_table.iter().copied().skip(1).chain(std::iter::once(code_end));
	for (start, end) in starts.zip(ends) {
		functions.push(code::read(&mut f.clone().at(start)?, game, Some(end))?);
	}

	Ok(Scena {
		path, map,
		town, bgm,
		item,
		includes,
		ch, cp,
		npcs, monsters,
		triggers, look_points,
		entries,
		functions,
	})
}

pub fn write(game: Game, scena: &Scena) -> Result<Vec<u8>, WriteError> {
	let mut f = Writer::new();

	f.sized_string::<10>(&scena.path)?;
	f.sized_string::<14>(&scena.map)?;
	f.u16(scena.town.0);
	f.u16(scena.bgm.0);
	f.u16(scena.item.0); f.u16(scena.item.1);
	for i in scena.includes {
		f.u32(match i.0 { 0 => 0xFFFFFFFF, a => a });
	}
	f.u16(0);

	let mut chs = f.ptr16();
	f.u16(cast(scena.ch.len())?);

	let mut cps = f.ptr16();
	f.u16(cast(scena.cp.len())?);

	let mut npcs = f.ptr16();
	f.u16(cast(scena.npcs.len())?);

	let mut monsters = f.ptr16();
	f.u16(cast(scena.monsters.len())?);

	let mut triggers = f.ptr16();
	f.u16(cast(scena.triggers.len())?);

	let mut lps = f.ptr16();
	f.u16(cast(scena.look_points.len())?);

	let mut strings = f.ptr16();
	strings.string("@FileName")?;

	let mut code = f.ptr16();
	f.u16(0);
	let mut func_table = f.ptr16();
	f.u16(cast(scena.functions.len() * 2)?);

	for e in &scena.entries {
		f.pos3(e.pos);
		f.u16(e.chr);
		f.i16(e.angle.0);
		f.pos3(e.cam_from);
		f.pos3(e.cam_at);
		f.i32(e.cam_zoom);
		f.i32(e.cam_pers);
		f.i16(e.cam_deg.0);
		f.i16(e.cam_limit.0.0);
		f.i16(e.cam_limit.1.0);
		f.i16(e.north.0);
		f.u16(e.flags.0);
		f.u16(e.town.0);
		f.u16(e.init.0); f.u16(e.init.1);
		f.u16(e.reinit.0); f.u16(e.reinit.1);
	}

	for ch in &scena.ch { chs.u32(ch.0); }
	chs.u8(0xFF);

	for cp in &scena.cp { cps.u32(cp.0); }
	cps.u8(0xFF);

	for npc in &scena.npcs {
		strings.string(npc.name.as_str())?;
		npcs.pos3(npc.pos);
		npcs.i16(npc.angle.0);
		npcs.u16(npc.x);
		npcs.u16(npc.cp.0);
		npcs.u16(npc.frame);
		npcs.u16(npc.ch.0);
		npcs.u16(npc.flags.0);
		npcs.u16(npc.init.0); npcs.u16(npc.init.1);
		npcs.u16(npc.talk.0); npcs.u16(npc.talk.1);
	}

	for monster in &scena.monsters {
		strings.string(monster.name.as_str())?;
		monsters.pos3(monster.pos);
		monsters.i16(monster.angle.0);
		monsters.u16(monster.chip.0);
		monsters.u16(monster.flags.0);
		monsters.i32(monster.unk2);
		monsters.u16(cast(monster.battle.0)?);
		monsters.u16(monster.flag.0);
		monsters.u16(monster.unk3);
	}

	for trigger in &scena.triggers {
		triggers.pos3(trigger.pos1);
		triggers.pos3(trigger.pos2);
		triggers.u16(trigger.flags.0);
		triggers.u16(trigger.func.0); triggers.u16(trigger.func.1);
		triggers.u16(trigger.unk1);
	}

	for lp in &scena.look_points {
		lps.pos3(lp.pos);
		lps.i32(lp.radius.0);
		lps.pos3(lp.bubble_pos);
		lps.u16(cast(lp.flags.0)?);
		lps.u16(lp.func.0); lps.u16(lp.func.1);
		lps.u16(lp.unk1);
	}

	for func in scena.functions.iter() {
		func_table.delay16(code.here());
		code::write(&mut code, game, func)?;
	}

	f.append(chs);
	f.append(cps);
	f.append(npcs);
	f.append(monsters);
	f.append(triggers);
	f.append(lps);
	f.append(code);
	f.append(func_table);
	f.append(strings);
	Ok(f.finish()?)
}
