use hamu::read::{In, Le};
use crate::util::{self, InExt};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("read error")]
	Read(#[from] hamu::read::Error),
	#[error("Invalid {0}: {1:X}")]
	Enum(&'static str, u32),
	#[error("decode error")]
	Decode(#[from] util::DecodeError),
	#[error("multiple errors")]
	Multi(#[from] util::MultiError<Error>),
}

impl From<util::StringError> for Error {
	fn from(e: util::StringError) -> Self {
		match e {
			util::StringError::Read(e) => e.into(),
			util::StringError::Decode(e) => e.into(),
		}
	}
}

pub type Result<T, E=Error> = std::result::Result<T, E>;

#[allow(non_upper_case_globals)]
mod itemflags { bitflags::bitflags! {
	pub struct ItemFlags: u8 {
		const Battle      = 0x01;
		const Use         = 0x02; // on battle items, whether they can target allies. Otherwise, equal to Book
		const Sell        = 0x04;
		const Discard     = 0x08;
		const TargetEnemy = 0x10;
		const TargetDead  = 0x20;
		const Book        = 0x40; // never used together with Battle
		const _80         = 0x80;
	}
} }
pub use itemflags::ItemFlags;

#[derive(Debug)]
pub struct BaseItem {
	pub id: u16,
	pub flags: ItemFlags,
	pub usable_by: u8,
	pub ty: [u8; 4],
	pub _unk1: u8, // 0 on quartz and key items, 2 on cannons, 1 otherwise
	pub stats: [i16; 10],
	pub limit: u16,
	pub price: u32,
}

#[derive(Debug)]
pub struct Item {
	pub name: String, // todo should be Text
	pub desc: String,
	pub base: BaseItem,
}

impl BaseItem {
	pub fn read_one(i: &mut In) -> Result<Self> {
		let id = i.u16()?;
		let flags = i.u8()?;
		let flags = ItemFlags::from_bits(flags).ok_or(Error::Enum("ItemFlags", flags as u32))?;
		let usable_by = i.u8()?; // Flags in FC, enum in others
		let ty = [ i.u8()?, i.u8()?, i.u8()?, i.u8()? ];
		i.check_u8(0)?;
		let _unk1 = i.u8()?;
		let stats = [
			i.i16()?, i.i16()?, i.i16()?, i.i16()?, i.i16()?,
			i.i16()?, i.i16()?, i.i16()?, i.i16()?, i.i16()?,
		];
		let limit = i.u16()?;
		let price = i.u32()?;

		Ok(BaseItem { id, flags, usable_by, ty, _unk1, stats, limit, price})
	}
}

impl Item {
	pub fn read(i: &[u8], j: &[u8]) -> Result<Vec<Self>> {
		let mut i = In::new(i);
		let mut j = In::new(j);
		let start = i.clone().u16()?;
		j.clone().check_u16(start)?;
		let mut items = Vec::with_capacity(start as usize/2);
		// These are not always in order. This can probably be safely ignored.
		for _ in 0..start/2 {
			let mut i = i.ptr_u16()?;
			let mut j = j.ptr_u16()?;

			let base = BaseItem::read_one(&mut i)?;
			let name = j.ptr_u16()?.string()?;
			let desc = j.ptr_u16()?.string()?;
			items.push(Item { name, desc, base });
		}
		Ok(items)
	}
}

#[derive(Debug)]
pub struct Quartz {
	pub id: u16, // 600 less than the corresponding item id
	pub element: u16, // should use magic::Element but whatever
	pub sepith: [u16; 7],
	pub value: [u16; 7],
}

impl Quartz {
	pub fn read(i: &[u8]) -> Result<Vec<Self>> {
		let mut i = In::new(i);
		let start = i.clone().u16()?;
		let mut items = Vec::with_capacity(start as usize/2);
		for _ in 0..start/2 {
			let mut i = i.ptr_u16()?;

			let id = i.u16()?;
			let element = i.u16()?;
			let sepith = [ i.u16()?, i.u16()?, i.u16()?, i.u16()?, i.u16()?, i.u16()?, i.u16()? ];
			let value  = [ i.u16()?, i.u16()?, i.u16()?, i.u16()?, i.u16()?, i.u16()?, i.u16()? ];
			items.push(Quartz { id, element, sepith, value });
		}
		Ok(items)
	}
}
