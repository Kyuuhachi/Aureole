use super::*;

use themelios::scena::{ed6::*, code::Code};

#[derive(Debug, Clone)]
pub struct Header {
	pub name: (String, String),
	pub town: TownId,
	pub bgm: BgmId,
	pub item_use: FuncId,
	pub scp: [FileId; 8],
}

#[derive(Debug, Clone, Default)]
struct ScenaBuild {
	header: One<Header>,
	entries: Vec<Entry>,
	ch: Many<ChipId, FileId>,
	cp: Many<ChipId, FileId>,
	chars: Many<CharDefId, NpcOrMonster<Npc, Monster>>,
	triggers: Many<TriggerId, Trigger>,
	look_points: Many<LookPointId, LookPoint>,
	functions: Many<FuncDefId, Code>,
}

pub fn parse(lines: &[Line], ctx: &Context) -> Result<Scena> {
	let mut scena = ScenaBuild::default();
	for line in lines {
		let _ = Parse::new(line, ctx).parse_with(|p| parse_line(&mut scena, p));
	}

	if !scena.header.is_present() {
		Diag::error(Span::new_at(0), "missing 'scena' block").emit();
	}

	let ch = scena.ch.get(|a| a.0 as usize);
	let cp = scena.cp.get(|a| a.0 as usize);
	let (npcs, monsters) = chars(scena.chars);
	let triggers = scena.triggers.get(|a| a.0 as usize);
	let look_points = scena.look_points.get(|a| a.0 as usize);
	let functions = scena.functions.get(|a| a.0 as usize);

	let h = scena.header.get().ok_or(Error)?;

	Ok(Scena {
		path: h.name.0,
		map: h.name.1,
		town: h.town,
		bgm: h.bgm,
		item_use: h.item_use,
		includes: h.scp,
		ch,
		cp,
		npcs,
		monsters,
		triggers,
		look_points,
		entries: scena.entries,
		functions,
	})
}

fn parse_line(scena: &mut ScenaBuild, p: &mut Parse) -> Result<()> {
	let Some(key) = test!(p, Token::Ident(a) => a) else {
		Diag::error(p.next_span(), "expected word").emit();
		p.pos = p.tokens.len();
		return Err(Error);
	};
	if test!(p, Token::Bracket(_)) {
		p.pos -= 2;
	}
	match *key {
		"fn" => {
			let S(s, n) = Val::parse(p)?;
			scena.functions.mark(p.tokens[0].0 | s, n);
			let f = parse_func(p);
			scena.functions.insert(n, f);
		}
		"scena" => {
			scena.header.mark(p.head_span());
			let mut scp = <[One<FileId>; 8]>::default();
			parse_data!(p => {
				name, town, bgm, item_use,
				scp => |p: &mut Parse| {
					let S(s, n) = Val::parse(p)?;
					let n: u32 = n;
					if n >= 8 {
						Diag::error(s, "only values 0-7 allowed").emit();
						return Err(Error)
					}
					scp[n as usize].mark(p.tokens[0].0 | s);
					let v = Val::parse(p)?;
					scp[n as usize].set(v);
					Ok(())
				}
			});
			let scp = scp.map(|a| a.get().unwrap_or(FileId(0)));
			scena.header.set(Header { name, town, bgm, item_use, scp });
		}
		"entry" => {
			parse_data!(p => {
				pos, chr, angle,
				cam_from, cam_at, cam_zoom, cam_pers, cam_deg, cam_limit, north,
				flags, town, init, reinit,
			});
			scena.entries.push(Entry {
				pos, chr, angle,
				cam_from, cam_at, cam_zoom, cam_pers, cam_deg, cam_limit, north,
				flags, town, init, reinit,
			});
		}
		"chip" => {
			let S(s, n) = Val::parse(p)?;
			scena.ch.mark(p.tokens[0].0 | s, n);
			scena.cp.mark(p.tokens[0].0 | s, n);
			let (ch, cp) = Val::parse(p)?;
			if ch != FileId(0) {
				scena.ch.insert(n, ch);
			}
			if cp != FileId(0) {
				scena.cp.insert(n, cp);
			}
		}
		"npc" => {
			let S(s, n) = Val::parse(p)?;
			scena.chars.mark(p.tokens[0].0 | s, n);
			parse_data!(p => {
				name, pos, angle,
				x, cp, frame, ch,
				flags, init, talk,
			});
			scena.chars.insert(n, NpcOrMonster::Npc(Npc {
				name, pos, angle,
				x, cp, frame, ch,
				flags, init, talk,
			}));
		}
		"monster" => {
			let S(s, n) = Val::parse(p)?;
			scena.chars.mark(p.tokens[0].0 | s, n);
			parse_data!(p => {
				name, pos, angle,
				chip, flags, unk2,
				battle, flag, unk3,
			});
			scena.chars.insert(n, NpcOrMonster::Monster(Monster {
				name, pos, angle,
				chip, flags, unk2,
				battle, flag, unk3,
			}));
		}
		"trigger" => {
			let S(s, n) = Val::parse(p)?;
			scena.triggers.mark(p.tokens[0].0 | s, n);
			parse_data!(p => {
				pos1, pos2, flags, func, unk1,
			});
			scena.triggers.insert(n, Trigger {
				pos1, pos2, flags, func, unk1,
			});
		}
		"look_point" => {
			let S(s, n) = Val::parse(p)?;
			scena.look_points.mark(p.tokens[0].0 | s, n);
			parse_data!(p => {
				pos, radius, bubble_pos, flags, func, unk1
			});
			scena.look_points.insert(n, LookPoint {
				pos, radius, bubble_pos, flags, func, unk1
			});
		}
		_ => {
			Diag::error(p.tokens[0].0, "unknown declaration")
				.note(p.tokens[0].0, "expected \
					'scena', 'entry', 'chip', 'npc', 'monster', \
					'trigger', 'look_point', 'fn'")
				.emit();
			p.pos = p.tokens.len();
		}
	}
	Ok(())
}
