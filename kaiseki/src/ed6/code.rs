use std::collections::HashMap;
use eyre::Result;
use color_eyre::{Section, SectionExt};
use derive_more::*;
use hamu::read::{In, Le};
use crate::util::{self, Text, InExt, ByteString};

use super::Archives;

#[derive(Debug, Clone)]
pub struct Asm {
	pub code: Vec<(usize, FlowInsn)>,
	pub end: usize,
}

pub type FlowInsn = crate::decompile::FlowInsn<Expr, Insn>;

pub type Stmt = crate::decompile::Stmt<Expr, Insn>;

pub fn decompile(asm: &Asm) -> Result<Vec<Stmt>> {
	crate::decompile::decompile(&asm.code, asm.end)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileRef {
	pub archive: u8,
	pub name: ByteString<12>,
}

impl FileRef {
	pub fn read(i: &mut In, archives: &Archives) -> Result<Self> {
		Ok(Self::read_opt(i, archives)?.ok_or_else(|| eyre::eyre!("invalid empty file ref"))?)
	}

	pub fn read_opt(i: &mut In, archives: &Archives) -> Result<Option<Self>> {
		let file = i.u16()?;
		let arch = i.u16()?;
		if (file, arch) == (0xFFFF, 0xFFFF) {
			Ok(None)
		} else {
			let archive = archives.archive(arch as u8)?;
			let ent = archive.entries().get(file as usize)
				.ok_or_else(|| eyre::eyre!("invalid file ref {arch:02X}/{file}"))?;
			Ok(Some(FileRef { archive: arch as u8, name: ent.name }))
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq, DebugCustom)]
#[debug(fmt = "FuncRef({_0}, {_1})")]
pub struct FuncRef(pub u16, pub u16);

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

pub fn read(i: In, end: usize, archives: &Archives) -> Result<Asm> {
	let code = CodeParser::new(i, archives).func(end)?;
	Ok(Asm { code, end })
}

#[derive(Deref, DerefMut)]
struct CodeParser<'a> {
	marks: HashMap<usize, String>,
	#[deref]
	#[deref_mut]
	inner: In<'a>,
	archives: &'a Archives,
}

impl<'a> CodeParser<'a> {
	#[allow(clippy::new_without_default)]
	fn new(i: In<'a>, archives: &'a Archives) -> Self {
		CodeParser {
			marks: HashMap::new(),
			inner: i,
			archives
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
			self.marks.insert(self.pos(), "\x1B[0;7m]".to_owned());
			eyre::ensure!(self.pos() == end, "Overshot: {:X} > {:X}", self.pos(), end);
			Ok(())
		})().map_err(|e| {
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

	fn flow_insn(&mut self) -> Result<FlowInsn> {
		let pos = self.pos();
		Ok(match self.u8()? {
			0x02 => FlowInsn::If(self.expr()?, self.u16()? as usize),
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
		FileRef::read(&mut self.inner, self.archives)
	}

	fn func_ref(&mut self) -> hamu::read::Result<FuncRef> {
		Ok(FuncRef(self.u8()? as u16, self.u16()?))
	}

	fn pos2(&mut self) -> hamu::read::Result<Pos2> {
		Ok(Pos2(self.i32()?, self.i32()?))
	}

	fn pos3(&mut self) -> hamu::read::Result<Pos3> {
		Ok(Pos3(self.i32()?, self.i32()?, self.i32()?))
	}
}

pub trait InsnVisitor {
	fn u8(&mut self, v: &u8);
	fn u16(&mut self, v: &u16);
	fn u32(&mut self, v: &u32);

	fn i8(&mut self, v: &i8);
	fn i16(&mut self, v: &i16);
	fn i32(&mut self, v: &i32);

	fn func_ref(&mut self, v: &FuncRef);
	fn file_ref(&mut self, v: &FileRef);

	fn pos2(&mut self, v: &Pos2);
	fn pos3(&mut self, v: &Pos3);
	fn relative(&mut self, v: &Pos3);

	fn time(&mut self, v: &u32);
	fn speed(&mut self, v: &u32);
	fn angle(&mut self, v: &u16);
	fn color(&mut self, v: &u32);

	fn time16(&mut self, v: &u16);
	fn angle32(&mut self, v: &i32);

	fn battle(&mut self, v: &u16);
	fn town(&mut self, v: &u16);
	fn bgmtbl(&mut self, v: &u8);
	fn quest(&mut self, v: &u16);
	fn sound(&mut self, v: &u16);
	fn item(&mut self, v: &u16);
	fn flag(&mut self, v: &u16);
	fn shop(&mut self, v: &u8);
	fn magic(&mut self, v: &u16);

	fn fork(&mut self, v: &[Insn]);
	fn expr(&mut self, v: &Expr);
	fn string(&mut self, v: &str);
	fn text(&mut self, v: &Text);
	fn menu(&mut self, v: &[String]);
	fn quests(&mut self, v: &[u16]);
	fn emote(&mut self, v: &(u8, u8, u32, u8));

	fn flags(&mut self, v: &u32);
	fn quest_flag(&mut self, v: &u8);
	fn char_flags(&mut self, v: &u16);
	fn quest_task(&mut self, v: &u16);
	fn member(&mut self, v: &u8);
	fn element(&mut self, v: &u8);

	fn var(&mut self, v: &u16);
	fn attr(&mut self, v: &u8);
	fn char_attr(&mut self, v: &u8);

	fn char(&mut self, v: &u16);
	fn chcp(&mut self, v: &u16);
	fn fork_id(&mut self, v: &u8);
	fn menu_id(&mut self, v: &u16);
	fn object(&mut self, v: &u16);

	fn data(&mut self, v: &[u8]);
}

#[kaiseki_macros::bytecode(
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub enum Insn {}
	pub fn visit(&self, vis: &mut impl InsnVisitor) {}
	pub fn name(&self) -> &'static str {}
)]
fn read(i: &mut CodeParser) -> Result<Self> {
	match u8 {
		0x01 => Return(),
		0x05 => Call(FuncRef),
		0x06 => NewScene(FileRef, u8, u8, u8, u8),
		0x08 => Sleep(time/u32),
		0x09 => FlagsSet(flags/u32),
		0x0A => FlagsUnset(flags/u32),
		0x0B => FadeOn(time/u32, color/u32, u8),
		0x0C => FadeOff(time/u32, color/u32),
		0x0D => _0D(),
		0x0E => Blur(time/u32),
		0x0F => Battle(battle/u16, u16, u16, u16, u8, u16, i8),
		0x12 => _12(i32, i32, u32),
		0x13 => PlaceSetName(town/u16),
		0x16 => Map(match u8 {
			0x00 => Hide(),
			0x01 => Show(),
			0x02 => Set(i32, Pos2, FileRef),
		}),
		0x17 => Save(),
		0x19 => EventBegin(u8),
		0x1A => EventEnd(u8),
		0x1B => _1B(u16, u16),
		0x1C => _1C(u16, u16),
		0x1D => BgmSet(bgmtbl/u8),
		0x1E => _1E(),
		0x1F => BgmSetVolume(u8, time/u32),
		0x20 => _20(time/u32),
		0x21 => _21(), // Always paired with _20
		0x22 => SoundPlay(sound/u16, u8, u8),
		0x23 => SoundStop(sound/u16),
		0x24 => SoundLoop(sound/u16, u8),
		0x26 => _Sound26(sound/u16),
		0x28 => Quest(quest/u16, match u8 {
			0x01 => TaskSet(quest_task/u16),
			0x02 => TaskUnset(quest_task/u16),
			0x03 => FlagsSet(quest_flag/u8),
			0x04 => FlagsUnset(quest_flag/u8),
		}),
		0x29 => Quest(quest/u16, match u8 {
			0x00 => FlagsGet(quest_flag/u8),
			0x01 => TaskGet(quest_task/u16),
		}),
		0x2A => QuestList(quests/{
			let mut quests = Vec::new();
			loop {
				match i.u16()? {
					0xFFFF => break,
					q => quests.push(q)
				}
			}
			quests
		} as Vec<u16>),
		0x2B => QuestBonusBp(quest/u16, u16),
		0x2C => QuestBonusMira(quest/u16, u16),
		0x2D => PartyAdd(member/u8, char/{i.u8()? as u16} as u16),
		0x2E => PartyRemove(member/u8, char/{i.u8()? as u16} as u16),
		0x30 => _Party30(u8),
		0x31 => PartySetAttr(member/u8, u8, u16), // what types is this?
		0x34 => PartyAddArt(member/u8, magic/u16),
		0x35 => PartyAddCraft(member/u8, magic/u16),
		0x36 => PartyAddSCraft(member/u8, magic/u16),
		0x37 => PartySet(member/u8, u8, u8),
		0x38 => SepithAdd(element/u8, u16),
		0x39 => SepithRemove(element/u8, u16),
		0x3A => MiraAdd(u16),
		0x3B => MiraSub(u16),
		0x3C => BpAdd(u16),
		// I have a guess what 3D is, but it doesn't exist in any scripts
		0x3E => ItemAdd(item/u16, u16),
		0x3F => ItemRemove(item/u16, u16),
		0x40 => ItemHas(item/u16), // or is it ItemGetCount?
		0x41 => PartyEquip(member/u8, item/u16, {
			if (600..800).contains(&_1) {
				i.i8()?
			} else {
				-1
			}
		} as i8),
		0x43 => CharForkFunc(char/u16, fork_id/u8, FuncRef),
		0x44 => CharForkQuit(char/u16, fork_id/u8),
		0x45 => CharFork(char/u16, fork_id/u8, u8, fork/{
			let len = i.u8()? as usize;
			let pos = i.pos();
			let mut insns = Vec::new();
			while i.pos() < pos+len {
				i.marks.insert(i.pos(), "\x1B[0;7;2m•".to_owned());
				insns.push(i.insn()?);
			}
			eyre::ensure!(i.pos() == pos+len, "Overshot: {:X} > {:X}", i.pos(), pos+len);
			i.check_u8(0)?;
			insns
		} as Vec<Insn>),
		0x46 => CharForkLoop(char/u16, fork_id/u8, u8, fork/{
			let len = i.u8()? as usize;
			let pos = i.pos();
			let mut insns = Vec::new();
			while i.pos() < pos+len {
				i.marks.insert(i.pos(), "\x1B[0;7;2m•".to_owned());
				insns.push((i.insn())?);
			}
			eyre::ensure!(i.pos() == pos+len, "Overshot: {:X} > {:X}", i.pos(), pos+len);
			eyre::ensure!(i.flow_insn()? == FlowInsn::Insn(Insn::Yield()), "Invalid loop");
			eyre::ensure!(i.flow_insn()? == FlowInsn::Goto(pos), "Invalid loop");
			insns
		} as Vec<Insn>),
		0x47 => CharForkAwait(char/u16, fork_id/u8, u8),
		0x48 => Yield(), // Used in tight loops, probably wait until next frame
		0x49 => Event(FuncRef), // Not sure how this differs from Call
		0x4A => _Char4A(char/u16, u8),
		0x4B => _Char4B(char/u16, u8),
		0x4D => Var(var/u16, Expr),
		0x4F => Attr(attr/u8, Expr),
		0x51 => CharAttr(char/u16, char_attr/u8, Expr),
		0x52 => TextStart(char/u16),
		0x53 => TextEnd(char/u16),
		0x54 => TextMessage(Text),
		0x56 => TextReset(u8),
		0x58 => TextWait(),
		0x59 => _59(),
		0x5A => TextSetPos(i16, i16, i16, i16),
		0x5B => TextTalk(char/u16, Text),
		0x5C => TextTalkNamed(char/u16, String, Text),
		0x5D => Menu(menu_id/u16, i16, i16, u8, menu/{i.string()?.split_terminator('\x01').map(|a| a.to_owned()).collect()} as Vec<String>),
		0x5E => MenuWait(var/u16),
		0x5F => MenuClose(menu_id/u16),
		0x60 => TextSetName(String),
		0x61 => _61(char/u16),
		0x62 => Emote(char/u16, i32, time/u32, emote/{(i.u8()?, i.u8()?, i.u32()?, i.u8()?)} as (u8, u8, u32, u8)),
		0x63 => EmoteStop(char/u16),
		0x64 => _64(u8, u16),
		0x65 => _65(u8, u16),
		0x66 => _Cam66(u16),
		0x6E => _Cam6E(data/{i.array()?} as [u8; 4], time/u32),
		0x67 => CamOffset(i32, i32, i32, time/u32),
		0x69 => CamLookAt(char/u16, time/u32),
		0x6A => _Char6A(char/u16),
		0x6B => CamDistance(i32, time/u32),
		0x6C => CamAngle(angle32/i32, time/u32),
		0x6D => CamPos(Pos3, time/u32),
		0x6F => _Obj6F(object/u16, u32),
		0x70 => _Obj70(object/u16, u32),
		0x71 => _Obj71(object/u16, u16),
		0x72 => _Obj72(object/u16, u16),
		0x73 => _Obj73(object/u16),
		0x77 => _77(color/u32, time/u32),
		0x7C => Shake(u32, u32, u32, time/u32),
		0x7F => EffLoad(u8, String),
		0x80 => EffPlay(u8, u8, i16, Pos3, u16, u16, u16, u32, u32, u32, u16, u32, u32, u32, u32),
		0x81 => EffPlay2(u16, u8, String, Pos3, u16, u16, u16, u32, u32, u32, u32),
		0x82 => _82(u16),
		0x83 => Achievement(u8, u8),
		0x86 => CharSetChcp(char/u16, chcp/u16),
		0x87 => CharSetFrame(char/u16, u16),
		0x88 => CharSetPos(char/u16, Pos3, angle/u16),
		0x89 => _Char89(char/u16, Pos3, u16),
		0x8A => CharLookAt(char/u16, char/u16, time16/u16),
		0x8C => CharSetAngle(char/u16, angle/u16, time16/u16),
		0x8D => CharIdle(char/u16, Pos2, Pos2, speed/u32),
		0x8E => CharWalkTo(char/u16, Pos3, speed/u32, u8),
		0x8F => CharWalkTo2(char/u16, Pos3, speed/u32, u8), // how are these two different?
		0x90 => DontGoThere(u16, relative/Pos3, u32, u8),
		0x91 => _Char91(char/u16, relative/Pos3, i32, u8),
		0x92 => _Char92(char/u16, char/u16, u32, time/u32, u8),
		0x94 => _94(u8, char/u16, u16, u32, u32, u8), // used with chickens
		0x95 => CharJump(char/u16, relative/Pos3, time/u32, u32),
		0x97 => _Char97(char/u16, Pos2, angle32/i32, time/u32, u16), // used with pigeons
		0x99 => CharAnimation(char/u16, u8, u8, time/u32),
		0x9A => CharFlagsSet(char/u16, char_flags/u16),
		0x9B => CharFlagsUnset(char/u16, char_flags/u16),
		0x9C => _Char9C(char/u16, u16),
		0x9D => _Char9D(char/u16, u16),
		0x9E => CharShake(char/u16, u32, u32, u32, time/u32),
		0x9F => CharColor(char/u16, color/u32, time/u32),
		0xA1 => _CharA1(char/u16, u16),
		0xA2 => FlagSet(flag/u16),
		0xA3 => FlagUnset(flag/u16),
		0xA5 => FlagAwaitUnset(flag/u16),
		0xA6 => FlagAwaitSet(flag/u16),
		0xA9 => ShopOpen(shop/u8),
		0xAC => RecipeLearn(u16), // TODO check type
		0xAD => ImageShow(FileRef, u16, u16, time/u32),
		0xAE => ImageHide(time/u32),
		0xAF => QuestSubmit(shop/u8, quest/u16),
		0xB1 => OpLoad(String),
		0xB2 => _B2(u8, u8, u16),
		0xB3 => Show(match u8 {
			0x00 => Video0(String),
			0x01 => Video1(u8),
		}),
		0xB4 => ReturnToTitle(u8),
		0xB5 => PartySlot(member/u8, u8, u8), // FC only
		0xB9 => ReadBook(item/u16, u16), // FC only
		0xBA => PartyHasSpell(member/u8, magic/u16),
		0xBB => PartyHasSlot(member/u8, u8), // FC only
		0xDE => SaveClearData(), // FC only
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
			0x1E => Expr::Flag(self.u16()?),
			0x1F => Expr::Var(self.u16()?),
			0x20 => Expr::Attr(self.u8()?),
			0x21 => Expr::CharAttr(self.u16()?, self.u8()?),
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
