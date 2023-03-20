use themelios::scena::*;
use themelios::scena::code::{Expr, ExprTerm, ExprOp, FlatInsn, Label, Insn, Code};
use themelios::scena::decompile::{decompile, TreeInsn};
use themelios::text::{Text, TextSegment};
use themelios::types::*;
use crate::writer::Context;

#[extend::ext(name = ContextExt)]
pub(crate) impl Context<'_> {
	fn val<I: Val>(&mut self, arg: &I) -> &mut Self {
		arg.write(self);
		self.space();
		self
	}

	fn expr(&mut self, arg: &Expr) -> &mut Self {
		expr(self, arg);
		self.space();
		self
	}
}

pub fn func(f: &mut Context, func: &Code) {
	let result = if f.decompile {
		decompile(func).map_err(Some)
	} else {
		Err(None)
	};
	match result {
		Ok(result) => {
			f.suf(":").line();
			f.indent(|f| tree_func(f, &result));
		}
		Err(err) => {
			f.kw("flat").suf(":");
			if let Some(err) = err {
				write!(f, " // {err}");
			}
			f.line();
			f.indent(|f| flat_func(f, func));
		}
	}
}

pub fn flat_func(f: &mut Context, func: &[FlatInsn]) {
	#[extend::ext]
	impl Context<'_> {
		fn label(&mut self, l: &Label) -> &mut Self {
			self.kw(&format!("L{}", l.0))
		}
	}

	for i in func {
		match i {
			FlatInsn::Unless(e, l) => {
				f.kw("Unless").expr(e).label(l).line();
			},
			FlatInsn::Goto(l) => {
				f.kw("Goto").label(l).line();
			},
			FlatInsn::Switch(e, cs, l) => {
				f.kw("Switch").expr(e).suf("{");
				for (v, l) in cs {
					f.val(v).suf(":").label(l).suf(",");
				}
				f.kw("default").suf(":").label(l);
				f.pre("}").line();
			},
			FlatInsn::Insn(i) => {
				insn(f, i, true);
			},
			FlatInsn::Label(l) => {
				f.pre("@").label(l).line();
			},
		}
	}
}

pub fn tree_func(f: &mut Context, func: &[TreeInsn]) {
	for i in func {
		match i {
			TreeInsn::If(cs) => {
				let mut first = true;
				for (e, body) in cs {
					match (first, e) {
						(true, Some(e)) => {
							f.kw("if").expr(e);
						},
						(false, Some(e)) => {
							f.kw("elif").expr(e);
						},
						(false, None) => {
							f.kw("else");
						},
						(true, None) => panic!(),
					}
					first = false;
					f.suf(":").line();
					f.indent(|f| tree_func(f, body));
				}
			},
			TreeInsn::Switch(e, cs) => {
				f.kw("switch").expr(e).suf(":").line();
				f.indent(|f| {
					for (v, body) in cs {
						match v {
							Some(v) => {
								f.kw("case");
								f.val(v);
							}
							None => {
								f.kw("default");
							}
						};
						f.suf(":").line();
						f.indent(|f| tree_func(f, body));
					}
				});
			},
			TreeInsn::While(e, body) => {
				f.kw("while").expr(e).suf(":").line();
				f.indent(|f| tree_func(f, body));
			},
			TreeInsn::Break => {
				f.kw("break").line();
			},
			TreeInsn::Continue => {
				f.kw("continue").line();
			},
			TreeInsn::Insn(i) => {
				insn(f, i, true);
			},
		}
	}
}

fn insn(f: &mut Context, i: &Insn, mut line: bool) {
	macro run([$(($ident:ident $(($_n:ident $($ty:tt)*))*))*]) {
		match i {
			$(Insn::$ident($($_n),*) => {
				insn!($ident $(($_n $($ty)*))*);
			})*
		}
	}

	macro insn {
		($ident:ident ($v1:ident $($ty:tt)*) ($v2:ident Expr)) => {
			op!($v1 $($ty)*);
			f.expr($v2);
		},
		($ident:ident $(($_n:ident $($ty:tt)*))*) => {
			f.kw(stringify!($ident));
			$(op!($_n $($ty)*);)*
		}
	}

	macro op {
		($_n:ident Vec<TString>) => {
			if line {
				f.line().indent(|f| {
					for (i, line) in $_n.iter().enumerate() {
						f.val(line);
						write!(f, "// {i}");
						f.line();
					}
				});
				line = false;
			} else {
				f.val($_n);
			}
		},
		($_n:ident Code) => {
			func(f, $_n);
			line = false;
		},
		($_n:ident $($ty:tt)*) => {
			f.val($_n);
		}
	}

	match i {
		Insn::VisSet(vis, prop, a, b, c, d) => {
			f.kw("VisSet").val(vis).val(prop);
			match prop {
				0..=2 => f.val(a).val(b).val(&Time(*c as u32)).val(d),
				3 => f.val(&Color(*a as u32)).val(&Time(*b as u32)).val(c).val(d),
				_ => f.val(a).val(b).val(c).val(d),
			};
		}
		_ => {
			themelios::scena::code::introspect!(run);
		}
	}
	if line {
		f.line();
	}
}

pub(crate) trait Val {
	fn write(&self, f: &mut Context);
}

macro prim_arg($t:ty, $fmt:literal) {
	impl Val for $t {
		fn write(&self, f: &mut Context) {
			write!(f, $fmt, self)
		}
	}
}

macro nt_arg($t:ty, $fmt:literal) {
	impl Val for $t {
		fn write(&self, f: &mut Context) {
			write!(f, $fmt, self.0)
		}
	}
}

impl<T: Val> Val for Option<T> {
	fn write(&self, f: &mut Context) {
		if let Some(a) = self {
			a.write(f)
		} else {
			write!(f, "null")
		}
	}
}

impl<T: Val> Val for [T] {
	fn write(&self, f: &mut Context) {
		for t in self {
			f.val(t);
		}
	}
}

impl<T: Val> Val for Vec<T> {
	fn write(&self, f: &mut Context) {
		self.as_slice().write(f)
	}
}

impl<T: Val, const K: usize> Val for [T; K] {
	fn write(&self, f: &mut Context) {
		self.as_slice().write(f)
	}
}

prim_arg!(u8, "{}");
prim_arg!(u16, "{}");
prim_arg!(u32, "{}");
prim_arg!(i8, "{}");
prim_arg!(i16, "{}");
prim_arg!(i32, "{}");

impl Val for String {
	fn write(&self, f: &mut Context) {
		write!(f, "\"");
		for c in self.chars() {
			match c {
				'\"' => write!(f, "\\\""),
				'\\' => write!(f, "\\\\"),
				c => write!(f, "{}", c),
			}
		}
		write!(f, "\"");
	}
}

impl Val for TString {
	fn write(&self, f: &mut Context) {
		self.0.write(f)
	}
}

nt_arg!(Time, "{}ms");
nt_arg!(Angle, "{}deg");
nt_arg!(Angle32, "{}mdeg");
nt_arg!(Speed, "{}mm/s");
nt_arg!(AngularSpeed, "{}deg/s");
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
	fn write(&self, f: &mut Context) {
		if f.game.is_ed7() {
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

nt_arg!(ChipId,  "chip[{}]");
nt_arg!(VisId,   "vis[{}]");
nt_arg!(ForkId,  "fork[{}]");
nt_arg!(EffId,   "eff[{}]");
nt_arg!(EffInstanceId, "eff_instance[{}]");
nt_arg!(MenuId,  "menu[{}]");

nt_arg!(SepithId,  "sepith[{}]");
nt_arg!(AtRollId,  "at_roll[{}]");
nt_arg!(PlacementId,  "placement[{}]");

impl Val for FileId {
	fn write(&self, f: &mut Context) {
		if self.0 == 0 {
			write!(f, "null")
		} else if let Some(name) = f.lookup.name(self.0) {
			name.write(f)
		} else {
			write!(f, "file[0x{:08X}]", self.0)
		}
	}
}

impl Val for CharAttr {
	fn write(&self, f: &mut Context) {
		write!(f, "char_attr[");
		self.0.write(f);
		write!(f, ",");
		self.1.write(f);
		write!(f, "]");
	}
}

impl Val for CharId {
	fn write(&self, f: &mut Context) {
		let v = self.0;
		match v {
			257.. => NameId(v - 257).write(f),
			256   => write!(f, "(ERROR)"),
			255   => write!(f, "null"),
			254   => write!(f, "self"),
			244.. if f.game.base() == BaseGame::Ao
			      => write!(f, "custom[{}]", v-244),
			246.. if f.game.base() == BaseGame::Sc
			      => write!(f, "party[{}]", v-246),
			238.. => write!(f, "party[{}]", v-238),
			16..  if f.game.base() == BaseGame::Tc
			      => write!(f, "char[{}]", v - 16),
			8..   if f.game.base() != BaseGame::Tc
			      => write!(f, "char[{}]", v - 8),
			0..   => write!(f, "field_party[{}]", v),
		}
	}
}

impl Val for Pos2 {
	fn write(&self, f: &mut Context) {
		write!(f, "({x}, null, {z})", x=self.0, z=self.1)
	}
}

impl Val for Pos3 {
	fn write(&self, f: &mut Context) {
		write!(f, "({x}, {y}, {z})", x=self.0, y=self.1, z=self.2)
	}
}

impl Val for Text {
	fn write(&self, f: &mut Context) {
		text(f, self)
	}
}

impl Val for FuncId {
	fn write(&self, f: &mut Context) {
		write!(f, "fn[{},{}]", self.0, self.1)
	}
}

fn expr(f: &mut Context, e: &Expr) {
	#[derive(Default)]
	enum E<'a> {
		Atom(&'a ExprTerm),
		Bin(ExprOp, Box<E<'a>>, Box<E<'a>>),
		Un(ExprOp, Box<E<'a>>),
		Ass(ExprOp, Box<E<'a>>),
		#[default]
		Error,
	}

	fn expr_prio(f: &mut Context, e: E, prio: u8) {
		match e {
			E::Atom(a) => match a {
				ExprTerm::Op(_)       => unreachable!(),
				ExprTerm::Const(v)    => { f.val(v); }
				ExprTerm::Insn(i)     => { insn(f, i, false); }
				ExprTerm::Flag(v)     => { f.val(v); }
				ExprTerm::Var(v)      => { f.val(v); }
				ExprTerm::Attr(v)     => { f.val(v); }
				ExprTerm::CharAttr(v) => { f.val(v); }
				ExprTerm::Rand        => { f.kw("random"); }
				ExprTerm::Global(v)   => { f.val(v); }
			},
			E::Bin(op, a, b) => {
				let (text, prio2) = op_str(op);
				if prio2 < prio {
					f.pre("(");
				}
				expr_prio(f, *a, prio2);
				f.kw(text);
				expr_prio(f, *b, prio2+1);
				if prio2 < prio {
					f.suf(")");
				}
			}
			E::Un(op, a) => {
				let (text, prio) = op_str(op);
				f.pre(text);
				expr_prio(f, *a, prio);
			}
			E::Ass(op, a) => {
				let (text, prio) = op_str(op);
				f.kw(text);
				expr_prio(f, *a, prio);
			},
			E::Error => { write!(f, "(EXPR MISSING)"); },
		}
	}

	fn op_str(op: ExprOp) -> (&'static str, u8) {
		match op {
			ExprOp::Eq      => ("==", 4),
			ExprOp::Ne      => ("!=", 4),
			ExprOp::Lt      => ("<",  4),
			ExprOp::Gt      => (">",  4),
			ExprOp::Le      => ("<=", 4),
			ExprOp::Ge      => (">=", 4),
			ExprOp::BoolAnd => ("&&", 3),
			ExprOp::And     => ("&", 3),
			ExprOp::Or      => ("|", 1),
			ExprOp::Add     => ("+", 5),
			ExprOp::Sub     => ("-", 5),
			ExprOp::Xor     => ("^", 2),
			ExprOp::Mul     => ("*", 6),
			ExprOp::Div     => ("/", 6),
			ExprOp::Mod     => ("%", 6),

			ExprOp::Not    => ("!", 10),
			ExprOp::Neg    => ("-", 10),
			ExprOp::Inv    => ("~", 10),

			ExprOp::Ass    => ("=",  0),
			ExprOp::MulAss => ("*=", 0),
			ExprOp::DivAss => ("/=", 0),
			ExprOp::ModAss => ("%=", 0),
			ExprOp::AddAss => ("+=", 0),
			ExprOp::SubAss => ("-=", 0),
			ExprOp::AndAss => ("&=", 0),
			ExprOp::XorAss => ("^=", 0),
			ExprOp::OrAss  => ("|=", 0),
		}
	}

	let mut stack = Vec::new();
	for t in &e.0 {
		if let ExprTerm::Op(op) = t {
			match op.kind() {
				code::OpKind::Unary => {
					let a = stack.pop().unwrap_or_default();
					stack.push(E::Un(*op, Box::new(a)));
				}
				code::OpKind::Binary => {
					let b = stack.pop().unwrap_or_default();
					let a = stack.pop().unwrap_or_default();
					stack.push(E::Bin(*op, Box::new(a), Box::new(b)));
				}
				code::OpKind::Assign => {
					let a = stack.pop().unwrap_or_default();
					stack.push(E::Ass(*op, Box::new(a)));
				}
			}
		} else {
			stack.push(E::Atom(t))
		}
	}

	if stack.is_empty() {
		write!(f, "(EXPR MISSING)");
	} else {
		for (i, e) in stack.into_iter().enumerate() {
			if i != 0 {
				f.kw("¤");
			}
			expr_prio(f, e, 0);
		}
	}
}

fn text(f: &mut Context, v: &Text) {
	let mut it = v.iter().peekable();
	loop {
		f.kw("{").line();
		let cont = f.indent(|f| {
			loop {
				let Some(next) = it.next() else { break false };
				match next {
					TextSegment::String(s) => {
						if f.is_line() && s.starts_with(' ') {
							write!(f, "{{}}")
						}
						let s = s
							.replace('\\', "\\\\")
							.replace('{', "\\{")
							.replace('}', "\\}");
						write!(f, "{s}")
					}
					TextSegment::Line => {
						f.line();
					}
					TextSegment::Wait => {
						write!(f, "{{wait}}")
					}
					TextSegment::Page => {
						break true
					}
					TextSegment::Color(n) => {
						write!(f, "{{color {n}}}");
					}
					TextSegment::Item(n) => {
						write!(f, "{{item ");
						f.val(n).no_space();
						write!(f, "}}");
					}
					TextSegment::Byte(n) => {
						write!(f, "{{0x{n:02X}}}");
						if *n == 0x0D && !matches!(it.peek(), None|Some(TextSegment::Line)) {
							write!(f, "\\");
							f.line();
						}
					}
				}
			}
		});
		f.line().kw("}");
		if !cont {
			break
		}
	}
}

pub(crate) fn game(game: Game) -> &'static str {
	use Game::*;
	match game {
		Fc      => "fc",
		Sc      => "sc",
		Tc      => "tc",
		Zero    => "zero",
		Ao      => "ao",
		FcEvo   => "fc_e",
		ScEvo   => "sc_e",
		TcEvo   => "tc_e",
		ZeroEvo => "zero_e",
		AoEvo   => "ao_e",
		FcKai   => "fc_k",
		ScKai   => "sc_k",
		TcKai   => "tc_k",
		ZeroKai => "zero_k",
		AoKai   => "ao_k",
	}
}
