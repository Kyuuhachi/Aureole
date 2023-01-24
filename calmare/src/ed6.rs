use themelios::scena::{FuncRef, CharId};
use themelios::scena::ed6;
use themelios::scena::code::{InsnArg as I, InstructionSet};
use strict_result::Strict;
use crate::writer::Context;
use crate::common::{self, Result, ContextExt};

pub fn write(mut f: Context, scena: &ed6::Scena) -> Result<()> {
	let ed6::Scena {
		path,
		map,
		town,
		bgm,
		item,
		includes,
		ch,
		cp,
		npcs,
		monsters,
		triggers,
		look_points,
		entries,
		functions,
	} = scena;

	f.kw("scena")?.kw("ed6")?.suf(":")?.line()?.indent(|f| {
		f.kw("name")?.val(I::String(path))?.val(I::String(map))?.line()?;
		f.kw("town")?.val(I::TownId(town))?.line()?;
		f.kw("bgm")?.val(I::BgmId(bgm))?.line()?;
		f.kw("item")?.val(I::FuncRef(item))?.line()?;
		Ok(())
	}).strict()?;

	for (i, a) in includes.iter().enumerate() {
		if let Some(a) = a {
			f.kw("scp")?.val(I::u16(&(i as u16)))?.val(I::String(a))?.line()?;
		}
	}
	if includes.iter().any(|a| a.is_some()) {
		f.line()?;
	}

	for entry in entries {
		f.kw("entry")?.suf(":")?.line()?.indent(|f| {
			f.kw("pos")?.val(I::Pos3(&entry.pos))?.line()?;
			f.kw("chr")?.val(I::u16(&entry.chr))?.line()?;
			f.kw("angle")?.val(I::Angle(&entry.angle))?.line()?;
			f.kw("cam_from")?.val(I::Pos3(&entry.cam_from))?.line()?;
			f.kw("cam_at")?.val(I::Pos3(&entry.cam_at))?.line()?;
			f.kw("cam_zoom")?.val(I::i32(&entry.cam_zoom))?.line()?;
			f.kw("cam_pers")?.val(I::i32(&entry.cam_pers))?.line()?;
			f.kw("cam_deg")?.val(I::Angle(&entry.cam_deg))?.line()?;
			f.kw("cam_limit")?.val(I::Angle(&entry.cam_limit1))?.val(I::Angle(&entry.cam_limit2))?.line()?;
			f.kw("north")?.val(I::Angle(&entry.north))?.line()?;
			f.kw("flags")?.val(I::u16(&entry.flags))?.line()?;
			f.kw("town")?.val(I::TownId(&entry.town))?.line()?;
			f.kw("init")?.val(I::FuncRef(&entry.init))?.line()?;
			f.kw("reinit")?.val(I::FuncRef(&entry.reinit))?.line()?;
			Ok(())
		}).strict()?;
		f.line()?;
	}

	let mut chcp = (ch.iter(), cp.iter(), 0);
	loop {
		let ch = chcp.0.next();
		let cp = chcp.1.next();
		f.kw("chcp")?.val(I::ChcpId(&chcp.2))?;
		if let Some(ch) = ch {
			f.val(I::String(ch))?;
		} else {
			f.kw("-")?;
		}
		if let Some(cp) = cp {
			f.val(I::String(cp))?;
		} else {
			f.kw("-")?;
		}
		f.line()?;
		chcp.2 += 1;
		if ch.is_none() && cp.is_none() {
			break
		}
	}
	if !ch.is_empty() || !cp.is_empty() {
		f.line()?;
	}

	let mut n = if matches!(f.game.iset, InstructionSet::Tc|InstructionSet::TcEvo) { 16 } else { 8 };

	for npc in npcs {
		f.kw("npc")?.val(I::CharId(&CharId(n)))?.suf(":")?.line()?.indent(|f| {
			f.kw("name")?.val(I::TextTitle(&npc.name))?.line()?;
			f.kw("pos")?.val(I::Pos3(&npc.pos))?.line()?;
			f.kw("angle")?.val(I::Angle(&npc.angle))?.line()?;
			f.kw("x")?.val(I::u16(&npc.x))?.line()?;
			f.kw("pt")?.val(I::ChcpId(&npc.cp))?.line()?;
			f.kw("no")?.val(I::u16(&npc.frame))?.line()?;
			f.kw("bs")?.val(I::ChcpId(&npc.ch))?.line()?;
			f.kw("flags")?.val(I::CharFlags(&npc.flags))?.line()?;
			f.kw("init")?.val(I::FuncRef(&npc.init))?.line()?;
			f.kw("talk")?.val(I::FuncRef(&npc.talk))?.line()?;
			Ok(())
		}).strict()?;
		n += 1;
		f.line()?;
	}

	for monster in monsters {
		f.kw("monster")?.val(I::CharId(&CharId(n)))?.suf(":")?.line()?.indent(|f| {
			f.kw("name")?.val(I::TextTitle(&monster.name))?.line()?;
			f.kw("pos")?.val(I::Pos3(&monster.pos))?.line()?;
			f.kw("angle")?.val(I::Angle(&monster.angle))?.line()?;
			f.kw("unk1")?.val(I::u16(&monster.unk1))?.line()?;
			f.kw("flags")?.val(I::CharFlags(&monster.flags))?.line()?;
			f.kw("unk2")?.val(I::i32(&monster.unk2))?.line()?;
			f.kw("battle")?.val(I::BattleId(&monster.battle))?.line()?;
			f.kw("flag")?.val(I::Flag(&monster.flag))?.line()?;
			f.kw("unk3")?.val(I::u16(&monster.unk3))?.line()?;
			Ok(())
		}).strict()?;
		n += 1;
		f.line()?;
	}

	for (i, trigger) in triggers.iter().enumerate() {
		f.kw("trigger")?.val(I::u16(&(i as u16)))?.suf(":")?.line()?.indent(|f| {
			f.kw("pos1")?.val(I::Pos3(&trigger.pos1))?.line()?;
			f.kw("pos2")?.val(I::Pos3(&trigger.pos2))?.line()?;
			f.kw("flags")?.val(I::u16(&trigger.flags))?.line()?;
			f.kw("func")?.val(I::FuncRef(&trigger.func))?.line()?;
			f.kw("unk1")?.val(I::u16(&trigger.unk1))?.line()?;
			Ok(())
		}).strict()?;
		f.line()?;
	}

	for (i, lp) in look_points.iter().enumerate() {
		f.kw("look_point")?.val(I::LookPointId(&(i as u16)))?.suf(":")?.line()?.indent(|f| {
			f.kw("pos")?.val(I::Pos3(&lp.pos))?.line()?;
			f.kw("radius")?.val(I::u32(&lp.radius))?.line()?;
			f.kw("bubble_pos")?.val(I::Pos3(&lp.bubble_pos))?.line()?;
			f.kw("flags")?.val(I::LookPointFlags(&lp.flags))?.line()?;
			f.kw("func")?.val(I::FuncRef(&lp.func))?.line()?;
			f.kw("unk1")?.val(I::u16(&lp.unk1))?.line()?;
			Ok(())
		}).strict()?;
		f.line()?;
	}

	for (i, func) in functions.iter().enumerate() {
		f.line()?;
		common::func(&mut f, FuncRef(0, i as u16), func)?;
	}

	Ok(())
}
