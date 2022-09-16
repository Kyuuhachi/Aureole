use std::{collections::HashMap, path::{Path, PathBuf}, io, fs::File, ops::Range};
use mapr::Mmap;
use hamu::read::{le::*, coverage::*};
use snafu::prelude::*;

use crate::decompress;
use crate::util;

#[derive(Debug, snafu::Snafu)]
pub enum Error {
	#[snafu(display("{source}"), context(false))]
	Read { source: ReadError, backtrace: snafu::Backtrace },

	#[snafu(display("{source}"), context(false))]
	Io { source: std::io::Error, backtrace: snafu::Backtrace },

	#[snafu(display("{source}"))]
	Encoding { source: util::DecodeError, backtrace: snafu::Backtrace },

	#[snafu(display("while reading {}\n{source}", dirpath.display()))]
	Archive {
		dirpath: PathBuf,
		#[snafu(source(from(Error, Box::new)), backtrace)]
		source: Box<Error>,
	},
}

#[derive(Debug)]
pub struct Archive {
	dat: Mmap,
	names: HashMap<String, usize>,
	entries: Vec<Entry>,
}

#[derive(Clone, Debug)]
pub struct Entry {
	pub index: usize,
	pub name: String,
	pub unk1: usize,
	pub unk2: usize,
	pub timestamp: u32,
	range: Range<usize>,
}

impl Entry {
	#[allow(clippy::len_without_is_empty)]
	pub fn len(&self) -> usize {
		self.range.end - self.range.start
	}
}

impl Archive {
	pub fn new(path: impl AsRef<Path>, num: u8) -> Result<Archive, Error> {
		let (dir, dat) = Self::dir_dat(path, num);
		Self::from_dir_dat(&File::open(dir)?, &File::open(dat)?)
	}

	pub fn dir_dat(path: impl AsRef<Path>, num: u8) -> (PathBuf, PathBuf) {
		let mut dir = path.as_ref().to_owned();
		let mut dat = path.as_ref().to_owned();
		dir.push(format!("ED6_DT{:02X}.dir", num));
		dat.push(format!("ED6_DT{:02X}.dat", num));
		(dir, dat)
	}

	pub fn from_dir_dat(dir: &File, dat: &File) -> Result<Archive, Error> {
		let dir = unsafe { Mmap::map(dir)? };
		let dat = unsafe { Mmap::map(dat)? };

		let mut names = HashMap::new();
		let mut entries = Vec::new();

		{
			let mut dir = Coverage::new(Bytes::new(&dir));
			let mut dat = Coverage::new(Bytes::new(&dat));
			dir.check(b"LB DIR\x1A\0")?;
			dat.check(b"LB DAT\x1A\0")?;
			let count = dir.u64()?;
			dat.check_u64(count)?;
			dat.check_u32(20 + count as u32 * 4)?;

			for index in 0..count as usize {
				let name = dir.slice(12)?;
				let name = util::decode(name).context(EncodingSnafu)?;
				let name = if let Some((name, ext)) = name.split_once('.') {
					format!("{}.{}", name.trim_end(), ext)
				} else {
					name.to_owned()
				};
				let name = name.to_lowercase();

				dir.check_u32(0)?; // I don't know what this is. It's nonzero on a few files in 3rd, and some sources (which are me in the past) say it's a second timestamp
				let unk1 = dir.u32()? as usize;
				let unk2 = dir.u32()? as usize;
				let len = dir.u32()? as usize;
				let timestamp = dir.u32()?;
				let offset = dir.u32()? as usize;

				dat.check_u32((offset+len) as u32)?;
				dat.clone().at(offset)?.slice(len)?;

				let entry = Entry {
					index,
					name: name.to_owned(),
					unk1,
					unk2,
					timestamp,
					range: offset..offset+len,
				};

				entries.push(entry);
				if name != "/_______.___" {
					names.insert(name.to_owned(), index);
				}
			}

			assert!(dir.uncovered().is_empty());
			assert!(dat.uncovered().is_empty());
		}

		Ok(Archive {
			dat,
			names,
			entries,
		})
	}


	pub fn name(&self, index: usize) -> Option<&str> {
		let ent = self.entries.get(index)?;
		Some(ent.name.as_str())
	}

	pub fn entry(&self, name: &str) -> Option<&Entry> {
		let index = *self.names.get(name)?;
		self.entries.get(index)
	}

	pub fn get(&self, name: &str) -> Option<&[u8]> {
		let ent = self.entry(name)?;
		Some(&self.dat[ent.range.clone()])
	}

	pub fn entries(&self) -> &[Entry] {
		&self.entries
	}
}

// TODO should this even be part of this? Not sure if it's general enough to be meaningful.
#[derive(Debug)]
pub struct Archives {
	names: HashMap<String, u16>,
	archives: HashMap<u16, Archive>,
}

impl Archives {
	pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
		let mut names = HashMap::new();
		let mut archives = HashMap::new();
		for num in 0..=255u16 {
			let (dirpath, datpath) = Archive::dir_dat(&path, num as u8);
			let dir = match File::open(&dirpath) {
				Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
				e => e?,
			};
			let dat = File::open(&datpath)?;
			let arch = Archive::from_dir_dat(&dir, &dat).context(ArchiveSnafu { dirpath })?;
			for ent in arch.entries() {
				names.insert(ent.name.to_owned(), num);
			}
			archives.insert(num, arch);
		}
		Ok(Self {
			names,
			archives,
		})
	}

	pub fn name(&self, index: [u8; 4]) -> Option<&str> {
		let arch  = u16::from_le_bytes([index[0], index[1]]);
		let index = u16::from_le_bytes([index[2], index[3]]);
		self.archives.get(&arch)?.name(index as usize)
	}

	pub fn get(&self, name: &str) -> Option<&[u8]> {
		let arch = *self.names.get(name)?;
		self.archives.get(&arch)?.get(name)
	}

	pub fn get_decomp(&self, name: &str) -> Option<std::io::Result<Vec<u8>>> {
		self.get(name).map(decompress::decompress)
	}

	pub fn archives(&self) -> impl Iterator<Item=(u8, &Archive)> {
		self.archives.iter().map(|(a, b)| (*a as u8, b))
	}
}
