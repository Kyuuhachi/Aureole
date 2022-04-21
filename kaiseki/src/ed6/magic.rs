use int_enum::IntEnum;
use eyre::{Result, eyre};
use hamu::read::{In, Le};
use crate::util::{self, InExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::Display)]
pub enum Element {
	None,
	Water,
	Earth,
	Fire,
	Wind,
	Time,
	Space,
	Mirage,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntEnum, strum_macros::Display)]
pub enum MagicEffect {
	None        =  0,
	PhysDamage  =  1,
	MagDamage   =  2,
	Heal        =  3,
	_4          =  4, // Fortune Coin, args=50,100
	_5          =  5, // Used on four unnamed skills in FC
	Impede      =  6,
	Delay       =  7,
	HpAbsorb    =  8,
	EpAbsorb    =  9,
	Poison      = 10,
	Freeze      = 11,
	Petrify     = 12,
	Sleep       = 13,
	Mute        = 14,
	Blind       = 15,
	Seal        = 16,
	Confuse     = 17,
	Faint       = 18,
	Death       = 19,
	Cooling     = 20, // Used by Reverie, no idea what it does
	Immunity    = 21,
	BreakFloor  = 22,
	BreakItem   = 23,
	CpAbsorb    = 24,
	AtkUp       = 25,
	AtkDn       = 26,
	DefUp       = 27,
	DefDn       = 28,
	SpdUp       = 29,
	SpdDn       = 30,
	AdfUp       = 31,
	AdfDn       = 32,
	AglUp       = 33,
	AglDn       = 34,
	AtAdvance   = 35,
	MaxHpDn     = 36, // Needs investigation. In FC it is MaxHpDn(30, 50), stated to give -30%HP +50CP.
	MovUp       = 37,
	Rage        = 38,
	MovDn       = 39,
	Cure        = 40,
	Resurrect   = 41,
	AntiSkill   = 42, // Reverie's Anti-Skill Barrier
	AntiMagic   = 43, // Reverie's Anti-Magic Barrier

	Fatten      = 45,
	_46         = 46,
	Shrink      = 47,
	HealPercent = 48,

	CureLoweredStatus     = 49, // S-Tablet
	CurePoison            = 50, // Herb Sandwich
	CureFreeze            = 51, // Hot-Hot Potato Fry
	CurePetrify           = 52, // Corner Castelia
	CureSleep             = 53, // Royal Gelato, Nap Killer
	CureMute              = 54, // Insulating Tape
	CureBlind             = 55, // Passion Omelet
	CureSeal              = 56, // Miso-Stewed Fish
	CureConfuse           = 57, // Mocking Pie
	CureFaint             = 58, // Sea 'Bubbles'
	CureConfuseSleepFaint = 59, // Smelling Salts
	CureFreezePetrify     = 60, // Softening Balm
	CurePoisonSealBlind   = 61, // Purging Balm
	CpUp                  = 62, // Sacrifice Arrow, Zeram Powder, Zeram Capsule (Does this include resurrect too?)
	CurePoisonConfuse     = 63, // Royal Crepe
	StrDefUp              = 64, // Bone Boullion, Mighty Juice, Spiral Noodles, Mighty Essence
	StrDefDn              = 65, // Kaempfer

	VitalCannon           = 68, // Cure debuffs (49?)
	SilentCross           = 69, // Impede crafts (Those don't take time though)
	Lichtkreis            = 70, // Revive/heal/def up if 200. Four parameters.
	JudgementCard         = 71, // Random status
	CureSealMuteConfuse   = 72, // Premium Herb Tea, Flame Tongue Stew, Seafood Jelly, Mystery Crepe
	CureFaintSleepPetrify = 73, // Anarchy Soup, Crimson Platter, Roast Fish
	CurePoisonBlindFreeze = 74, // Fevered Gaze, Fruit Kingdom, Ambrosial Egg, Flower Milkshake
	Mov1Spd10p            = 75, // Turnin' Tempura, Sweeeeet Crepe, Heavenly Tempura
	RandomStatus2         = 76, // Rainbow Surprise, Aurora Ball (71?)

	GrailSphere           = 90, // Block 1 or 2 attacks
	TruePummel            = 91, // Also True Barrage

	EpDrain  = 92, // Dunno how this differes from EP_ABSORB
	Steal    = 93,
	Confuse2 = 94, // Supreme Evil Eye

	_95 = 95
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntEnum, strum_macros::Display)]
pub enum MagicTarget {
	None           =  0,
	WalkTarget     =  1,
	WalkTargetArea =  2,
	WalkTargetLine =  3,
	Target         =  4,
	TargetArea     =  5,
	Combo          =  6,
	SetArea        = 11,
	SetLine        = 12,
	All            = 13,
	SelfArea       = 14,
	WalkSetArea    = 15,
	_16            = 16, // Ragnard
	_17            = 17,
	FloorTile      = 18,
	Transform      = 19,
	Walk           = 50,
}

#[allow(non_upper_case_globals)]
mod magicflags { bitflags::bitflags! {
	pub struct MagicFlags: u16 {
		const Healing      = 0x0001; // Tear and Curia (but not Thelas or crafts)
		const Exists       = 0x0002; // used on all skills that actually exist ingame, and a few others. Not sure what exactly it means.
		const TargetEnemy  = 0x0010; // spell can target enemies
		const TargetDead   = 0x0020; // spell can target dead allies (used on resurrection spells)
		const Beneficial   = 0x0040; // buffs, healing, etc.
		const TargetFriend = 0x0080; // spell can target allies
		const _0100        = 0x0100;
		const _0200        = 0x0200;
		const UsesRadius   = 0x0400; // only used on normal attacks and a few of Tita's crafts
		const _1000        = 0x1000;
		const Magic        = 0x2000; // this is an orbal art
		const _4000        = 0x4000;
		// Wonder if any of the unknown ones is whether it can be impeded?
	}
} }
pub use magicflags::MagicFlags;

#[derive(Debug)]
pub struct Magic<T> {
	pub id: u16,
	pub name: T,
	pub desc: T,
	pub flags: MagicFlags,
	pub element: Element,
	pub target: MagicTarget,
	pub effect1: MagicEffect,
	pub effect2: MagicEffect,
	pub target_p1: u16,
	pub target_p2: u16,
	pub warmup: u16,
	pub cooldown: u16,
	pub cost: u16,
	pub sort: u16,
	pub effect_p1: i16,
	pub effect_p2: i16,
	pub effect_p3: i16,
	pub effect_p4: i16,
}

impl<A> Magic<A> {
	pub fn read_base(i: &mut In, read_str: &mut impl FnMut(&mut In) -> Result<A>) -> Result<Self> {
		let id = i.u16()?;
		let flags = MagicFlags::from_bits(i.u16()?).ok_or_else(|| eyre!("invalid flags"))?;

		let elements = &[
			if flags.contains(MagicFlags::Magic) { Element::Time } else { Element::None },
			Element::Earth,
			Element::Water,
			Element::Fire,
			Element::Wind,
			Element::Space,
			Element::Mirage,
		];

		let element = *elements.get(i.u8()? as usize).ok_or_else(|| eyre!("invalid element"))?;
		let target = MagicTarget::from_int(i.u8()?)?;
		let effect1 = MagicEffect::from_int(i.u8()?)?;
		let effect2 = MagicEffect::from_int(i.u8()?)?;
		let target_p1 = i.u16()?;
		let target_p2 = i.u16()?;
		let warmup = i.u16()?;
		let cooldown = i.u16()?;
		let cost = i.u16()?;
		let sort = i.u16()?;
		let effect_p1 = i.i16()?;
		let effect_p2 = i.i16()?;
		let effect_p3 = i.i16()?;
		let effect_p4 = i.i16()?;

		let name = read_str(i)?;
		let desc = read_str(i)?;

		Ok(Magic {
			id,
			name,
			desc,
			flags,
			element,
			target,
			effect1,
			effect2,
			target_p1,
			target_p2,
			warmup,
			cooldown,
			cost,
			sort,
			effect_p1,
			effect_p2,
			effect_p3,
			effect_p4,
		})
	}
}

impl Magic<()> {
	pub fn read_one(i: &mut In) -> Result<Self> {
		Magic::read_base(i, &mut |_| Ok(()))
	}
}

impl Magic<String> {
	pub fn read_one(i: &mut In) -> Result<Self> {
		// This can be made slightly less hacky with #[feature(type_changing_struct_update)]
		Magic::read_base(i, &mut |i| i.ptr_u16()?.str())
	}

	pub fn read(i: &[u8]) -> Result<Vec<Self>> {
		util::toc(i, |i, _| Self::read_one(i))
	}
}
