use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use themelios_scena::types::*;
use themelios_scena::util::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ED7Name {
	pub id: NameId,
	pub name: TString,
	pub chcp1: FileId,
	pub chcp2: FileId,
	pub ms1: FileId,
	pub ms2: FileId,
}

pub fn read_ed7(data: &[u8]) -> Result<Vec<ED7Name>, ReadError> {
	let mut f = Coverage::new(Reader::new(data));
	let mut table = Vec::new();
	loop {
		let id = NameId(f.u16()?);
		let name = TString(f.ptr()?.string()?);
		let chcp1 = FileId(f.u32()?);
		let chcp2 = FileId(f.u32()?);
		let ms1 = FileId(f.u32()?);
		let ms2 = FileId(f.u32()?);
		if id == NameId(999) { break }
		table.push(ED7Name { id, name, chcp1, chcp2, ms1, ms2 });
	}
	Ok(table)
}

pub fn write_ed7(table: &[ED7Name]) -> Result<Vec<u8>, WriteError> {
	let mut f = Writer::new();
	let mut g = Writer::new();
	for name in table {
		f.u16(name.id.0);
		f.delay_u16(g.here());
		f.u32(name.chcp1.0);
		f.u32(name.chcp2.0);
		f.u32(name.ms1.0);
		f.u32(name.ms2.0);
		g.string(&name.name.0)?;
	}

	f.u16(999);
	f.delay_u16(g.here());
	f.u32(0);
	f.u32(0);
	f.u32(0);
	f.u32(0);
	g.string(" ")?;

	f.append(g);
	Ok(f.finish()?)
}
