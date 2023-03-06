use super::*;

use themelios::scena::ed7::*;

themelios::util::newtype!(CharDefId, u16);
newtype!(CharDefId, "char");

newtype!(SepithId, "sepith");
newtype!(AtRollId, "at_roll");
newtype!(PlacementId, "placement");

#[derive(Debug, Clone)]
pub struct Header {
	pub name: (String, String, String),
	pub town: TownId,
	pub bgm: BgmId,
	pub flags: u32,
	pub unk: (u8, u16, u8),
	pub scp: [FileId; 6],
}

#[derive(Debug, Clone)]
pub enum NpcOrMonster {
	Npc(Npc),
	Monster(Monster),
}

#[derive(Debug, Clone, Default)]
struct ScenaBuild {
	header: One<Header>,
	entry: One<Entry>,
	chcp: Many<ChcpId, FileId>,
	npcs_monsters: Many<CharDefId, NpcOrMonster>,
	look_points: Many<LookPointId, LookPoint>,
	labels: Many<LabelId, Label>,
	animations: Many<AnimId, Animation>,
	sepith: Many<SepithId, [u8; 8]>,
	at_rolls: Many<AtRollId, [u8; 16]>,
	placements: Many<PlacementId, [(u8, u8, Angle); 8]>,
	battles: Many<BattleId, Battle>,
}

pub fn lower(file: &File, lookup: Option<&dyn Lookup>) -> Result<Scena> {
	let ctx = &Context::new(file, lookup);
	let mut scena = ScenaBuild::default();
	for decl in &file.decls {
		match decl {
			Decl::Function(Function { id, body }) => {
				// lower_scena_function(body);
			}
			Decl::Data(d) => {
				let _ = lower_data(&mut scena, ctx, d);
			},
		}
	}

	let misorder = scena.npcs_monsters.0.iter()
		.skip_while(|a| matches!(&a.1.1, NpcOrMonster::Npc(_)))
		.find(|a| matches!(&a.1.1, NpcOrMonster::Npc(_)));
	if let Some((k, (s, _))) = misorder {
		let (_, (prev, _)) = scena.npcs_monsters.0.range(..k).last().unwrap();
		Diag::error(*prev, "monsters must come after npcs")
			.note(*s, "is before this npc")
			.emit();
	}

	let mut npcs = Vec::new();
	let mut monsters = Vec::new();
	for m in scena.npcs_monsters.get(|a| a.0 as usize) {
		match m {
			NpcOrMonster::Npc(n) => npcs.push(n),
			NpcOrMonster::Monster(m) => monsters.push(m),
		}
	}
	let h = scena.header.get().ok_or_else(|| {
		Diag::error(Span::new_at(0), "missing 'scena' block").emit();
		Error
	})?;

	Ok(Scena {
		name1: h.name.0,
		name2: h.name.1,
		filename: h.name.2,
		town: h.town,
		bgm: h.bgm,
		flags: h.flags,
		includes: h.scp,
		chcp: scena.chcp.get(|a| a.0 as usize),
		labels: Some(scena.labels.get(|a| a.0 as usize)),
		npcs,
		monsters,
		triggers: todo!(),
		look_points: scena.look_points.get(|a| a.0 as usize),
		animations: scena.animations.get(|a| a.0 as usize),
		entry: scena.entry.get(),
		functions: todo!(),
		sepith: scena.sepith.get(|a| a.0 as usize),
		at_rolls: scena.at_rolls.get(|a| a.0 as usize),
		placements: scena.placements.get(|a| a.0 as usize),
		battles: scena.battles.get(|a| a.0 as usize),
		unk1: h.unk.0,
		unk2: h.unk.1,
		unk3: h.unk.2,
	})
}

fn lower_data(scena: &mut ScenaBuild, ctx: &Context, d: &Data) -> Result<()> {
	match d.head.key.1.as_str() {
		"scena" => {
			let mut scp: [One<FileId>; 6] = [(); 6].map(|_| One::Empty);
			parse_data!(d, ctx => (), {
				name,
				town,
				bgm,
				flags,
				unk,
				scp => |l: &Data| {
					parse_data!(l, ctx => (S(s, n), v));
					let n: u32 = n;
					if n >= 6 {
						Diag::error(s, "only values 0-5 allowed").emit();
						return Err(Error)
					}
					scp[n as usize].set(l.head.key.0 | s, v);
					Ok(())
				}
			});
			let scp = scp.map(|a| a.get().unwrap_or(FileId(0)));
			scena.header.set(d.head.span(), Header { name, town, bgm, flags, unk, scp });
		}
		"entry" => {
			parse_data!(d, ctx => (), {
				pos,
				unk1,
				cam_from,
				cam_pers,
				unk2,
				cam_deg,
				cam_limit,
				cam_at,
				unk3,
				unk4,
				flags,
				town,
				init,
				reinit,
			});
			scena.entry.set(d.head.key.0, Entry {
				pos, unk1, cam_from, cam_pers, unk2, cam_deg, cam_limit,
				cam_at, unk3, unk4, flags, town, init, reinit,
			});
		}
		"chcp" => {
			parse_data!(d, ctx => (S(s, n), v));
			scena.chcp.insert(d.head.key.0 | s, n, v);
		}
		"npc" => {
			parse_data!(d, ctx => S(s, n), {
				name,
				pos,
				angle,
				flags,
				unk2,
				chcp,
				init,
				talk,
				unk4,
			});
			scena.npcs_monsters.insert(d.head.key.0 | s, n, NpcOrMonster::Npc(Npc {
				name, pos, angle, flags, unk2,
				chcp, init, talk, unk4,
			}));
		}
		"monster" => {
			parse_data!(d, ctx => S(s, n), {
				pos,
				angle,
				flags,
				battle,
				flag,
				chcp,
				unk2,
				stand_anim,
				walk_anim,
			});
			scena.npcs_monsters.insert(d.head.key.0 | s, n, NpcOrMonster::Monster(Monster {
				pos, angle, flags, battle, flag,
				chcp, unk2, stand_anim, walk_anim,
			}));
		}
		"look_point" => {
			parse_data!(d, ctx => S(s, n), {
				pos,
				radius,
				bubble_pos,
				unk1,
				unk2,
				function,
				unk3,
				unk4,
			});
			scena.look_points.insert(d.head.key.0 | s, n, LookPoint {
				pos,
				radius,
				bubble_pos,
				unk1,
				unk2,
				function,
				unk3,
				unk4,
			});
		}
		"label" => {
			struct FPos3(f32, f32, f32);
			impl Val for FPos3 {
				fn parse(p: &mut Parse) -> Result<Self> {
					if let Some((x, y, z)) = p.term("")? {
						Ok(FPos3(x, y, z))
					} else {
						Diag::error(p.pos(), "expected fpos3").emit();
						Err(Error)
					}
				}
			}
			parse_data!(d, ctx => S(s, n), {
				name,
				pos,
				unk1,
				unk2,
			});
			let FPos3(x, y, z) = pos;
			scena.labels.insert(d.head.key.0 | s, n, Label {
				name,
				pos: (x / 1000., y / 1000., z / 1000.),
				unk1,
				unk2,
			});
		}
		"anim" => {
			parse_data!(d, ctx => (S(s, n), speed, frames));
			scena.animations.insert(d.head.key.0 | s, n, Animation {
				speed,
				frames,
			});
		}
		"sepith" => {
			parse_data!(d, ctx => (S(s, n), values));
			scena.sepith.insert(d.head.key.0 | s, n, values);
		}
		"at_roll" => {
			let mut values = [(); 16].map(|_| One::<u8>::Empty);
			macro fd($n:literal) {
				|l: &Data| {
					parse_data!(l, ctx => v);
					values[$n].set(l.head.key.0, v);
					Ok(())
				}
			}
			parse_data!(d, ctx => S(s, n), {
				none => fd!(0),
				hp10 => fd!(1),
				hp50 => fd!(2),
				ep10 => fd!(3),
				ep50 => fd!(4),
				cp10 => fd!(5),
				cp50 => fd!(6),
				unk1 => fd!(7),
				unk2 => fd!(8),
				unk3 => fd!(9),
				unk4 => fd!(10),
				unk5 => fd!(11),
				unk6 => fd!(12),
				unk7 => fd!(13),
				unk8 => fd!(14),
				unk9 => fd!(15),
			});
			let values = values.map(|a| a.get().unwrap_or_default());
			scena.at_rolls.insert(d.head.key.0 | s, n, values);
		}
		"placement" => {
			let mut vs = Vec::new();
			parse_data!(d, ctx => S(s, n), {
				pos => |l: &Data| {
					parse_data!(l, ctx => v);
					vs.push(v);
					Ok(())
				}
			});
			if let Ok(vs) = vs.try_into() {
				scena.placements.insert(d.head.key.0 | s, n, vs);
			} else {
				scena.placements.insert(d.head.key.0 | s, n, [(0,0,Angle(180));8]);
				Diag::error(d.head.span(), "needs exactly 8 'pos'").emit();
			}
		}
		"battle" => {
			let mut setups = Vec::new();
			parse_data!(d, ctx => S(s, n), {
				flags, level, unk1, vision_range, move_range,
				can_move, move_speed, unk2, battlefield, sepith,
				setup => |l: &Data| {
					parse_data!(l, ctx => weight, {
						enemies, placement, bgm, at_roll
					});
					let (placement, placement_ambush) = placement;
					let (bgm, bgm_ambush) = bgm;
					if setups.len() >= 4 {
						Diag::error(l.head.span(), "only up to 4 setups allowed").emit();
						return Err(Error)
					}
					setups.push(BattleSetup {
						weight,
						enemies,
						placement,
						placement_ambush,
						bgm,
						bgm_ambush,
						at_roll,
					});
					Ok(())
				}
			});
			scena.battles.insert(d.head.key.0 | s, n, Battle {
				flags, level, unk1, vision_range, move_range,
				can_move, move_speed, unk2, battlefield, sepith,
				setups,
			});
		}
		_ => {
			Diag::error(d.head.key.0, "unknown declaration")
				.note(d.head.key.0, "expected 'scena', 'entry', 'chcp', 'npc', 'monster', 'look_point', \
					  'label', 'anim', 'sepith', 'at_roll', 'placement', 'battle', 'fn'")
				.emit();
		}
	}
	Ok(())
}
