use themelios_common::types::*;
use themelios_common::util::*;
use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _, Label as GLabel};

use crate::Addr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ISet {
	Ao,
}

fn iset(g: Game) -> ISet {
	match g {
		Game::Ao    => ISet::Ao,
		Game::AoEvo => ISet::Ao,
		Game::AoKai => ISet::Ao,
		_ => panic!(),
	}
}

themelios_common::newtype!(XId(u8));

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct CharId(pub u8);

themelios_common::impl_from_into!(CharId(u8));
impl std::fmt::Debug for CharId {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		if self.0 == 0xFF {
			write!(f, "self")
		} else if self.0 == 0xFE {
			write!(f, "target")
		} else {
			write!(f, "char[{}]", self.0)
		}
	}
}

themelios_macros::bytecode! {
	(game: Game)
	#[games(iset(game) => ISet::{Ao})]
	[
		End(),
		Goto(Addr),
		CharSetChipPattern(u8 as CharId, u8),
		CharTurnTo(u8 as CharId, i16 as Angle),
		CharRotateAdd(u8 as CharId, u8, i16 as Angle),
		_05(u8 as CharId, u8, i32),
		Sleep(u16 as u32 as Time),
		Update(),
		CharSetPos(u8 as CharId, u8 as CharId, Pos3),
		_09(u8 as CharId, u8 as CharId, Pos3),
		_0A(u8 as CharId, u8, u8, u32),
		_0B(u16, u16),

		def! _07(CharId, CharId, i16, u16, u8),
		def! _07FC(CharId, CharId, u16, u16, u16, u16, u16, u8),

		custom! {
			read => |f| {
				let ch1 = CharId(f.u8()?);
				let ch2 = CharId(f.u8()?);
				if ch2 != CharId(0xFC) {
					Ok(Self::_07(ch1, ch2, f.i16()?, f.u16()?, f.u8()?))
				} else {
					Ok(Self::_07FC(ch1, ch2, f.u16()?, f.u16()?, f.u16()?, f.u16()?, f.u16()?, f.u8()?))
				}
			},
			write _07(ch1, ch2, a, b, x) => |f| {
				f.u8(ch1.0);
				f.u8(ch2.0);
				f.i16(*a);
				f.u16(*b);
				f.u8(*x);
				Ok(())
			},
			write _07FC(ch1, ch2, a, b, c, d, e, x) => |f| {
				f.u8(ch1.0);
				f.u8(ch2.0);
				f.u16(*a);
				f.u16(*b);
				f.u16(*c);
				f.u16(*d);
				f.u16(*e);
				f.u8(*x);
				Ok(())
			},
		},

		CharJump(
			u8 as CharId,
			u8 as CharId,
			Pos3,
			i16 as i32 as Length,
			u16 as u32 as Speed,
		),
		_0E(u8),
		CharJumpToTarget(
			i16 as i32 as Length,
			u16 as u32 as Speed,
		),
		CharJumpBack(
			i16 as i32 as Length,
			u16 as u32 as Speed,
		),

		_11(
			u8 as CharId,
			u8 as CharId,
			Pos3,
			u32,
			u8,
		),

		EffLoad(u8 as EffId, String),
		EffUnload(u8 as EffId),
		_14(u8), // Almost always shortly before UNLOAD_EFF
		EffWait(u8 as CharId, u8 as EffInstanceId),
		_EffFinish(u8 as CharId, u8 as EffInstanceId),
		_EffStop(u8 as CharId, u8 as EffInstanceId),
		EffPlay(
			u8 as EffId, u8 as CharId, u8 as CharId, u8,
			Pos3,
			i16, i16, i16,
			u16, u16, u16,
			u8 as EffInstanceId,
		),
		EffPlay2(
			u8 as EffId, u8 as CharId, String, u16,
			Pos3,
			i16, i16, i16,
			u16, u16, u16,
			u8 as EffInstanceId,
		),
		_1A(u8 as EffInstanceId, u8, u16),

		CharSetChipBase(u8 as CharId, u8 as u16 as ChipId),
		Damage(u8 as CharId),
		DamageAnim(u8 as CharId, u8, u8 as u32 as Time),
		_1E(i32),
		Sc_1F(u16, u16, u8),

		_20(u8, u8 as CharId, u8 as CharId, i32, i32),
		_21(u8, u8 as CharId, i32, i32),
		Fork(u8 as CharId, u8 as u16 as ForkId, Addr, u8),
		ForkWait(u8 as CharId, u8 as u16 as ForkId),
		CharFlagSet(u8, u8 as CharId, u16),
		CharFlagUnset(u8, u8 as CharId, u16),
		CharFlag2Set(u8, u8 as CharId, u16),
		CharFlag2Unset(u8, u8 as CharId, u16),

		TextTalk(u8 as CharId, String, u16 as u32 as Time),
		TextWait(u8 as CharId),
		TextMessage(String, u16 as u32 as Time),
		TextMessageWait(),

		_ShadowBegin(u8 as CharId, u16, u16),
		_ShadowEnd(u8 as CharId),

		CharShake(i8, i32, i32, i32),

		ForkQuit(u8 as CharId, u8 as u16 as ForkId),

		TextTalkRandom(u8, Vec<String> via talk_random),

		_31(u8, u16 as u32 as Time),
		ChipSetXOffset(u8, u8),
		ChipSetYOffset(u8, u8),
		_34(),
		_35(u8 as CharId, Pos3, u16 as u32 as Time),
		_36(),
		CamSetDistance(i32, i32, i32, u16 as u32 as Time),
		_38(u8 as CharId, u16),
		_39(u16, u16, u16, u16),
		_3A(i16 as Angle, u16 as u32 as Time),
		_3B(i32, u16 as u32 as Time),
		_3C(i16, u16 as u32 as Time),
		CamShake(u16, u16, u16, u16 as u32 as Time),
		CamPers(u16, u16 as u32 as Time),

		_3F(u8 as CharId),
		_40(u8),
		_LockAngle(u8 as CharId),
		_42(u8 as CharId, u16 as u32 as Time),
		_43(u8 as CharId, u16 as u32 as Time, u32 as Color),
		_44(u8 as CharId, u16 as u32 as Time, u32 as Color),

		// I think these are related to model animation
		_45(u8 as CharId, u32),
		_46(u8 as CharId, u32, i32),
		_47(u8 as CharId),
		_48(u8 as CharId, u32),

		_SetControl(u8 as CharId, u16),

		skip!(1),
		If(u8, u8, i32, Addr),
		For(Addr),
		ForReset(),
		ForNext(),

		_4F(u8 as CharId, u8),

		Call(Addr),
		Return(),

		_52(u8),
		_53(u8),
		_54(u8),
		AsmagStart(i16),
		AsmagEnd(),
		_57(u8 as CharId, u8 as CharId),
		Knockback(i8),
		skip!(1),
		_5A(u8, u8, u16 as u32 as Time),
		_5B(u16 as u32 as Time),
		CharShow(u8 as CharId, u16 as u32 as Time),
		CharHide(u8 as CharId, u16 as u32 as Time),
		_5E(u8 as CharId),
		_5F(u8 as CharId, u8),

		_60(u8 as CharId),
		TimeRate(u32), // ms/ms
		_62([u8; 10]),
		_63(u8, u32),

		SoundLoad(u16 as u32 as SoundId),
		SoundPlay(u16 as u32 as SoundId, u8),
		SoundStop(u16 as u32 as SoundId),
		_67(u16, u16),
		skip!(1),
		ChipUnload(),
		LoadChip(FileId, u16),
		UnloadSomething(), // reset_scraft_chip
		_6C(),
		_6DSet(u32),
		_6EUnset(u32),

		BS_6F([u8;2]),
		BS_70([u8;6]),
		BS_71([u8;1]),
		BS_72(),
		BS_73([u8;2]),
		BS_74([u8;2]),
		BS_75([u8;2]),
		BS_76([u8;1]),
		BS_77([u8;1]),

		BracketLoad(u8),
		_79(u8),
		_7A(u8),
		// #[game(Fc)] skip!(2),
		_7D(u8, u8),
		_7E(u8 as CharId, u8 as u16 as ForkId),
		BS_7F(u8, u32),
		_80(i32),

		Blur(u16 as u32 as Time, u32 as Color, [u8; 3]),
		BlurOff(u16 as u32 as Time),

		_82(u8, u8 as CharId, u16),
		_83(), // might have to do with rotation
		SortTargets(),
		_85(u8 as CharId, i16 as Angle, i16 as Angle, i16 as Angle, u32 as Time, u8),
		_86(u8 as CharId, u8 as CharId, u32),
		_87(i16, i16, i16, u8, u32),

		skip!(1),
		SoundVoice(u16 as u32 as SoundId, u8),
		CharSavePos(u8 as CharId),
		CharClone(u8 as CharId, u8 as CharId),
		AsitemStart(),
		AsitemEnd(),
		_8D(u8, i32, u16, u16, u16, u16, u16, u16),

		_(match u8 {
			1  => _8ELoad(u8 as XId, String),
			2  => _8E_2(u8 as XId, u32, u32, u32, u32),
			4  => _8E_4(u8 as XId, u32, u32, u32, u32),
			5  => _8E_5(u8 as XId, u32 as u8 as CharId, i32, i32, i32),
			6  => _8E_6(u8 as XId, u32, u32, u32, u32),
			7  => _8ESetColor(u8 as XId, u32 as Color, u32 as Time, u32, u32),
			8  => _8E_8(u8 as XId, u32, u32, u32, u32),
			9  => _8E_9(u8 as XId, u32, u32, u32, u32),
			10 => _8E_10(u8 as XId, u32, u32, u32, u32),
			11 => _8E_11(u8 as XId, u32, u32, u32, u32),
			12 => _8E_12(u8 as XId, u32, u32, u32, u32),
			13 => _8E_13(u8 as XId, u32 as u8 as CharId, i32, i32, i32, i32),
		}),
		_8F(u8),

		_90(u8),
		skip!(1),

		_92(u8 as CharId, u8 as CharId, Pos3, i16 as Angle, u32),
		_93(u8 as CharId, u8 as CharId, String),
		_94(u8 as CharId, String, u32 as Time),
		_95(),
		_96([u8; 3]),
		_97(u16 as u32 as Time, u8, u8),
		_98(u8 as CharId, u8, u32, u32),
		_99(u8 as CharId),
		_9A(),

		_9B(u8 as CharId), // These two are often used with Damage
		_9C(u8 as CharId),
		_9D(u8 as CharId),
		_9E(u8, String),
		_9F(u8, u32),

		_A0([u8; 5]),
		Summon(u8 as CharId, FileId /*ms#####._dt*/),
		Unsummon(u8 as CharId),
		SortPillars(u8 as CharId, u8), // used in the Pillars
		_(match u8 {
			0 => ForPillarReset(),
			1 => ForPillarNext(),
			2 => ForPillar(Addr),
		}),
		_A5(u8 as CharId, u8 as EffInstanceId, u32, u32, u8),
		_A6(u8 as CharId, u8 as EffInstanceId, u32, u32, u8),
		_A7(u8 as CharId, i16, i16, i16, i16, i16, i16, i16, i16),
		_A8(u8 as CharId, u8),
		_A9(u8 as CharId, [u8; 5]),
		_AA(i32, i32),
		_AB(u8, u8 as CharId, u8, u32),
		_AC(u32, u32),
		_AD(u8),
		ForkInline(u8 as CharId, u8 as u16 as ForkId, Vec<Insn> via fork),
		_AF(u8, u8, u32, u32, u32, u32),
		_B0(u16, u16),
		_B1(u8, String, u16, u16),
		skip!(1),
		skip!(1),
		skip!(1),
		_B5(u8),
		_B6(u8),
		skip!(1),
		_B8(u8),
		skip!(3),
		_BC(u8, u16),
	]
}

impl Insn {
	pub fn name(&self) -> &'static str {
		macro run(
			[$(($ident:ident $(($_n:ident $ty:ty))*))*]
		) {
			return match self {
				$(Self::$ident(..) => stringify!($ident),)*
			}
		}
		introspect!(run);
	}

	pub fn validate(game: Game, i: &Insn) -> Result<(), WriteError> {
		let mut w = Writer::new();
		Self::write(&mut w, game, i)
	}
}

trait Arg: Sized {
	fn read(f: &mut Reader, _: Game) -> Result<Self, ReadError>;
	fn write(f: &mut Writer, _: Game, v: &Self) -> Result<(), WriteError>;
}

macro arg($t:ty,
	|$fr:pat_param, $gr:pat_param| $r:expr,
	|$fw:pat_param, $gw:pat_param, $v:pat_param| $w:expr $(,)?
) {
	impl Arg for $t {
		fn read<'a>($fr: &mut Reader, $gr: Game) -> Result<$t, ReadError> {
			Ok($r)
		}

		fn write($fw: &mut Writer, $gw: Game, $v: &$t) -> Result<(), WriteError> {
			Ok($w)
		}
	}
}

macro prim_arg($t:ty, $i:ident) {
	arg!($t,
		|f, _| f.$i()?,
		|f, _, v| f.$i(*v),
	);
}

prim_arg!(u8, u8);
prim_arg!(u16, u16);
prim_arg!(u32, u32);
prim_arg!(i8, i8);
prim_arg!(i16, i16);
prim_arg!(i32, i32);

impl<const T: usize> Arg for [u8; T] {
	fn read<'a>(r: &mut Reader, _: Game) -> Result<Self, ReadError> {
		Ok(r.array()?)
	}

	fn write(w: &mut Writer, _: Game, v: &Self) -> Result<(), WriteError> {
		w.array(*v);
		Ok(())
	}
}

arg!(String,
	|f, _| f.string()?,
	|f, _, v| f.string(v.as_str())?,
);

arg!(FileId,
	|f, _| FileId(f.u32()?),
	|f, _, v| f.u32(v.0),
);

arg!(Addr,
	|f, _| Addr(f.u16()? as usize),
	|f, _, v| f.u16(cast(v.0)?),
);

arg!(Pos3,
	|f, _| f.pos3()?,
	|f, _, v| f.pos3(*v),
);

mod talk_random {
	use super::*;
	pub(super) fn read(f: &mut Reader, _: Game) -> Result<Vec<String>, ReadError> {
		let mut strings = Vec::new();
		loop {
			let pos = f.pos();
			if let Ok(s) = f.string() {
				if s.is_empty() {
					break;
				}
				strings.push(s);
			} else {
				f.seek(pos)?;
				strings.push("".to_owned());
				break
			}
		}
		Ok(strings)
	}

	pub(super) fn write(f: &mut Writer, _: Game, v: &[String]) -> Result<(), WriteError> {
		todo!()
	}
}

mod fork {
	use super::*;
	pub(super) fn read(f: &mut Reader, game: Game) -> Result<Vec<Insn>, ReadError> {
		let len = f.u8()? as usize;
		let pos = f.pos();
		let mut code = Vec::new();
		while f.pos() < pos + len {
			code.push(Insn::read(f, game)?);
		}
		f.check_u8(0)?;
		Ok(code)
	}

	pub(super) fn write(f: &mut Writer, game: Game, v: &[Insn]) -> Result<(), WriteError> {
		todo!()
	}
}
