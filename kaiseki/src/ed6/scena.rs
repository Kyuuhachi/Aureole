use std::collections::HashMap;
use eyre::Result;
use hamu::read::{self, In, Le};
use crate::util::{self, ByteString, InExt};

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

#[derive(Debug, Clone)]
pub struct Npc {
	pub name: String,
	pub pos: Pos3,
	pub angle: u16,
	pub ch: (u16, u16), // First entry seems to always be zero. Probably include index, just like for functions
	pub cp: (u16, u16),
	pub flags: u16,
	pub func1: FuncRef, // I think one of them is idle and one is speak
	pub func2: FuncRef,
}

#[derive(Debug, Clone)]
pub struct Monster {
	pub name: String,
	pub pos: Pos3,
	pub angle: u16,
	pub _1: u16, // this is likely related to chcp, but not sure
	pub flags: u16,
	pub _2: i32, // Is this maybe a funcref? It's always -1
	pub battle: u16, // T_BATTLE index
	pub flag: u16, // set when defeated
	pub _3: u16,
}

#[derive(Debug, Clone)]
pub struct Trigger {
	pub pos1: Pos3,
	pub pos2: Pos3,
	pub flags: u16,
	pub func: FuncRef,
	pub _1: u16,
}

#[derive(Debug, Clone)]
pub struct Object {
	pub pos: Pos3,
	pub radius: u32,
	pub bubble_pos: Pos3,
	pub flags: u16,
	pub func: FuncRef,
	pub _1: u16,
}

// I'm not 100% sure, but I *think* this one has to do with camera
#[derive(Debug, Clone)]
pub struct CameraAngle {
	pub pos: Pos3,
	pub _1: u32,
	pub _2: u32,
	pub _3: u32,
	pub _4: i32,
	pub _5: u32,
	pub _6: u32,
	pub _7: u32,
	pub _8: u32,
	pub _9: u32,
	pub _10: u32,
	pub _11: u32,
	pub _12: [u16; 8],
}

#[derive(Debug, Clone)]
pub struct Scena {
	pub dir: ByteString<10>,
	pub fname: ByteString<14>,
	pub town: u16, // T_TOWN index
	pub bgm: u16, // T_BGMTBL index
	pub entry_func: FuncRef,
	pub includes: [FileRef; 8],
	pub ch: Vec<FileRef>,
	pub cp: Vec<FileRef>,
	pub npcs: Vec<Npc>,
	pub monsters: Vec<Monster>,
	pub triggers: Vec<Trigger>,
	pub objects: Vec<Object>,
	pub camera_angles: Vec<CameraAngle>,
	pub code: Vec<Vec<(usize, Insn)>>,
}

#[extend::ext(name=InExtForScena)]
impl In<'_> {
	fn file_ref(&mut self) -> read::Result<FileRef> {
		Ok(FileRef(self.u16()?, self.u16()?))
	}

	fn func_ref(&mut self) -> read::Result<FuncRef> {
		Ok(FuncRef(self.u16()?, self.u16()?))
	}

	fn pos3(&mut self) -> read::Result<Pos3> {
		Ok(Pos3(self.i32()?, self.i32()?, self.i32()?))
	}
}

pub fn read(i: &[u8]) -> Result<Scena> {
	let mut i = In::new(i);

	let dir = i.bytestring::<10>()?;
	let fname = i.bytestring::<14>()?;
	let town = i.u16()?;
	let bgm = i.u16()?; // T_BGMTBL index
	let entry_func = i.func_ref()?;
	let includes = [
		i.file_ref()?, i.file_ref()?, i.file_ref()?, i.file_ref()?,
		i.file_ref()?, i.file_ref()?, i.file_ref()?, i.file_ref()?,
	];
	i.check_u16(0)?;

	let head_end = i.clone().u16()? as usize;

	let ch       = (i.ptr_u16()?, i.u16()?);
	let cp       = (i.ptr_u16()?, i.u16()?);
	let npcs     = (i.ptr_u16()?, i.u16()?);
	let monsters = (i.ptr_u16()?, i.u16()?);
	let triggers = (i.ptr_u16()?, i.u16()?);
	let objects  = (i.ptr_u16()?, i.u16()?);

	let mut strings = i.ptr_u16()?;
	let code_start = i.u16()? as usize;
	i.check_u16(0)?;
	let code_end = i.clone().u16()? as usize;
	let func_table = (i.ptr_u16()?, i.u16()? / 2);

	eyre::ensure!(strings.str()? == "@FileName", stringify!(strings.str()? == "@FileName"));

	let ch = chcp_list(ch)?;
	let cp = chcp_list(cp)?;

	let npcs = list(npcs, |i| Ok(Npc {
		name: strings.str()?,
		pos: i.pos3()?,
		angle: i.u16()?,
		ch: (i.u16()?, i.u16()?),
		cp: (i.u16()?, i.u16()?),
		flags: i.u16()?,
		func1: i.func_ref()?,
		func2: i.func_ref()?,
	}))?;

	let monsters = list(monsters, |i| Ok(Monster {
		name: strings.str()?,
		pos: i.pos3()?,
		angle: i.u16()?,
		_1: i.u16()?,
		flags: i.u16()?,
		_2: i.i32()?,
		battle: i.u16()?,
		flag: i.u16()?,
		_3: i.u16()?,
	}))?;

	let triggers = list(triggers, |i| Ok(Trigger {
		pos1: i.pos3()?,
		pos2: i.pos3()?,
		flags: i.u16()?,
		func: i.func_ref()?,
		_1: i.u16()?,
	}))?;

	let objects = list(objects, |i| Ok(Object {
		pos: i.pos3()?,
		radius: i.u32()?,
		bubble_pos: i.pos3()?,
		flags: i.u16()?,
		func: i.func_ref()?,
		_1: i.u16()?,
	}))?;

	let func_table = list(func_table, |i| Ok(i.u16()? as usize))?;
	eyre::ensure!(func_table.is_empty() || func_table[0] == code_start,
		"Unexpected func table: {:X?} does not start with {:X?}", func_table, code_start);

	let mut camera_angles = Vec::new();
	while i.pos() < head_end {
		camera_angles.push(CameraAngle {
			pos: i.pos3()?,
			_1: i.u32()?,
			_2: i.u32()?,
			_3: i.u32()?,
			_4: i.i32()?,
			_5: i.u32()?,
			_6: i.u32()?,
			_7: i.u32()?,
			_8: i.u32()?,
			_9: i.u32()?,
			_10: i.u32()?,
			_11: i.u32()?,
			_12: [i.u16()?, i.u16()?, i.u16()?, i.u16()?, i.u16()?, i.u16()?, i.u16()?, i.u16()?]
		});
	}

	let mut funcparser = FuncParser::new();
	let code = util::multiple(&i, &func_table, code_end, |i, len| {
		Ok(funcparser.read_func(i, i.pos() + len)?)
	})?;

	// i.dump_uncovered(|a| a.to_stderr())?;

	Ok(Scena {
		dir, fname,
		town, bgm,
		entry_func,
		includes,
		ch, cp,
		npcs, monsters,
		triggers, objects,
		camera_angles,
		code,
	})
}

fn chcp_list((mut i, count): (In, u16)) -> Result<Vec<FileRef>> {
	let mut out = Vec::with_capacity(count as usize);
	for _ in 0..count {
		out.push(i.file_ref()?);
	}
	i.check_u8(0xFF)?; // Nope, no idea what this is for
	Ok(out)
}

fn list<A>((mut i, count): (In, u16), mut f: impl FnMut(&mut In) -> Result<A>) -> Result<Vec<A>> {
	let mut out = Vec::with_capacity(count as usize);
	for _ in 0..count {
		out.push(f(&mut i)?);
	}
	Ok(out)
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
	Var(u16),
	Attr(u8),
	CharAttr(Character, u8),
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
	/*16*/ Map(MapInsn),
	/*19*/ EventBegin(u8),
	/*1A*/ EventEnd(u8),
	/*1B*/ _1B(u16, u16),
	/*1C*/ _1C(u16, u16),
	/*43*/ CharForkFunc(Character, u8 /*ForkId*/, FuncRef),
	/*45*/ CharFork(Character, u16 /*ForkId*/, Vec<Insn>), // why is this is u16?
	/*49*/ Event(FuncRef), // Not sure if this is different from Call
	/*53*/ TextEnd(Character),
	/*54*/ TextMessage(util::Text),
	/*56*/ TextReset(u8), // Not sure what this does, is it always 0?
	/*58*/ TextWait,
	/*5A*/ TextSetPos(i16, i16, i16, i16),
	/*5B*/ TextTalk(Character, util::Text),
	/*5C*/ TextTalkNamed(Character, String, util::Text),
	/*60*/ TextSetName(String),
	/*6C*/ CamAngle(i32 /*Angle*/, u32 /*Time*/),
	/*6D*/ CamPos(Pos3, u32 /*Time*/),
	/*88*/ CharSetPos(Character, Pos3, u16),
	/*8A*/ CharLookAt(Character, Character, u16 /*Time*/),
	/*8E*/ CharWalkTo(Character, Pos3, u32 /*Speed*/, u8),
	/*90*/ CharWalk(Character, Pos3, u32 /*Speed*/, u8), // I don't know how this differs from CharWalkTo; is it relative maybe?
	/*92*/ _Char92(Character, Character, u32, u32, u8),
	/*A2*/ FlagSet(Flag), // Is this order really right?
	/*A3*/ FlagUnset(Flag),
	/*A5*/ AwaitFlagUnset(Flag),
	/*A6*/ AwaitFlagSet(Flag),
	/*B1*/ OpLoad(String /*._OP filename*/),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapInsn {
	/*00*/ Hide,
	/*01*/ Show,
	/*02*/ Set(i32, (i32, i32), FileRef /* archive 03 */), // XXX this seems to be (arch, index) while others are (index, arch)?
}

struct FuncParser {
	marks: HashMap<usize, String>,
}
impl FuncParser {
	fn new() -> Self {
		FuncParser {
			marks: HashMap::new(),
		}
	}

	fn read_func(&mut self, i: &mut In, end: usize) -> Result<Vec<(usize, Insn)>> {
		let start = i.clone();
		let mut ops = Vec::new();
		(|| -> Result<_> {
			self.marks.insert(i.pos(), "\x1B[0;7m[".to_owned());
			while i.pos() < end {
				ops.push((i.pos(), self.read_insn(i)?));
				self.marks.insert(i.pos(), "\x1B[0;7m•".to_owned());
			}
			self.marks.insert(i.pos(), "\x1B[0;7m]".to_owned());
			eyre::ensure!(i.pos() == end, "Overshot: {:X} > {:X}", i.pos(), end);
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
					.mark(i.pos()-1, "\x1B[0;7m ")
					.number_width(4)
					.newline(false)
					.to_string()
					.header("Dump:")
			})
		})?;
		Ok(ops)
	}

	fn read_insn(&mut self, i: &mut In) -> Result<Insn> {
		Ok(match i.u8()? {
			0x01 => Insn::Return,
			0x02 => Insn::If(self.read_expr(i)?, i.u16()? as usize),
			0x03 => Insn::Goto(i.u16()? as usize),
			0x04 => Insn::Switch(self.read_expr(i)?, {
				let mut out = Vec::new();
				for _ in 0..i.u16()? {
					out.push((i.u16()?, i.u16()? as usize));
				}
				out
			}, i.u16()? as usize),
			0x08 => Insn::Sleep(i.u32()?),
			0x09 => Insn::FlagsSet(i.u32()?),
			0x0A => Insn::FlagsUnset(i.u32()?),
			0x0B => Insn::FadeOn(i.u32()?, i.u32()?, i.u8()?),
			0x0C => Insn::FadeOff(i.u32()?, i.u32()?),
			0x16 => Insn::Map(match i.u8()? {
				0x00 => MapInsn::Hide,
				0x01 => MapInsn::Show,
				0x02 => MapInsn::Set(i.i32()?, (i.i32()?, i.i32()?), i.file_ref()?),
				op => eyre::bail!("Unknown map op: {:02X}", op)
			}),
			0x19 => Insn::EventBegin(i.u8()?),
			0x1A => Insn::EventEnd(i.u8()?),
			0x1B => Insn::_1B(i.u16()?, i.u16()?),
			0x1C => Insn::_1C(i.u16()?, i.u16()?),
			0x43 => Insn::CharForkFunc(Character(i.u16()?), i.u8()?, FuncRef(i.u8()? as u16, i.u16()?)),
			0x45 => Insn::CharFork(Character(i.u16()?), i.u16()?, {
				let end = i.u8()? as usize + i.pos();
				let mut insns = Vec::new();
				while i.pos() < end {
					self.marks.insert(i.pos(), "\x1B[0;7;2m•".to_owned());
					insns.push(self.read_insn(i)?);
				}
				eyre::ensure!(i.pos() == end, "Overshot: {:X} > {:X}", i.pos(), end);
				i.check_u8(0)?;
				insns
			}),
			0x49 => Insn::Event(FuncRef(i.u8()? as u16, i.u16()?)),
			0x53 => Insn::TextEnd(Character(i.u16()?)),
			0x54 => Insn::TextMessage(self.read_text(i)?),
			0x56 => Insn::TextReset(i.u8()?),
			0x58 => Insn::TextWait,
			0x5A => Insn::TextSetPos(i.i16()?, i.i16()?, i.i16()?, i.i16()?),
			0x5B => Insn::TextTalk(Character(i.u16()?), self.read_text(i)?),
			0x5C => Insn::TextTalkNamed(Character(i.u16()?), i.str()?, self.read_text(i)?),
			0x60 => Insn::TextSetName(i.str()?),
			0x6C => Insn::CamAngle(i.i32()?, i.u32()?),
			0x6D => Insn::CamPos(i.pos3()?, i.u32()?),
			0x88 => Insn::CharSetPos(Character(i.u16()?), i.pos3()?, i.u16()?),
			0x8A => Insn::CharLookAt(Character(i.u16()?), Character(i.u16()?), i.u16()?),
			0x8E => Insn::CharWalkTo(Character(i.u16()?), i.pos3()?, i.u32()?, i.u8()?),
			0x90 => Insn::CharWalk(Character(i.u16()?), i.pos3()?, i.u32()?, i.u8()?),
			0x92 => Insn::_Char92(Character(i.u16()?), Character(i.u16()?), i.u32()?, i.u32()?, i.u8()?),
			0xA2 => Insn::FlagSet(Flag(i.u16()?)),
			0xA3 => Insn::FlagUnset(Flag(i.u16()?)),
			0xA5 => Insn::AwaitFlagUnset(Flag(i.u16()?)),
			0xA6 => Insn::AwaitFlagSet(Flag(i.u16()?)),
			0xB1 => Insn::OpLoad(i.str()?),

			op => eyre::bail!("Unknown op: {:02X}", op)
		})
	}

	fn read_expr(&mut self, i: &mut In) -> Result<Box<Expr>> {
		#[allow(clippy::vec_box)]
		struct Stack(Vec<Box<Expr>>);
		impl Stack {
			fn push(&mut self, expr: Expr) {
				self.0.push(Box::new(expr))
			}

			fn binop(&mut self, op: ExprBinop) -> Result<Expr> {
				Ok(Expr::Binop(op, self.pop()?, self.pop()?))
			}

			fn unop(&mut self, op: ExprUnop) -> Result<Expr> {
				Ok(Expr::Unop(op, self.pop()?))
			}

			fn pop(&mut self) -> Result<Box<Expr>> {
				Ok(self.0.pop().ok_or_else(|| eyre::eyre!("Empty expr stack"))?)
			}
		}
		let mut stack = Stack(Vec::new());
		self.marks.insert(i.pos(), "\x1B[0;7;2m[".to_owned());
		loop {
			let op = match i.u8()? {
				0x00 => Expr::Const(i.u32()?),
				0x01 => break,
				0x02 => stack.binop(ExprBinop::Eq)?,
				0x03 => stack.binop(ExprBinop::Ne)?,
				0x04 => stack.binop(ExprBinop::Lt)?,
				0x05 => stack.binop(ExprBinop::Gt)?,
				0x06 => stack.binop(ExprBinop::Le)?,
				0x07 => stack.binop(ExprBinop::Ge)?,
				0x08 => stack.unop(ExprUnop::Not)?,
				0x09 => stack.binop(ExprBinop::BoolAnd)?,
				0x0A => stack.binop(ExprBinop::And)?,
				0x0B => stack.binop(ExprBinop::Or)?,
				0x0C => stack.binop(ExprBinop::Add)?,
				0x0D => stack.binop(ExprBinop::Sub)?,
				0x0E => stack.unop(ExprUnop::Neg)?,
				0x0F => stack.binop(ExprBinop::Xor)?,
				0x10 => stack.binop(ExprBinop::Mul)?,
				0x11 => stack.binop(ExprBinop::Div)?,
				0x12 => stack.binop(ExprBinop::Mod)?,
				0x13 => stack.unop(ExprUnop::Ass)?,
				0x14 => stack.unop(ExprUnop::MulAss)?,
				0x15 => stack.unop(ExprUnop::DivAss)?,
				0x16 => stack.unop(ExprUnop::ModAss)?,
				0x17 => stack.unop(ExprUnop::AddAss)?,
				0x18 => stack.unop(ExprUnop::SubAss)?,
				0x19 => stack.unop(ExprUnop::AndAss)?,
				0x1A => stack.unop(ExprUnop::XorAss)?,
				0x1B => stack.unop(ExprUnop::OrAss)?,
				0x1C => Expr::Exec(self.read_insn(i)?),
				0x1D => stack.unop(ExprUnop::Inv)?,
				0x1E => Expr::Flag(Flag(i.u16()?)),
				0x1F => Expr::Var(i.u16()?),
				0x20 => Expr::Attr(i.u8()?),
				0x21 => Expr::CharAttr(Character(i.u16()?), i.u8()?),
				0x22 => Expr::Rand,
				op => eyre::bail!("Unknown expr op: {:02X}", op)
			};
			stack.push(op);
			self.marks.insert(i.pos(), "\x1B[0;7;2m•".to_owned());
		}
		self.marks.insert(i.pos(), "\x1B[0;7;2m]".to_owned());
		match stack.0.len() {
			1 => Ok(stack.pop()?),
			_ => eyre::bail!("Invalid expr: {:?}", stack.0)
		}
	}

	fn read_text(&mut self, i: &mut In) -> Result<util::Text> {
		self.marks.insert(i.pos(), "\x1B[0;7;2m\"".to_owned());
		let v = util::read_text(i)?;
		self.marks.insert(i.pos(), "\x1B[0;7;2m\"".to_owned());
		Ok(v)
	}
}
