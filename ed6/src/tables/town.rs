use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use crate::archive::Archives;
use crate::util::InExt;

#[derive(Debug, Clone)]
pub struct Town(String, TownType);

#[derive(Debug, Clone, num_enum::TryFromPrimitive, num_enum::IntoPrimitive)]
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

pub fn load(arcs: &Archives) -> Result<Vec<Town>, super::Error> {
	let data = arcs.get_decomp("t_town._dt").expect("no t_town")?; // TODO error rather than panic
	let mut f = Coverage::new(Bytes::new(&data));
	let n = f.u16()?;
	let mut names = Vec::with_capacity(n as usize);
	for _ in 0..n {
		let pos = f.u16()? as usize;
		let mut g = f.clone().at(pos)?;
		let name = g.string().map_err(|a| a.either_into::<super::Error>())?;
		let type_ = if name.is_empty() {
			0
		} else {
			g.u8()?
		};
		let type_ = TownType::try_from(type_)?;
		names.push(Town(name, type_));
	}
	f.assert_covered()?;
	Ok(names)
}
