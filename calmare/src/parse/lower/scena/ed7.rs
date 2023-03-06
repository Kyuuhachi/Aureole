use super::*;

use themelios::scena::ed7::*;

#[derive(Debug, Clone)]
pub struct Header {
	pub name: (String, String, String),
	pub town: TownId,
	pub bgm: BgmId,
	pub flags: u32,
	pub unk: (u8, u16, u8),
	pub scp: [FileId; 6],
}

#[derive(Debug, Clone, Default)]
struct ScenaBuild {
	header: One<Header>,
	entry: One<Entry>,
	chcp: Many<ChcpId, FileId>,
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
			scena.entry.set(d.head.span(), Entry {
				pos, unk1, cam_from, cam_pers, unk2, cam_deg, cam_limit,
				cam_at, unk3, unk4, flags, town, init, reinit,
			});
		}
		"chcp" => {
			parse_data!(d, ctx => (S(s, n), v));
			scena.chcp.insert(d.head.key.0 | s, n, v);
		}
		"npc" => {
		}
		"monster" => {
		}
		"look_point" => {
		}
		"label" => {
		}
		"anim" => {
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