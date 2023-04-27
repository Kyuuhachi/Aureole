use std::collections::BTreeMap;

use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _};
use crate::types::*;
use themelios_common::util::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Town {
	pub id: TownId,
	pub name: TString,
	pub kind: u8,
}

impl Town {
	pub fn read(game: Game, data: &[u8]) -> Result<Vec<Town>, ReadError> {
		let mut f = Reader::new(data);
		let mut table = Vec::new();
		let mut pos = Vec::new();
		for i in 0..f.u16()? {
			pos.push((f.u16()? as usize, TownId(i)));
		}
		pos.sort_by_key(|i| i.0);
		for (pos, id) in pos {
			let mut g = f.clone().at(pos)?;
			let name = TString(g.string()?);
			let kind = if game.is_ed7() || !name.is_empty() {
				g.u8()?
			} else {
				0
			};
			table.push(Town { id, name, kind })
		}
		Ok(table)
	}

	pub fn write(game: Game, table: &[Town]) -> Result<Vec<u8>, WriteError> {
		let mut f = Writer::new();
		let mut pos = BTreeMap::new();
		let mut g = Writer::new();
		for town in table {
			pos.insert(town.id, g.here());
			g.string(&town.name.0)?;
			if game.is_ed7() || !town.name.is_empty() {
				g.u8(town.kind);
			} else {
				ensure!(town.kind == 0);
			}
		}

		f.u16(cast(pos.len())?);
		let mut expect = TownId(0);
		for (id, lbl) in pos {
			ensure!(id == expect);
			expect.0 += 1;
			f.delay16(lbl);
		}

		f.append(g);
		Ok(f.finish()?)
	}
}
