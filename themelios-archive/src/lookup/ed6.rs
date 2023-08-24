use std::{collections::HashMap, path::Path};
use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _};

/// Lookup for ED6: *Trails in the Sky FC*, *SC*, and *the 3rd*.
///
/// For the PC versions, file ids tell which *file archive* the file is located in, and the *index*
/// inside these archives. For the PSVita versions, the file ids still follow this scheme, but instead
/// uses a number of .txt files with lists of filenames for resolving the numbers to filenames.
#[derive(Clone)]
pub struct ED6Lookup {
	name: [Vec<String>; 64],
	index: HashMap<String, u32>,
}

impl std::fmt::Debug for ED6Lookup {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let mut d = f.debug_struct("ED6Lookup");
		for (n, x) in self.name.iter().enumerate() {
			if !x.is_empty() {
				d.field(&format!("ED6_DT{n:02X}"), &format_args!("{} entries", x.len()));
			}
		}
		d.finish()
	}
}

impl ED6Lookup {
	/// Creates a lookup resolving to the given names.
	///
	/// The reason for the `64` size is that that's how many archives the games load.
	/// Any indices above that would just segfault, so they are not supported.
	pub fn new(name: [Vec<String>; 64]) -> Self {
		let mut index = HashMap::new();
		for (n, x) in name.iter().enumerate() {
			for (i, v) in x.iter().enumerate() {
				index.insert(v.clone(), (n << 16) as u32 | i as u32);
			}
		}
		Self { name, index }
	}

	/// Get the lists of names.
	pub fn names(&self) -> &[Vec<String>; 64] {
		&self.name
	}
}

impl super::Lookup for ED6Lookup {
	fn name(&self, index: u32) -> Option<String> {
		let (arch, index) = (index >> 16, index & 0xFFFF);
		Some(self.name.get(arch as usize)?.get(index as usize)?.clone())
	}

	fn index(&self, name: &str) -> Option<u32> {
		Some(*self.index.get(name)?)
	}
}

impl ED6Lookup {
	/// Loads the indexes for the PC versions of the games.
	///
	/// `dir` should be a directory containing the `ED6_DTxx.dir/.dat` files, normally located at
	/// `C:\Program Files (x86)\Steam\steamapps\common\Trails in the Sky {FC/SC/the 3rd}`.
	///
	/// The filenames used in the .dir files are uppercase 8.3 format. Later games, as well as filenames given as strings, use lowercase variable-length format, so the index filenames are normalized to the latter format.
	///
	/// For ch/cp files, a prefix is added to denote which archive they are in; names are taken from the Vita's index files.
	pub fn for_pc(dir: impl AsRef<Path>) -> std::io::Result<ED6Lookup> {
		let dir = dir.as_ref();
		dir.read_dir()?;
		let mut x = [(); 64].map(|_| Vec::new());
		for (n, x) in x.iter_mut().enumerate() {
			let Ok(data) = std::fs::read(dir.join(format!("ED6_DT{n:02X}.dir"))) else { continue };
			*x = crate::dirdat::read_dir(&data)
				.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
				.into_iter()
				.filter(|a| a.raw_name != *b"/_______.___")
				.map(|a| match n {
					0x06 => format!("apl/{}", a.name()),
					0x26 => format!("apl2/{}", a.name()),
					0x07 => format!("npl/{}", a.name()),
					0x27 => format!("npl2/{}", a.name()),
					0x09 => format!("mons/{}", a.name()),
					0x29 => format!("mons2/{}", a.name()),
					_ => a.name(),
				})
			.collect();
		}
		Ok(ED6Lookup::new(x))
	}

	/// Loads the indexes for the PSVita (Evolution) versions of the games.
	///
	/// `dir` should be the data directory extracted from `data.psarc`, normally named `data`, `data_sc`, or `data_3rd`.
	///
	/// The index files do not include file extensions, so these are added. Note that the inferred exensions are made to resemble the PC version, *not* the files actually used by the Vita version.
	///
	/// For ch/cp files, a prefix is added to denote which index file they are in.
	pub fn for_vita(dir: impl AsRef<Path>) -> std::io::Result<ED6Lookup> {
		let dir = dir.as_ref();
		dir.read_dir()?;

		fn txt(lines: &mut Vec<String>, path: std::path::PathBuf, format: impl Fn(&str) -> String) -> std::io::Result<()> {
			if let Ok(text) = std::fs::read_to_string(path) {
				for line in text.lines() {
					lines.push(format(&line.to_lowercase()));
				}
			}
			Ok(())
		}

		fn bin(lines: &mut Vec<String>, path: std::path::PathBuf, format: impl Fn(&str) -> String) -> std::io::Result<()> {
			if let Ok(text) = std::fs::read(path) {
				for line in text.chunks_exact(8) {
					let line = std::str::from_utf8(line).unwrap();
					lines.push(format(&line.trim_matches(' ').to_lowercase()));
				}
			}
			Ok(())
		}

		let chcp = |s1| move |s: &str| {
			if s.ends_with('p') {
				format!("{s1}/{s}._cp")
			} else {
				format!("{s1}/{s}._ch")
			}
		};

		let suf = |suf| move |s: &str| {
			format!("{s}.{suf}")
		};

		let mut x = [(); 64].map(|_| Vec::new());

		let o = if dir.join("scenario/0").exists() { 0x00 } else { 0x20 };

		txt(&mut x[0x01], dir.join("scenario/0/map.txt"), suf("_sn"))?;
		txt(&mut x[0x21], dir.join("scenario/1/map.txt"), suf("_sn"))?;
		txt(&mut x[0x21], dir.join("scenario/2/map.txt"), suf("_sn"))?;

		txt(&mut x[o+0x03], dir.join("minimap/_minimap.txt"), suf("_ch"))?;

		txt(&mut x[0x06], dir.join("chr/apl_pt.txt"),   chcp("apl"))?;
		txt(&mut x[0x26], dir.join("chr/apl2_pt.txt"),  chcp("apl2"))?;
		txt(&mut x[0x07], dir.join("chr/npl_pt.txt"),   chcp("npl"))?;
		txt(&mut x[0x27], dir.join("chr/npl2_pt.txt"),  chcp("npl2"))?;
		txt(&mut x[0x09], dir.join("chr/mons_pt.txt"),  chcp("mons"))?;
		txt(&mut x[0x29], dir.join("chr/mons2_pt.txt"), chcp("mons2"))?;

		bin(&mut x[0x04], dir.join("visual/dt4.txt"), suf("_ch"))?;
		bin(&mut x[0x24], dir.join("visual/dt24.txt"), suf("_ch"))?;

		// There is also system/chrpt[12].txt, not sure what that is for.

		Ok(ED6Lookup::new(x))
	}
}

/// .ed6i reading and writing.
///
/// .ed6i is a simple format that encodes an `ED6Lookup`.
/// You probably won't have much need for these functions, they're mainly intended for use by `themelios`' `indexes` feature.
impl ED6Lookup {
	pub fn read_ed6i(data: &[u8]) -> Result<Self, gospel::read::Error> {
		let mut f = Reader::new(data);
		f.check(b"ED6I")?;
		f.check_u8(0)?; // version
		let mut x = [(); 64].map(|_| Vec::new());
		for i in x.iter_mut() {
			let n = f.u16()?;
			i.reserve(n as usize);
			for _ in 0..n {
				let len = f.u8()? as usize;
				let pos = f.pos();
				let s = f.slice(len)?.to_vec();
				let s = String::from_utf8(s)
					.map_err(|e| gospel::read::Error::Other { pos, source: e.into() })?;
				i.push(s);
			}
		}
		Ok(Self::new(x))
	}

	pub fn write_ed6i(&self) -> Result<Vec<u8>, gospel::write::Error> {
		let mut f = Writer::new();
		f.slice(b"ED6I");
		f.u8(0); // version
		for i in &self.name {
			f.u16(i.len() as u16);
			for s in i {
				f.u8(s.len() as u8);
				f.slice(s.as_bytes());
			}
		}
		f.finish()
	}
}
