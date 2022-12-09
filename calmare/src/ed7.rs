use std::io::{self, Write};

use themelios::scena::{Pos2, Pos3, FuncRef};
use themelios::scena::ed7::Scena;
use themelios::scena::code::{InsnArg as I, Expr, ExprBinop, ExprUnop, FlatInsn, Label, Insn};
use themelios::scena::code::decompile::{decompile, TreeInsn};
use themelios::text::{Text, TextSegment};
use strict_result::ResultAsStrict;

type Result<T, E = io::Error> = std::result::Result<T, E>;

#[derive(Clone, Copy, Debug)]
enum Space {
	None,
	Space,
	Newline,
}

pub struct Context<'a> {
	pub blind: bool,
	pub decompile: bool,
	pub indent: usize,
	space: Space,
	out: Box<dyn Write + 'a>,
}

impl<'a> Context<'a> {
	pub fn new(out: impl Write + 'a) -> Self {
		Self {
			blind: false,
			decompile: true,
			indent: 0,
			space: Space::None,
			out: Box::new(out),
		}
	}

	pub fn blind(mut self) -> Self {
		self.blind = true;
		self
	}

	pub fn flat(mut self) -> Self {
		self.decompile = false;
		self
	}
}


impl<'a> Context<'a> {
	fn put_space(&mut self) -> Result<()> {
		match self.space {
			Space::None => {}
			Space::Space => {
				write!(&mut self.out, " ")?;
			}
			Space::Newline => {
				for _ in 0..self.indent {
					write!(&mut self.out, "\t")?;
				}
			}
		}
		self.space = Space::None;
		Ok(())
	}

	pub fn space(&mut self) -> Result<&mut Self> {
		// Cannot fail, but let's Result it for consistency.
		self.space = Space::Space;
		Ok(self)
	}

	pub fn kw(&mut self, arg: &str) -> Result<&mut Self> {
		self.put_space()?;
		write!(&mut self.out, "{arg}")?;
		self.space()?;
		Ok(self)
	}

	pub fn pre(&mut self, arg: &str) -> Result<&mut Self> {
		self.put_space()?;
		write!(&mut self.out, "{arg}")?;
		Ok(self)
	}

	pub fn suf(&mut self, arg: &str) -> Result<&mut Self> {
		write!(&mut self.out, "{arg}")?;
		self.space()?;
		Ok(self)
	}

	pub fn line(&mut self) -> Result<&mut Self> {
		writeln!(&mut self.out)?;
		self.space = Space::Newline;
		Ok(self)
	}

	pub fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> Result<()> {
		self.put_space()?;
		self.out.write_fmt(args)
	}

	pub fn indent<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
		self.indent += 1;
		let v = f(self);
		self.indent -= 1;
		v
	}

	fn val(&mut self, arg: I) -> Result<&mut Self> {
		val(self, arg)?;
		self.space()?;
		Ok(self)
	}
}

pub fn dump(mut f: Context, scena: &Scena) -> Result<()> {
	let Scena {
		name1,
		name2,
		filename,
		town,
		bgm,
		flags,
		unk1,
		unk2,
		unk3,

		includes,

		entry,
		chcp,
		labels,
		npcs,
		monsters,
		triggers,
		look_points,
		animations,

		field_sepith,
		at_rolls,
		placements,
		battles,

		functions,
	} = scena;

	f.kw("ed7")?
		.line()?;
	f.line()?;

	f.kw("name")?
		.val(I::String(name1))?
		.val(I::String(name2))?
		.val(I::String(filename))?
		.line()?;
	f.kw("town")?
		.val(I::TownId(town))?
		.line()?;
	f.kw("bgm")?
		.val(I::BgmId(bgm))?
		.line()?;
	f.kw("flags")?
		.val(I::u32(flags))?
		.line()?;
	f.kw("unknown")?
		.val(I::u8(unk1))?
		.val(I::u16(unk2))?
		.val(I::u8(unk3))?
		.line()?;
	f.line()?;

	// TODO more stuff

	for (i, func) in functions.iter().enumerate() {
		if i != 0 {
			f.line()?;
		}

		let result = if f.decompile {
			decompile(func).map_err(Some)
		} else {
			Err(None)
		};
		match result {
			Ok(result) => {
				f.kw("fn")?
					.val(I::FuncRef(&FuncRef(0, i as u16)))?
					.suf(":")?
					.line()?;
				f.indent(|f| tree_func(f, &result))?;
			}
			Err(err) => {
				f.kw("fn")?
					.val(I::FuncRef(&FuncRef(0, i as u16)))?
					.kw("flat")?
					.suf(":")?;
				if let Some(err) = err {
					write!(f, " // {err}")?;
				}
				f.line()?;
				f.indent(|f| flat_func(f, func))?;
			}
		}
	}

	Ok(())
}

pub fn flat_func(f: &mut Context, func: &[FlatInsn]) -> Result<()> {
	#[extend::ext]
	impl Context<'_> {
		fn label(&mut self, l: &Label) -> Result<&mut Self> {
			self.kw(&format!("L{}", l.0))
		}
	}

	for i in func {
		match i {
			FlatInsn::Unless(e, l) => {
				f.kw("Unless")?.val(I::Expr(e))?.label(l)?.line()?;
			},
			FlatInsn::Goto(l) => {
				f.kw("Goto")?.label(l)?.line()?;
			},
			FlatInsn::Switch(e, cs, l) => {
				f.kw("Switch")?.val(I::Expr(e))?.suf("{")?;
				for (v, l) in cs {
					f.val(I::u16(v))?.suf(":")?.label(l)?.suf(",")?;
				}
				f.kw("default")?.suf(":")?.label(l)?;
				f.pre("}")?.line()?;
			},
			FlatInsn::Insn(i) => {
				insn(f, i)?;
				f.line()?;
			},
			FlatInsn::Label(l) => {
				f.pre("@")?.label(l)?.line()?;
			},
		}
	}
	Ok(())
}

pub fn tree_func(f: &mut Context, func: &[TreeInsn]) -> Result<()> {
	for i in func {
		match i {
			TreeInsn::If(cs) => {
				let mut first = true;
				for (e, body) in cs {
					match (first, e) {
						(true, Some(e)) => {
							f.kw("if")?.val(I::Expr(e))?;
						},
						(false, Some(e)) => {
							f.kw("elif")?.val(I::Expr(e))?;
						},
						(false, None) => {
							f.kw("else")?;
						},
						(true, None) => panic!(),
					}
					first = false;
					f.suf(":")?.line()?;
					f.indent(|f| tree_func(f, body))?;
				}
			},
			TreeInsn::Switch(e, cs) => {
				f.kw("switch")?.val(I::Expr(e))?.suf(":")?.line()?;
				f.indent(|f| {
					for (v, body) in cs {
						match v {
							Some(v) => f.val(I::u16(v))?,
							None => f.kw("default")?
						};
						f.kw("=>")?.line()?;
						f.indent(|f| tree_func(f, body))?;
					}
					Ok(())
				}).strict()?;
			},
			TreeInsn::While(e, body) => {
				f.kw("while")?.val(I::Expr(e))?.suf(":")?.line()?;
				f.indent(|f| tree_func(f, body))?;
			},
			TreeInsn::Break => {
				f.kw("break")?.line()?;
			},
			TreeInsn::Continue => {
				f.kw("continue")?.line()?;
			},
			TreeInsn::Insn(i) => {
				insn(f, i)?;
				f.line()?;
			},
		}
	}
	Ok(())
}

fn insn(f: &mut Context, i: &Insn) -> Result<()> {
	f.kw(i.name())?;
	for &a in i.args().iter() {
		match a {
			I::Fork(a) => {
				f.suf(":")?.line()?;
				f.indent(|f| {
					for line in a {
						insn(f, line)?;
					}
					Ok(())
				}).strict()?;
			}

			I::Menu(a) => {
				f.suf(":")?.line()?;
				f.indent(|f| {
					for line in a {
						f.val(I::MenuItem(line))?.line()?;
					}
					Ok(())
				}).strict()?;
			}

			I::QuestList(a) => {
				f.suf(":")?.line()?;
				f.indent(|f| {
					for line in a {
						f.val(I::QuestId(line))?.line()?;
					}
					Ok(())
				}).strict()?;
			}

			a => {
				f.val(a)?;
			}
		}
	}
	Ok(())
}

fn val(f: &mut Context, a: I) -> Result<()> {
	match a {
		| I::Fork(_)
		| I::Menu(_)
		| I::QuestList(_)
		=> panic!("block elements should be handled outside"),

		// I::i8(v)  => write!(f, "{v}")),
		I::i16(v) => write!(f, "{v}")?,
		I::i32(v) => write!(f, "{v}")?,
		I::u8(v)  => write!(f, "{v}")?,
		I::u16(v) => write!(f, "{v}")?,
		I::u32(v) => write!(f, "{v}")?,
		I::String(v) => write!(f, "{v:?}")?,

		I::Flag(v) => write!(f, "flag[{}]", v.0)?,
		I::Attr(v) => write!(f, "system[{}]", v.0)?,
		I::Var(v) => write!(f, "var[{}]", v.0)?,
		I::Global(v) => write!(f, "global[{}]", v.0)?,
		I::CharAttr(v) => {
			f.val(I::CharId(&v.0))?;
			write!(f, ":{}", v.1)?;
		},

		I::SystemFlags(v) => write!(f, "0x{:08X}", v.0)?,
		I::CharFlags(v)   => write!(f, "0x{:04X}", v.0)?,
		I::QuestFlags(v)  => write!(f, "0x{:02X}", v.0)?,
		I::ObjectFlags(v) => write!(f, "0x{:04X}", v.0)?,
		I::Color(v)       => write!(f, "#{:08X}", v.0)?,

		I::Member(v)   => write!(f, "{v:?}")?,
		I::CharId(v)   => write!(f, "{v:?}")?,
		I::BattleId(v) => write!(f, "{v:?}")?,
		I::BgmId(v)    => write!(f, "{v:?}")?,
		I::ItemId(v)   => write!(f, "{v:?}")?,
		I::MagicId(v)  => write!(f, "{v:?}")?,
		I::QuestId(v)  => write!(f, "{v:?}")?,
		I::ShopId(v)   => write!(f, "{v:?}")?,
		I::SoundId(v)  => write!(f, "{v:?}")?,
		I::TownId(v)   => write!(f, "{v:?}")?,

		I::EntranceId(v) => write!(f, "EntranceId({v})")?,
		I::ForkId(v)   => write!(f, "ForkId({v})")?,
		I::MenuId(v)   => write!(f, "MenuId({v})")?,
		I::SelectId(v) => write!(f, "SelectId({v})")?,
		I::ObjectId(v) => write!(f, "ObjectId({v})")?,
		I::VisId(v)    => write!(f, "VisId({v})")?,
		I::EffId(v)    => write!(f, "EffId({v})")?,

		I::Expr(v) => expr(f, v)?,
		I::FuncRef(v) => {
			if v.0 != 0 {
				write!(f, "{}", v.0)?;
			}
			write!(f, ":{}", v.1)?;
		},

		I::TextTitle(_) if f.blind => write!(f, "\"…\"")?,
		I::TextTitle(v) => write!(f, "{v:?}")?,
		I::MenuItem(_)  if f.blind => write!(f, "\"…\"")?,
		I::MenuItem(v) => write!(f, "{v:?}")?,
		I::Text(v) if f.blind => text_blind(f, v)?,
		I::Text(v) => text(f, v)?,

		I::Angle(v)   => write!(f, "{v}°")?,
		I::Angle32(v) => write!(f, "{v}°₃₂")?,
		I::Speed(v)   => write!(f, "{v}mm/s")?,
		I::Time(v)    => write!(f, "{v}ms")?,
		I::Time16(v)  => write!(f, "{v}ms₁₆")?,

		I::Pos2(Pos2(x,z))   => write!(f, "({x}, -, {z})")?,
		I::Pos3(Pos3(x,y,z)) => write!(f, "({x}, {y}, {z})")?,

		I::Emote(v) => write!(f, "{v:?}")?,
		I::MemberAttr(v) => write!(f, "{v:?}")?,
		I::QuestTask(v) => write!(f, "{v:?}")?,
		I::Animation(v) => write!(f, "{v:?}")?,

		I::MandatoryMembers(v) => write!(f, "{v:?}")?,
		I::OptionalMembers(v)  => write!(f, "{v:?}")?,
		I::TcMembers(v)        => write!(f, "{v:016b}")?,
		I::NpcBattleCombatants(v) => write!(f, "{v:?}")?,

		I::AviFileRef(v)   => write!(f, "{v:?}")?,
		I::EffFileRef(v)   => write!(f, "{v:?}")?,
		I::MapFileRef(v)   => write!(f, "{v:?}")?,
		I::OpFileRef(v)    => write!(f, "{v:?}")?,
		I::ScenaFileRef(v) => write!(f, "{v:?}")?,
		I::VisFileRef(v)   => write!(f, "{v:?}")?,
	}
	Ok(())
}

fn expr(f: &mut Context, e: &Expr) -> Result<()> {
	fn expr_prio(f: &mut Context, e: &Expr, prio: u8) -> Result<()> {
		match e {
			Expr::Const(v)    => { f.val(I::u32(v))?; }
			Expr::Flag(v)     => { f.val(I::Flag(v))?; }
			Expr::Var(v)      => { f.val(I::Var(v))?; }
			Expr::Attr(v)     => { f.val(I::Attr(v))?; }
			Expr::CharAttr(v) => { f.val(I::CharAttr(v))?; }
			Expr::Rand        => { f.kw("Rand")?; }
			Expr::Global(v)   => { f.val(I::Global(v))?; }

			Expr::Binop(op, a, b) => {
				let (text, prio2) = binop(*op);
				if prio2 < prio {
					f.pre("(")?;
				}
				expr_prio(f, a, prio2)?;
				f.kw(text)?;
				expr_prio(f, b, prio2+1)?;
				if prio2 < prio {
					f.suf(")")?;
				}
			}

			Expr::Unop(op, e) => {
				let (text, is_assign) = unop(*op);
				if is_assign {
					f.kw(text)?;
					expr_prio(f, e, 0)?;
				} else {
					f.pre(text)?;
					expr_prio(f, e, 100)?;
				}
			}

			Expr::Insn(i) => insn(f, i)?,
		}
		Ok(())
	}

	fn binop(op: ExprBinop) -> (&'static str, u8) {
		match op {
			ExprBinop::Eq      => ("==", 4),
			ExprBinop::Ne      => ("!=", 4),
			ExprBinop::Lt      => ("<",  4),
			ExprBinop::Gt      => (">",  4),
			ExprBinop::Le      => ("<=", 4),
			ExprBinop::Ge      => (">=", 4),
			ExprBinop::BoolAnd => ("&&", 3),
			ExprBinop::And     => ("&", 3),
			ExprBinop::Or      => ("|", 1),
			ExprBinop::Add     => ("+", 5),
			ExprBinop::Sub     => ("-", 5),
			ExprBinop::Xor     => ("^", 2),
			ExprBinop::Mul     => ("*", 6),
			ExprBinop::Div     => ("/", 6),
			ExprBinop::Mod     => ("%", 6),
		}
	}

	fn unop(op: ExprUnop) -> (&'static str, bool) {
		match op {
			ExprUnop::Not    => ("!", false),
			ExprUnop::Neg    => ("-", false),
			ExprUnop::Inv    => ("~", false),
			ExprUnop::Ass    => ("=",  true),
			ExprUnop::MulAss => ("*=", true),
			ExprUnop::DivAss => ("/=", true),
			ExprUnop::ModAss => ("%=", true),
			ExprUnop::AddAss => ("+=", true),
			ExprUnop::SubAss => ("-=", true),
			ExprUnop::AndAss => ("&=", true),
			ExprUnop::XorAss => ("^=", true),
			ExprUnop::OrAss  => ("|=", true),
		}
	}

	expr_prio(f, e, 0)
}

fn text(f: &mut Context, v: &Text) -> Result<()> {
	let mut it = v.iter();
	loop {
		f.kw("{")?.line()?;
		let cont = f.indent(|f| {
			Ok(loop {
				let Some(next) = it.next() else { break false };
				match next {
					TextSegment::String(s) => {
						let s = s
							.replace('\\', "\\\\")
							.replace('{', "\\{")
							.replace('}', "\\}");
						write!(f, "{s}")?
					}
					TextSegment::Line => {
						f.line()?;
					}
					TextSegment::Wait => {
						write!(f, "{{wait}}")?
					}
					TextSegment::Page => {
						break true
					}
					TextSegment::Color(n) => {
						write!(f, "{{color {n}}}")?;
					}
					TextSegment::Line2 => {
						write!(f, "\\")?;
						f.line()?;
					}
					TextSegment::Item(n) => {
						write!(f, "{{item ")?;
						f.val(I::ItemId(n))?;
						write!(f, "}}")?;
					}
					TextSegment::Byte(n) => {
						write!(f, "{{0x{n:02X}}}")?
					}
				}
			})
		}).strict()?;
		f.line()?.kw("}")?;
		if !cont {
			break
		}
	}
	Ok(())
}

fn text_blind(f: &mut Context, v: &Text) -> Result<()> {
	let mut it = v.iter();
	loop {
		f.kw("{…}")?;
		let cont = loop {
			match it.next() {
				None => break false,
				Some(TextSegment::Page) => break true,
				_ => {}
			}
		};
		if !cont {
			break
		}
	}
	Ok(())
}

#[test]
fn test() {
	use themelios::gamedata::GameData;
	let path = "../data/zero/data/scena/t113b.bin";
	let data = std::fs::read(path).unwrap();
	let scena = themelios::scena::ed7::read(GameData::ZERO_KAI, &data).unwrap();
	let c = Context::new(std::io::stdout());
	dump(c, &scena).unwrap();
}
