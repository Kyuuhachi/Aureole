//! Utilities for reading ED6 PC's .dir/.dat archives.
use std::ops::Range;

use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _};

/// A name of a file in an archive.
///
/// Internally this is represented as a `[u8; 12]` usually denoting a 8.3 uppercase shift-jis encoded name, but
/// the public interface treats it like a more normal looking string.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Name([u8; 12]);

impl std::ops::Deref for Name {
	type Target = [u8; 12];

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Default for Name {
	fn default() -> Self {
		Self(*b"/_______.___")
	}
}

impl std::fmt::Debug for Name {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "Name({:?})", self.to_string())
	}
}

impl std::fmt::Display for Name {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let name = cp932::decode_lossy(&self.0);
		let name = name.to_lowercase();
		if let Some((name, ext)) = name.split_once('.') {
			write!(f, "{}.{}", name.trim_end_matches(' '), ext.trim_end_matches(' '))
		} else {
			f.write_str(name.trim_end_matches(' '))
		}
	}
}

impl TryFrom<String> for Name {
	type Error = NameError;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		Name::try_from(value.as_str())
	}
}

impl TryFrom<&str> for Name {
	type Error = NameError;

	fn try_from(name: &str) -> Result<Self, Self::Error> {
		let (_, name) = name.rsplit_once(['/', '\\']).unwrap_or(("", name));
		let name = name.to_uppercase();
		let (name, ext) = name.split_once('.').unwrap_or((&name, ""));
		let name = cp932::encode(name).map_err(|_| NameError)?;
		let ext = cp932::encode(ext).map_err(|_| NameError)?;
		if name.len() > 8 || ext.len() > 3 { return Err(NameError); }
		let mut o = *b"        .   ";
		o[..name.len()].copy_from_slice(&name);
		o[9..][..ext.len()].copy_from_slice(&ext);
		Ok(Name(o))
	}
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NameError;

impl std::fmt::Display for NameError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.write_str("cannot convert to archive name")
	}
}

impl std::error::Error for NameError {}

/// An entry in a .dir file,
///
/// As far as I am aware, only three of the fields are actually used by the games: `name`, `compressed_size`, and `offset`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DirEntry {
	/// The name of the file.
	pub name: Name,
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

impl DirEntry {
	pub fn range(&self) -> Option<Range<usize>> {
		if self.timestamp == 0 {
			None
		} else {
			Some(self.offset .. self.offset + self.archived_size)
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatEntry {
	pub offset: usize,
	pub end: usize,
}

impl DatEntry {
	pub fn range(&self) -> Option<Range<usize>> {
		if self.end == 0 {
			None
		} else {
			Some(self.offset .. self.end)
		}
	}
}

/// Read the list of entries from a .dir file.
///
/// In many cases, .dir files contain a number of trailing entries named `/_______.___`.
/// These entries are retained as well.
pub fn read_dir(data: &[u8]) -> Result<Vec<DirEntry>, gospel::read::Error> {
	let mut f = Reader::new(data);
	f.check(b"LB DIR\x1A\0")?;
	let count = f.u64()? as usize;

	let mut items = Vec::with_capacity(count);

	for _ in 0..count {
		let name            = Name(f.array::<12>()?);
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

	Ok(items)
}

/// Writes a list of entries into a .dir file.
pub fn write_dir(entries: &[DirEntry]) -> Vec<u8> {
	let mut f = Writer::new();
	f.slice(b"LB DIR\x1A\0");
	f.u64(entries.len() as u64);

	for e in entries {
		f.array::<12>(e.name.0);
		f.u32(e.unk1);
		f.u32(e.compressed_size as u32);
		f.u32(e.unk3 as u32);
		f.u32(e.archived_size as u32);
		f.u32(e.timestamp);
		f.u32(e.offset as u32);
	}

	f.finish().unwrap()
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

	Ok(items)
}
