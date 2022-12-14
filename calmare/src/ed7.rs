use themelios::scena::{FuncRef, CharId};
use themelios::scena::ed7;
use themelios::scena::code::InsnArg as I;
use strict_result::Strict;
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

		field_sepith,
		at_rolls,
		placements,
		battles,

		functions,
	} = scena;

	f.kw("scena")?.kw("ed7")?.suf(":")?.line()?.indent(|f| {
		f.kw("name")?.val(I::String(name1))?.val(I::String(name2))?.val(I::String(filename))?.line()?;
		f.kw("town")?.val(I::TownId(town))?.line()?;
		f.kw("bgm")?.val(I::BgmId(bgm))?.line()?;
		f.kw("flags")?.val(I::u32(flags))?.line()?;
		f.kw("unk")?.val(I::u8(unk1))?.val(I::u16(unk2))?.val(I::u8(unk3))?.line()?;
		Ok(())
	}).strict()?;
	f.line()?;

	for (i, a) in includes.iter().enumerate() {
		if let Some(a) = a {
			f.kw("scp")?.val(I::u16(&(i as u16)))?.val(I::String(a))?.line()?;
		}
	}
	if includes.iter().any(|a| a.is_some()) {
		f.line()?;
	}

	for entry in entry {
		f.kw("entry")?.suf(":")?.line()?.indent(|f| {
			f.kw("pos")?.val(I::Pos3(&entry.pos))?.line()?;
			f.kw("unk1")?.val(I::u32(&entry.unk1))?.line()?;
			f.kw("cam_from")?.val(I::Pos3(&entry.cam_from))?.line()?;
			f.kw("cam_pers")?.val(I::u32(&entry.cam_pers))?.line()?;
			f.kw("unk2")?.val(I::u16(&entry.unk2))?.line()?;
			f.kw("cam_deg")?.val(I::u16(&entry.cam_deg))?.line()?;
			f.kw("cam_limit")?.val(I::u16(&entry.cam_limit1))?.val(I::u16(&entry.cam_limit2))?.line()?;
			f.kw("cam_at")?.val(I::Pos3(&entry.cam_at))?.line()?;
			f.kw("unk3")?.val(I::u16(&entry.unk3))?.line()?;
			f.kw("unk4")?.val(I::u16(&entry.unk4))?.line()?;
			f.kw("flags")?.val(I::u16(&entry.flags))?.line()?;
			f.kw("town")?.val(I::TownId(&entry.town))?.line()?;
			f.kw("init")?.val(I::FuncRef(&entry.init))?.line()?;
			f.kw("reinit")?.val(I::FuncRef(&entry.reinit))?.line()?;
			Ok(())
		}).strict()?;
		f.line()?;
	}

	for (i, chcp) in chcp.iter().enumerate() {
		f.kw("chcp")?.val(I::ChcpId(&(i as u16)))?;
		if let Some(chcp) = chcp {
			f.val(I::String(chcp))?;
		} else {
			f.kw("-")?;
		}
		f.line()?;
	}
	if !chcp.is_empty() {
		f.line()?;
	}

	let mut n = 8;

	for npc in npcs {
		f.kw("npc")?.val(I::CharId(&CharId(n)))?.suf(":")?.line()?.indent(|f| {
			f.kw("name")?.val(I::TextTitle(&npc.name))?.line()?;
			f.kw("pos")?.val(I::Pos3(&npc.pos))?.line()?;
			f.kw("angle")?.val(I::Angle(&npc.angle))?.line()?;
			f.kw("unk1")?.val(I::u16(&npc.unk1))?.line()?;
			f.kw("unk2")?.val(I::u16(&npc.unk2))?.line()?;
			f.kw("unk3")?.val(I::u16(&npc.unk3))?.line()?;
			f.kw("init")?.val(I::FuncRef(&npc.init))?.line()?;
			f.kw("talk")?.val(I::FuncRef(&npc.talk))?.line()?;
			f.kw("unk4")?.val(I::u32(&npc.unk4))?.line()?;
			Ok(())
		}).strict()?;
		n += 1;
		f.line()?;
	}

	for monster in monsters {
		f.kw("monster")?.val(I::CharId(&CharId(n)))?.suf(":")?.line()?.indent(|f| {
			f.kw("pos")?.val(I::Pos3(&monster.pos))?.line()?;
			f.kw("angle")?.val(I::Angle(&monster.angle))?.line()?;
			f.kw("unk1")?.val(I::u16(&monster.unk1))?.line()?;
			f.kw("battle")?.val(I::BattleId(&monster.battle))?.line()?;
			f.kw("flag")?.val(I::Flag(&monster.flag))?.line()?;
			f.kw("chcp")?.val(I::u16(&monster.chcp))?.line()?;
			f.kw("unk2")?.val(I::u16(&monster.unk2))?.line()?;
			f.kw("stand_anim")?.val(I::u32(&monster.stand_anim))?.line()?;
			f.kw("walk_anim")?.val(I::u32(&monster.walk_anim))?.line()?;
			Ok(())
		}).strict()?;
		n += 1;
		f.line()?;
	}

	for (i, tr) in triggers.iter().enumerate() {
		f.kw("trigger")?.val(I::u16(&(i as u16)))?.suf(":")?.line()?.indent(|f| {
			f.kw("pos")?;
			write!(f, "({}, {}, {})", tr.pos.0, tr.pos.1, tr.pos.2)?;
			f.line()?;

			f.kw("radius")?;
			write!(f, "{}", tr.radius)?;
			f.line()?;

			f.kw("transform")?;
			f.line()?.indent(|f| {
				for r in &tr.transform {
					write!(f, "({}, {}, {}, {})", r[0], r[1], r[2], r[3])?;
					f.line()?;
				}
				Ok(())
			}).strict()?;

			f.kw("unk1")?.val(I::u8(&tr.unk1))?.line()?;
			f.kw("unk2")?.val(I::u16(&tr.unk2))?.line()?;
			f.kw("function")?.val(I::FuncRef(&tr.function))?.line()?;
			f.kw("unk3")?.val(I::u8(&tr.unk3))?.line()?;
			f.kw("unk4")?.val(I::u16(&tr.unk4))?.line()?;
			f.kw("unk5")?.val(I::u32(&tr.unk5))?.line()?;
			f.kw("unk6")?.val(I::u32(&tr.unk6))?.line()?;

			Ok(())
		}).strict()?;
		f.line()?;
	}

	for (i, lp) in look_points.iter().enumerate() {
		f.kw("look_point")?.val(I::LookPointId(&(i as u16)))?.suf(":")?.line()?.indent(|f| {
			f.kw("pos")?.val(I::Pos3(&lp.pos))?.line()?;
			f.kw("radius")?.val(I::u32(&lp.radius))?.line()?;
			f.kw("bubble_pos")?.val(I::Pos3(&lp.bubble_pos))?.line()?;
			f.kw("unk1")?.val(I::u8(&lp.unk1))?.line()?;
			f.kw("unk2")?.val(I::u16(&lp.unk2))?.line()?;
			f.kw("function")?.val(I::FuncRef(&lp.function))?.line()?;
			f.kw("unk3")?.val(I::u8(&lp.unk3))?.line()?;
			f.kw("unk4")?.val(I::u16(&lp.unk4))?.line()?;
			Ok(())
		}).strict()?;
		f.line()?;
	}

	if let Some(labels) = labels {
		for (i, lb) in labels.iter().enumerate() {
			f.kw("label")?.val(I::u16(&(i as u16)))?.suf(":")?.line()?.indent(|f| {
				f.kw("name")?.val(I::TextTitle(&lb.name))?.line()?;

				f.kw("pos")?;
				write!(f, "({}, {}, {})", lb.pos.0, lb.pos.1, lb.pos.2)?;
				f.line()?;

				f.kw("unk1")?.val(I::u16(&lb.unk1))?.line()?;
				f.kw("unk2")?.val(I::u16(&lb.unk2))?.line()?;

				Ok(())
			}).strict()?;
			f.line()?;
		}
		if !labels.is_empty() {
			f.line()?;
		}
	} else {
		// need to keep this for roundtripping
		f.kw("labels")?.kw("-")?.line()?.line()?;
	}

	for (i, anim) in animations.iter().enumerate() {
		f.kw("anim")?.val(I::u16(&(i as u16)))?.suf(":")?;
		f.val(I::Time(&(anim.speed as u32)))?.val(I::u8(&anim.unk))?.suf(";")?;
		for val in &anim.frames {
			f.val(I::u8(val))?;
		}
		f.line()?;
	}
	if !animations.is_empty() {
		f.line()?;
	}

	let junk_sepith = matches!(field_sepith.as_slice(), &[
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
	for (i, sep) in field_sepith.iter().enumerate() {
		f.kw("sepith")?.val(I::u16(&(i as u16)))?.suf(":")?;
		for val in sep {
			f.val(I::u8(val))?;
		}
		f.line()?;
		if junk_sepith && i == 4 && field_sepith.len() != 5 {
			f.line()?;
		}
	}
	if !field_sepith.is_empty() {
		f.line()?;
	}

	for (i, roll) in at_rolls.iter().enumerate() {
		f.kw("at_roll")?.val(I::u16(&(i as u16)))?.suf(":")?;
		for val in roll {
			f.val(I::u8(val))?;
		}
		f.line()?;
	}
	if !at_rolls.is_empty() {
		f.line()?;
	}

	for (i, plac) in placements.iter().enumerate() {
		f.kw("battle_placement")?.val(I::u16(&(i as u16)))?.suf(":")?;
		for (i, (x, y, r)) in plac.iter().enumerate() {
			f.val(I::u8(x))?;
			f.val(I::u8(y))?;
			f.val(I::Angle(r))?;
			if i != 7 {
				f.suf(",")?;
			}
		}
		f.line()?;
	}
	if !placements.is_empty() {
		f.line()?;
	}

	for (i, btl) in battles.iter().enumerate() {
		f.kw("battle")?.val(I::BattleId(&(i as u32).into()))?.suf(":")?.line()?.indent(|f| {
			f.kw("flags")?.val(I::u16(&btl.flags))?.line()?;
			f.kw("level")?.val(I::u16(&btl.level))?.line()?;
			f.kw("unk1")?.val(I::u8(&btl.unk1))?.line()?;
			f.kw("vision_range")?.val(I::u8(&btl.vision_range))?.line()?;
			f.kw("move_range")?.val(I::u8(&btl.move_range))?.line()?;
			f.kw("can_move")?.val(I::u8(&btl.can_move))?.line()?;
			f.kw("move_speed")?.val(I::u16(&btl.move_speed))?.line()?;
			f.kw("unk2")?.val(I::u16(&btl.unk2))?.line()?;
			f.kw("battlefiled")?.val(I::String(&btl.battlefield))?.line()?;

			f.kw("sepith")?;
			if let Some(sepith) = &btl.sepith {
				f.val(I::u16(sepith))?;
			} else {
				f.kw("-")?;
			}
			f.line()?;

			for setup in &btl.setups {
				f.kw("setup")?.val(I::u8(&setup.weight))?.suf(":")?.line()?.indent(|f| {
					f.kw("enemies")?;
					for e in &setup.enemies {
						if let Some(e) = e {
							f.val(I::String(e))?;
						} else {
							f.kw("-")?;
						}
					}
					f.line()?;
					f.kw("placement")?.val(I::u16(&setup.placement))?.val(I::u16(&setup.placement_ambush))?.line()?;
					f.kw("bgm")?.val(I::BgmId(&setup.bgm))?.val(I::BgmId(&setup.bgm))?.line()?;
					f.kw("at_roll")?.val(I::u16(&setup.at_roll))?.line()?;
					Ok(())
				}).strict()?;
			}

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

#[test]
fn test() {
	use themelios::gamedata::GameData;
	let path = "../data/zero/data/scena/r0100.bin";
	let data = std::fs::read(path).unwrap();
	let scena = themelios::scena::ed7::read(GameData::ZERO_KAI, &data).unwrap();
	let c = Context::new(std::io::stdout());
	write(c, &scena).unwrap();
}
