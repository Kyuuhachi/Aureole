use themelios::scena::*;
use themelios::scena::ed7::{self, PlacementId, AtRollId, SepithId};
use strict_result::Strict;
use themelios::types::BattleId;
use crate::writer::Context;
use crate::common::{self, Result, ContextExt};

pub fn write(mut f: Context, scena: &ed7::Scena) -> Result<()> {
	let ed7::Scena {
		name1,
		name2,
		filename,
		town,
		bgm,
		flags,
		unk1,
		unk2,
		unk3,

		includes,

		entry,
		chcp,
		labels,
		npcs,
		monsters,
		triggers,
		look_points,
		animations,

		sepith,
		at_rolls,
		placements,
		battles,

		functions,
	} = scena;

	// TODO: a lot of these declarations use nonstandard syntax. Will need to refine that later.

	let g = common::game(f.game);
	f.kw("type")?.kw(g)?.kw("scena")?.line()?;

	f.kw("scena")?.suf(":")?.line()?.indent(|f| {
		f.kw("name")?.val(name1)?.val(name2)?.val(filename)?.line()?;
		f.kw("town")?.val(town)?.line()?;
		f.kw("bgm")?.val(bgm)?.line()?;
		f.kw("flags")?.val(flags)?.line()?;
		f.kw("unk")?.val(unk1)?.val(unk2)?.val(unk3)?.line()?;
		for (i, a) in includes.iter().enumerate() {
			if a.0 != 0 {
				f.kw("scp")?.val(&(i as u16))?.val(a)?.line()?;
			}
		}
		Ok(())
	}).strict()?;
	f.line()?;

	for entry in entry {
		f.kw("entry")?.suf(":")?.line()?.indent(|f| {
			f.kw("pos")?.val(&entry.pos)?.line()?;
			f.kw("unk1")?.val(&entry.unk1)?.line()?;
			f.kw("cam_from")?.val(&entry.cam_from)?.line()?;
			f.kw("cam_pers")?.val(&entry.cam_pers)?.line()?;
			f.kw("unk2")?.val(&entry.unk2)?.line()?;
			f.kw("cam_deg")?.val(&entry.cam_deg)?.line()?;
			f.kw("cam_limit")?.val(&entry.cam_limit.0)?.val(&entry.cam_limit.1)?.line()?;
			f.kw("cam_at")?.val(&entry.cam_at)?.line()?;
			f.kw("unk3")?.val(&entry.unk3)?.line()?;
			f.kw("unk4")?.val(&entry.unk4)?.line()?;
			f.kw("flags")?.val(&entry.flags)?.line()?;
			f.kw("town")?.val(&entry.town)?.line()?;
			f.kw("init")?.val(&entry.init)?.line()?;
			f.kw("reinit")?.val(&entry.reinit)?.line()?;
			Ok(())
		}).strict()?;
		f.line()?;
	}

	for (i, chcp) in chcp.iter().enumerate() {
		f.val(&ChcpId(i as u16))?.val(chcp)?.line()?;
	}
	if !chcp.is_empty() {
		f.line()?;
	}

	let mut n = 8;

	for npc in npcs {
		f.kw("npc")?.val(&CharId(n))?.suf(":")?.line()?.indent(|f| {
			f.kw("name")?.val(&npc.name)?.line()?;
			f.kw("pos")?.val(&npc.pos)?.line()?;
			f.kw("angle")?.val(&npc.angle)?.line()?;
			f.kw("flags")?.val(&npc.flags)?.line()?;
			f.kw("unk2")?.val(&npc.unk2)?.line()?;
			f.kw("chcp")?.val(&npc.chcp)?.line()?;
			f.kw("init")?.val(&npc.init)?.line()?;
			f.kw("talk")?.val(&npc.talk)?.line()?;
			f.kw("unk4")?.val(&npc.unk4)?.line()?;
			Ok(())
		}).strict()?;
		n += 1;
		f.line()?;
	}

	for monster in monsters {
		f.kw("monster")?.val(&CharId(n))?.suf(":")?.line()?.indent(|f| {
			f.kw("pos")?.val(&monster.pos)?.line()?;
			f.kw("angle")?.val(&monster.angle)?.line()?;
			f.kw("flags")?.val(&monster.flags)?.line()?;
			f.kw("battle")?.val(&monster.battle)?.line()?;
			f.kw("flag")?.val(&monster.flag)?.line()?;
			f.kw("chcp")?.val(&monster.chcp)?.line()?;
			f.kw("unk2")?.val(&monster.unk2)?.line()?;
			f.kw("stand_anim")?.val(&monster.stand_anim)?.line()?;
			f.kw("walk_anim")?.val(&monster.walk_anim)?.line()?;
			Ok(())
		}).strict()?;
		n += 1;
		f.line()?;
	}

	for (i, tr) in triggers.iter().enumerate() {
		f.val(&TriggerId(i as u16))?.suf(":")?.line()?.indent(|f| {
			f.kw("pos")?;
			write!(f, "({}, {}, {})", tr.pos.0 * 1000., tr.pos.1 * 1000., tr.pos.2 * 1000.)?;
			f.line()?;

			f.kw("radius")?;
			write!(f, "{}", tr.radius * 1000.)?;
			f.line()?;

			f.kw("transform")?;
			f.line()?.indent(|f| {
				for r in &tr.transform {
					write!(f, "{:?} {:?} {:?} {:?}", r[0], r[1], r[2], r[3])?;
					f.line()?;
				}
				Ok(())
			}).strict()?;
			// TODO add a comment with decomposition

			f.kw("unk1")?.val(&tr.unk1)?.line()?;
			f.kw("unk2")?.val(&tr.unk2)?.line()?;
			f.kw("function")?.val(&tr.function)?.line()?;
			f.kw("unk3")?.val(&tr.unk3)?.line()?;
			f.kw("unk4")?.val(&tr.unk4)?.line()?;
			f.kw("unk5")?.val(&tr.unk5)?.line()?;
			f.kw("unk6")?.val(&tr.unk6)?.line()?;

			Ok(())
		}).strict()?;
		f.line()?;
	}

	for (i, lp) in look_points.iter().enumerate() {
		f.val(&LookPointId(i as u16))?.suf(":")?.line()?.indent(|f| {
			f.kw("pos")?.val(&lp.pos)?.line()?;
			f.kw("radius")?.val(&lp.radius)?.line()?;
			f.kw("bubble_pos")?.val(&lp.bubble_pos)?.line()?;
			f.kw("unk1")?.val(&lp.unk1)?.line()?;
			f.kw("unk2")?.val(&lp.unk2)?.line()?;
			f.kw("function")?.val(&lp.function)?.line()?;
			f.kw("unk3")?.val(&lp.unk3)?.line()?;
			f.kw("unk4")?.val(&lp.unk4)?.line()?;
			Ok(())
		}).strict()?;
		f.line()?;
	}

	if let Some(labels) = labels {
		for (i, lb) in labels.iter().enumerate() {
			f.val(&LabelId(i as u16))?.suf(":")?.line()?.indent(|f| {
				f.kw("name")?.val(&lb.name)?.line()?;

				f.kw("pos")?;
				write!(f, "({}, {}, {})", lb.pos.0 * 1000., lb.pos.1 * 1000., lb.pos.2 * 1000.)?;
				f.line()?;

				f.kw("unk1")?.val(&lb.unk1)?.line()?;
				f.kw("unk2")?.val(&lb.unk2)?.line()?;

				Ok(())
			}).strict()?;
			f.line()?;
		}
	} else {
		// need to keep this for roundtripping
		f.kw("labels")?.kw("null")?.line()?.line()?;
	}

	for (i, anim) in animations.iter().enumerate() {
		f.val(&AnimId(i as u16))?.val(&anim.speed)?;
		for val in &anim.frames {
			f.val(val)?;
		}
		f.line()?;
	}
	if !animations.is_empty() {
		f.line()?;
	}

	let junk_sepith = matches!(sepith.as_slice(), &[
		[100, 1, 2, 3, 70, 89, 99, 0],
		[100, 5, 1, 5, 1, 5, 1, 0],
		[100, 5, 1, 5, 1, 5, 1, 0],
		[100, 5, 0, 5, 0, 5, 0, 0],
		[100, 5, 0, 5, 0, 5, 0, 0],
		..
	]);
	if junk_sepith {
		write!(f, "// NB: the first five sepith sets are seemingly junk data.")?;
		f.line()?;
	}
	for (i, sep) in sepith.iter().enumerate() {
		f.val(&SepithId(i as u16))?;
		for val in sep {
			f.val(val)?;
		}
		f.line()?;
		if junk_sepith && i == 4 && sepith.len() != 5 {
			f.line()?;
		}
	}

	if !sepith.is_empty() {
		f.line()?;
	}

	for (i, roll) in at_rolls.iter().enumerate() {
		f.val(&AtRollId(i as u16))?.suf(":")?;
		let names = [
			"none", "hp10", "hp50", "ep10", "ep50", "cp10", "cp50",
			"unk1", "unk2", "unk3", "unk4", "unk5", "unk6", "unk7", "unk8", "unk9",
		];
		let mut first = true;
		for (name, val) in names.iter().zip(roll)  {
			if *val != 0 {
				if !first {
					f.suf(";")?;
				}
				first = false;
				f.kw(name)?.val(val)?;
			}
		}
		f.line()?;
	}

	if !at_rolls.is_empty() {
		f.line()?;
	}

	for (i, plac) in placements.iter().enumerate() {
		f.val(&PlacementId(i as u16))?.suf(":")?.line()?.indent(|f| {
			for (x, y, r) in plac {
				f.kw("pos")?.val(x)?.val(y)?.val(r)?.line()?;
			}
			Ok(())
		}).strict()?;
		f.line()?;
	}

	for (i, btl) in battles.iter().enumerate() {
		f.val(&BattleId(i as u32))?.suf(":")?.line()?.indent(|f| {
			f.kw("flags")?.val(&btl.flags)?.line()?;
			f.kw("level")?.val(&btl.level)?.line()?;
			f.kw("unk1")?.val(&btl.unk1)?.line()?;
			f.kw("vision_range")?.val(&btl.vision_range)?.line()?;
			f.kw("move_range")?.val(&btl.move_range)?.line()?;
			f.kw("can_move")?.val(&btl.can_move)?.line()?;
			f.kw("move_speed")?.val(&btl.move_speed)?.line()?;
			f.kw("unk2")?.val(&btl.unk2)?.line()?;
			f.kw("battlefiled")?.val(&btl.battlefield)?.line()?;

			f.kw("sepith")?;
			if let Some(sepith) = &btl.sepith {
				f.val(sepith)?;
			} else {
				f.kw("null")?;
			}
			f.line()?;

			for setup in &btl.setups {
				f.kw("setup")?.val(&setup.weight)?.suf(":")?.line()?.indent(|f| {
					f.kw("enemies")?;
					for e in &setup.enemies {
						f.val(e)?;
					}
					f.line()?;
					f.kw("placement")?.val(&setup.placement)?.val(&setup.placement_ambush)?.line()?;
					f.kw("bgm")?.val(&setup.bgm)?.val(&setup.bgm)?.line()?;
					f.kw("at_roll")?.val(&setup.at_roll)?.line()?;
					Ok(())
				}).strict()?;
			}

			Ok(())
		}).strict()?;
		f.line()?;
	}

	for (i, func) in functions.iter().enumerate() {
		if i != 0 {
			f.line()?;
		}
		common::func(&mut f, i, func)?;
	}

	Ok(())
}
