use std::collections::HashMap;
use eyre::Result;
use derive_more::*;
use hamu::read::{In, Le};
use crate::util::{self, Text, InExt};

pub type Code = Vec<(usize, Insn)>;

#[derive(Clone, Copy, PartialEq, Eq, DebugCustom)]
#[debug(fmt = "FileRef({_0:#02X}, {_1})")]
pub struct FileRef(pub u16, pub u16); // (index, arch)

#[derive(Clone, Copy, PartialEq, Eq, DebugCustom)]
#[debug(fmt = "FuncRef({_0}, {_1})")]
pub struct FuncRef(pub u16, pub u16);

#[derive(Clone, Copy, PartialEq, Eq, DebugCustom)]
#[debug(fmt = "Pos3({_0}, {_1}, {_2})")]
pub struct Pos3(pub i32, pub i32, pub i32);

#[extend::ext(name=InExtForCode)]
impl In<'_> {
	fn file_ref(&mut self) -> hamu::read::Result<FileRef> {
		Ok(FileRef(self.u16()?, self.u16()?))
	}

	fn func_ref(&mut self) -> hamu::read::Result<FuncRef> {
		Ok(FuncRef(self.u16()?, self.u16()?))
	}

	fn pos3(&mut self) -> hamu::read::Result<Pos3> {
		Ok(Pos3(self.i32()?, self.i32()?, self.i32()?))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Char(pub u16);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flag(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprBinop {
	Eq, Ne, Lt, Gt, Le, Ge,
	BoolAnd, And, Or,
	Add, Sub, Xor, Mul, Div, Mod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprUnop {
	Not, Neg, Inv,
	Ass, MulAss, DivAss, ModAss, AddAss, SubAss, AndAss, XorAss, OrAss
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
	Const(u32),
	Binop(ExprBinop, Box<Expr>, Box<Expr>),
	Unop(ExprUnop, Box<Expr>),
	Exec(Box<Insn>),
	Flag(Flag),
	Var(u16 /*Var*/),
	Attr(u8 /*Attr*/),
	CharAttr(Char, u8 /*CharAttr*/),
	Rand,
}

pub fn read(i: In, end: usize) -> Result<Code> {
	CodeParser::new(i).func(end)
}

#[derive(Deref, DerefMut)]
struct CodeParser<'a> {
	marks: HashMap<usize, String>,
	#[deref]
	#[deref_mut]
	inner: In<'a>,
}

impl<'a> CodeParser<'a> {
	#[allow(clippy::new_without_default)]
	fn new(i: In<'a>) -> Self {
		CodeParser {
			marks: HashMap::new(),
			inner: i,
		}
	}

	fn func(&mut self, end: usize) -> Result<Code> {
		let start = self.inner.clone();
		let mut ops = Vec::new();
		(|| -> Result<_> {
			self.marks.insert(self.pos(), "\x1B[0;7m[".to_owned());
			while self.pos() < end {
				ops.push((self.pos(), self.insn()?));
				self.marks.insert(self.pos(), "\x1B[0;7m•".to_owned());
			}
			self.marks.insert(self.pos(), "\x1B[0;7m]".to_owned());
			eyre::ensure!(self.pos() == end, "Overshot: {:X} > {:X}", self.pos(), end);
			Ok(())
		})().map_err(|e| {
			use color_eyre::{Section, SectionExt};
			use std::fmt::Write;
			e.section({
				let mut s = String::new();
				for (addr, op) in &ops {
					writeln!(s, "{:04X}: {:?}", addr, op).unwrap();
				}
				s.pop(); // remove newline
				s.header("Code:")
			}).section({
				start.dump().end(end)
					.marks(self.marks.iter())
					.mark(self.pos()-1, "\x1B[0;7m ")
					.number_width(4)
					.newline(false)
					.to_string()
					.header("Dump:")
			})
		})?;
		Ok(ops)
	}

	fn insn(&mut self) -> Result<Insn> {
		insn(self)
	}

	fn expr(&mut self) -> Result<Expr> {
		ExprParser::new(self).expr()
	}

	fn text(&mut self) -> Result<Text> {
		self.marks.insert(self.pos(), "\x1B[0;7;2m\"".to_owned());
		let v = util::Text::read(self)?;
		self.marks.insert(self.pos(), "\x1B[0;7;2m\"".to_owned());
		Ok(v)
	}

	fn string(&mut self) -> Result<String> { self.str() }
	fn char(&mut self) -> Result<Char> { Ok(Char(self.u16()?)) }
	fn flag(&mut self) -> Result<Flag> { Ok(Flag(self.u16()?)) }
}

#[kaiseki_macros::bytecode(
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub enum Insn
)]
fn insn(i: &mut CodeParser) -> Result<Insn> {
	match u8 {
		0x01 => Return(),
		0x02 => If(Expr, {i.u16()? as usize} as usize + addr),
		0x03 => Goto({i.u16()? as usize} as usize + addr),
		0x04 => Switch(Expr, {
			let mut out = Vec::new();
			for _ in 0..i.u16()? {
				out.push((i.u16()?, i.u16()? as usize));
			}
			(out, i.u16()? as usize)
		} as (Vec<(u16, usize)>, usize) + switch_table),
		0x08 => Sleep(u32 + time),
		0x09 => FlagsSet(u32 + flags),
		0x0A => FlagsUnset(u32 + flags),
		0x0B => FadeOn(u32 + time, u32 + color, u8),
		0x0C => FadeOff(u32 + time, u32 + color),
		0x0D => _0D(),
		0x0F => Battle(u16 + battle, u16, u16, u16, u8, u16, i8),
		0x16 => Map(match u8 {
			0x00 => Hide(),
			0x01 => Show(),
			0x02 => Set(i32, i32, i32, FileRef),
		}),
		0x19 => EventBegin(u8),
		0x1A => EventEnd(u8),
		0x1B => _1B(u16, u16),
		0x1C => _1C(u16, u16),
		0x22 => SoundPlay(u16 + sound, u8, u8),
		0x23 => SoundStop(u16 + sound),
		0x24 => SoundLoop(u16 + sound, u8),
		0x28 => Quest(u16 + quest, match u8 {
			0x01 => TaskSet(u16 + quest_task),
			0x02 => TaskUnset(u16 + quest_task),
			0x03 => FlagsSet(u8 + quest_flag),
			0x04 => FlagsUnset(u8 + quest_flag),
		}),
		0x29 => QuestGet(u16 + quest, match u8 {
			0x00 => Task(u16 + quest_task),
			0x01 => Flags(u8 + quest_flag),
		}),
		0x30 => _Party30(u8),
		0x43 => CharForkFunc(Char, u8 /*ForkId*/, FuncRef),
		0x45 => CharFork(Char, u16 /*ForkId*/, {
			let end = i.u8()? as usize + i.pos();
			let mut insns = Vec::new();
			while i.pos() < end {
				i.marks.insert(i.pos(), "\x1B[0;7;2m•".to_owned());
				insns.push(insn(i)?);
			}
			eyre::ensure!(i.pos() == end, "Overshot: {:X} > {:X}", i.pos(), end);
			i.check_u8(0)?;
			insns
		} as Vec<Insn> + fork),
		0x49 => Event(FuncRef), // Not sure if this is different from Call
		0x4D => ExprVar(u16 + var, Expr),
		0x4F => ExprAttr(u8 + attr, Expr),
		0x51 => ExprCharAttr(Char, u8 + char_attr, Expr),
		0x53 => TextEnd(Char),
		0x54 => TextMessage(Text),
		0x56 => TextReset(u8),
		0x58 => TextWait(),
		0x5A => TextSetPos(i16, i16, i16, i16),
		0x5B => TextTalk(Char, Text),
		0x5C => TextTalkNamed(Char, String, Text),
		0x5D => Menu(u16 + menu_id, i16, i16, u8, {i.string()?.split_terminator('\x01').map(|a| a.to_owned()).collect()} as Vec<String> + menu),
		0x5E => MenuWait(u16 + menu_id),
		0x5F => _Menu5F(u16 + menu_id), // MenuClose?
		0x60 => TextSetName(String),
		0x69 => CamLookAt(Char, u32 + time),
		0x6C => CamAngle(i32 + angle, u32 + time),
		0x6D => CamPos(Pos3, u32 + time),
		0x87 => CharSetFrame(Char, u16),
		0x88 => CharSetPos(Char, Pos3, u16 + anle),
		0x8A => CharLookAt(Char, Char, u16 + time),
		0x8E => CharWalkTo(Char, Pos3, u32 + speed, u8),
		0x90 => CharWalk(Char, Pos3, u32 + speed, u8),  // I don't know how this differs from CharWalkTo; is it relative maybe?
		0x92 => _Char92(Char, Char, u32, u32 + time, u8),
		0x99 => CharAnimation(Char, u8, u8, u32),
		0x9A => CharFlagsSet(Char, u16 + char_flags),
		0x9B => CharFlagsUnset(Char, u16 + char_flags),
		0xA2 => FlagSet(Flag),
		0xA3 => FlagUnset(Flag),
		0xA5 => AwaitFlagUnset(Flag),
		0xA6 => AwaitFlagSet(Flag),
		0xB1 => OpLoad(String),
		0xB2 => _B2(u8, u8, u16),
		0xB4 => ReturnToTitle(u8),
	}
}

#[derive(Deref, DerefMut)]
struct ExprParser<'a, 'b> {
	stack: Vec<Expr>,
	#[deref]
	#[deref_mut]
	inner: &'a mut CodeParser<'b>,
}

impl<'a, 'b> ExprParser<'a, 'b> {
	fn new(inner: &'a mut CodeParser<'b>) -> ExprParser<'a, 'b> {
		ExprParser {
			stack: Vec::new(),
			inner,
		}
	}

	fn expr(mut self) -> Result<Expr> {
		self.inner.marks.insert(self.inner.pos(), "\x1B[0;7;2m[".to_owned());
		while let Some(op) = self.op()? {
			self.stack.push(op);
			self.inner.marks.insert(self.inner.pos(), "\x1B[0;7;2m•".to_owned());
		}
		self.inner.marks.insert(self.inner.pos(), "\x1B[0;7;2m]".to_owned());
		match self.stack.len() {
			1 => Ok(self.pop()?),
			_ => eyre::bail!("Invalid Expr: {:?}", self.stack)
		}
	}

	fn op(&mut self) -> Result<Option<Expr>> {
		Ok(Some(match self.u8()? {
			0x00 => Expr::Const(self.u32()?),
			0x01 => return Ok(None),
			0x02 => self.binop(ExprBinop::Eq)?,
			0x03 => self.binop(ExprBinop::Ne)?,
			0x04 => self.binop(ExprBinop::Lt)?,
			0x05 => self.binop(ExprBinop::Gt)?,
			0x06 => self.binop(ExprBinop::Le)?,
			0x07 => self.binop(ExprBinop::Ge)?,
			0x08 => self.unop(ExprUnop::Not)?,
			0x09 => self.binop(ExprBinop::BoolAnd)?,
			0x0A => self.binop(ExprBinop::And)?,
			0x0B => self.binop(ExprBinop::Or)?,
			0x0C => self.binop(ExprBinop::Add)?,
			0x0D => self.binop(ExprBinop::Sub)?,
			0x0E => self.unop(ExprUnop::Neg)?,
			0x0F => self.binop(ExprBinop::Xor)?,
			0x10 => self.binop(ExprBinop::Mul)?,
			0x11 => self.binop(ExprBinop::Div)?,
			0x12 => self.binop(ExprBinop::Mod)?,
			0x13 => self.unop(ExprUnop::Ass)?,
			0x14 => self.unop(ExprUnop::MulAss)?,
			0x15 => self.unop(ExprUnop::DivAss)?,
			0x16 => self.unop(ExprUnop::ModAss)?,
			0x17 => self.unop(ExprUnop::AddAss)?,
			0x18 => self.unop(ExprUnop::SubAss)?,
			0x19 => self.unop(ExprUnop::AndAss)?,
			0x1A => self.unop(ExprUnop::XorAss)?,
			0x1B => self.unop(ExprUnop::OrAss)?,
			0x1C => Expr::Exec(Box::new(self.insn()?)),
			0x1D => self.unop(ExprUnop::Inv)?,
			0x1E => Expr::Flag(Flag(self.u16()?)),
			0x1F => Expr::Var(self.u16()?),
			0x20 => Expr::Attr(self.u8()?),
			0x21 => Expr::CharAttr(Char(self.u16()?), self.u8()?),
			0x22 => Expr::Rand,
			op => eyre::bail!("Unknown Expr: {:02X}", op)
		}))
	}

	fn binop(&mut self, op: ExprBinop) -> Result<Expr> {
		let r = Box::new(self.pop()?);
		let l = Box::new(self.pop()?);
		Ok(Expr::Binop(op, l, r))
	}

	fn unop(&mut self, op: ExprUnop) -> Result<Expr> {
		let v = Box::new(self.pop()?);
		Ok(Expr::Unop(op, v))
	}

	fn pop(&mut self) -> Result<Expr> {
		Ok(self.stack.pop().ok_or_else(|| eyre::eyre!("Empty expr stack"))?)
	}
}
