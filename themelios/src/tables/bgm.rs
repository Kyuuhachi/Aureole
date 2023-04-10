use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _};
use crate::types::BgmId;
use themelios_common::util::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ED7Bgm {
	pub loop_start: u32,
	pub loop_end: u32,
	pub file_num: u32,
	pub id: BgmId,
	pub loops: bool,
}

impl ED7Bgm {
	pub fn read(data: &[u8]) -> Result<Vec<ED7Bgm>, ReadError> {
		let mut f = Reader::new(data);
		let mut table = Vec::new();
		loop {
			let loop_start = f.u32()?;
			let loop_end = f.u32()?;
			let file_num = f.u32()?;
			let id = BgmId(f.u16()?);
			let loops = cast_bool(f.u16()?)?;
			if id == BgmId(7999) { break }
			table.push(ED7Bgm { loop_start, loop_end, file_num, id, loops });
		}
		Ok(table)
	}

	pub fn write(table: &[ED7Bgm]) -> Result<Vec<u8>, WriteError> {
		let mut f = Writer::new();
		for bgm in table {
			f.u32(bgm.loop_start);
			f.u32(bgm.loop_end);
			f.u32(bgm.file_num);
			f.u16(bgm.id.0);
			f.u16(bgm.loops.into());
		}
		f.u32(0);
		f.u32(0);
		f.u32(7999);
		f.u16(7999);
		f.u16(0);

		Ok(f.finish()?)
	}
}
