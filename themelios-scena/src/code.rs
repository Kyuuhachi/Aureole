use std::collections::HashMap;
use std::collections::{BTreeMap, BTreeSet};

use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _, Label as GLabel};
use crate::types::*;
use crate::util::*;
use crate::text::Text;

mod insn;
pub use insn::{Insn, introspect};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Label(pub usize);

impl std::fmt::Debug for Label {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "L{}", self.0)
	}
}

#[extend::ext]
impl Reader<'_> {
	fn pos2(&mut self) -> Result<Pos2, gospel::read::Error> {
		Ok(Pos2 { x: self.i32()?, z: self.i32()? })
	}

	fn pos3(&mut self) -> Result<Pos3, gospel::read::Error> {
		Ok(Pos3 { x: self.i32()?, y: self.i32()?, z: self.i32()? })
	}
}

#[extend::ext]
impl Writer {
	fn pos2(&mut self, p: Pos2) {
		self.i32(p.x);
		self.i32(p.z);
	}

	fn pos3(&mut self, p: Pos3) {
		self.i32(p.x);
		self.i32(p.y);
		self.i32(p.z);
	}
}

// TODO make this one stricter so it does not permit duplicate labels
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Code(pub Vec<FlatInsn>);

impl std::ops::Deref for Code {
	type Target = Vec<FlatInsn>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl std::ops::DerefMut for Code {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
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
	Unless(&'a Expr, GLabel),
	Goto(GLabel),
	Switch(&'a Expr, Vec<(u16, GLabel)>, GLabel),
	Insn(&'a Insn),
	Label(GLabel),
}

impl Code {
	pub fn read(f: &mut Reader, game: Game, end: Option<usize>) -> Result<Code, ReadError> {
		let mut insns = Vec::new();
		let mut extent = f.pos();
		loop {
			if let Some(end) = end && f.pos() >= end {
				ensure!(f.pos() == end, "overshot while reading function");
				break
			}
			insns.push((f.pos(), read_raw_insn(f, game)?));
			match &insns.last().unwrap().1 {
				RawIInsn::Insn(Insn::Return()) if end.is_none() && f.pos() > extent => break,
				RawIInsn::Insn(_) => {}
				RawIInsn::Unless(_, l) => extent = extent.max(*l),
				RawIInsn::Goto(l) => extent = extent.max(*l),
				RawIInsn::Switch(_, cs, l) => {
					extent = cs.iter().map(|a| a.1)
						.chain(Some(*l))
						.chain(Some(extent))
						.max().unwrap();
				}
			}
		}

		let mut labels = BTreeSet::new();
		for (_, insn) in &insns {
			match insn {
				RawIInsn::Unless(_, target) => {
					labels.insert(*target);
				}
				RawIInsn::Goto(target) => {
					labels.insert(*target);
				}
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

		Ok(Code(insns2))
	}

	pub fn write(f: &mut Writer, game: Game, insns: &Code) -> Result<(), WriteError> {
		let mut labels = HashMap::new();
		let mut labeldefs = HashMap::new();
		let mut label = |k| {
			if let std::collections::hash_map::Entry::Vacant(e) = labels.entry(k) {
				let l = GLabel::new();
				e.insert(l);
				labeldefs.insert(k, l);
			}
		};

		for insn in &insns.0 {
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

		for insn in &insns.0 {
			write_raw_insn(f, game, match insn {
				FlatInsn::Unless(e, l) => RawOInsn::Unless(e, labels[l]),
				FlatInsn::Goto(l) => RawOInsn::Goto(labels[l]),
				FlatInsn::Switch(e, cs, l) => RawOInsn::Switch(e, cs.iter().map(|(a, l)| (*a, labels[l])).collect(), labels[l]),
				FlatInsn::Insn(i) => RawOInsn::Insn(i),
				FlatInsn::Label(l) => RawOInsn::Label(labeldefs.remove(l).unwrap()),
			})?;
		}

		ensure!(labeldefs.is_empty(), "unreferenced labels: {:?}", Vec::from_iter(labeldefs.keys()));

		Ok(())
	}
}

fn read_raw_insn(f: &mut Reader, game: Game) -> Result<RawIInsn, ReadError> {
	let pos = f.pos();
	fn addr(f: &mut Reader, game: Game) -> Result<usize, ReadError> {
		if game.is_ed7() {
			Ok(f.u32()? as usize)
		} else {
			Ok(f.u16()? as usize)
		}
	}
	let insn = match f.u8()? {
		0x02 => {
			let e = Expr::read(f, game)?;
			let l = addr(f, game)?;
			RawIInsn::Unless(e, l)
		}
		0x03 => {
			let l = addr(f, game)?;
			RawIInsn::Goto(l)
		}
		0x04 => {
			let e = Expr::read(f, game)?;
			let count = if game.is_ed7() {
				f.u8()? as u16
			} else {
				f.u16()?
			};
			let mut cs = Vec::with_capacity(count as usize);
			for _ in 0..count {
				cs.push((f.u16()?, addr(f, game)?));
			}
			let l = addr(f, game)?;
			RawIInsn::Switch(e, cs, l)
		}
		_ => {
			f.seek(pos)?;
			let i = Insn::read(f, game)?;
			RawIInsn::Insn(i)
		}
	};
	Ok(insn)
}

fn write_raw_insn(f: &mut Writer, game: Game, insn: RawOInsn) -> Result<(), WriteError> {
	fn addr(f: &mut Writer, game: Game, l: GLabel) {
		if game.is_ed7() {
			f.delay32(l)
		} else {
			f.delay16(l)
		}
	}
	match insn {
		RawOInsn::Unless(e, l) => {
			f.u8(0x02);
			Expr::write(f, game, e)?;
			addr(f, game, l);
		},
		RawOInsn::Goto(l) => {
			f.u8(0x03);
			addr(f, game, l);
		},
		RawOInsn::Switch(e, cs, l) => {
			f.u8(0x04);
			Expr::write(f, game, e)?;
			if game.is_ed7() {
				f.u8(cast(cs.len())?)
			} else {
				f.u16(cast(cs.len())?)
			}
			for (k, v) in cs {
				f.u16(k);
				addr(f, game, v);
			}
			addr(f, game, l);
		}
		RawOInsn::Insn(i) => {
			Insn::write(f, game, i)?
		}
		RawOInsn::Label(l) => {
			f.label(l)
		}
	}
	Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
	Unary,
	Binary, // includes comparisons
	Assign
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(num_enum::TryFromPrimitive, num_enum::IntoPrimitive)]
#[repr(u8)]
pub enum ExprOp {
	Eq      = 0x02, // ==
	Ne      = 0x03, // !=
	Lt      = 0x04, // <
	Gt      = 0x05, // >
	Le      = 0x06, // <=
	Ge      = 0x07, // >=
	Not     = 0x08, // !
	BoolAnd = 0x09, // &&
	And     = 0x0A, // &
	Or      = 0x0B, // | and ||
	Add     = 0x0C, // +
	Sub     = 0x0D, // -
	Neg     = 0x0E, // -
	Xor     = 0x0F, // ^
	Mul     = 0x10, // *
	Div     = 0x11, // /
	Mod     = 0x12, // %
	Ass     = 0x13, // =
	MulAss  = 0x14, // *=
	DivAss  = 0x15, // /=
	ModAss  = 0x16, // %=
	AddAss  = 0x17, // +=
	SubAss  = 0x18, // -=
	AndAss  = 0x19, // &=
	XorAss  = 0x1A, // ^=
	OrAss   = 0x1B, // |=
	Inv     = 0x1D, // ~
}

impl ExprOp {
	pub fn kind(self) -> OpKind {
		use ExprOp::*;
		match self {
			Not|Neg|Inv => OpKind::Unary,
			Eq|Ne|Lt|Le|Gt|Ge => OpKind::Binary,
			BoolAnd|And|Or|Add|Sub|Xor|Mul|Div|Mod => OpKind::Binary,
			Ass|MulAss|DivAss|ModAss|AddAss|SubAss|AndAss|XorAss|OrAss => OpKind::Assign,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum ExprTerm {
	Const(u32)         = 0x00,
	Op(ExprOp),
	Insn(Box<Insn>)    = 0x1C,
	Flag(Flag)         = 0x1E,
	Var(Var)           = 0x1F,
	Attr(Attr)         = 0x20,
	CharAttr(CharAttr) = 0x21,
	Rand               = 0x22, // random 15-bit number
	Global(Global)     = 0x23,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr(pub Vec<ExprTerm>);

impl Expr {
	pub fn read(f: &mut Reader, game: Game) -> Result<Expr, ReadError> {
		let mut terms = Vec::new();
		loop {
			let op = f.u8()?;
			let term = if let Ok(op) = ExprOp::try_from(op) {
				ExprTerm::Op(op)
			} else {
				match op {
					0x00 => ExprTerm::Const(f.u32()?),
					0x01 => break,
					0x1C => ExprTerm::Insn(Box::new(Insn::read(f, game)?)),
					0x1E => ExprTerm::Flag(Flag(f.u16()?)),
					0x1F => ExprTerm::Var(Var(f.u16()?)),
					0x20 => ExprTerm::Attr(Attr(f.u8()?)),
					0x21 => ExprTerm::CharAttr(insn::char_attr::read(f, game)?),
					0x22 => ExprTerm::Rand,
					0x23 => ExprTerm::Global(Global(f.u8()?)),
					op => return Err(format!("unknown Expr: 0x{op:02X}").into())
				}
			};
			terms.push(term);
		}
		Ok(Expr(terms))
	}

	pub fn write(f: &mut Writer, game: Game, v: &Expr) -> Result<(), WriteError> {
		for term in &v.0 {
			match *term {
				ExprTerm::Const(n)       => { f.u8(0x00); f.u32(n); }
				ExprTerm::Op(op)         => { f.u8(op.into()) }
				ExprTerm::Insn(ref insn) => { f.u8(0x1C); Insn::write(f, game, insn)?; }
				ExprTerm::Flag(v)        => { f.u8(0x1E); f.u16(v.0); }
				ExprTerm::Var(v)         => { f.u8(0x1F); f.u16(v.0); }
				ExprTerm::Attr(v)        => { f.u8(0x20); f.u8(v.0); }
				ExprTerm::CharAttr(v)    => { f.u8(0x21); insn::char_attr::write(f, game, &v)?; }
				ExprTerm::Rand           => { f.u8(0x22); }
				ExprTerm::Global(v)      => { f.u8(0x23); f.u8(v.0); }
			}
		}
		f.u8(0x01);
		Ok(())
	}
}
