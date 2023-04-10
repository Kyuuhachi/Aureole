use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _};
use crate::types::SoundId;
use themelios_common::util::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ED7Sound {
	pub id: SoundId,
	pub file_num: u16,
	pub unk1: u16,
	pub unk2: [u8; 4],
}

impl ED7Sound {
	pub fn read(data: &[u8]) -> Result<Vec<ED7Sound>, ReadError> {
		let mut f = Reader::new(data);
		let mut table = Vec::new();
		let mut xs = Vec::new();
		for _ in 0..f.clone().u16()?/2 {
			xs.push(f.u16()? as usize);
		}
		let mut chunkstart = 0;
		for (start, end) in xs.iter().copied().zip(xs.iter().copied().skip(1).chain(Some(f.len()))) {
			let mut id = chunkstart;
			chunkstart += 500;
			let mut g = f.clone().at(start)?;
			while g.pos() < end {
				let se = ED7Sound {
					id: SoundId(id),
					file_num: g.u16()?,
					unk1: g.u16()?,
					unk2: g.array()?,
				};
				id += 1;
				table.push(se);
			}
		}
		Ok(table)
	}

	pub fn write(table: &[ED7Sound]) -> Result<Vec<u8>, WriteError> {
		let mut table = table.to_owned();
		table.sort_by_key(|a| a.id);
		let mut f = Writer::new();
		let mut g = Writer::new();
		let mut next_chunk = 0;
		let mut next_id = 0;
		for se in table {
			while se.id.0 >= next_chunk {
				f.delay16(g.here());
				next_id = next_chunk;
				next_chunk += 500;
			}
			while se.id.0 > next_id {
				g.u16(799);
				g.u16(1);
				g.slice(&[0, 0, 0, 0]);
				next_id += 1;
			}
			g.u16(se.file_num);
			g.u16(se.unk1);
			g.slice(&se.unk2);
			next_id += 1;
		}
		f.append(g);
		Ok(f.finish()?)
	}
}
