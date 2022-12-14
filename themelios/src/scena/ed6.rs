use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::gamedata::GameData;
use crate::tables::bgmtbl::BgmId;
use crate::tables::btlset::BattleId;
use crate::tables::town::TownId;
use crate::util::*;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scena {
	pub path: String, // [Path; フォルダ]
	pub map: String, // [Map; マップファイル]
	pub town: TownId, // [Town; 町名]
	pub bgm: BgmId, // [BGM; BGM 番号]
	pub item: FuncRef, // [Item; アイテム使用時イベント]
	pub includes: [Option<String>; 8], // [Scp0..7; スクリプト(１つだけは必須), これ以降は必要な場合のみ定義する]

	// The script puts cp before ch.
	pub ch: Vec<String>, // [Char_Data; キャラデータファイル]
	pub cp: Vec<String>, // [Char_Ptn; キャラパターンファイル]

	pub npcs: Vec<Npc>,
	pub monsters: Vec<Monster>,
	pub triggers: Vec<Trigger>,
	pub look_points: Vec<LookPoint>,
	pub entries: Vec<Entry>,
	pub functions: Vec<Vec<code::FlatInsn>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {  // [Entry]
	pub pos: Pos3,  // [PlyX, PlyY, PlyZ; Ｘ/Ｙ/Ｚ座標(1m単位)]
	pub chr: u16,   // [PlyChr; キャラパターン] a chcp?
	pub angle: i16, // [PlyVec; キャラ方角]

	pub cam_from: Pos3,  // [CameraFrom: カメラ位置(1m単位)]
	pub cam_at: Pos3,    // [CameraAt; 注目点⟩]
	pub cam_zoom: i32,   // [CameraZoom; ズーム(1mm単位)]
	pub cam_pers: i32,   // [CameraPers; パース]
	pub cam_deg: i16,    // [CameraDeg; 角度(1度単位)]
	pub cam_limit1: i16, // [CameraLimitDeg; カメラの回転可能角度]
	pub cam_limit2: i16, // ↑
	pub north: i16,      // [NorthDeg; 北角度]

	pub flags: u16,   // [Flag]
	pub town: TownId, // [Place; 地名]
	pub init: FuncRef, // [Init; 初期化用イベント]
	pub reinit: FuncRef, // [ReInit; ロード後の再初期化用イベント]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Npc { // [Player]
	// They put name last, but that sucks
	pub name: String, // [Name]
	pub pos: Pos3, // [X, Y, Z]
	pub angle: i16, // [ANG]
	pub x: u16, // [X]
	pub cp: u16, // [Pt]
	pub frame: u16, // [No]
	pub ch: u16, // [Bs]
	pub flags: CharFlags, // [BXPNAWTDS]
	pub init: FuncRef, // [MOVE_FUNC]
	pub talk: FuncRef, // [EVENT_FUNC]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monster { // [Monster]
	pub name: String,
	pub pos: Pos3,
	pub angle: i16,
	pub unk1: u16, // This looks like a chcp index, but npcs have 4×u16 while this only has 1×u16?
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
	pub flags: u16, // [  SN6428]
	pub func: FuncRef, // [Scp:Func]
	pub unk1: u16, // (absent)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookPoint { // [LookPoint]
	pub pos: Pos3, // [X, Y, Z]
	pub radius: u32, // [R],
	pub bubble_pos: Pos3, // (absent)
	pub flags: LookPointFlags, // [_N____],
	pub func: FuncRef, // [Scp:Func]
	pub unk1: u16, // (absent)
}

pub fn read(game: &GameData, data: &[u8]) -> Result<Scena, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));

	let path = f.sized_string::<10>()?;
	let map = f.sized_string::<14>()?;
	let town = TownId(f.u16()?);
	let bgm = BgmId(f.u16()?);
	let item = FuncRef(f.u16()?, f.u16()?);
	let includes = f.multiple_loose::<8, _>(&[0xFF;4], |g| Ok(game.lookup.name(g.u32()?)?))?;
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
	let ch = list(n as usize, || Ok(game.lookup.name(g.u32()?)?)).strict()?;

	let (mut g, n) = cp;
	let cp = list(n as usize, || Ok(game.lookup.name(g.u32()?)?)).strict()?;

	let (mut g, n) = npcs;
	let npcs = list(n as usize, || Ok(Npc {
		name: strings.string()?,
		pos: g.pos3()?,
		angle: g.i16()?,
		x: g.u16()?,
		cp: g.u16()?,
		frame: g.u16()?,
		ch: g.u16()?,
		flags: CharFlags(g.u16()?),
		init: FuncRef(g.u16()?, g.u16()?),
		talk: FuncRef(g.u16()?, g.u16()?),
	})).strict()?;

	let (mut g, n) = monsters;
	let monsters = list(n as usize, || Ok(Monster {
		name: strings.string()?,
		pos: g.pos3()?,
		angle: g.i16()?,
		unk1: g.u16()?,
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
		flags: g.u16()?,
		func: FuncRef(g.u16()?, g.u16()?),
		unk1: g.u16()?,
	})).strict()?;

	let (mut g, n) = look_points;
	let look_points = list(n as usize, || Ok(LookPoint {
		pos: g.pos3()?,
		radius: g.u32()?,
		bubble_pos: g.pos3()?,
		flags: LookPointFlags(cast(g.u16()?)?),
		func: FuncRef(g.u16()?, g.u16()?),
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
			angle: f.i16()?,
			cam_from: f.pos3()?,
			cam_at: f.pos3()?,
			cam_zoom: f.i32()?,
			cam_pers: f.i32()?,
			cam_deg: f.i16()?,
			cam_limit1: f.i16()?,
			cam_limit2: f.i16()?,
			north: f.i16()?,
			flags: f.u16()?,
			town: TownId(f.u16()?),
			init: FuncRef(f.u16()?, f.u16()?),
			reinit: FuncRef(f.u16()?, f.u16()?),
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

pub fn write(game: &GameData, scena: &Scena) -> Result<Vec<u8>, WriteError> {
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
	let mut f = OutBytes::new();
	let mut g = OutBytes::new();
	let mut func_table = OutBytes::new();
	let mut strings = OutBytes::new();

	f.sized_string::<10>(path)?;
	f.sized_string::<14>(map)?;
	f.u16(town.0);
	f.u16(bgm.0);
	f.u16(item.0); f.u16(item.1);
	f.multiple_loose::<8, _>(&[0xFF; 4], includes, |g, a| { g.u32(game.lookup.index(a)?); Ok(()) }).strict()?;
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
	for ch in ch { g.u32(game.lookup.index(ch)?); }
	g.u8(0xFF);

	g.label(l_cp_);
	for cp in cp { g.u32(game.lookup.index(cp)?); }
	g.u8(0xFF);

	g.label(l_npcs_);
	for &Npc { ref name, pos, angle, x, cp, frame, ch, flags, init, talk } in npcs {
		strings.string(name)?;
		g.pos3(pos);
		g.i16(angle);
		g.u16(x);
		g.u16(cp);
		g.u16(frame);
		g.u16(ch);
		g.u16(flags.0);
		g.u16(init.0); g.u16(init.1);
		g.u16(talk.0); g.u16(talk.1);
	}

	g.label(l_monsters_);
	for &Monster { ref name, pos, angle, unk1, flags, unk2, battle, flag, unk3 } in monsters {
		strings.string(name)?;
		g.pos3(pos);
		g.i16(angle);
		g.u16(unk1);
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
		g.u16(flags);
		g.u16(func.0); g.u16(func.1);
		g.u16(unk1);
	}

	g.label(l_look_points_);
	for &LookPoint { pos, radius, bubble_pos, flags, func, unk1 } in look_points {
		g.pos3(pos);
		g.u32(radius);
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
		cam_from, cam_at, cam_zoom, cam_pers, cam_deg, cam_limit1, cam_limit2, north,
		flags, town, init, reinit,
	} in entries {
		f.pos3(pos);
		f.u16(chr);
		f.i16(angle);
		f.pos3(cam_from);
		f.pos3(cam_at);
		f.i32(cam_zoom);
		f.i32(cam_pers);
		f.i16(cam_deg);
		f.i16(cam_limit1);
		f.i16(cam_limit2);
		f.i16(north);
		f.u16(flags);
		f.u16(town.0);
		f.u16(init.0); f.u16(init.1);
		f.u16(reinit.0); f.u16(reinit.1);
	}

	Ok(f.concat(g).concat(func_table).concat(strings).finish()?)
}

#[cfg(test)]
mod test {
	use std::path::Path;
	use super::code::InstructionSet;
	use crate::scena::code::decompile::fixup_eddec;
	use crate::util::test::*;
	use crate::gamedata::{Lookup, GameData};

	macro_rules! test {
		($a:item) => {
			#[test_case::test_case(InstructionSet::Fc,    &*FC, true, "../data/fc.extract/01/", "._sn"; "fc")]
			#[test_case::test_case(InstructionSet::FcEvo, &*FC, true, "../data/vita/extract/fc/gamedata/data/data/scenario/0/", ".bin"; "fc_evo")]
			#[test_case::test_case(InstructionSet::Sc,    &*SC, true, "../data/sc.extract/21/", "._sn"; "sc")]
			#[test_case::test_case(InstructionSet::ScEvo, &*SC, true, "../data/vita/extract/sc/gamedata/data/data_sc/scenario/1/", ".bin"; "sc_evo")]
			#[test_case::test_case(InstructionSet::Tc,    &*TC, true, "../data/3rd.extract/21/", "._sn"; "tc")]
			#[test_case::test_case(InstructionSet::TcEvo, &*TC, true, "../data/vita/extract/3rd/gamedata/data/data_3rd/scenario/2/", ".bin"; "tc_evo")]
			$a
		}
	}

	test! {
	#[test_case::test_case(InstructionSet::Fc,    &*FC, false, "../data/fc-voice/scena/", "._SN"; "fc_voice")]
	#[test_case::test_case(InstructionSet::Sc,    &*SC, false, "../data/sc-voice/scena/", "._SN"; "sc_voice")]
	#[test_case::test_case(InstructionSet::Tc,    &*TC, false, "../data/3rd-voice/scena/", "._SN"; "tc_voice")]
	fn roundtrip(iset: InstructionSet, lookup: &dyn Lookup, strict: bool, scenapath: &str, suffix: &str) -> Result<(), Error> {
		let game = GameData { iset, lookup, kai: false };
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

			if let Err(err) = check_roundtrip_flex(strict, &data, |a| super::read(&game, a), |a| super::write(&game, a)) {
				println!("{name}: {err:?}");
				failed = true;
			};
		}

		assert!(!failed);
		Ok(())
	}
	}

	test! {
	fn decompile(iset: InstructionSet, lookup: &dyn Lookup, _strict: bool, scenapath: &str, suffix: &str) -> Result<(), Error> {
		let game = GameData { iset, lookup, kai: false };
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

			let scena = super::read(&game, &data)?;
			for (i, func) in scena.functions.iter().enumerate() {
				let decomp = super::code::decompile::decompile(func).map_err(|e| format!("{name}:{i}: {e}"))?;
				let recomp = super::code::decompile::recompile(&decomp).map_err(|e| format!("{name}:{i}: {e}"))?;
				if &recomp != func {
					println!("{name}:{i}: incorrect recompile");

					let mut ctx = super::text::Context::new().blind();
					ctx.indent += 1;
					super::text::flat_func(&mut ctx, func);
					print!("{}", ctx.output);
					println!("\n======\n");

					let mut ctx = super::text::Context::new().blind();
					ctx.indent += 1;
					super::text::tree_func(&mut ctx, &decomp);
					print!("{}", ctx.output);
					println!("\n======\n");

					let mut ctx = super::text::Context::new().blind();
					ctx.indent += 1;
					super::text::flat_func(&mut ctx, &recomp);
					println!("{}", ctx.output);

					failed = true;
				}
			}
		}

		assert!(!failed);

		Ok(())
	}
	}

	#[test_case::test_case(InstructionSet::Fc, &*FC, "../data/fc.extract/01/", "../data/fc-voice/scena/";  "fc")]
	#[test_case::test_case(InstructionSet::Sc, &*SC, "../data/sc.extract/21/", "../data/sc-voice/scena/";  "sc")]
	#[test_case::test_case(InstructionSet::Tc, &*TC, "../data/3rd.extract/21/","../data/3rd-voice/scena/"; "tc")]
	fn eddec(iset: InstructionSet, lookup: &dyn Lookup, vanilla: impl AsRef<Path>, voice: impl AsRef<Path>) -> Result<(), Error> {
		let game = GameData { iset, lookup, kai: false };
		let mut failed = false;

		let mut paths = std::fs::read_dir(voice)?
			.map(|r| r.unwrap())
			.collect::<Vec<_>>();
		paths.sort_by_key(|dir| dir.path());
		for file in paths {
			let vpath = file.path();
			let vname = vpath.file_name().unwrap().to_str().unwrap();
			let name = vname.replace(' ', "").to_lowercase();
			let path = vanilla.as_ref().join(&name);

			if !path.exists() {
				println!("{} does not exist (from {})", path.display(), vpath.display());
				continue;
			}
			let data = std::fs::read(path)?;
			let vdata = std::fs::read(vpath)?;
			let scena = super::read(&game, &data)?;
			let vscena = match super::read(&game, &vdata) {
				Ok(a) => a,
				Err(err) =>  {
					println!("{name}: {err:?}");
					failed = true;
					continue;
				}
			};

			for (i, (func, vfunc)) in scena.functions.iter().zip(vscena.functions.iter()).enumerate() {
				if let Some(vfunc2) = fixup_eddec(func, vfunc) {
					let decomp = super::code::decompile::decompile(&vfunc2).map_err(|e| format!("{name}:{i}: {e}"));
					let decomp = match decomp {
						Ok(d) => d,
						Err(e) => {
							println!("{name}:{i}: failed to decompile: {e}");
							let mut ctx = super::text::Context::new().blind();
							ctx.indent += 1;
							super::text::flat_func(&mut ctx, func);
							print!("{}", ctx.output);
							println!("\n======\n");

							let mut ctx = super::text::Context::new().blind();
							ctx.indent += 1;
							super::text::flat_func(&mut ctx, vfunc);
							print!("{}", ctx.output);
							println!("\n======\n");

							let mut ctx = super::text::Context::new().blind();
							ctx.indent += 1;
							super::text::flat_func(&mut ctx, &vfunc2);
							print!("{}", ctx.output);
							continue
						}
					};
					let recomp = super::code::decompile::recompile(&decomp).map_err(|e| format!("{name}:{i}: {e}"))?;
					if recomp != vfunc2 {
						println!("{name}:{i}: incorrect recompile");

						let mut ctx = super::text::Context::new().blind();
						ctx.indent += 1;
						super::text::flat_func(&mut ctx, func);
						print!("{}", ctx.output);
						println!("\n======\n");

						let mut ctx = super::text::Context::new().blind();
						ctx.indent += 1;
						super::text::flat_func(&mut ctx, &vfunc2);
						print!("{}", ctx.output);
						println!("\n======\n");

						let mut ctx = super::text::Context::new().blind();
						ctx.indent += 1;
						super::text::flat_func(&mut ctx, &recomp);
						println!("{}", ctx.output);

						failed = true;
					}
				}
			}
		}

		assert!(!failed);

		Ok(())
	}
}
