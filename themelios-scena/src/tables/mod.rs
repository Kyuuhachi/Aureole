pub mod bgmtbl;
pub mod book;
pub mod btlset;
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
pub mod se;
// t_shop
// t_sltget
pub mod status;
pub mod town;
pub mod world;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[derive(num_enum::TryFromPrimitive, num_enum::IntoPrimitive)]
#[repr(u8)]
pub enum Element {
	Earth = 0,
	Water = 1,
	Fire = 2,
	Wind = 3,
	Time = 4,
	Space = 5,
	Mirage = 6,
}

impl Element {
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
