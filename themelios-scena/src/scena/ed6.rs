use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::types::*;
use crate::util::*;

use super::*;
use super::code::Code;

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
	pub cp: ChcpId, // [Pt]
	pub frame: u16, // [No]
	pub ch: ChcpId, // [Bs]
	pub flags: CharFlags, // [BXPNAWTDS]
	pub init: FuncId, // [MOVE_FUNC]
	pub talk: FuncId, // [EVENT_FUNC]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monster { // [Monster]
	pub name: TString,
	pub pos: Pos3,
	pub angle: Angle,
	pub chcp: ChcpId, // This looks like a chcp index, but npcs have 4×u16 while this only has 1×u16?
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
	let mut f = Coverage::new(Reader::new(data));

	let path = f.sized_string::<10>()?;
	let map = f.sized_string::<14>()?;
	let town = TownId(f.u16()?);
	let bgm = BgmId(f.u16()?);
	let item = FuncId(f.u16()?, f.u16()?);
	let includes = array(|| Ok(FileId(match f.u32()? { 0xFFFFFFFF => 0, a => a}))).strict()?;
	f.check_u16(0)?;

	let head_end = f.clone().u16()? as usize;

	let ch       = (f.ptr()?, f.u16()?);
	let cp       = (f.ptr()?, f.u16()?);
	let npcs     = (f.ptr()?, f.u16()?);
	let monsters = (f.ptr()?, f.u16()?);
	let triggers = (f.ptr()?, f.u16()?);
	let look_points = (f.ptr()?, f.u16()?);

	let mut strings = f.ptr()?;

	let code_start = f.u16()? as usize;
	f.check_u16(0)?;
	let code_end = f.clone().u16()? as usize;
	let func_table = (f.ptr()?, f.u16()? / 2);

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
		cp: ChcpId(g.u16()?),
		frame: g.u16()?,
		ch: ChcpId(g.u16()?),
		flags: CharFlags(g.u16()?),
		init: FuncId(g.u16()?, g.u16()?),
		talk: FuncId(g.u16()?, g.u16()?),
	})).strict()?;

	let (mut g, n) = monsters;
	let monsters = list(n as usize, || Ok(Monster {
		name: TString(strings.string()?),
		pos: g.pos3()?,
		angle: Angle(g.i16()?),
		chcp: ChcpId(g.u16()?),
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
	let &Scena {
		ref path,
		ref map,
		town,
		bgm,
		item,
		ref includes,
		ref ch,
		ref cp,
		ref npcs,
		ref monsters,
		ref triggers,
		ref look_points,
		ref entries,
		ref functions,
	} = scena;
	let mut f = Writer::new();
	let mut g = Writer::new();
	let mut func_table = Writer::new();
	let mut strings = Writer::new();

	f.sized_string::<10>(path)?;
	f.sized_string::<14>(map)?;
	f.u16(town.0);
	f.u16(bgm.0);
	f.u16(item.0); f.u16(item.1);
	for i in includes {
		f.u32(match i.0 { 0 => 0xFFFFFFFF, a => a });
	}
	f.u16(0);

	let (l_ch, l_ch_) = Label::new();
	f.delay_u16(l_ch);
	f.u16(cast(ch.len())?);

	let (l_cp, l_cp_) = Label::new();
	f.delay_u16(l_cp);
	f.u16(cast(cp.len())?);

	let (l_npcs, l_npcs_) = Label::new();
	f.delay_u16(l_npcs);
	f.u16(cast(npcs.len())?);

	let (l_monsters, l_monsters_) = Label::new();
	f.delay_u16(l_monsters);
	f.u16(cast(monsters.len())?);

	let (l_triggers, l_triggers_) = Label::new();
	f.delay_u16(l_triggers);
	f.u16(cast(triggers.len())?);

	let (l_look_points, l_look_points_) = Label::new();
	f.delay_u16(l_look_points);
	f.u16(cast(look_points.len())?);

	f.delay_u16(strings.here());
	strings.string("@FileName")?;

	let (l_code_start, l_code_start_) = Label::new();
	f.delay_u16(l_code_start);
	f.u16(0);
	let (l_func_table, l_func_table_) = Label::new();
	f.delay_u16(l_func_table);
	f.u16(cast(functions.len() * 2)?);

	g.label(l_ch_);
	for ch in ch { g.u32(ch.0); }
	g.u8(0xFF);

	g.label(l_cp_);
	for cp in cp { g.u32(cp.0); }
	g.u8(0xFF);

	g.label(l_npcs_);
	for &Npc { ref name, pos, angle, x, cp, frame, ch, flags, init, talk } in npcs {
		strings.string(name.as_str())?;
		g.pos3(pos);
		g.i16(angle.0);
		g.u16(x);
		g.u16(cp.0);
		g.u16(frame);
		g.u16(ch.0);
		g.u16(flags.0);
		g.u16(init.0); g.u16(init.1);
		g.u16(talk.0); g.u16(talk.1);
	}

	g.label(l_monsters_);
	for &Monster { ref name, pos, angle, chcp, flags, unk2, battle, flag, unk3 } in monsters {
		strings.string(name.as_str())?;
		g.pos3(pos);
		g.i16(angle.0);
		g.u16(chcp.0);
		g.u16(flags.0);
		g.i32(unk2);
		g.u16(cast(battle.0)?);
		g.u16(flag.0);
		g.u16(unk3);
	}

	g.label(l_triggers_);
	for &Trigger { pos1, pos2, flags, func, unk1 } in triggers {
		g.pos3(pos1);
		g.pos3(pos2);
		g.u16(flags.0);
		g.u16(func.0); g.u16(func.1);
		g.u16(unk1);
	}

	g.label(l_look_points_);
	for &LookPoint { pos, radius, bubble_pos, flags, func, unk1 } in look_points {
		g.pos3(pos);
		g.i32(radius.0);
		g.pos3(bubble_pos);
		g.u16(cast(flags.0)?);
		g.u16(func.0); g.u16(func.1);
		g.u16(unk1);
	}

	func_table.label(l_func_table_);
	g.label(l_code_start_);
	for func in functions.iter() {
		func_table.delay_u16(g.here());
		code::write(&mut g, game, func)?;
	}

	for &Entry {
		pos, chr, angle,
		cam_from, cam_at, cam_zoom, cam_pers, cam_deg, cam_limit, north,
		flags, town, init, reinit,
	} in entries {
		f.pos3(pos);
		f.u16(chr);
		f.i16(angle.0);
		f.pos3(cam_from);
		f.pos3(cam_at);
		f.i32(cam_zoom);
		f.i32(cam_pers);
		f.i16(cam_deg.0);
		f.i16(cam_limit.0.0);
		f.i16(cam_limit.1.0);
		f.i16(north.0);
		f.u16(flags.0);
		f.u16(town.0);
		f.u16(init.0); f.u16(init.1);
		f.u16(reinit.0); f.u16(reinit.1);
	}

	f.append(g);
	f.append(func_table);
	f.append(strings);
	Ok(f.finish()?)
}
