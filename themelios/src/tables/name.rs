use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _, Label};
use crate::types::*;
use themelios_common::util::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ED6Name {
	pub id: NameId,
	pub name: TString,
	pub chip1: (FileId, FileId),
	pub chip2: (FileId, FileId),
	pub ms1: FileId,
	pub ms2: FileId,
	pub stch: FileId,
}

impl ED6Name {
	// This does not roundtrip in SC and 3rd: there's an unreferenced entry labeled "Joshua" just before 999.

	pub fn read(game: Game, data: &[u8]) -> Result<(Vec<ED6Name>, Vec<ED6Name>), ReadError> {
		let f = &mut Reader::new(data);
		if game.base() != BaseGame::Fc {
			let s1 = f.u16()? as usize;
			let t1 = Self::read_chunk(game, f, NameId(999))?;
			ensure!(f.pos() == s1);
			let t2 = Self::read_chunk(game, f, NameId(2999))?;
			Ok((t1, t2))
		} else {
			let t1 = Self::read_chunk(game, f, NameId(999))?;
			let t2 = Vec::new();
			Ok((t1, t2))
		}
	}

	fn read_chunk(game: Game, f: &mut Reader, endid: NameId) -> Result<Vec<ED6Name>, ReadError> {
		let mut t = Vec::new();
		loop {
			let r = Self::read_one(game, f)?;
			if r.id == endid {
				break
			} else {
				t.push(r);
			}
		}
		Ok(t)
	}

	fn read_one(game: Game, f: &mut Reader) -> Result<ED6Name, ReadError> {
		let g = &mut f.ptr16()?;
		let id = NameId(g.u16()?);
		g.check_u16(0)?;
		let ch1 = FileId(g.u32()?);
		let ch2 = FileId(g.u32()?);
		let cp1 = FileId(g.u32()?);
		let cp2 = FileId(g.u32()?);
		let ms1 = FileId(g.u32()?);
		let ms2 = FileId(g.u32()?);
		let stch = if game.base() != BaseGame::Fc {
			FileId(g.u32()?)
		} else {
			FileId::NONE
		};
		let name = TString(g.ptr16()?.string()?);
		Ok(ED6Name { id, name, chip1: (ch1, cp1), chip2: (ch2, cp2), ms1, ms2, stch })
	}

	pub fn write(game: Game, t1: &[ED6Name], t2: &[ED6Name]) -> Result<Vec<u8>, WriteError> {
		let mut f = Writer::new();
		let mut g = Writer::new();
		if game.base() != BaseGame::Fc {
			let l = Label::new();
			f.delay16(l);
			Self::write_chunk(game, &mut f, &mut g, t1, NameId(999))?;
			f.label(l);
			Self::write_chunk(game, &mut f, &mut g, t2, NameId(2999))?;
		} else {
			Self::write_chunk(game, &mut f, &mut g, t1, NameId(999))?;
			ensure!(t2.is_empty());
		}
		f.append(g);
		Ok(f.finish()?)
	}

	fn write_chunk(game: Game, f: &mut Writer, g: &mut Writer, t: &[ED6Name], endid: NameId) -> Result<(), WriteError> {
		for i in t {
			Self::write_one(game, f, g, i)?;
		}
		Self::write_one(game, f, g, &ED6Name {
			id: endid,
			name: TString::from(" "),
			chip1: (FileId::NONE, FileId::NONE),
			chip2: (FileId::NONE, FileId::NONE),
			ms1: FileId::NONE,
			ms2: FileId::NONE,
			stch: FileId::NONE,
		})?;
		Ok(())
	}

	fn write_one(game: Game, f: &mut Writer, g: &mut Writer, i: &ED6Name) -> Result<(), WriteError> {
		f.delay16(g.here());
		g.u16(i.id.0);
		g.u16(0);
		g.u32(i.chip1.0.0);
		g.u32(i.chip2.0.0);
		g.u32(i.chip1.1.0);
		g.u32(i.chip2.1.0);
		g.u32(i.ms1.0);
		g.u32(i.ms2.0);
		if game.base() != BaseGame::Fc {
			g.u32(i.stch.0);
		} else {
			ensure!(i.stch == FileId::NONE);
		};
		let mut h = g.ptr16();
		h.string(&i.name.0)?;
		g.append(h);
		Ok(())
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ED7Name {
	pub id: NameId,
	pub name: TString,
	pub chip1: FileId,
	pub chip2: FileId,
	pub ms1: FileId,
	pub ms2: FileId,
}

impl ED7Name {
	pub fn read(data: &[u8]) -> Result<Vec<ED7Name>, ReadError> {
		let mut f = Reader::new(data);
		let mut table = Vec::new();
		loop {
			let id = NameId(f.u16()?);
			let name = TString(f.ptr16()?.string()?);
			let chip1 = FileId(f.u32()?);
			let chip2 = FileId(f.u32()?);
			let ms1 = FileId(f.u32()?);
			let ms2 = FileId(f.u32()?);
			if id == NameId(999) { break }
			table.push(ED7Name { id, name, chip1, chip2, ms1, ms2 });
		}
		Ok(table)
	}

	pub fn write(table: &[ED7Name]) -> Result<Vec<u8>, WriteError> {
		let mut f = Writer::new();
		let mut g = Writer::new();
		for name in table {
			f.u16(name.id.0);
			f.delay16(g.here());
			f.u32(name.chip1.0);
			f.u32(name.chip2.0);
			f.u32(name.ms1.0);
			f.u32(name.ms2.0);
			g.string(&name.name.0)?;
		}

		f.u16(999);
		f.delay16(g.here());
		f.u32(0);
		f.u32(0);
		f.u32(0);
		f.u32(0);
		g.string(" ")?;

		f.append(g);
		Ok(f.finish()?)
	}
}
