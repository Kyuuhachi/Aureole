use std::collections::HashMap;

use derive_more::*;

use hamu::read::{In, Le};
use crate::util::{self, ByteString, InExt, Text};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("read error")]
	Read(#[from] hamu::read::Error),
	#[error("decode error")]
	Decode(#[from] util::DecodeError),
	#[error("text error")]
	Text(#[from] util::TextError),
	#[error("multi error")]
	Multi(#[from] util::MultiError<Error>),
	#[error("Overshot: {pos:X} > {end:X}")]
	Overshot { pos: usize, end: usize },
	#[error("Unknown {ty}: {op:02X}")]
	Unknown { ty: &'static str, op: u8 },
	#[error("{0}")]
	Misc(String),
}
impl From<util::StringError> for Error {
	fn from(e: util::StringError) -> Self {
		match e {
			util::StringError::Read(e) => e.into(),
			util::StringError::Decode(e) => e.into(),
		}
	}
}
pub type Result<T, E=Error> = std::result::Result<T, E>;

mod code;
pub use code::{Insn, InsnArg};

#[derive(Clone, Copy, PartialEq, Eq, DebugCustom)]
#[debug(fmt = "FuncRef({_0}, {_1})")]
pub struct FuncRef(pub u16, pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileRef(pub u16, pub u16);

#[derive(Clone, Copy, PartialEq, Eq, DebugCustom)]
#[debug(fmt = "Pos2({_0}, {_1})")]
pub struct Pos2(pub i32, pub i32);

#[derive(Clone, Copy, PartialEq, Eq, DebugCustom)]
#[debug(fmt = "Pos3({_0}, {_1}, {_2})")]
pub struct Pos3(pub i32, pub i32, pub i32);

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
	Flag(u16 /*Flag*/),
	Var(u16 /*Var*/),
	Attr(u8 /*Attr*/),
	CharAttr(u16 /*Char*/, u8 /*CharAttr*/),
	Rand,
}

#[derive(Debug, Clone)]
pub struct Asm {
	pub code: Vec<(usize, FlowInsn)>,
	pub end: usize,
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
	pub battle: u16 /*battle*/,
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
	pub _1: u16,
	pub _1b: u16,
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
	pub _12: [u16; 6],
}

#[derive(Debug, Clone)]
pub struct Scena {
	pub dir: ByteString<10>,
	pub fname: ByteString<14>,
	pub town: u16 /*town*/,
	pub bgm: u16 /*bgmtbl*/,
	pub entry_func: FuncRef,
	pub includes: [Option<FileRef>; 8],
	pub ch: Vec<FileRef>,
	pub cp: Vec<FileRef>,
	pub npcs: Vec<Npc>,
	pub monsters: Vec<Monster>,
	pub triggers: Vec<Trigger>,
	pub objects: Vec<Object>,
	pub camera_angles: Vec<CameraAngle>,
	pub functions: Vec<Asm>,
}

#[extend::ext(name=InExtForScena)]
impl In<'_> {
	fn func_ref(&mut self) -> hamu::read::Result<FuncRef> {
		Ok(FuncRef(self.u16()?, self.u16()?))
	}

	fn pos2(&mut self) -> hamu::read::Result<Pos2> {
		Ok(Pos2(self.i32()?, self.i32()?))
	}

	fn pos3(&mut self) -> hamu::read::Result<Pos3> {
		Ok(Pos3(self.i32()?, self.i32()?, self.i32()?))
	}

	fn check_pos(&self, end: usize) -> Result<()> {
		assert!(self.pos() >= end);
		if self.pos() == end {
			Ok(())
		} else {
			Err(Error::Overshot { pos: self.pos(), end })
		}
	}
}

#[tracing::instrument(skip(i))]
pub fn read(i: &[u8]) -> Result<Scena> {
	let mut i = In::new(i);

	let dir = i.bytestring::<10>()?;
	let fname = i.bytestring::<14>()?;
	let town = i.u16()?;
	let bgm = i.u16()?;
	let entry_func = i.func_ref()?;
	let includes = {
		let mut r = || FileRef::read_opt(&mut i);
		[ r()?, r()?, r()?, r()?, r()?, r()?, r()?, r()? ]
	};
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

	check_eq(&*strings.string()?, "@FileName")?;

	let ch = chcp_list(ch)?;
	let cp = chcp_list(cp)?;

	let npcs = list(npcs, |i| Ok(Npc {
		name: strings.string()?,
		pos: i.pos3()?,
		angle: i.u16()?,
		ch: (i.u16()?, i.u16()?),
		cp: (i.u16()?, i.u16()?),
		flags: i.u16()?,
		func1: i.func_ref()?,
		func2: i.func_ref()?,
	}))?;

	let monsters = list(monsters, |i| Ok(Monster {
		name: strings.string()?,
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
	if !func_table.is_empty() && func_table[0] != code_start {
		return Err(Error::Misc(format!("Unexpected func table: {func_table:X?} does not start with {code_start:X?}")));
	}

	let mut camera_angles = Vec::new();
	while i.pos() < head_end {
		camera_angles.push(CameraAngle {
			pos: i.pos3()?,
			_1: i.u16()?,
			_1b: i.u16()?,
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
			_12: [i.u16()?, i.u16()?, i.u16()?, i.u16()?, i.u16()?, i.u16()?]
		});
	}
	i.check_pos(head_end)?;

	let mut n = 0u32;
	let functions = util::multiple::<_, Error, _>(&i, &func_table, code_end, |i, len| {
		let _span = tracing::info_span!("function", n, start = i.pos(), end = i.pos()+len).entered();
		n += 1;
		let end = i.pos() + len;
		let code = CodeParser::new(i.clone()).func(end)?;
		Ok(Asm { code, end })
	})?;

	i.dump_uncovered(|a| a.to_stderr())?;

	Ok(Scena {
		dir, fname,
		town, bgm,
		entry_func,
		includes,
		ch, cp,
		npcs, monsters,
		triggers, objects,
		camera_angles,
		functions,
	})
}

fn check_eq<T: Eq + std::fmt::Debug>(got: T, exp: T) -> Result<()> {
	if exp == got {
		Ok(())
	} else {
		Err(Error::Misc(format!("Expected {:?}, got {:?}", exp, got)))
	}
}

fn chcp_list((mut i, count): (In, u16)) -> Result<Vec<FileRef>> {
	let mut out = Vec::with_capacity(count as usize);
	for _ in 0..count {
		out.push(FileRef::read(&mut i)?);
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

pub type FlowInsn = crate::decompile::FlowInsn<Expr, Insn>;
pub type Stmt = crate::decompile::Stmt<Expr, Insn>;

pub fn decompile(asm: &Asm) -> Result<Vec<Stmt>, crate::decompile::Error> {
	crate::decompile::decompile(&asm.code, asm.end)
}

impl FileRef {
	pub fn read(i: &mut In) -> Result<Self> {
		Ok(Self::read_opt(i)?.ok_or_else(|| Error::Misc("invalid empty file ref".to_owned()))?)
	}

	pub fn read_opt(i: &mut In) -> Result<Option<Self>> {
		let file = i.u16()?;
		let arch = i.u16()?;
		if (file, arch) == (0xFFFF, 0xFFFF) {
			Ok(None)
		} else {
			Ok(Some(FileRef(arch, file)))
		}
	}
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

	fn func(&mut self, end: usize) -> Result<Vec<(usize, FlowInsn)>> {
		let start = self.inner.clone();
		let mut ops = Vec::new();
		(|| -> Result<_> {
			self.marks.insert(self.pos(), "\x1B[0;7m[".to_owned());
			while self.pos() < end {
				ops.push((self.pos(), self.flow_insn()?));
				self.marks.insert(self.pos(), "\x1B[0;7m•".to_owned());
			}
			self.check_pos(end)?;
			self.marks.insert(self.pos(), "\x1B[0;7m]".to_owned());
			Ok(())
		})().map_err(|e| {
			use std::fmt::Write as _;
			let mut mess = e.to_string();
			mess.push('\n');
			mess.push('\n');
			for (addr, op) in &ops {
				writeln!(mess, "  {addr:04X} {addr}: {op:?}").unwrap();
			}
			mess.push('\n');
			write!(mess, "{}",
				start.dump().end(end)
					.marks(self.marks.iter().map(|(a,b)|(*a,b)))
					.mark(self.pos()-1, "\x1B[0;7m ")
					.num_width(4)
					.newline(false)
					.to_string(true)
			).unwrap();
			tracing::error!("Parse error: {}", mess);

			e
		})?;
		Ok(ops)
	}

	fn flow_insn(&mut self) -> Result<FlowInsn> {
		let pos = self.pos();
		Ok(match self.u8()? {
			0x02 => FlowInsn::Unless(self.expr()?, self.u16()? as usize),
			0x03 => FlowInsn::Goto(self.u16()? as usize),
			0x04 => FlowInsn::Switch(self.expr()?, {
				let mut out = Vec::new();
				for _ in 0..self.u16()? {
					out.push((self.u16()?, self.u16()? as usize));
				}
				out
			}, self.u16()? as usize),
			_ => {
				self.seek(pos).unwrap();
				FlowInsn::Insn(self.insn()?)
			}
		})
	}

	fn insn(&mut self) -> Result<Insn> {
		Insn::read(self)
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

	fn file_ref(&mut self) -> Result<FileRef> {
		FileRef::read(&mut self.inner)
	}

	fn func_ref(&mut self) -> Result<FuncRef> {
		Ok(FuncRef(self.u8()? as u16, self.u16()?))
	}

	fn unknown(&self, ty: &'static str, op: u8) -> Error {
		Error::Unknown { ty, op }
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
			_ => return Err(Error::Misc(format!("invalid Expr: {:?}", self.stack))),
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
			0x1E => Expr::Flag(self.u16()?),
			0x1F => Expr::Var(self.u16()?),
			0x20 => Expr::Attr(self.u8()?),
			0x21 => Expr::CharAttr(self.u16()?, self.u8()?),
			0x22 => Expr::Rand,
			op => return Err(self.unknown("Expr", op))
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
		Ok(self.stack.pop().ok_or_else(|| Error::Misc("empty Expr stack".to_owned()))?)
	}
}
