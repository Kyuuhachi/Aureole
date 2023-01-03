
use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use themelios_scena::gamedata::GameData;
use themelios_scena::scena::code::InstructionSet;
use themelios_scena::types::NameId;
use themelios_scena::util::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ED7Name {
	pub id: NameId,
	pub name: String,
	pub chcp1: Option<String>,
	pub chcp2: Option<String>,
	pub ms1: Option<String>,
	pub ms2: Option<String>,
}

pub fn read_ed7(game: &GameData, data: &[u8]) -> Result<Vec<ED7Name>, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));
	let mut table = Vec::new();
	let fileref = |a| if a == 0 { Ok(None) } else { game.lookup.name(a).map(Some) };
	loop {
		let id = NameId(f.u16()?);
		let name = f.ptr()?.string()?;
		let chcp1 = fileref(f.u32()?)?;
		let chcp2 = fileref(f.u32()?)?;
		let ms1 = fileref(f.u32()?)?;
		let ms2 = fileref(f.u32()?)?;
		if id == NameId(999) { break }
		table.push(ED7Name { id, name, chcp1, chcp2, ms1, ms2 });
	}
	Ok(table)
}

pub fn write_ed7(game: &GameData, table: &[ED7Name]) -> Result<Vec<u8>, WriteError> {
	let mut f = OutBytes::new();
	let mut g = OutBytes::new();
	let fileref = |a| Option::map_or(a, Ok(0), |a| game.lookup.index(a));
	for name in table {
		f.u16(name.id.0);
		f.delay_u16(g.here());
		f.u32(fileref(name.chcp1.as_deref())?);
		f.u32(fileref(name.chcp2.as_deref())?);
		f.u32(fileref(name.ms1.as_deref())?);
		f.u32(fileref(name.ms2.as_deref())?);
		g.string(&name.name)?;

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
