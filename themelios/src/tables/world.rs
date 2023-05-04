use glam::IVec2;
use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _};
use themelios_common::util::*;
use crate::types::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ED6World {
	pub scena: FileId,
	pub pos: IVec2,
}

impl ED6World {
	pub fn read(data: &[u8]) -> Result<Vec<ED6World>, ReadError> {
		let mut f = Reader::new(data);
		let mut table = Vec::new();
		loop {
			let scena = FileId(f.u32()?);
			let pos = IVec2 { x: f.i32()?, y: f.i32()? };
			if scena == FileId(0xFFFFFFFF) {
				break
			}
			table.push(ED6World { scena, pos });
		}
		Ok(table)
	}

	pub fn write(table: &[ED6World]) -> Result<Vec<u8>, WriteError> {
		let mut f = Writer::new();
		for a in table {
			f.u32(a.scena.0);
			f.i32(a.pos.x);
			f.i32(a.pos.y);
		}
		Ok(f.finish()?)
	}
}

#[test]
fn test() -> Result<(), Box<dyn std::error::Error>> {
	ED6World::read(&std::fs::read("../data/fc.extract/02/t_world._dt")?)?;
	ED6World::read(&std::fs::read("../data/sc.extract/22/t_world._dt")?)?;
	ED6World::read(&std::fs::read("../data/3rd.extract/22/t_world._dt")?)?;
	Ok(())
}
