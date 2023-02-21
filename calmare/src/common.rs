use themelios::scena::*;
use themelios::scena::code::{InstructionSet, Expr, ExprBinop, ExprUnop, FlatInsn, Label, Insn};
use themelios::scena::code::decompile::{decompile, TreeInsn};
use themelios::text::{Text, TextSegment};
use strict_result::Strict;
use themelios::types::*;
use crate::writer::Context;

pub type Result<T, E = std::io::Error> = std::result::Result<T, E>;

#[extend::ext(name = ContextExt)]
pub(crate) impl Context<'_> {
	fn val<I: Val>(&mut self, arg: &I) -> Result<&mut Self> {
		arg.write(self)?;
		self.space()?;
		Ok(self)
	}

	fn expr(&mut self, arg: &Expr) -> Result<&mut Self> {
		expr(self, arg)?;
		self.space()?;
		Ok(self)
	}
}

pub fn func(f: &mut Context, n: FuncRef, func: &[FlatInsn]) -> Result<()> {
	let result = if f.decompile {
		decompile(func).map_err(Some)
	} else {
		Err(None)
	};
	match result {
		Ok(result) => {
			f.kw("fn")?
				.val(&n)?
				.suf(":")?
				.line()?;
			f.indent(|f| tree_func(f, &result))?;
		}
		Err(err) => {
			f.kw("fn")?
				.val(&n)?
				.kw("flat")?
				.suf(":")?;
			if let Some(err) = err {
				write!(f, " // {err}")?;
			}
			f.line()?;
			f.indent(|f| flat_func(f, func))?;
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
				f.kw("Unless")?.expr(e)?.label(l)?.line()?;
			},
			FlatInsn::Goto(l) => {
				f.kw("Goto")?.label(l)?.line()?;
			},
			FlatInsn::Switch(e, cs, l) => {
				f.kw("Switch")?.expr(e)?.suf("{")?;
				for (v, l) in cs {
					f.val(v)?.suf(":")?.label(l)?.suf(",")?;
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
							f.kw("if")?.expr(e)?;
						},
						(false, Some(e)) => {
							f.kw("elif")?.expr(e)?;
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
				f.kw("switch")?.expr(e)?.suf(":")?.line()?;
				f.indent(|f| {
					for (v, body) in cs {
						match v {
							Some(v) => {
								f.kw("case")?;
								f.val(v)?;
							}
							None => {
								f.kw("default")?;
							}
						};
						f.suf(":")?.line()?;
						f.indent(|f| tree_func(f, body))?;
					}
					Ok(())
				}).strict()?;
			},
			TreeInsn::While(e, body) => {
				f.kw("while")?.expr(e)?.suf(":")?.line()?;
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
	macro run {
		([$(($ident:ident $(($_n:ident $($ty:tt)*))*))*]) => {
			match i {
				$(Insn::$ident($($_n),*) => {
					run!($ident $(($_n $($ty)*))*);
				})*
			}
		},
		($ident:ident ($v1:ident $_:ty) ($v2:ident Expr)) => {
			f.val($v1)?.expr($v2)?
		},
		($ident:ident $(($_n:ident $ty:ty))*) => {
			f.kw(stringify!($ident))?
				$(.val($_n)?)*
		}
	}

	match i {
		Insn::Menu(a, b, c, d, e) => {
			f.kw("Menu")?.val(a)?.val(b)?.val(c)?.val(d)?;
			f.indent(|f| {
				for (i, line) in e.iter().enumerate() {
					f.line()?;
					f.val(line)?;
					write!(f, "// {i}")?;
				}
				Ok(())
			}).strict()?;
		}
		Insn::VisSet(v, p@0..=2, a,b,c,d) => {
			f.kw("VisSet")?.val(v)?.val(p)?.val(a)?.val(b)?.val(&Time(*c as u32))?.val(d)?;
		}
		Insn::VisSet(v, p@3, a,b,c,d) => {
			f.kw("VisSet")?.val(v)?.val(p)?.val(&Color(*a as u32))?.val(&Time(*b as u32))?.val(c)?.val(d)?;
		}
		_ => {
			themelios::scena::code::introspect!(run);
		}
	}
	Ok(())
}

pub(crate) trait Val {
	fn write(&self, f: &mut Context) -> Result<()>;
}

macro prim_arg($t:ty, $fmt:literal) {
	impl Val for $t {
		fn write(&self, f: &mut Context) -> Result<()> {
			write!(f, $fmt, self)
		}
	}
}

macro nt_arg($t:ty, $fmt:literal) {
	impl Val for $t {
		fn write(&self, f: &mut Context) -> Result<()> {
			write!(f, $fmt, self.0)
		}
	}
}

impl<T: Val> Val for Option<T> {
	fn write(&self, f: &mut Context) -> Result<()> {
		if let Some(a) = self {
			a.write(f)
		} else {
			write!(f, "null")
		}
	}
}

impl<T: Val> Val for [T] {
	fn write(&self, f: &mut Context) -> Result<()> {
		for t in self {
			f.val(t)?;
		}
		Ok(())
	}
}

impl<T: Val> Val for Vec<T> {
	fn write(&self, f: &mut Context) -> Result<()> {
		self.as_slice().write(f)
	}
}

impl<T: Val, const K: usize> Val for [T; K] {
	fn write(&self, f: &mut Context) -> Result<()> {
		self.as_slice().write(f)
	}
}

impl Val for Vec<Insn> {
	fn write(&self, f: &mut Context) -> Result<()> {
		f.suf(":")?;
		f.indent(|f| {
			for line in self.iter() {
				f.line()?;
				insn(f, line)?;
			}
			Ok(())
		})
	}
}

prim_arg!(u8, "{}");
prim_arg!(u16, "{}");
prim_arg!(u32, "{}");
prim_arg!(i8, "{}");
prim_arg!(i16, "{}");
prim_arg!(i32, "{}");
prim_arg!(String, "{:?}");
nt_arg!(TString, "{:?}");

nt_arg!(Time, "{}ms");
nt_arg!(Angle, "{}deg");
nt_arg!(Angle32, "{}mdeg");
nt_arg!(Speed, "{}mm/s");
nt_arg!(Length, "{}mm");

nt_arg!(Flag, "flag[{}]");
nt_arg!(Attr, "system[{}]");
nt_arg!(Var, "var[{}]");
nt_arg!(Global, "global[{}]");

nt_arg!(SystemFlags,    "0x{:08X}");
nt_arg!(CharFlags,      "0x{:04X}");
nt_arg!(QuestFlags,     "0x{:02X}");
nt_arg!(ObjectFlags,    "0x{:04X}");
nt_arg!(LookPointFlags, "0x{:04X}");
nt_arg!(TriggerFlags,   "0x{:04X}");
nt_arg!(EntryFlags,     "0x{:04X}");

nt_arg!(Color,          "0x{:08X}");
nt_arg!(TcMembers,      "0x{:08X}");

impl Val for QuestTask {
	fn write(&self, f: &mut Context) -> Result<()> {
		if f.game.iset.is_ed7() {
			write!(f, "{}", self.0)
		} else {
			write!(f, "0x{:04X}", self.0)
		}
	}
}

nt_arg!(NameId,   "name[{}]");
nt_arg!(BgmId,    "bgm[{}]");
nt_arg!(MagicId,  "magic[{}]");
nt_arg!(QuestId,  "quest[{}]");
nt_arg!(ShopId,   "shop[{}]");
nt_arg!(SoundId,  "sound[{}]");
nt_arg!(TownId,   "town[{}]");
nt_arg!(BattleId, "battle[{}]");
nt_arg!(ItemId,   "item[{}]");

nt_arg!(LookPointId, "look_point[{}]");
nt_arg!(EntranceId,  "entrance[{}]");
nt_arg!(ObjectId,    "object[{}]");
nt_arg!(TriggerId,   "trigger[{}]");
nt_arg!(LabelId,     "label[{}]");
nt_arg!(AnimId,      "anim[{}]");

nt_arg!(ChcpId,  "chcp[{}]");
nt_arg!(VisId,   "vis[{}]");
nt_arg!(ForkId,  "fork[{}]");
nt_arg!(EffId,   "eff[{}]");
nt_arg!(EffInstanceId, "eff_instance[{}]");
nt_arg!(SelectId, "select[{}]");
nt_arg!(MenuId,  "menu[{}]");

impl Val for CharAttr {
	fn write(&self, f: &mut Context) -> Result<()> {
		self.0.write(f)?;
		f.no_space()?;
		write!(f, ".{}", self.1)
	}
}

impl Val for CharId {
	fn write(&self, f: &mut Context) -> Result<()> {
		let v = self.0;
		use InstructionSet::*;
		match v {
			257.. => NameId(v - 257).write(f),
			256   => write!(f, "(ERROR)"),
			255   => write!(f, "null"),
			254   => write!(f, "self"),
			244.. if matches!(f.game.iset, Ao|AoEvo)
			      => write!(f, "custom[{}]", v-244),
			246.. if matches!(f.game.iset, Sc|ScEvo)
			      => write!(f, "party[{}]", v-246),
			238.. => write!(f, "party[{}]", v-238),
			16..  if matches!(f.game.iset, Tc|TcEvo)
			      => write!(f, "char[{}]", v - 16),
			8..   => write!(f, "char[{}]", v - 8),
			0..   => write!(f, "field_party[{}]", v),
		}
	}
}

impl Val for Emote {
	fn write(&self, f: &mut Context) -> Result<()> {
		write!(f, "emote[{},{},{}ms]", self.0, self.1, self.2)
	}
}

impl Val for Pos2 {
	fn write(&self, f: &mut Context) -> Result<()> {
		write!(f, "({x}, null, {z})", x=self.0, z=self.1)
	}
}

impl Val for Pos3 {
	fn write(&self, f: &mut Context) -> Result<()> {
		write!(f, "({x}, {y}, {z})", x=self.0, y=self.1, z=self.2)
	}
}

impl Val for Text {
	fn write(&self, f: &mut Context) -> Result<()> {
		text(f, self)
	}
}

impl Val for FuncRef {
	fn write(&self, f: &mut Context) -> Result<()> {
		write!(f, "fn[{},{}]", self.0, self.1)
	}
}

fn expr(f: &mut Context, e: &Expr) -> Result<()> {
	fn expr_prio(f: &mut Context, e: &Expr, prio: u8) -> Result<()> {
		match e {
			Expr::Const(v)    => { f.val(v)?; }
			Expr::Flag(v)     => { f.val(v)?; }
			Expr::Var(v)      => { f.val(v)?; }
			Expr::Attr(v)     => { f.val(v)?; }
			Expr::CharAttr(v) => { f.val(v)?; }
			Expr::Rand        => { f.kw("random")?; }
			Expr::Global(v)   => { f.val(v)?; }

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
						f.val(n)?.no_space()?;
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

pub(crate) fn game(game: &themelios::gamedata::GameData) -> &'static str {
	use InstructionSet::*;
	match game.iset {
		Fc   => if game.kai { "ed61k" } else { "ed61" },
		Sc   => if game.kai { "ed62k" } else { "ed62" },
		Tc   => if game.kai { "ed63k" } else { "ed63" },
		Zero => if game.kai { "ed71k" } else { "ed71" },
		Ao   => if game.kai { "ed72k" } else { "ed72" },
		FcEvo   => "ed61e",
		ScEvo   => "ed62e",
		TcEvo   => "ed63e",
		ZeroEvo => "ed71e",
		AoEvo   => "ed72e",
	}
}
