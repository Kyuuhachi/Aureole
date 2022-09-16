use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::archive::Archives;
use crate::util::{InExt, OutExt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Town(String, TownType);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

pub fn read(_arcs: &Archives, t_town: &[u8]) -> Result<Vec<Town>, super::Error> {
	let mut f = Coverage::new(Bytes::new(t_town));
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
		let type_ = type_.try_into()?;
		names.push(Town(name, type_));
	}
	f.assert_covered()?;
	Ok(names)
}

pub fn write(_arcs: &Archives, towns: &[Town]) -> Vec<u8> {
	let mut head = Out::new();
	let mut body = Out::new();
	let mut count = Count::new();
	head.u16(towns.len().try_into().unwrap());
	for Town(name, kind) in towns {
		let l = count.next();
		head.delay_u16(l);
		body.label(l);
		body.string(name);
		if name.is_empty() {
			assert_eq!(kind, &TownType::None);
		} else {
			body.u8(kind.clone().into());
		}
	}
	head.concat(body);
	head.finish()
}

#[cfg(test)]
mod test {
	use crate::archive::Archives;
	use super::super::test::*;

	#[test_case::test_case(&FC; "fc")]
	fn roundtrip(arc: &Archives) -> Result<(), Error> {
		let t_town = arc.get_decomp("t_town._dt").expect("no t_town")?;
		let town = super::read(arc, &t_town)?;
		let t_town_ = super::write(arc, &town);
		let town_ = super::read(arc, &t_town_)?;
		check_equal(&town, &town_)
	}
}
