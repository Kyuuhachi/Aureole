use std::path::Path;
use hamu::read::le::*;
use crate::lookup::ED6Lookup;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
	/// This field is only informative: it might be useful for consumers, but this module does not make use of it.
	pub index: u16,
	pub name: String,
	/// There are only three files in 3rd/dt29 where this field is nonzero.
	/// In those cases, it looks like a timestamp pointing to 2009-08-12 21:58:33.
	pub unk1: u32,
	pub compressed_size: usize,
	/// Unknown. In many cases it is equal to `compressed_size`, but not always.
	pub unk3: usize,
	/// Usually equal to `compressed_size`, but in sc/dt31 it is bigger. The difference is filled with null bytes.
	pub archived_size: usize,
	pub timestamp: u32,
	pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatEntry {
	pub offset: usize,
	pub end: usize,
}

pub fn read_dir(data: &[u8]) -> Result<Vec<DirEntry>, hamu::read::Error> {
	let mut f = Reader::new(data);
	f.check(b"LB DIR\x1A\0")?;
	let count = f.u64()? as usize;

	let mut items = Vec::with_capacity(count);
	assert_eq!(items.capacity(), count);

	for index in 0..count {
		if f.clone().check(b"/_______.___").is_ok() {
			break
		}

		let name = normalize_name(&cp932::decode_lossy(f.slice(12)?));
		let unk1            = f.u32()?;
		let compressed_size = f.u32()? as usize;
		let unk3            = f.u32()? as usize;
		let archived_size   = f.u32()? as usize;
		let timestamp       = f.u32()?;
		let offset          = f.u32()? as usize;

		items.push(DirEntry {
			index: index as u16,
			name,
			unk1,
			compressed_size,
			unk3,
			archived_size,
			timestamp,
			offset,
		});
	}

	for _ in items.len()..count {
		f.check(b"/_______.___")?;
		f.check_u32(0)?;
		f.check_u32(0)?;
		f.check_u32(0)?;
		f.check_u32(0)?;
		f.check_u32(0)?;
		f.check_u32(0)?;
	}
	Ok(items)
}

pub fn read_dat(data: &[u8]) -> Result<Vec<DatEntry>, hamu::read::Error> {
	let mut f = Reader::new(data);
	f.check(b"LB DAT\x1A\0")?;
	let count = f.u64()? as usize;

	let mut items = Vec::with_capacity(count);
	assert_eq!(items.capacity(), count);
	for _ in 0..count {
		let offset = f.u32()? as usize;
		let end = f.clone().u32()? as usize;
		items.push(DatEntry { offset, end });
	}
	f.u32()?;
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
