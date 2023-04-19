use themelios_common::types::*;
use themelios_common::util::*;
use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _};

use crate::Addr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// InstructionSet
enum ISet {
	Fc,
	Sc,
	Tc,
}

fn iset(g: Game) -> ISet {
	match g {
		Game::Fc => ISet::Fc,
		Game::FcEvo => ISet::Fc,
		Game::FcKai => ISet::Fc,
		Game::Sc    => ISet::Sc,
		Game::ScEvo => ISet::Sc,
		Game::ScKai => ISet::Sc,
		Game::Tc    => ISet::Tc,
		Game::TcEvo => ISet::Tc,
		Game::TcKai => ISet::Tc,
		Game::Zero    => todo!(),
		Game::ZeroEvo => todo!(),
		Game::ZeroKai => todo!(),
		Game::Ao    => todo!(),
		Game::AoEvo => todo!(),
		Game::AoKai => todo!(),
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
	#[games(iset(game) => ISet::{Fc, Sc, Tc})]
	[
		End(),
		Goto(Addr),
		CharSetChipPattern(u8 as CharId, if game.base() == BaseGame::Fc { u16 } else { u8 as u16 }),
		CharTurnTo(u8 as CharId, i16 as Angle),
		CharRotateAdd(u8 as CharId, u8, i16 as Angle),
		_05(u8 as CharId, u8, i32),
		Sleep(u32 as Time),
		Update(),
		CharSetPos(u8 as CharId, u8 as CharId, Pos3),
		_09(
			if matches!(game.base(), BaseGame::Fc|BaseGame::Sc) { const 255u8 } else { u8 } as CharId,
			u8 as CharId,
			Pos3
		),
		_0A(u8 as CharId, u8, u8, u32),
		CharTurnToChar(
			u8 as CharId,
			u8 as CharId,
			if game.base() == BaseGame::Fc { u32 as u16 } else { u16 } as AngularSpeed,
		),
		_0C(
			u8 as CharId,
			u8 as CharId,
			i16 as Angle,
			i16,
			if game.base() == BaseGame::Fc { u16 as u8 } else { u8 },
		),

		CharJump(
			if game.base() == BaseGame::Fc { const 255u8 } else { u8 } as CharId,
			u8 as CharId,
			Pos3,
			if game.base() == BaseGame::Fc { i32 } else { i16 as i32 } as Length,
			if game.base() == BaseGame::Fc { u32 } else { u16 as u32 } as Speed,
		),
		CharDropDown(
			u8 as CharId,
			Pos3,
			if game.base() == BaseGame::Fc { i32 } else { i16 as i32 } as Length,
			if game.base() == BaseGame::Fc { u32 } else { u16 as u32 } as Speed,
		),
		CharJumpToTarget(
			if game.base() == BaseGame::Fc { i32 } else { i16 as i32 } as Length,
			if game.base() == BaseGame::Fc { u32 } else { u16 as u32 } as Speed,
		),
		CharJumpBack(
			if game.base() == BaseGame::Fc { i32 } else { i16 as i32 } as Length,
			if game.base() == BaseGame::Fc { u32 } else { u16 as u32 } as Speed,
		),

		_11(
			u8 as CharId,
			u8 as CharId,
			Pos3,
			u32,
			u8,
		),

		EffLoad(u8 as EffId, u8, String),
		EffUnload(u8 as EffId, u8),
		_14(u16), // Almost always shortly before UNLOAD_EFF
		EffWait(if game.base() == BaseGame::Fc { const 255u8 } else { u8 } as CharId, u8 as EffInstanceId),
		_EffFinish(if game.base() == BaseGame::Fc { const 255u8 } else { u8 } as CharId, u8 as EffInstanceId),
		_EffStop(if game.base() == BaseGame::Fc { const 255u8 } else { u8 } as CharId, u8 as EffInstanceId),
		EffPlay(
			u8 as EffId, u8 as CharId, u8 as CharId, u16,
			Pos3,
			i16, i16, i16,
			if game.base() == BaseGame::Fc { u32 } else { u16 as u32 },
			if game.base() == BaseGame::Fc { u32 } else { u16 as u32 },
			if game.base() == BaseGame::Fc { u32 } else { u16 as u32 },
			u8 as EffInstanceId,
		),
		EffPlay2(
			u8 as CharId, u8 as CharId, String, u16,
			Pos3,
			i16, i16, i16,
			if game.base() == BaseGame::Fc { u32 } else { u16 as u32 },
			if game.base() == BaseGame::Fc { u32 } else { u16 as u32 },
			if game.base() == BaseGame::Fc { u32 } else { u16 as u32 },
			u8 as EffInstanceId,
		),
		_1A(u8 as EffInstanceId, u16),

		CharSetChipBase(u8 as CharId, if game.base() == BaseGame::Fc { u16 } else { u8 as u16 } as ChipId),
		Damage(u8 as CharId),
		DamageAnim(u8 as CharId, u8, u32 as Time),
		_1E(i32),
		#[game(Fc)] _1F(),
		#[game(Sc,Tc)] Sc_1F(u16, u16, u8),

		_20(u8, u8 as CharId, u8 as CharId, u8, i32, i32),
		_21(u8, u8 as CharId, i32, i32),
		Fork(u8 as CharId, u8 as u16 as ForkId, Addr, u8),
		ForkWait(u8 as CharId, u8 as u16 as ForkId),
		CharFlagSet(u8, u8 as CharId, u16),
		CharFlagUnset(u8, u8 as CharId, u16),
		CharFlag2Set(u8, u8 as CharId, u16),
		CharFlag2Unset(u8, u8 as CharId, u16),

		TextTalk(u8 as CharId, String, u32 as Time),
		TextWait(u8 as CharId),
		TextMessage(String, u32 as Time),
		TextMessageWait(),

		_ShadowBegin(if matches!(game.base(), BaseGame::Fc|BaseGame::Sc) { const 255u8 } else { u8 } as CharId, u16, u16),
		_ShadowEnd(if matches!(game.base(), BaseGame::Fc|BaseGame::Sc) { const 255u8 } else { u8 } as CharId),

		CharShake(i8, i32, i32, i32),

		ForkQuit(u8 as CharId, u8 as u16 as ForkId),

		TextTalkRandom(u8, Vec<String> via talk_random),

		_31(u8, u32 as Time),
		ChipSetXOffset(u8, u8),
		ChipSetYOffset(u8, u8),
		_34(),
		_35(u8 as CharId, Pos3, u32 as Time),
		_36(i32, i32, i32, u32 as Time),
		CamSetDistance(i32, i32, i32, u32 as Time),
		_38(i32, i32, i32, u32 as Time),
		_39(i32 as i16 as Angle, u32 as Time),
		_3A(i32 as i16 as Angle, u32 as Time),
		_3B(i32, u32 as Time),
		_3C(i16, u32 as Time),
		CamShake(u32, u32, u32, u32 as Time),
		CamPers(u32, u32 as Time),

		_3F(u8 as CharId),
		_40(u8),
		_LockAngle(u8 as CharId),
		_42(u8 as CharId, u32 as Time),
		_43(u8 as CharId, u32 as Time, u32 as Color),
		_44(u8 as CharId, u32 as Time, u32 as Color),

		// I think these are related to model animation
		_45(
			if game.base() == BaseGame::Fc { const 255u8 } else { u8 } as CharId,
			u32,
		),
		_46(
			if game.base() == BaseGame::Fc { const 255u8 } else { u8 } as CharId,
			u32,
			if game.base() == BaseGame::Fc { const -1i32 } else { i32 },
		),
		_47(
			if game.base() == BaseGame::Fc { const 255u8 } else { u8 } as CharId,
		),
		_48(
			if game.base() == BaseGame::Fc { const 255u8 } else { u8 } as CharId,
			u32,
		),

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
		_5A(u8, u8, u32),
		_5B(u32),
		CharShow(u8 as CharId, u32 as Time),
		CharHide(u8 as CharId, u32 as Time),
		_5E(u8 as CharId),
		_5F(u8 as CharId, u8),

		_60(u8 as CharId),
		TimeRate(u32), // ms/ms
		_62(u8 as CharId, u8, u8, u8, u16),
		_63(u8, u32),

		SoundLoad(u16 as u32 as SoundId),
		SoundPlay(u16 as u32 as SoundId, u8),
		SoundStop(u16 as u32 as SoundId),
		CutIn(String),
		skip!(1),
		ChipUnload(),
		LoadChip(u8 as u16 as ChipId, FileId, FileId),
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
		#[game(Fc)] skip!(2),
		_7D(u8, u8),
		_7E(u8 as CharId, u8 as u16 as ForkId),
		BS_7F(u8, u32),
		_80(i32),
		#[game(Fc)] _81(u8, u8, u32),

		#[game(Sc,Tc)] Blur(u32 as Time, u32 as Color, u32, u8, u32),
		#[game(Sc,Tc)] BlurOff(u32 as Time),

		_82(u8, u8 as CharId, u16),
		_83(), // might have to do with rotation
		SortTargets(u8),
		_85(u8 as CharId, i16 as Angle, i16 as Angle, i16 as Angle, u32 as Time, u8),
		_86(u8 as CharId, u8 as CharId, u32),
		_87(i16, i16, i16, u8, u32),
		#[game(Fc)] BS_88([u8;3]),

		#[game(Sc,Tc)] skip!(1),
		#[game(Sc,Tc)] SoundVoice(u16 as u32 as SoundId),
		#[game(Sc,Tc)] CharSavePos(u8 as CharId),
		#[game(Sc,Tc)] CharClone(u8 as CharId, u8 as CharId),
		#[game(Sc,Tc)] AsitemStart(),
		#[game(Sc,Tc)] AsitemEnd(),
		#[game(Sc,Tc)] _8D(u8, i32, i32, i32, i32),

		// This seems to be similarly dependently typed as VisSet
		#[game(Sc,Tc)] X(match {
			1  => Load(u8 as XId, String),
			2  => _2(u8 as XId, u32, u32, u32, u32),
			4  => _4(u8 as XId, u32, u32, u32, u32),
			5  => _5(u8 as XId, u32 as u8 as CharId, i32, i32, i32),
			6  => _6(u8 as XId, u32, u32, u32, u32),
			7  => SetColor(u8 as XId, u32 as Color, u32 as Time, u32, u32),
			8  => _8(u8 as XId, u32, u32, u32, u32),
			9  => _9(u8 as XId, u32, u32, u32, u32),
			10 => _10(u8 as XId, u32, u32, u32, u32),
			11 => _11(u8 as XId, u32, u32, u32, u32),
			12 => _12(u8 as XId, u32, u32, u32, u32),
			13 => _13(u8 as XId, u32 as u8 as CharId, i32, i32, i32, i32),
		}),
		#[game(Sc,Tc)] _8F(u8),

		#[game(Sc,Tc)] _90(u8),
		#[game(Sc,Tc)] skip!(1),

		#[game(Sc,Tc)] _92(u8 as CharId, u8 as CharId, Pos3, i16 as Angle, u32),
		#[game(Sc,Tc)] _93(u8 as CharId, u8 as CharId, String),
		#[game(Sc,Tc)] _94(u8 as CharId, String, u32 as Time),
		#[game(Sc,Tc)] _95(),
		#[game(Sc,Tc)] _96(u8 as CharId, String, u16),
		#[game(Sc,Tc)] _97(u32, u16, u16),
		#[game(Sc,Tc)] _98(u8 as CharId, u8, u32, u32),
		#[game(Sc,Tc)] _99(u8 as CharId),
		#[game(Sc,Tc)] skip!(1),

		#[game(Sc,Tc)] _9B(u8 as CharId), // These two are often used with Damage
		#[game(Sc,Tc)] _9C(u8 as CharId),
		#[game(Sc,Tc)] _9D(u8 as CharId),
		#[game(Tc)] skip!(1),
		#[game(Tc)] _9F(u8, u32),

		#[game(Tc)] _A0(u8 as CharId, [u8; 7]),
		#[game(Tc)] Summon(u8 as CharId, FileId /*ms#####._dt*/),
		#[game(Tc)] Unsummon(u8 as CharId),
		#[game(Tc)] SortPillars(u8 as CharId, u8), // used in the Pillars
		#[game(Tc)] For(match {
			0 => PillarReset(),
			1 => PillarNext(),
			2 => Pillar(Addr),
		}),
		#[game(Tc)] _A5(u8 as CharId, u8 as EffInstanceId, u32, u32, u8),
		#[game(Tc)] _A6(u8 as CharId, u8 as EffInstanceId, u8 as CharId, i32, i32, i32, u32),
		#[game(Tc)] _A7(u8, u16),
		#[game(Tc)] _A8(u8 as CharId, u8),
		#[game(Tc)] _A9(u32 as Time),
		#[game(Tc)] _AA(i32, i32),
		#[game(Tc)] _AB(u8, u8 as CharId, u8, u32),
		#[game(Tc)] _AC(u8 as CharId, u8 as EffInstanceId, u32, u32, u8),
		#[game(Tc)] skip!(1),
		#[game(Tc)] _AE(i16 as Angle, u32),
		#[game(Tc)] _AF(u8, u8, u32, u32, u32, u32),
		#[game(Tc)] skip!(1),
		#[game(Tc)] Flag(match {
			0 => Set(u16 as Flag),
			1 => Unset(u16 as Flag),
		}),
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
	|f, _| Pos3(f.i32()?, f.i32()?, f.i32()?),
	|f, _, v| { f.i32(v.0); f.i32(v.1); f.i32(v.2); },
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
