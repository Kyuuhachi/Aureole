use super::*;

use themelios::scena::ed7::*;

themelios::util::newtype!(CharDefId, u16);
newtype!(CharDefId, "char");

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
	battles: Many<BattleId, Battle>,
}

pub fn lower(file: &File, lookup: Option<&dyn Lookup>) {
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

	let scp = scena.chcp.finish(|a| a.0 as usize);

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
	for m in scena.npcs_monsters.finish(|a| a.0 as usize) {
		match m {
			NpcOrMonster::Npc(n) => npcs.push(n),
			NpcOrMonster::Monster(m) => monsters.push(m),
		}
	}
	let npcs = npcs;
	let monsters = monsters;

	let look_points = scena.look_points.finish(|a| a.0 as usize);
	let labels = scena.labels.finish(|a| a.0 as usize);
	let animations = scena.animations.finish(|a| a.0 as usize);
	let sepith = scena.sepith.finish(|a| a.0 as usize);
	let at_rolls = scena.at_rolls.finish(|a| a.0 as usize);
	let battles = scena.battles.finish(|a| a.0 as usize);
}

fn lower_data(scena: &mut ScenaBuild, ctx: &Context, d: &Data) -> Result<()> {
	match d.head.key.1.as_str() {
		"scena" => {
			let mut scp: [One<FileId>; 6] = [(); 6].map(|_| One::Empty);
			parse_data!(d, ctx => (), {
				name: _,
				town: _,
				bgm: _,
				flags: _,
				unk: _,
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
			let scp = scp.map(|a| a.optional().unwrap_or(FileId(0)));
			scena.header.set(d.head.span(), Header { name, town, bgm, flags, unk, scp });
		}
		"entry" => {
			parse_data!(d, ctx => (), {
				pos: _,
				unk1: _,
				cam_from: _,
				cam_pers: _,
				unk2: _,
				cam_deg: _,
				cam_limit: _,
				cam_at: _,
				unk3: _,
				unk4: _,
				flags: _,
				town: _,
				init: _,
				reinit: _,
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
				name: _,
				pos: _,
				angle: _,
				flags: _,
				unk2: _,
				chcp: _,
				init: _,
				talk: _,
				unk4: _,
			});
			scena.npcs_monsters.insert(d.head.key.0 | s, n, NpcOrMonster::Npc(Npc {
				name, pos, angle, flags, unk2,
				chcp, init, talk, unk4,
			}));
		}
		"monster" => {
			parse_data!(d, ctx => S(s, n), {
				pos: _,
				angle: _,
				flags: _,
				battle: _,
				flag: _,
				chcp: _,
				unk2: _,
				stand_anim: _,
				walk_anim: _,
			});
			scena.npcs_monsters.insert(d.head.key.0 | s, n, NpcOrMonster::Monster(Monster {
				pos, angle, flags, battle, flag,
				chcp, unk2, stand_anim, walk_anim,
			}));
		}
		"look_point" => {
			parse_data!(d, ctx => S(s, n), {
				pos: _,
				radius: _,
				bubble_pos: _,
				unk1: _,
				unk2: _,
				function: _,
				unk3: _,
				unk4: _,
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
			parse_data!(d, ctx => S(s, n), {
				name: _,
				pos: _ => |l: &Data| {
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
					parse_data!(l, ctx => FPos3(x,y,z));
					Ok((x / 1000., y / 1000., z / 1000.))
				},
				unk1: _,
				unk2: _,
			});
			scena.labels.insert(d.head.key.0 | s, n, Label {
				name,
				pos,
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
		}
		"at_roll" => {
		}
		"placement" => {
		}
		"battle" => {
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
