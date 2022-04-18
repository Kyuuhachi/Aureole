use std::{
	fs::File,
	path::Path,
	ops::Range,
	borrow::Cow,
};
use chrono::NaiveDateTime;
use mapr::Mmap;
use anyhow::{Result, Context};
use hamu::read::{In, Le};

#[derive(Clone)]
pub struct Entry {
	pub name: [u8; 12],
	pub size: usize,
	pub timestamp: NaiveDateTime,
	range: Range<usize>,
}

impl std::fmt::Debug for Entry {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Entry")
			.field("name", &format_args!("b\"{}\"",
				self.name.into_iter()
					.flat_map(std::ascii::escape_default)
					.map(|a| a as char)
					.collect::<String>()
			))
			.field("size", &self.size)
			.field("timestamp", &self.timestamp)
			.field("range", &self.range)
			.finish()
	}
}

#[derive(Debug)]
pub struct Archive {
	dat: Mmap,
	pub entries: Vec<Entry>,
}

impl Archive {
	pub fn new(path: impl AsRef<Path>, num: usize) -> Result<Archive> {
		let mut dir_path = path.as_ref().to_owned();
		let mut dat_path = path.as_ref().to_owned();
		dir_path.push(format!("ED6_DT{:02X}.dir", num));
		dat_path.push(format!("ED6_DT{:02X}.dat", num));
		let dir = unsafe { Mmap::map(&File::open(&dir_path)?)? };
		let dat = unsafe { Mmap::map(&File::open(&dat_path)?)? };

		let mut i = In::new(&dir);
		let mut j = In::new(&dat);
		i.check(b"LB DIR\x1A\0")?;
		j.check(b"LB DAT\x1A\0")?;
		let count = i.u64()?;
		j.check_u64(count)?;
		j.check_u32(20 + count as u32 * 4)?;

		let mut entries = Vec::new();
		for _ in 0..count {
			let name = i.array::<12>()?;
			i.check_u32(0)?; // I don't know what this is
			let len = i.u32()? as usize;
			let size = i.u32()? as usize;
			i.check_u32(len as u32)?;
			let timestamp = NaiveDateTime::from_timestamp(i.u32()? as i64, 0);
			let offset = i.u32()? as usize;
			j.check_u32((offset + len) as u32)?;
			j.clone().at(offset)?.slice(len)?;

			if &name != b"/_______.___" {
				entries.push(Entry {
					name,
					size,
					timestamp,
					range: offset..offset+len,
				});
			}
		}
		assert!(i.uncovered().is_empty());
		assert!(j.uncovered().is_empty());

		Ok(Archive {
			dat,
			entries,
		})
	}

	pub fn entry(&self, entry: usize) -> Result<&Entry> {
		Ok(self.entries.get(entry).with_context(|| format!("index {}", entry))?)
	}

	pub fn get(&self, entry: usize) -> Result<Cow<[u8]>> {
		let ent = self.entry(entry)?;
		let data = &self.dat[ent.range.clone()];
		Ok(Cow::Borrowed(data))
	}
}
