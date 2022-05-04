use hamu::read::{In, Le};
use crate::util::{self, InExt, Text};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("read error")]
	Read(#[from] hamu::read::Error),
	#[error("Invalid {0}: {1:X}")]
	Enum(&'static str, u32),
	#[error("text error")]
	Text(#[from] util::TextError),
	#[error("string error")]
	String(#[from] util::StringError),
}

pub type Result<T, E=Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct Quest {
	pub id: u32,
	pub section: u16, // chapter * 10 + is_main
	pub index: u16,
	pub bp: u16,
	pub mira: u16,
	pub flags: (u16, u16, u16),

	pub name: String,
	pub desc: Text,
	pub steps: Vec<Text>,
}

impl Quest {
	pub fn read(i: &[u8]) -> Result<Vec<Self>> {
		let mut i = In::new(i);
		let start = i.clone().u16()?;
		let mut items = Vec::with_capacity(start as usize/2);
		for _ in 0..start/2 {
			let mut i = i.ptr_u16()?;

			let id = i.u32()?;
			let section = i.u16()?;
			let index = i.u16()?;
			let bp = i.u16()?;
			let mira = i.u16()?;
			let flags = (i.u16()?, i.u16()?, i.u16()?);

			let name = i.ptr_u16()?.string()?;
			let desc = Text::read(&mut i.ptr_u16()?)?;
			let mut steps = Vec::with_capacity(16);
			for _ in 0..16 {
				steps.push(Text::read(&mut i.ptr_u16()?)?);
			}
			items.push(Quest {
				id, section, index,
				bp, mira, flags,
				name, desc, steps,
			});
		}
		Ok(items)
	}

	pub fn is_main(&self) -> bool {
		self.section % 10 == 0
	}

	pub fn chapter(&self) -> u16 {
		self.section / 10
	}
}
