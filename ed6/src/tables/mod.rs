pub mod bgmtbl;
// t_book{00..=07}
// t_btlset
// t_cook // seems unimportant, it's the same 36-byte file in both fc and sc
pub mod cook2;
// t_crfget
pub mod exp;
pub mod face;
pub mod item; // t_item, t_item2
// t_magget // only exists in fc
// t_magic
// t_magqrt
// t_memo
pub mod name;
pub mod orb;
// t_quartz
// t_quest
pub mod se;
// t_shop
// t_sltget
pub mod status;
pub mod town;
pub mod world;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Element {
	Earth,
	Water,
	Fire,
	Wind,
	Time,
	Space,
	Mirage,
}

impl Element {
	// I want to use TryFrom and From, but Rust's orphan rules don't allow that for Option<Element>.

	pub fn from_u8(v: u8) -> Result<Element, crate::util::CastError> {
		match v {
			0 => Ok(Element::Earth),
			1 => Ok(Element::Water),
			2 => Ok(Element::Fire),
			3 => Ok(Element::Wind),
			4 => Ok(Element::Time),
			5 => Ok(Element::Space),
			6 => Ok(Element::Mirage),
			_ => Err(crate::util::cast_error::<Element>(v.to_string(), "invalid enum value")),
		}
	}

	pub fn from_u8_opt(v: u8) -> Result<Option<Element>, crate::util::CastError> {
		match v {
			0 => Ok(None),
			1 => Ok(Some(Element::Earth)),
			2 => Ok(Some(Element::Water)),
			3 => Ok(Some(Element::Fire)),
			4 => Ok(Some(Element::Wind)),
			5 => Ok(Some(Element::Time)),
			6 => Ok(Some(Element::Space)),
			7 => Ok(Some(Element::Mirage)),
			_ => Err(crate::util::cast_error::<Option<Element>>(v.to_string(), "invalid enum value")),
		}
	}

	pub fn to_u8(v: Element) -> u8 {
		match v {
			Element::Earth => 0,
			Element::Water => 1,
			Element::Fire => 2,
			Element::Wind => 3,
			Element::Time => 4,
			Element::Space => 5,
			Element::Mirage => 6,
		}
	}

	pub fn to_u8_opt(v: Option<Element>) -> u8 {
		match v {
			None => 0,
			Some(Element::Earth) => 1,
			Some(Element::Water) => 2,
			Some(Element::Fire) => 3,
			Some(Element::Wind) => 4,
			Some(Element::Time) => 5,
			Some(Element::Space) => 6,
			Some(Element::Mirage) => 7,
		}
	}
}
