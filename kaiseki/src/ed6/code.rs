use std::collections::HashMap;
use eyre::Result;
use hamu::read::{In, Le};
use crate::util::{self, Text, InExt};

pub type Code = Vec<(usize, Insn)>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FileRef(pub u16, pub u16); // (index, arch)

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FuncRef(pub u16, pub u16);

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Pos3(pub i32, pub i32, pub i32);

impl std::fmt::Debug for FileRef {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "FileRef({:#02X}, {})", self.0, self.1)
	}
}

impl std::fmt::Debug for FuncRef {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "FuncRef({}, {})", self.0, self.1)
	}
}

impl std::fmt::Debug for Pos3 {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "Pos3({}, {}, {})", self.0, self.1, self.2)
	}
}

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
pub struct Character(pub u16);
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
	Exec(Insn),
	Flag(Flag),
	Var(u16 /*Var*/),
	Attr(u8 /*Attr*/),
	CharAttr(Character, u8 /*CharAttr*/),
	Rand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Insn {
	/*01*/ Return,
	/*02*/ If(Box<Expr>, usize /*Addr*/),
	/*03*/ Goto(usize /*Addr*/),
	/*04*/ Switch(Box<Expr>, Vec<(u16, usize /*Addr*/)>, usize /*Addr*/ /*(default)*/),
	/*08*/ Sleep(u32 /*Time*/),
	/*09*/ FlagsSet(u32 /*Flags*/),
	/*0A*/ FlagsUnset(u32 /*Flags*/),
	/*0B*/ FadeOn(u32 /*Time*/, u32 /*Color*/, u8),
	/*0C*/ FadeOff(u32 /*Time*/, u32 /*Color*/),
	/*0D*/ _0D,
	/*0F*/ Battle(u16 /*BattleId*/, u16, u16, u16, u8, u16, i8),
	/*16*/ Map(MapInsn),
	/*19*/ EventBegin(u8),
	/*1A*/ EventEnd(u8),
	/*1B*/ _1B(u16, u16),
	/*1C*/ _1C(u16, u16),
	/*22*/ SoundPlay(u16 /*Sound*/, u8, u8 /*Volume*/),
	/*23*/ SoundStop(u16 /*Sound*/),
	/*24*/ SoundLoop(u16 /*Sound*/, u8),
	/*28*/ Quest(u16 /*Quest*/, QuestInsn),
	/*29*/ QuestGet(u16 /*Quest*/, QuestGetInsn),
	/*30*/ _Party30(u8),
	/*43*/ CharForkFunc(Character, u8 /*ForkId*/, FuncRef),
	/*45*/ CharFork(Character, u16 /*ForkId*/, Vec<Insn>), // why is this is u16?
	/*49*/ Event(FuncRef), // Not sure if this is different from Call
	/*4D*/ ExprVar(u16 /*Var*/, Box<Expr>),
	/*4F*/ ExprAttr(u8 /*Attr*/, Box<Expr>),
	/*51*/ ExprCharAttr(Character, u8 /*CharAttr*/, Box<Expr>),
	/*53*/ TextEnd(Character),
	/*54*/ TextMessage(Text),
	/*56*/ TextReset(u8),
	/*58*/ TextWait,
	/*5A*/ TextSetPos(i16, i16, i16, i16),
	/*5B*/ TextTalk(Character, Text),
	/*5C*/ TextTalkNamed(Character, String, Text),
	/*5D*/ Menu(u16 /*MenuId*/, (i16, i16) /*Pos*/, u8, Vec<String>),
	/*5E*/ MenuWait(u16 /*MenuId*/),
	/*5F*/ _Menu5F(u16 /*MenuId*/), // MenuClose?
	/*60*/ TextSetName(String),
	/*69*/ CamLookAt(Character, u32 /*Time*/),
	/*6C*/ CamAngle(i32 /*Angle*/, u32 /*Time*/),
	/*6D*/ CamPos(Pos3, u32 /*Time*/),
	/*87*/ CharSetFrame(Character, u16),
	/*88*/ CharSetPos(Character, Pos3, u16 /*Angle*/),
	/*8A*/ CharLookAt(Character, Character, u16 /*Time*/),
	/*8E*/ CharWalkTo(Character, Pos3, u32 /*Speed*/, u8),
	/*90*/ CharWalk(Character, Pos3, u32 /*Speed*/, u8), // I don't know how this differs from CharWalkTo; is it relative maybe?
	/*92*/ _Char92(Character, Character, u32, u32, u8),
	/*99*/ CharAnimation(Character, u8, u8, u32 /*Time*/),
	/*9A*/ CharFlagsSet(Character, u16 /*CharFlags*/),
	/*9B*/ CharFlagsUnset(Character, u16 /*CharFlags*/),
	/*A2*/ FlagSet(Flag),
	/*A3*/ FlagUnset(Flag),
	/*A5*/ AwaitFlagUnset(Flag),
	/*A6*/ AwaitFlagSet(Flag),
	/*B1*/ OpLoad(String /*._OP filename*/),
	/*B2*/ _B2(u8, u8, u16),
	/*B4*/ ReturnToTitle(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapInsn {
	/*00*/ Hide,
	/*01*/ Show,
	/*02*/ Set(i32, (i32, i32), FileRef /* archive 03 */), // XXX this seems to be (arch, index) while others are (index, arch)?
}

// I am unsure whether these are Set or Unset
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestInsn {
	/*01*/ TaskSet(u16),
	/*02*/ TaskUnset(u16),
	/*03*/ FlagsSet(u8),
	/*04*/ FlagsUnset(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestGetInsn {
	/*00*/ Task(u16),
	/*01*/ Flags(u8),
}

pub fn read(i: In, end: usize) -> Result<Code> {
	CodeParser::new(i).func(end)
}

#[derive(derive_more::Deref, derive_more::DerefMut)]
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
		Ok(match self.u8()? {
			0x01 => Insn::Return,
			0x02 => Insn::If(self.expr()?, self.u16()? as usize),
			0x03 => Insn::Goto(self.u16()? as usize),
			0x04 => Insn::Switch(self.expr()?, {
				let mut out = Vec::new();
				for _ in 0..self.u16()? {
					out.push((self.u16()?, self.u16()? as usize));
				}
				out
			}, self.u16()? as usize),
			0x08 => Insn::Sleep(self.u32()?),
			0x09 => Insn::FlagsSet(self.u32()?),
			0x0A => Insn::FlagsUnset(self.u32()?),
			0x0B => Insn::FadeOn(self.u32()?, self.u32()?, self.u8()?),
			0x0C => Insn::FadeOff(self.u32()?, self.u32()?),
			0x0D => Insn::_0D,
			0x0F => Insn::Battle(self.u16()?, self.u16()?, self.u16()?, self.u16()?, self.u8()?, self.u16()?, self.i8()?),
			0x16 => Insn::Map(match self.u8()? {
				0x00 => MapInsn::Hide,
				0x01 => MapInsn::Show,
				0x02 => MapInsn::Set(self.i32()?, (self.i32()?, self.i32()?), self.file_ref()?),
				op => eyre::bail!("Unknown MapInsn: {:02X}", op)
			}),
			0x19 => Insn::EventBegin(self.u8()?),
			0x1A => Insn::EventEnd(self.u8()?),
			0x1B => Insn::_1B(self.u16()?, self.u16()?),
			0x1C => Insn::_1C(self.u16()?, self.u16()?),
			0x22 => Insn::SoundPlay(self.u16()?, self.u8()?, self.u8()?),
			0x23 => Insn::SoundStop(self.u16()?),
			0x24 => Insn::SoundLoop(self.u16()?, self.u8()?),
			0x28 => Insn::Quest(self.u16()?, match self.u8()? {
				0x01 => QuestInsn::TaskSet(self.u16()?),
				0x02 => QuestInsn::TaskUnset(self.u16()?),
				0x03 => QuestInsn::FlagsSet(self.u8()?),
				0x04 => QuestInsn::FlagsUnset(self.u8()?),
				op => eyre::bail!("Unknown QuestInsn: {:02X}", op)
			}),
			0x29 => Insn::QuestGet(self.u16()?, match self.u8()? {
				0x00 => QuestGetInsn::Task(self.u16()?),
				0x01 => QuestGetInsn::Flags(self.u8()?),
				op => eyre::bail!("Unknown QuestGetInsn: {:02X}", op)
			}),
			0x30 => Insn::_Party30(self.u8()?),
			0x43 => Insn::CharForkFunc(Character(self.u16()?), self.u8()?, FuncRef(self.u8()? as u16, self.u16()?)),
			0x45 => Insn::CharFork(Character(self.u16()?), self.u16()?, {
				let end = self.u8()? as usize + self.pos();
				let mut insns = Vec::new();
				while self.pos() < end {
					self.marks.insert(self.pos(), "\x1B[0;7;2m•".to_owned());
					insns.push(self.insn()?);
				}
				eyre::ensure!(self.pos() == end, "Overshot: {:X} > {:X}", self.pos(), end);
				self.check_u8(0)?;
				insns
			}),
			0x49 => Insn::Event(FuncRef(self.u8()? as u16, self.u16()?)),
			0x4D => Insn::ExprVar(self.u16()?, self.expr()?),
			0x4F => Insn::ExprAttr(self.u8()?, self.expr()?),
			0x51 => Insn::ExprCharAttr(Character(self.u16()?), self.u8()?, self.expr()?),
			0x53 => Insn::TextEnd(Character(self.u16()?)),
			0x54 => Insn::TextMessage(self.text()?),
			0x56 => Insn::TextReset(self.u8()?),
			0x58 => Insn::TextWait,
			0x5A => Insn::TextSetPos(self.i16()?, self.i16()?, self.i16()?, self.i16()?),
			0x5B => Insn::TextTalk(Character(self.u16()?), self.text()?),
			0x5C => Insn::TextTalkNamed(Character(self.u16()?), self.str()?, self.text()?),
			0x5D => Insn::Menu(self.u16()?, (self.i16()?, self.i16()?), self.u8()?, self.str()?.split_terminator('\x01').map(|a| a.to_owned()).collect()),
			0x5E => Insn::MenuWait(self.u16()?),
			0x5F => Insn::_Menu5F(self.u16()?),
			0x60 => Insn::TextSetName(self.str()?),
			0x69 => Insn::CamLookAt(Character(self.u16()?), self.u32()?),
			0x6C => Insn::CamAngle(self.i32()?, self.u32()?),
			0x6D => Insn::CamPos(self.pos3()?, self.u32()?),
			0x87 => Insn::CharSetFrame(Character(self.u16()?), self.u16()?),
			0x88 => Insn::CharSetPos(Character(self.u16()?), self.pos3()?, self.u16()?),
			0x8A => Insn::CharLookAt(Character(self.u16()?), Character(self.u16()?), self.u16()?),
			0x8E => Insn::CharWalkTo(Character(self.u16()?), self.pos3()?, self.u32()?, self.u8()?),
			0x90 => Insn::CharWalk(Character(self.u16()?), self.pos3()?, self.u32()?, self.u8()?),
			0x92 => Insn::_Char92(Character(self.u16()?), Character(self.u16()?), self.u32()?, self.u32()?, self.u8()?),
			0x99 => Insn::CharAnimation(Character(self.u16()?), self.u8()?, self.u8()?, self.u32()?),
			0x9A => Insn::CharFlagsSet(Character(self.u16()?), self.u16()?),
			0x9B => Insn::CharFlagsUnset(Character(self.u16()?), self.u16()?),
			0xA2 => Insn::FlagSet(Flag(self.u16()?)),
			0xA3 => Insn::FlagUnset(Flag(self.u16()?)),
			0xA5 => Insn::AwaitFlagUnset(Flag(self.u16()?)),
			0xA6 => Insn::AwaitFlagSet(Flag(self.u16()?)),
			0xB1 => Insn::OpLoad(self.str()?),
			0xB2 => Insn::_B2(self.u8()?, self.u8()?, self.u16()?),
			0xB4 => Insn::ReturnToTitle(self.u8()?),

			op => eyre::bail!("Unknown Insn: {:02X}", op)
		})
	}

	fn expr(&mut self) -> Result<Box<Expr>> {
		ExprParser::new(self).expr()
	}

	fn text(&mut self) -> Result<Text> {
		self.marks.insert(self.pos(), "\x1B[0;7;2m\"".to_owned());
		let v = util::Text::read(self)?;
		self.marks.insert(self.pos(), "\x1B[0;7;2m\"".to_owned());
		Ok(v)
	}
}

#[derive(derive_more::Deref, derive_more::DerefMut)]
struct ExprParser<'a, 'b> {
	#[allow(clippy::vec_box)]
	stack: Vec<Box<Expr>>,
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

	fn expr(mut self) -> Result<Box<Expr>> {
		self.inner.marks.insert(self.inner.pos(), "\x1B[0;7;2m[".to_owned());
		while let Some(op) = self.op()? {
			self.push(op);
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
			0x1C => Expr::Exec(self.insn()?),
			0x1D => self.unop(ExprUnop::Inv)?,
			0x1E => Expr::Flag(Flag(self.u16()?)),
			0x1F => Expr::Var(self.u16()?),
			0x20 => Expr::Attr(self.u8()?),
			0x21 => Expr::CharAttr(Character(self.u16()?), self.u8()?),
			0x22 => Expr::Rand,
			op => eyre::bail!("Unknown Expr: {:02X}", op)
		}))
	}

	fn push(&mut self, expr: Expr) {
		self.stack.push(Box::new(expr))
	}

	fn binop(&mut self, op: ExprBinop) -> Result<Expr> {
		let r = self.pop()?;
		let l = self.pop()?;
		Ok(Expr::Binop(op, l, r))
	}

	fn unop(&mut self, op: ExprUnop) -> Result<Expr> {
		Ok(Expr::Unop(op, self.pop()?))
	}

	fn pop(&mut self) -> Result<Box<Expr>> {
		Ok(self.stack.pop().ok_or_else(|| eyre::eyre!("Empty expr stack"))?)
	}
}
