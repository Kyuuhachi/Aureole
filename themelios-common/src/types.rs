#[macro_export]
macro_rules! impl_from_into {
	($outer:ident($inner:ident)) => {
		impl From<$inner> for $outer {
			fn from(v: $inner) -> $outer {
				$outer(v)
			}
		}

		impl From<$outer> for $inner {
			fn from($outer(v): $outer) -> $inner {
				v
			}
		}
	}
}

#[macro_export]
macro_rules! newtype {
	($outer:ident($inner:ident)) => {
		#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
		#[repr(transparent)]
		pub struct $outer(pub $inner);
		$crate::impl_from_into!($outer($inner));
	};
	($outer:ident($inner:ident), $fmt:literal) => {
		#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
		#[repr(transparent)]
		pub struct $outer(pub $inner);
		$crate::impl_from_into!($outer($inner));

		impl ::core::fmt::Debug for $outer {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				f.debug_tuple(stringify!($outer))
					.field(&format_args!($fmt, &self.0))
					.finish()
			}
		}
	};
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FuncId(pub u16, pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pos2(pub i32, pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pos3(pub i32, pub i32, pub i32);

newtype!(NameId(u16));
newtype!(BgmId(u16));
newtype!(SoundId(u32));
newtype!(ItemId(u16));
newtype!(RecipeId(u16));
newtype!(TownId(u16));
newtype!(ShopId(u8));
newtype!(MagicId(u16));

newtype!(FileId(u32), "0x{:08X}");
newtype!(Color(u32), "0x{:08X}");

newtype!(Flag(u16));

newtype!(Angle(i16));
newtype!(Length(i32));
newtype!(Time(u32));
newtype!(Speed(u32));
newtype!(AngularSpeed(u16));
newtype!(Angle32(i32));

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CharId {
	FieldParty(u16),
	Local(LocalCharId),
	Party(u16),
	Custom(u16),
	Null,
	Self_,
	Name(NameId),
}
newtype!(LocalCharId(u16));

newtype!(ChipId(u16));
newtype!(LookPointId(u16));
newtype!(ObjectId(u16));
newtype!(EntranceId(u8));
newtype!(TriggerId(u16));
newtype!(LabelId(u16));
newtype!(AnimId(u16));

newtype!(BattleId(u32));
newtype!(SepithId(u16));
newtype!(PlacementId(u16));
newtype!(AtRollId(u16));

// 0x00000001 SF_CAMERA_AUTO
// 0x00400000 SF_ENTRY_DISABLE
// 0x02000000 SF_FADEBGM_DISABLE
newtype!(SystemFlags(u32));

// 0x10 done
newtype!(QuestId(u16));
newtype!(QuestFlags(u8));
newtype!(QuestTask(u16));

// 0x0002 PF_NOVEC
// 0x0004 PF_NOHEIGHT
// 0x0008 PF_NODISP
// 0x0010 PF_NOTURN
// 0x0020 PF_NOANIME
// 0x0040 PF_NOATARI
// 0x0080 PF_UNDEF
newtype!(CharFlags(u16));

// 0x0004 MOF_NODISP
// 0x0020 MOF_LOOPPLAY
newtype!(ObjectFlags(u32));

newtype!(LookPointFlags(u16));
newtype!(EntryFlags(u16));
newtype!(TriggerFlags(u16));

// A bitfield in 3rd's party select menu
newtype!(TcMembers(u32));

newtype!(Var(u16)); // called Work internally
newtype!(Global(u8));

// 0 SW_ENTRY_NO
// 1 SW_BGM_NO
// 3 battle result
// 4 current chapter
// 10 party lead
// 11 party second
// 12 party third
// 13 party fourth
// 14 party fifth (guest)
// 15 party sixth (guest)
// 18 current mira
// 19 ItemId used in item handler
// 21 number of battles
// 26 used much during the Madrigal, and when Joshua is activated in SC.
// 27 Boolean. Often set together with 26.
// 35 set to 255 once after rescuing Tita in FC
// 28 Boolean. Generally only set to true for a short time
// 40 SW_CURSOR_FORM (24 MSCRS_NORMAL, FFFF MSCRS_VOID)
// 41 BattleId, generally checked in reinit
// 42 sometimes set to a negative number in reinit. I suspect it's something with altitude.
// 43 set to 255 in some reinit in FC
// 45 SW_MOVIE_STATE
// 46 CharId, set together with 26 and 27 in the Madrogal. Spotlight character?
// 47 Bracer rank
// 49 TownId for the next save, values include 19, 140, 302, 400, 401, 274, 259, 297, 296, 299
newtype!(Attr(u8));

newtype!(EffId(u8));
newtype!(EffInstanceId(u8));
newtype!(MenuId(u16));
newtype!(VisId(u8));
newtype!(ForkId(u16));

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CharAttr(pub CharId, pub u8);

// Translatable string
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct TString(pub String);
impl_from_into!(TString(String));

impl std::ops::Deref for TString {
	type Target = String;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl std::ops::DerefMut for TString {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

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
