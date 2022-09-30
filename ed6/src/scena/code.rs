use std::collections::{BTreeMap, BTreeSet};

use hamu::read::le::*;
use hamu::write::le::*;
use crate::archive::Archives;
use crate::tables::{bgmtbl::BgmId, Element};
use crate::tables::btlset::BattleId;
use crate::tables::item::ItemId;
use crate::tables::quest::QuestId;
use crate::tables::se::SoundId;
use crate::tables::town::TownId;
use crate::util::*;

use super::{FuncRef, CharId, CharAttr, Emote, Pos2, Pos3, Var, Flag, Attr, InExt2, OutExt2, Text};

type Color = u32;
type ShopId = u8;
type Member = u8;
type MagicId = u16;
type MemberAttr = u8;

type Flags = u32;
type QuestFlags = u8;
type CharFlags = u16;

mod insn;
pub use insn::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowInsn_<E, I, L> {
	Unless(E, L),
	Goto(L),
	Switch(E, Vec<(u16, L)>, L),
	Insn(I),
	Label(L), // Doesn't exist in RawFlowInsn
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Label(usize);

impl std::fmt::Debug for Label {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "L{}", self.0)
	}
}

type RawFlowInsn = FlowInsn_<Expr, Insn, usize>;
pub type FlowInsn = FlowInsn_<Expr, Insn, Label>;

pub fn read_func<'a>(f: &mut impl In<'a>, arc: &Archives, end: usize) -> Result<Vec<FlowInsn>, ReadError> {
	let mut insns = Vec::new();
	while f.pos() < end {
		insns.push((f.pos(), read_raw_insn(f, arc)?));
	}
	ensure!(f.pos() == end, "overshot while reading function");

	let mut labels = BTreeSet::new();
	for (i, insn) in &insns {
		match insn {
			RawFlowInsn::Unless(_, target) => {
				labels.insert(*target);
			},
			RawFlowInsn::Goto(target) => {
				labels.insert(*target);
			},
			RawFlowInsn::Switch(_, branches, default) => {
				for (_, target) in branches {
					labels.insert(*target);
				}
				labels.insert(*default);
			}
			RawFlowInsn::Insn(_) => {}
			RawFlowInsn::Label(_) => unreachable!(),
		}
	}

	let labels = labels.into_iter().enumerate().map(|(a,b)|(b,Label(a))).collect::<BTreeMap<_, _>>();

	let mut insns2 = Vec::with_capacity(insns.len() + labels.len());
	for (pos, insn) in insns {
		if let Some(label) = labels.get(&pos) {
			insns2.push(FlowInsn::Label(*label));
		}
		insns2.push(match insn {
			RawFlowInsn::Unless(e, a) => FlowInsn::Unless(e, labels[&a]),
			RawFlowInsn::Goto(a) => FlowInsn::Goto(labels[&a]),
			RawFlowInsn::Switch(e, cs, a) => FlowInsn::Switch(e, cs.into_iter().map(|(a, b)| (a, labels[&b])).collect(), labels[&a]),
			RawFlowInsn::Insn(i) => FlowInsn::Insn(i),
			RawFlowInsn::Label(_) => unreachable!(),
		})
	}

	Ok(insns2)
}

fn read_raw_insn<'a>(f: &mut impl In<'a>, arc: &Archives) -> Result<RawFlowInsn, ReadError> {
	f.dump().oneline().to_stdout();
	let pos = f.pos();
	Ok(match f.u8()? {
		0x02 => RawFlowInsn::Unless(expr::read(f, arc)?, f.u16()? as usize),
		0x03 => RawFlowInsn::Goto(f.u16()? as usize),
		0x04 => RawFlowInsn::Switch(expr::read(f, arc)?, {
			let mut out = Vec::new();
			for _ in 0..f.u16()? {
				out.push((f.u16()?, f.u16()? as usize));
			}
			out
		}, f.u16()? as usize),
		_ => {
			f.seek(pos)?;
			RawFlowInsn::Insn(Insn::read(f, arc)?)
		}
	})
}

pub fn write_insn(v: &str, f: &mut impl OutDelay<usize>, arc: &Archives) -> Result<(), WriteError> {
	todo!()
}

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

	pub(super) fn write(v: &Vec<QuestId>, f: &mut impl Out) -> Result<(), WriteError> {
		for &i in v {
			f.u16(i.0);
		}
		f.u16(0xFFFF);
		Ok(())
	}
}

mod fork {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, arc: &Archives) -> Result<Vec<Insn>, ReadError> {
		let len = f.u8()? as usize;
		let pos = f.pos();
		let mut insns = Vec::new();
		while f.pos() < pos+len {
			insns.push(Insn::read(f, arc)?);
		}
		ensure!(f.pos() == pos+len, "overshot while reading fork");
		f.check_u8(0)?;
		Ok(insns)
	}

	pub(super) fn write(v: &Vec<Insn>, f: &mut impl Out, arc: &Archives) -> Result<(), WriteError> {
		todo!()
	}
}

mod fork_loop {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, arc: &Archives) -> Result<Vec<Insn>, ReadError> {
		let len = f.u8()? as usize;
		let pos = f.pos();
		let mut insns = Vec::new();
		while f.pos() < pos+len {
			insns.push(Insn::read(f, arc)?);
		}
		ensure!(f.pos() == pos+len, "overshot while reading fork loop");
		ensure!(read_raw_insn(f, arc)? == RawFlowInsn::Insn(Insn::Yield()), "invalid loop");
		ensure!(read_raw_insn(f, arc)? == RawFlowInsn::Goto(pos), "invalid loop");
		Ok(insns)
	}

	pub(super) fn write(v: &Vec<Insn>, f: &mut impl Out, arc: &Archives) -> Result<(), WriteError> {
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

	pub(super) fn write(v: &i8, f: &mut impl Out, arg1: &ItemId) -> Result<(), WriteError> {
		todo!()
	}
}

mod menu {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Vec<String>, ReadError> {
		Ok(f.string()?.split_terminator('\x01').map(|a| a.to_owned()).collect())
	}

	pub(super) fn write(v: &Vec<String>, f: &mut impl Out) -> Result<(), WriteError> {
		todo!()
	}
}

mod emote {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Emote, ReadError> {
		Ok(Emote(f.u8()?, f.u8()?, f.u32()?))
	}

	pub(super) fn write(v: &Emote, f: &mut impl Out) -> Result<(), WriteError> {
		todo!()
	}
}

mod char_attr {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<CharAttr, ReadError> {
		Ok(CharAttr(CharId(f.u16()?), f.u8()?))
	}

	pub(super) fn write(v: &CharAttr, f: &mut impl Out) -> Result<(), WriteError> {
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
	pub(super) fn read<'a>(f: &mut impl In<'a>, arc: &Archives) -> Result<Expr, ReadError> {
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
					0x1C => Expr::Exec(Box::new(Insn::read(f, arc)?)),
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

	pub(super) fn write(v: &Expr, f: &mut impl Out, arc: &Archives) -> Result<(), WriteError> {
		todo!()
	}
}

mod file_ref {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, arc: &Archives) -> Result<String, ReadError> {
		Ok(arc.name(f.array()?)?.to_owned())
	}

	pub(super) fn write(v: &str, f: &mut impl Out, arc: &Archives) -> Result<(), WriteError> {
		f.array(arc.index(v)?);
		Ok(())
	}
}

mod text {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Text, ReadError> {
		todo!()
	}

	pub(super) fn write(v: &Text, f: &mut impl Out) -> Result<(), WriteError> {
		todo!()
	}
}

mod func_ref {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<FuncRef, ReadError> {
		Ok(FuncRef(f.u8()? as u16, f.u16()?))
	}

	pub(super) fn write(v: &FuncRef, f: &mut impl Out) -> Result<(), WriteError> {
		f.u16(cast(v.0)?);
		f.u16(v.1);
		Ok(())
	}
}
