use themelios::scena::{FuncRef, CharId};
use themelios::scena::ed6;
use themelios::scena::code::InstructionSet as IS;
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

	let g = common::game(f.game);
	f.kw("type")?.kw(g)?.kw("scena")?.line()?;

	f.kw("scena")?.suf(":")?.line()?.indent(|f| {
		f.kw("name")?.val(path)?.val(map)?.line()?;
		f.kw("town")?.val(town)?.line()?;
		f.kw("bgm")?.val(bgm)?.line()?;
		f.kw("item")?.val(item)?.line()?;
		for (i, a) in includes.iter().enumerate() {
			if let Some(a) = a {
				f.kw("scp")?.val(&(i as u16))?.val(a)?.line()?;
			}
		}
		Ok(())
	}).strict()?;
	f.line()?;

	for entry in entries {
		f.kw("entry")?.suf(":")?.line()?.indent(|f| {
			f.kw("pos")?.val(&entry.pos)?.line()?;
			f.kw("chr")?.val(&entry.chr)?.line()?;
			f.kw("angle")?.val(&entry.angle)?.line()?;
			f.kw("cam_from")?.val(&entry.cam_from)?.line()?;
			f.kw("cam_at")?.val(&entry.cam_at)?.line()?;
			f.kw("cam_zoom")?.val(&entry.cam_zoom)?.line()?;
			f.kw("cam_pers")?.val(&entry.cam_pers)?.line()?;
			f.kw("cam_deg")?.val(&entry.cam_deg)?.line()?;
			f.kw("cam_limit")?.val(&entry.cam_limit1)?.val(&entry.cam_limit2)?.line()?;
			f.kw("north")?.val(&entry.north)?.line()?;
			f.kw("flags")?.val(&entry.flags)?.line()?;
			f.kw("town")?.val(&entry.town)?.line()?;
			f.kw("init")?.val(&entry.init)?.line()?;
			f.kw("reinit")?.val(&entry.reinit)?.line()?;
			Ok(())
		}).strict()?;
		f.line()?;
	}

	let mut chcp = (ch.iter(), cp.iter(), 0);
	loop {
		let ch = chcp.0.next();
		let cp = chcp.1.next();
		if ch.is_none() && cp.is_none() {
			break
		}
		f.kw("chcp")?.val(&chcp.2)?;
		if let Some(ch) = ch {
			f.val(ch)?;
		} else {
			f.kw("null")?;
		}
		if let Some(cp) = cp {
			f.val(cp)?;
		} else {
			f.kw("null")?;
		}
		f.line()?;
		chcp.2 += 1;
	}
	if !ch.is_empty() || !cp.is_empty() {
		f.line()?;
	}

	let mut n = if matches!(f.game.iset, IS::Tc|IS::TcEvo) { 16 } else { 8 };

	for npc in npcs {
		f.kw("npc")?.val(&CharId(n))?.suf(":")?.line()?.indent(|f| {
			f.kw("name")?.val(&npc.name)?.line()?;
			f.kw("pos")?.val(&npc.pos)?.line()?;
			f.kw("angle")?.val(&npc.angle)?.line()?;
			f.kw("x")?.val(&npc.x)?.line()?;
			f.kw("pt")?.val(&npc.cp)?.line()?;
			f.kw("no")?.val(&npc.frame)?.line()?;
			f.kw("bs")?.val(&npc.ch)?.line()?;
			f.kw("flags")?.val(&npc.flags)?.line()?;
			f.kw("init")?.val(&npc.init)?.line()?;
			f.kw("talk")?.val(&npc.talk)?.line()?;
			Ok(())
		}).strict()?;
		n += 1;
		f.line()?;
	}

	for monster in monsters {
		f.kw("monster")?.val(&CharId(n))?.suf(":")?.line()?.indent(|f| {
			f.kw("name")?.val(&monster.name)?.line()?;
			f.kw("pos")?.val(&monster.pos)?.line()?;
			f.kw("angle")?.val(&monster.angle)?.line()?;
			f.kw("unk1")?.val(&monster.unk1)?.line()?;
			f.kw("flags")?.val(&monster.flags)?.line()?;
			f.kw("unk2")?.val(&monster.unk2)?.line()?;
			f.kw("battle")?.val(&monster.battle)?.line()?;
			f.kw("flag")?.val(&monster.flag)?.line()?;
			f.kw("unk3")?.val(&monster.unk3)?.line()?;
			Ok(())
		}).strict()?;
		n += 1;
		f.line()?;
	}

	for (i, trigger) in triggers.iter().enumerate() {
		f.kw("trigger")?.val(&(i as u16))?.suf(":")?.line()?.indent(|f| {
			f.kw("pos1")?.val(&trigger.pos1)?.line()?;
			f.kw("pos2")?.val(&trigger.pos2)?.line()?;
			f.kw("flags")?.val(&trigger.flags)?.line()?;
			f.kw("func")?.val(&trigger.func)?.line()?;
			f.kw("unk1")?.val(&trigger.unk1)?.line()?;
			Ok(())
		}).strict()?;
		f.line()?;
	}

	for (i, lp) in look_points.iter().enumerate() {
		f.kw("look_point")?.val(&(i as u16))?.suf(":")?.line()?.indent(|f| {
			f.kw("pos")?.val(&lp.pos)?.line()?;
			f.kw("radius")?.val(&lp.radius)?.line()?;
			f.kw("bubble_pos")?.val(&lp.bubble_pos)?.line()?;
			f.kw("flags")?.val(&lp.flags)?.line()?;
			f.kw("func")?.val(&lp.func)?.line()?;
			f.kw("unk1")?.val(&lp.unk1)?.line()?;
			Ok(())
		}).strict()?;
		f.line()?;
	}

	for (i, func) in functions.iter().enumerate() {
		if i != 0 {
			f.line()?;
		}
		common::func(&mut f, FuncRef(0, i as u16), func)?;
	}

	Ok(())
}
