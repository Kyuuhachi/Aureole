use std::collections::{BTreeMap, BTreeSet};

use hamu::read::le::*;
use hamu::write::le::*;
use hamu::write::Label as HLabel;
use hamu::write::LabelDef as HLabelDef;
use crate::gamedata::GameData;
use crate::tables::{bgmtbl::BgmId, Element};
use crate::tables::btlset::BattleId;
use crate::tables::item::ItemId;
use crate::tables::quest::QuestId;
use crate::tables::se::SoundId;
use crate::tables::town::TownId;
use crate::util::*;
use crate::text::Text;

use super::{
	Attr, CharAttr, CharFlags, CharId, Color, Emote, Flag, Flags, FuncRef, InExt2, MagicId, Member,
	MemberAttr, OutExt2, Pos2, Pos3, QuestFlags, ShopId, Var,
};

mod insn;
pub use insn::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Label(pub usize);

impl std::fmt::Debug for Label {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "L{}", self.0)
	}
}

// I *could* make this generic over <Expr, Insn, Label, LabelDef>, but honestly, no.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlatInsn {
	Unless(Expr, Label),
	Goto(Label),
	Switch(Expr, Vec<(u16, Label)>, Label),
	Insn(Insn),
	Label(Label),
}

#[derive(Debug, PartialEq, Eq)]
enum RawIInsn {
	Unless(Expr, usize),
	Goto(usize),
	Switch(Expr, Vec<(u16, usize)>, usize),
	Insn(Insn),
}

#[derive(Debug, PartialEq, Eq)]
enum RawOInsn<'a> {
	Unless(&'a Expr, HLabel),
	Goto(HLabel),
	Switch(&'a Expr, Vec<(u16, HLabel)>, HLabel),
	Insn(&'a Insn),
	Label(HLabelDef),
}

pub fn read<'a>(f: &mut impl In<'a>, arc: &GameData, end: usize) -> Result<Vec<FlatInsn>, ReadError> {
	let mut insns = Vec::new();
	while f.pos() < end {
		insns.push((f.pos(), read_raw_insn(f, arc)?));
	}
	ensure!(f.pos() == end, "overshot while reading function");

	let mut labels = BTreeSet::new();
	for (_, insn) in &insns {
		match insn {
			RawIInsn::Unless(_, target) => {
				labels.insert(*target);
			},
			RawIInsn::Goto(target) => {
				labels.insert(*target);
			},
			RawIInsn::Switch(_, branches, default) => {
				for (_, target) in branches {
					labels.insert(*target);
				}
				labels.insert(*default);
			}
			RawIInsn::Insn(_) => {}
		}
	}

	let labels = labels.into_iter().enumerate().map(|(a,b)|(b,Label(a))).collect::<BTreeMap<_, _>>();

	let mut insns2 = Vec::with_capacity(insns.len() + labels.len());
	for (pos, insn) in insns {
		if let Some(label) = labels.get(&pos) {
			insns2.push(FlatInsn::Label(*label));
		}
		insns2.push(match insn {
			RawIInsn::Unless(e, l) => FlatInsn::Unless(e, labels[&l]),
			RawIInsn::Goto(l) => FlatInsn::Goto(labels[&l]),
			RawIInsn::Switch(e, cs, l) => FlatInsn::Switch(e, cs.into_iter().map(|(a, l)| (a, labels[&l])).collect(), labels[&l]),
			RawIInsn::Insn(i) => FlatInsn::Insn(i),
		})
	}

	Ok(insns2)
}

fn read_raw_insn<'a>(f: &mut impl In<'a>, arc: &GameData) -> Result<RawIInsn, ReadError> {
	let pos = f.pos();
	let res = try {
		let insn = match f.u8()? {
			0x02 => {
				let e = expr::read(f, arc)?;
				let l = f.u16()? as usize;
				RawIInsn::Unless(e, l)
			}
			0x03 => {
				let l = f.u16()? as usize;
				RawIInsn::Goto(l)
			}
			0x04 => {
				let e = expr::read(f, arc)?;
				let mut cs = Vec::new();
				for _ in 0..f.u16()? {
					cs.push((f.u16()?, f.u16()? as usize));
				}
				let l = f.u16()? as usize;
				RawIInsn::Switch(e, cs, l)
			}
			_ => {
				f.seek(pos)?;
				let i = Insn::read(f, arc)?;
				RawIInsn::Insn(i)
			}
		};
		insn
	};
	match res {
		Ok(a) => Ok(a),
		Err(e) => {
			f.seek(pos.saturating_sub(48*2))?;
			f.dump().lines(4).mark(pos, 9).to_stderr();
			Err(e)
		}
	}
}

pub fn write(f: &mut impl OutDelay, arc: &GameData, insns: &[FlatInsn]) -> Result<(), WriteError> {
	let mut labels = BTreeMap::new();
	let mut labeldefs = BTreeMap::new();
	let mut label = |k| {
		if let std::collections::btree_map::Entry::Vacant(e) = labels.entry(k) {
			let (l, l_) = HLabel::new();
			e.insert(l);
			labeldefs.insert(k, l_);
		}
	};

	for insn in insns {
		match insn {
			FlatInsn::Unless(_, target) => {
				label(*target);
			},
			FlatInsn::Goto(target) => {
				label(*target);
			},
			FlatInsn::Switch(_, branches, default) => {
				for (_, target) in branches {
					label(*target);
				}
				label(*default);
			}
			FlatInsn::Insn(_) => {}
			FlatInsn::Label(l) => label(*l),
		}
	}

	for insn in insns {
		write_raw_insn(f, arc, match insn {
			FlatInsn::Unless(e, l) => RawOInsn::Unless(e, labels[l].clone()),
			FlatInsn::Goto(l) => RawOInsn::Goto(labels[l].clone()),
			FlatInsn::Switch(e, cs, l) => RawOInsn::Switch(e, cs.iter().map(|(a, l)| (*a, labels[l].clone())).collect(), labels[l].clone()),
			FlatInsn::Insn(i) => RawOInsn::Insn(i),
			FlatInsn::Label(l) => RawOInsn::Label(labeldefs.remove(l).unwrap()),
		})?;
	}

	ensure!(labeldefs.is_empty(), "unreferenced labels: {:?}", Vec::from_iter(labeldefs.keys()));

	Ok(())
}

fn write_raw_insn(f: &mut impl OutDelay, arc: &GameData, insn: RawOInsn) -> Result<(), WriteError> {
	match insn {
		RawOInsn::Unless(e, l) => {
			f.u8(0x02);
			expr::write(f, arc, e)?;
			f.delay_u16(l);
		},
		RawOInsn::Goto(l) => {
			f.u8(0x03);
			f.delay_u16(l);
		},
		RawOInsn::Switch(e, cs, l) => {
			f.u8(0x04);
			expr::write(f, arc, e)?;
			f.u16(cast(cs.len())?);
			for (k, v) in cs {
				f.u16(k);
				f.delay_u16(v);
			}
			f.delay_u16(l);
		}
		RawOInsn::Insn(i) => {
			Insn::write(f, arc, i)?
		}
		RawOInsn::Label(l) => {
			f.label(l)
		}
	}
	Ok(())
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
	pub(super) fn read<'a>(f: &mut impl In<'a>, arc: &GameData) -> Result<Vec<Insn>, ReadError> {
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

	pub(super) fn write(f: &mut impl OutDelay, arc: &GameData, v: &[Insn]) -> Result<(), WriteError> {
		let (l1, l1_) = HLabel::new();
		let (l2, l2_) = HLabel::new();
		f.delay(move |l| Ok(u8::to_le_bytes(hamu::write::cast_usize(l(l2)? - l(l1)?)?)));
		f.label(l1_);
		for i in v {
			Insn::write(f, arc, i)?;
		}
		f.label(l2_);
		f.u8(0);
		Ok(())
	}
}

mod fork_loop {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, arc: &GameData) -> Result<Vec<Insn>, ReadError> {
		let len = f.u8()? as usize;
		let pos = f.pos();
		let mut insns = Vec::new();
		while f.pos() < pos+len {
			insns.push(Insn::read(f, arc)?);
		}
		ensure!(f.pos() == pos+len, "overshot while reading fork loop");
		ensure!(read_raw_insn(f, arc)? == RawIInsn::Insn(Insn::Yield()), "invalid loop");
		ensure!(read_raw_insn(f, arc)? == RawIInsn::Goto(pos), "invalid loop");
		Ok(insns)
	}

	pub(super) fn write(f: &mut impl OutDelay, arc: &GameData, v: &[Insn]) -> Result<(), WriteError> {
		let (l1, l1_) = HLabel::new();
		let (l2, l2_) = HLabel::new();
		let l1c = l1.clone();
		f.delay(|l| Ok(u8::to_le_bytes(hamu::write::cast_usize(l(l2)? - l(l1)?)?)));
		f.label(l1_);
		for i in v {
			Insn::write(f, arc, i)?;
		}
		f.label(l2_);
		write_raw_insn(f, arc, RawOInsn::Insn(&Insn::Yield()))?;
		write_raw_insn(f, arc, RawOInsn::Goto(l1c))?;
		Ok(())
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

	pub(super) fn write(f: &mut impl Out, arg1: &ItemId, v: &i8) -> Result<(), WriteError> {
		if (600..800).contains(&arg1.0) {
			f.i8(*v);
		} else {
			ensure!(*v == -1, "invalid PartyEquipSlot");
		}
		Ok(())
	}
}

mod menu {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Vec<String>, ReadError> {
		Ok(f.string()?.split_terminator('\x01').map(|a| a.to_owned()).collect())
	}

	pub(super) fn write(f: &mut impl Out, v: &[String]) -> Result<(), WriteError> {
		let mut s = String::new();
		for line in v {
			s.push_str(line.as_str());
			s.push('\x01');
		}
		f.string(&s)?;
		Ok(())
	}
}

mod emote {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Emote, ReadError> {
		let a = f.u8()?;
		let b = f.u8()?;
		let c = f.u32()?;
		Ok(Emote(a, b, c))
	}

	pub(super) fn write(f: &mut impl Out, &Emote(a, b, c): &Emote) -> Result<(), WriteError> {
		f.u8(a);
		f.u8(b);
		f.u32(c);
		Ok(())
	}
}

mod char_attr {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<CharAttr, ReadError> {
		let a = CharId(f.u16()?);
		let b = f.u8()?;
		Ok(CharAttr(a, b))
	}

	pub(super) fn write(f: &mut impl Out, &CharAttr(a, b): &CharAttr) -> Result<(), WriteError> {
		f.u16(a.0);
		f.u8(b);
		Ok(())
	}
}

mod file_ref {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, arc: &GameData) -> Result<String, ReadError> {
		Ok(arc.name(f.u32()?)?.to_owned())
	}

	pub(super) fn write(f: &mut impl Out, arc: &GameData, v: &str) -> Result<(), WriteError> {
		f.u32(arc.index(v)?);
		Ok(())
	}
}

mod func_ref {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<FuncRef, ReadError> {
		let a = f.u8()? as u16;
		let b = f.u16()?;
		Ok(FuncRef(a, b))
	}

	pub(super) fn write(f: &mut impl Out, &FuncRef(a, b): &FuncRef) -> Result<(), WriteError> {
		f.u8(cast(a)?);
		f.u16(b);
		Ok(())
	}
}

mod text {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Text, ReadError> {
		crate::text::Text::read(f)
	}

	pub(super) fn write(f: &mut impl Out, v: &Text) -> Result<(), WriteError> {
		crate::text::Text::write(f, v)
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
	Insn(Box<Insn>),
	Flag(Flag),
	Var(Var),
	Attr(Attr),
	CharAttr(CharAttr),
	Rand,
}

mod expr {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, arc: &GameData) -> Result<Expr, ReadError> {
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
					0x1C => Expr::Insn(Box::new(Insn::read(f, arc)?)),
					0x1E => Expr::Flag(Flag(f.u16()?)),
					0x1F => Expr::Var(Var(f.u16()?)),
					0x20 => Expr::Attr(Attr(f.u8()?)),
					0x21 => Expr::CharAttr(char_attr::read(f)?),
					0x22 => Expr::Rand,
					op => return Err(format!("unknown Expr: 0x{op:02X}").into())
				}
			};
			stack.push(expr);
		}
		ensure!(stack.len() == 1, "invalid stack");
		Ok(stack.pop().unwrap())
	}

	pub(super) fn write(f: &mut impl OutDelay, arc: &GameData, v: &Expr) -> Result<(), WriteError> {
		fn write_node(f: &mut impl OutDelay, arc: &GameData, v: &Expr) -> Result<(), WriteError> {
			match *v {
				Expr::Binop(op, ref a, ref b) => {
					write_node(f, arc, a)?;
					write_node(f, arc, b)?;
					f.u8(op.into());
				},
				Expr::Unop(op, ref v) => {
					write_node(f, arc, v)?;
					f.u8(op.into());
				},
				Expr::Const(n)       => { f.u8(0x00); f.u32(n); },
				// 0x01 handled below
				Expr::Insn(ref insn) => { f.u8(0x1C); Insn::write(f, arc, insn)?; },
				Expr::Flag(v)        => { f.u8(0x1E); f.u16(v.0); },
				Expr::Var(v)         => { f.u8(0x1F); f.u16(v.0); },
				Expr::Attr(v)        => { f.u8(0x20); f.u8(v.0); },
				Expr::CharAttr(v)    => { f.u8(0x21); char_attr::write(f, &v)?; },
				Expr::Rand           => { f.u8(0x22); },
			}
			Ok(())
		}
		write_node(f, arc, v)?;
		f.u8(0x01);
		Ok(())
	}
}
