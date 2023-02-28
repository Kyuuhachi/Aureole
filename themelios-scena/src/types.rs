use crate::util::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[derive(derive_more::From, derive_more::Into)]
#[derive(derive_more::DebugCustom)]
#[debug(fmt = "FileId(0x{_0:08X})")]
pub struct FileId(pub u32);

newtype!(QuestId, u16);
newtype!(NameId, u16);
newtype!(BgmId, u16);
newtype!(SoundId, u32);
newtype!(ItemId, u16);
newtype!(RecipeId, u16);
newtype!(TownId, u16);
newtype!(BattleId, u32);
newtype!(ShopId, u8);
newtype!(MagicId, u16);

newtype!(Color, u32);

newtype!(Flag, u16);

newtype!(Angle, i16);
newtype!(Length, i32);
newtype!(Time, u32);
newtype!(Speed, u32);
newtype!(Angle32, i32);

// Translatable string
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
#[derive(derive_more::From, derive_more::Into, derive_more::Deref, derive_more::DerefMut)]
#[repr(transparent)]
pub struct TString(pub String);

impl<'a> From<&'a String> for &'a TString {
	fn from(value: &'a String) -> Self {
		// SAFETY: repr(transparent)
		unsafe { std::mem::transmute(value) }
	}
}

impl<'a> From<&'a TString> for &'a String {
	fn from(value: &'a TString) -> Self {
		&value.0
	}
}

impl From<&str> for TString {
	fn from(value: &str) -> Self {
		String::from(value).into()
	}
}

impl std::fmt::Debug for TString {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "t{:?}", &self.0)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseGame {
	Fc, Sc, Tc,
	Zero, Ao,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Game {
	Fc, FcEvo, FcKai,
	Sc, ScEvo, ScKai,
	Tc, TcEvo, TcKai,

	Zero, ZeroEvo, ZeroKai,
	Ao, AoEvo, AoKai,
}

impl Game {
	pub fn base(self) -> BaseGame {
		use Game::*;
		match self {
			Fc|FcEvo|FcKai => BaseGame::Fc,
			Sc|ScEvo|ScKai => BaseGame::Sc,
			Tc|TcEvo|TcKai => BaseGame::Tc,

			Zero|ZeroEvo|ZeroKai => BaseGame::Zero,
			Ao|AoEvo|AoKai => BaseGame::Ao,
		}
	}
}

impl Game {
	pub fn is_ed7(self) -> bool {
		matches!(self.base(), BaseGame::Zero|BaseGame::Ao)
	}
}
