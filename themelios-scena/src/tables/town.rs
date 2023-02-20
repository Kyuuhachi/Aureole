use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::types::TownId;
use crate::util::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Town(pub String, pub TownType);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[derive(num_enum::TryFromPrimitive, num_enum::IntoPrimitive)]
#[repr(u8)]
pub enum TownType {
	None       = 0,
	Weapons    = 1, // Arms & Guards    武器・防具
	Goods      = 2, // General Goods    薬・雑貨・食材
	Lodgings   = 3, // Lodgings         休憩・宿泊
	Guild      = 4, // Bracer Guild     遊撃士協会
	Orbment    = 5, // Orbment Factory  オーブメント
	Restaurant = 6, // Restaurant/Inn   食事・休憩
	Church     = 7, // Septian Church   七耀教会
	Cafe       = 8, // Cafe             飲食・喫茶
}

pub fn read(t_town: &[u8]) -> Result<Vec<Town>, ReadError> {
	let mut f = Coverage::new(Reader::new(t_town));
	let n = f.u16()?;
	let mut names = Vec::with_capacity(n as usize);
	for _ in 0..n {
		let mut g = f.ptr()?;
		let name = g.string()?;
		let type_ = if name.is_empty() {
			0
		} else {
			g.u8()?
		};
		let type_ = cast(type_)?;
		names.push(Town(name, type_));
	}
	f.assert_covered()?;
	Ok(names)
}

pub fn write(towns: &[Town]) -> Result<Vec<u8>, WriteError> {
	let mut f = Writer::new();
	let mut g = Writer::new();
	f.u16(cast(towns.len())?);
	for &Town(ref name, kind) in towns {
		f.delay_u16(g.here());
		g.string(name)?;
		if name.is_empty() {
			ensure!(kind == TownType::None, "empty town must be type None");
		} else {
			g.u8(kind.into());
		}
	}
	f.append(g);
	Ok(f.finish()?)
}
