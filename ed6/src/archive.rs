use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	io,
	fs::File,
	ops::Range,
};
use mapr::Mmap;
use hamu::read::coverage::*;
use hamu::read::le::*;

use crate::gamedata::GameDataImpl;
use crate::decompress;
use crate::util;

type Backtrace = Box<std::backtrace::Backtrace>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("{source}")]
	Read { #[from] source: hamu::read::Error, backtrace: Backtrace },

	#[error("{source}")]
	Io { #[from] source: std::io::Error, backtrace: Backtrace },

	#[error("{source}")]
	Encoding { #[from] source: util::DecodeError, backtrace: Backtrace },

	#[error("while reading {}\n{source}", dirpath.display())]
	Archive {
		#[backtrace]
		source: Box<Error>,
		dirpath: PathBuf,
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
	pub unk1: u32,
	pub unk2: usize,
	pub unk3: usize,
	pub timestamp: u32,
	range: Range<usize>,
}

impl Entry {
	pub fn len(&self) -> usize {
		self.range.end - self.range.start
	}

	pub fn is_empty(&self) -> bool {
		self.len() == 0
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
				let name = util::decode(name)?;
				let name = if let Some((name, ext)) = name.split_once('.') {
					format!("{}.{}", name.trim_end(), ext)
				} else {
					name.to_owned()
				};
				let name = name.to_lowercase();

				let unk1 = dir.u32()?; // Zero in all but a few files in 3rd; in those cases it looks kinda like a timestamp
				let unk2 = dir.u32()? as usize;
				let unk3 = dir.u32()? as usize;
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
					unk3,
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


	pub fn name(&self, index: usize) -> io::Result<&str> {
		let ent = self.entries.get(index).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
		Ok(ent.name.as_str())
	}

	pub fn index(&self, name: &str) -> io::Result<usize> {
		self.names.get(name).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound)).copied()
	}

	pub fn entry(&self, name: &str) -> io::Result<&Entry> {
		let index = self.index(name)?;
		Ok(self.entries.get(index).unwrap())
	}

	pub fn get(&self, name: &str) -> io::Result<&[u8]> {
		let ent = self.entry(name)?;
		Ok(&self.dat[ent.range.clone()])
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
			let arch = Archive::from_dir_dat(&dir, &dat).map_err(|e| Error::Archive { dirpath, source: e.into() })?;
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

	pub fn archive(&self, n: u16) -> io::Result<&Archive> {
		self.archives.get(&n).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
	}

	pub fn get_decomp(&self, name: &str) -> std::io::Result<Vec<u8>> {
		self.get(name).and_then(decompress::decompress)
	}

	pub fn archives(&self) -> impl Iterator<Item=(u8, &Archive)> {
		self.archives.iter().map(|(a, b)| (*a as u8, b))
	}
}

impl GameDataImpl for Archives {
	fn name(&self, a: u32) -> io::Result<&str> {
		let index = (a & 0xFFFF) as u16;
		let mut arch  = (a >> 16) as u16;
		if arch == 0x1A && !self.archives.contains_key(&0x1A) {
			arch = 0x1B;
		}
		self.archive(arch)?.name(index as usize)
	}

	fn index(&self, name: &str) -> io::Result<u32> {
		let mut arch = *self.names.get(name).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
		let index = self.archive(arch)?.index(name)?;
		if arch == 0x1B {
			arch = 0x1A;
		}
		Ok((index as u32) | (arch as u32) << 16)
	}

	fn get(&self, name: &str) -> io::Result<&[u8]> {
		let arch = *self.names.get(name).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
		self.archive(arch)?.get(name)
	}

	fn get_decomp(&self, name: &str) -> io::Result<Vec<u8>> {
		self.get(name).and_then(decompress::decompress)
	}

	fn list(&self) -> Box<dyn Iterator<Item=&str> + '_> {
		Box::new(
			self.archives()
			.map(|a| a.1)
			.flat_map(|a| a.entries())
			.filter(|a| !a.is_empty())
			.map(|a| a.name.as_str())
		)
	}
}
