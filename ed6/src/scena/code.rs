use std::collections::HashMap;
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
pub mod decompile;
pub use decompile::decompile;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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
	let mut labels = HashMap::new();
	let mut labeldefs = HashMap::new();
	let mut label = |k| {
		if let std::collections::hash_map::Entry::Vacant(e) = labels.entry(k) {
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
