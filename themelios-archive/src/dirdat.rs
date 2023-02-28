use std::path::Path;
use hamu::read::le::*;
use crate::lookup::ED6Lookup;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
	pub name: String,
	pub unk1: u32,
	pub unk2: usize,
	pub unk3: usize,
	pub archived_size: usize,
	pub timestamp: u32,
	pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatEntry<'a> {
	pub offset: usize,
	pub data: &'a [u8],
}

pub fn read_dir(data: &[u8]) -> Result<Vec<DirEntry>, hamu::read::Error> {
	let mut f = Reader::new(data);
	f.check(b"LB DIR\x1A\0")?;
	let count = f.u64()?;

	let mut items = Vec::with_capacity(count as usize);
	for _ in 0..count {
		let name = cp932::decode_lossy(f.slice(12)?);
		let unk1 = f.u32()?; // Zero in all but a few files in 3rd; in those cases it looks kinda like a timestamp
		let unk2 = f.u32()? as usize;
		let unk3 = f.u32()? as usize;
		let archived_size = f.u32()? as usize;
		let timestamp = f.u32()?;
		let offset = f.u32()? as usize;

		items.push(DirEntry {
			name: normalize_name(&name),
			unk1,
			unk2,
			unk3,
			archived_size,
			timestamp,
			offset,
		});
	}
	Ok(items)
}

pub fn read_dat(data: &[u8]) -> Result<Vec<DatEntry>, hamu::read::Error> {
	let mut f = Reader::new(data);
	f.check(b"LB DAT\x1A\0")?;
	let count = f.u64()?;

	let mut items = Vec::with_capacity(count as usize);
	for _ in 0..count {
		let offset = f.u32()? as usize;
		let end = f.clone().u32()? as usize;
		let data = f.clone().at(offset)?.slice(end - offset)?;
		items.push(DatEntry { offset, data });
	}
	f.check_u32(f.pos() as u32)?;
	Ok(items)
}

pub fn normalize_name(name: &str) -> String {
	let name = name.to_lowercase();
	if let Some((name, ext)) = name.split_once('.') {
		format!("{}.{ext}", name.trim_end_matches(' '))
	} else {
		name
	}
}

pub fn to_lookup(dir: impl AsRef<Path>) -> std::io::Result<ED6Lookup> {
	let dir = dir.as_ref();
	let mut x = [(); 64].map(|_| Vec::new());
	for (n, x) in x.iter_mut().enumerate() {
		let Ok(data) = std::fs::read(dir.join(format!("ED6_DT{n:02X}.dir"))) else { continue };
		*x = read_dir(&data)
			.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
			.into_iter()
			.filter(|a| a.name != "/_______.___")
			.map(|a| match n {
				0x06 => format!("apl/{}", a.name),
				0x26 => format!("apl2/{}", a.name),
				0x07 => format!("npl/{}", a.name),
				0x27 => format!("npl2/{}", a.name),
				0x08 => format!("mons/{}", a.name),
				0x28 => format!("mons2/{}", a.name),
				_ => a.name,
			})
			.collect();
	}
	Ok(ED6Lookup::new(x))
}
