use hamu::read::le::*;
use hamu::write::le::*;
use crate::tables::{bgmtbl::BgmId, Element};
use crate::tables::btlset::BattleId;
use crate::tables::item::ItemId;
use crate::tables::quest::QuestId;
use crate::tables::se::SoundId;
use crate::tables::town::TownId;
use crate::util::*;

use super::{FuncRef, CharId, CharAttr, Emote, Pos2, Pos3, Var, Flag, Attr, InExt2, OutExt2, Text};

type FileRef = String;
type QuestTask = u16;
type QuestList = Vec<QuestId>;
type Color = u32;
type ShopId = u8;
type Member = u8;
type MagicId = u16;
type MemberAttr = u8;

type Flags = u16;
type QuestFlags = u8;
type CharFlags = u16;

mod insn;
pub use insn::*;

mod quest_list {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Vec<QuestId>, ReadError> {
		let mut quests = Vec::new();
		loop {
			match f.u16()? {
				0xFFFF => break,
				q => quests.push(QuestId(q))
			}
		}
		Ok(quests)
	}

	pub(super) fn write(f: &mut impl Out, v: &Vec<QuestId>) -> Result<(), WriteError> {
		for &i in v {
			f.u16(i.0);
		}
		f.u16(0xFFFF);
		Ok(())
	}
}

mod fork {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Vec<Insn>, ReadError> {
		let len = f.u8()? as usize;
		let pos = f.pos();
		let mut insns = Vec::new();
		while f.pos() < pos+len {
			insns.push(Insn::read(f)?);
		}
		ensure!(f.pos() == pos+len, "overshot while reading fork");
		f.check_u8(0)?;
		Ok(insns)
	}

	pub(super) fn write(f: &mut impl Out, v: &Vec<Insn>) -> Result<(), WriteError> {
		todo!()
	}
}

mod fork_loop {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Vec<Insn>, ReadError> {
		let len = f.u8()? as usize;
		let pos = f.pos();
		let mut insns = Vec::new();
		while f.pos() < pos+len {
			insns.push(Insn::read(f)?);
		}
		ensure!(f.pos() == pos+len, "overshot while reading fork loop");
		ensure!(f.flow_insn()? == FlowInsn::Insn(Insn::Yield()), "invalid loop");
		ensure!(f.flow_insn()? == FlowInsn::Goto(pos), "invalid loop");
		Ok(insns)
	}

	pub(super) fn write(f: &mut impl Out, v: &Vec<Insn>) -> Result<(), WriteError> {
		todo!()
	}
}

mod party_equip_slot {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, arg1: &ItemId) -> Result<i8, ReadError> {
		if (600..800).contains(&arg1.0) {
			Ok(f.i8()?)
		} else {
			Ok(-1)
		}
	}

	pub(super) fn write(f: &mut impl Out, v: &i8, arg1: &ItemId) -> Result<(), WriteError> {
		todo!()
	}
}

mod menu {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Vec<String>, ReadError> {
		Ok(f.string()?.split_terminator('\x01').map(|a| a.to_owned()).collect())
	}

	pub(super) fn write(f: &mut impl Out, v: &Vec<String>) -> Result<(), WriteError> {
		todo!()
	}
}

mod emote {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Emote, ReadError> {
		Ok(Emote(f.u8()?, f.u8()?, f.u32()?))
	}

	pub(super) fn write(f: &mut impl Out, v: &Emote) -> Result<(), WriteError> {
		todo!()
	}
}

mod char_attr {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<CharAttr, ReadError> {
		Ok(CharAttr(CharId(f.u16()?), f.u8()?))
	}

	pub(super) fn write(f: &mut impl Out, v: &CharAttr) -> Result<(), WriteError> {
		todo!()
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(num_enum::TryFromPrimitive, num_enum::IntoPrimitive)]
#[repr(u8)]
pub enum ExprBinop {
	Eq = 0x02,
	Ne = 0x03,
	Lt = 0x04,
	Gt = 0x05,
	Le = 0x06,
	Ge = 0x07,

	BoolAnd = 0x09,
	And = 0x0A,
	Or = 0x0B,

	Add = 0x0C,
	Sub = 0x0D,
	Xor = 0x0F,
	Mul = 0x10,
	Div = 0x11,
	Mod = 0x12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(num_enum::TryFromPrimitive, num_enum::IntoPrimitive)]
#[repr(u8)]
pub enum ExprUnop {
	Not = 0x08,
	Neg = 0x0E,
	Ass = 0x13,
	MulAss = 0x14,
	DivAss = 0x15,
	ModAss = 0x16,
	AddAss = 0x17,
	SubAss = 0x18,
	AndAss = 0x19,
	XorAss = 0x1A,
	OrAss = 0x1B,
	Inv = 0x1D,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
	Const(u32),
	Binop(ExprBinop, Box<Expr>, Box<Expr>),
	Unop(ExprUnop, Box<Expr>),
	Exec(Box<Insn>),
	Flag(Flag),
	Var(Var),
	Attr(Attr),
	CharAttr(CharAttr),
	Rand,
}

mod expr {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Expr, ReadError> {
		let mut stack = Vec::new();
		loop {
			let op = f.u8()?;
			let expr = if let Ok(op) = ExprBinop::try_from(op) {
				let r = Box::new(stack.pop().ok_or("empty stack")?);
				let l = Box::new(stack.pop().ok_or("empty stack")?);
				Expr::Binop(op, l, r)
			} else if let Ok(op) = ExprUnop::try_from(op) {
				let v = Box::new(stack.pop().ok_or("empty stack")?);
				Expr::Unop(op, v)
			} else {
				match op {
					0x00 => Expr::Const(f.u32()?),
					0x01 => break,
					0x1C => Expr::Exec(Box::new(Insn::read(f)?)),
					0x1E => Expr::Flag(Flag(f.u16()?)),
					0x1F => Expr::Var(Var(f.u16()?)),
					0x20 => Expr::Attr(Attr(f.u8()?)),
					0x21 => Expr::CharAttr(CharAttr(CharId(f.u16()?), f.u8()?)),
					0x22 => Expr::Rand,
					op => return Err(format!("unknown Expr: 0x{op:02X}").into())
				}
			};
			stack.push(expr);
		}
		ensure!(stack.len() == 1, "invalid stack");
		Ok(stack.pop().unwrap())
	}

	pub(super) fn write(f: &mut impl Out, v: &Expr) -> Result<(), WriteError> {
		todo!()
	}
}
