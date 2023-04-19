//! Utilities for reading ED6 PC's .dir/.dat archives.
//!
//! There is currently no support for writing archives; this may be added later.
use gospel::read::{Reader, Le as _};

/// An entry in a .dir file,
///
/// As far as I am aware, only three of the fields are actually used by the games: `name`, `compressed_size`, and `offset`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
	/// The name of the file.
	pub name: String,
	/// There are only three files in 3rd/dt29 where this field is nonzero.
	/// In those cases, it looks like a timestamp pointing to 2009-08-12 21:58:33.
	pub unk1: u32,
	/// The size of the file data in the archive.
	pub compressed_size: usize,
	/// Unknown. In many cases it is equal to `compressed_size`, but not always.
	/// In other cases it is a power of two that is often fairly consistent with adjacent files, but is wholly uncorrelated with any file sizes.
	pub unk3: usize,
	/// Usually equal to `compressed_size`, but in SC/dt31 it is bigger. The difference is filled with null bytes.
	pub archived_size: usize,
	/// A unix timestamp, presumably when the file was last edited. Timezone is unknown.
	pub timestamp: u32,
	/// Offset in the .dat file where the data starts.
	pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatEntry {
	pub offset: usize,
	pub end: usize,
}

/// Reads the contents of entries from a .dir file.
///
/// In many cases, .dir files contain a number of trailing entries named `/_______.___`.
/// These entries are not returned, but the capacity of the returned Vec is set to accomodate them.
pub fn read_dir(data: &[u8]) -> Result<Vec<DirEntry>, gospel::read::Error> {
	let mut f = Reader::new(data);
	f.check(b"LB DIR\x1A\0")?;
	let count = f.u64()? as usize;

	let mut items = Vec::with_capacity(count);

	for _ in 0..count {
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

	assert_eq!(items.capacity(), count);
	Ok(items)
}

pub fn read_dat(data: &[u8]) -> Result<Vec<DatEntry>, gospel::read::Error> {
	let mut f = Reader::new(data);
	f.check(b"LB DAT\x1A\0")?;
	let count = f.u64()? as usize;

	let mut items = Vec::with_capacity(count);
	for _ in 0..count {
		let offset = f.u32()? as usize;
		let end = f.clone().u32()? as usize;
		items.push(DatEntry { offset, end });
	}
	f.u32()?;

	assert_eq!(items.capacity(), count);
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
