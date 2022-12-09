use super::{ed6::{Scena, Npc, Monster, Trigger, Object, Entry}, FuncRef, CharId, Pos2, Pos3};
use super::code::{InsnArg as I, FlatInsn, Label, Insn, Expr, ExprBinop, ExprUnop};
use super::code::decompile::{decompile, TreeInsn};
use crate::text::{Text, TextSegment};

pub struct Context {
	pub blind: bool,
	pub decompile: bool,
	pub indent: usize,
	is_line: bool,
	pub output: String,
}

impl Context {
	pub fn new() -> Self {
		Self {
			blind: false,
			decompile: true,
			indent: 0,
			is_line: true,
			output: String::new(),
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

impl Default for Context {
	fn default() -> Self {
		Self::new()
	}
}

impl Context {
	pub fn write(&mut self, arg: &str) {
		assert!(!arg.contains('\n'));
		assert!(!arg.contains('\t'));
		if self.is_line {
			for _ in 0..self.indent {
				self.output.push('\t');
			}
		}
		self.output.push_str(arg);
		self.is_line = false
	}

	pub fn line(&mut self) {
		self.output.push('\n');
		self.is_line = true;
	}

	pub fn writeln(&mut self, arg: &str) {
		self.write(arg);
		self.line();
	}

	pub fn indent(&mut self, body: impl FnOnce(&mut Self)) {
		self.indent += 1;
		body(self);
		self.indent -= 1;
	}
}

pub fn dump(f: &mut Context, scena: &Scena) {
	let Scena {
		path,
		map,
		town,
		bgm,
		item,
		includes,
		ch,
		cp,
		npcs,
		monsters,
		triggers,
		objects,
		entries,
		functions,
	} = scena;

	f.write("scena");
	object(f, &[
		("path", I::String(path)),
		("map", I::String(map)),
		("town", I::TownId(town)),
		("bgm", I::BgmId(bgm)),
		("item", I::FuncRef(item)),
	]);
	f.line();

	for (i, a) in includes.iter().enumerate() {
		if let Some(a) = a {
			f.write(&format!("scp {i} "));
			val(f, I::String(a));
			f.line();
		}
	}
	if includes.iter().any(|a| a.is_some()) {
		f.line();
	}

	for Entry {
		pos, chr, angle,
		cam_from, cam_at, cam_zoom, cam_pers, cam_deg, cam_limit1, cam_limit2, north,
		flags, town, init, reinit,
	} in entries {
		f.write("entry");
		object(f, &[
			("pos", I::Pos3(pos)),
			("chr", I::u16(chr)),
			("angle", I::Angle(angle)),
			("cam_from", I::Pos3(cam_from)),
			("cam_at", I::Pos3(cam_at)),
			("cam_zoom", I::i32(cam_zoom)),
			("cam_pers", I::i32(cam_pers)),
			("cam_deg", I::Angle(cam_deg)),
			("cam_limit1", I::Angle(cam_limit1)),
			("cam_limit2", I::Angle(cam_limit2)),
			("north", I::Angle(north)),
			("flags", I::u16(flags)),
			("town", I::TownId(town)),
			("init", I::FuncRef(init)),
			("reinit", I::FuncRef(reinit)),
		]);
		f.line();
	}

	for (i, a) in cp.iter().enumerate() {
		f.write("char_pattern ");
		val(f, I::String(a));
		f.writeln(&format!(" // {i}"));
	}
	if !cp.is_empty() {
		f.line();
	}


	for (i, a) in ch.iter().enumerate() {
		f.write("char_data ");
		val(f, I::String(a));
		f.writeln(&format!(" // {i}"));
	}
	if !ch.is_empty() {
		f.line();
	}

	let mut n = 8;

	for Npc { name, pos, angle, x, cp, frame, ch, flags, init, talk } in npcs {
		f.write("npc ");
		val(f, I::CharId(&CharId(n)));
		object(f, &[
			("name", I::TextTitle(name)),
			("pos", I::Pos3(pos)),
			("angle", I::Angle(angle)),
			("x", I::u16(x)),
			("pt", I::u16(cp)),
			("no", I::u16(frame)),
			("bs", I::u16(ch)),
			("flags", I::CharFlags(flags)),
			("init", I::FuncRef(init)),
			("talk", I::FuncRef(talk)),
		]);
		f.line();
		n += 1;
	}

	for Monster { name, pos, angle, _1, flags, _2, battle, flag, _3 } in monsters {
		f.write("monster ");
		val(f, I::CharId(&CharId(n)));
		object(f, &[
			("name", I::TextTitle(name)),
			("pos", I::Pos3(pos)),
			("angle", I::Angle(angle)),
			("_1", I::u16(_1)),
			("flags", I::CharFlags(flags)),
			("_2", I::i32(_2)),
			("battle", I::BattleId(battle)),
			("flag", I::Flag(flag)),
			("_3", I::u16(_3)),
		]);
		f.line();
		n += 1;
	}

	for Trigger { pos1, pos2, flags, func, _1 } in triggers {
		f.write("trigger");
		object(f, &[
			("pos1", I::Pos3(pos1)),
			("pos2", I::Pos3(pos2)),
			("flags", I::u16(flags)),
			("func", I::FuncRef(func)),
			("_1", I::u16(_1)),
		]);
		f.line();
	}

	for (n, Object { pos, radius, bubble_pos, flags, func, _1 }) in objects.iter().enumerate() {
		f.write("object ");
		val(f, I::ObjectId(&(n as u16)));
		object(f, &[
			("pos", I::Pos3(pos)),
			("radius", I::u32(radius)),
			("bubble_pos", I::Pos3(bubble_pos)),
			("flags", I::ObjectFlags(flags)),
			("func", I::FuncRef(func)),
			("_1", I::u16(_1)),
		]);
		f.line();
	}

	for (i, func) in functions.iter().enumerate() {
		f.write("fn ");
		val(f, I::FuncRef(&FuncRef(0, i as u16)));
		let result = if f.decompile {
			decompile(func).map_err(Some)
		} else {
			Err(None)
		};
		match result {
			Ok(result) => {
				f.writeln(":");
				f.indent(|f| tree_func(f, &result));
			}
			Err(err) => {
				f.write(" flat:");
				if let Some(err) = err {
					f.write(&format!(" // {err}"));
				}
				f.line();
				f.indent(|f| flat_func(f, func));
			}
		}
		f.line();
	}
}

fn object(f: &mut Context, vals: &[(&str, I)]) {
	f.writeln(":");
	f.indent(|f| {
		for (k, v) in vals {
			f.write(k);
			f.write(" ");
			val(f, *v);
			f.line();
		}
	});
}

pub fn flat_func(f: &mut Context, func: &[FlatInsn]) {
	for i in func {
		match i {
			FlatInsn::Unless(e, l) => {
				f.write("Unless ");
				val(f, I::Expr(e));
				f.write(" ");
				label(f, l);
				f.line();
			},
			FlatInsn::Goto(l) => {
				f.write("Goto ");
				label(f, l);
				f.line();
			},
			FlatInsn::Switch(e, cs, l) => {
				f.write("Switch ");
				val(f, I::Expr(e));
				f.write(" {");
				for (v, l) in cs {
					val(f, I::u16(v));
					f.write(": ");
					label(f, l);
					f.write(", ");
				}
				f.write("default: ");
				label(f, l);
				f.write("}");
				f.line();
			},
			FlatInsn::Insn(i) => {
				insn(f, i);
				f.line();
			},
			FlatInsn::Label(l) => {
				f.write("@");
				label(f, l);
				f.line();
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
							f.write("if ");
							val(f, I::Expr(e));
						},
						(false, Some(e)) => {
							f.write("elif ");
							val(f, I::Expr(e));
						},
						(false, None) => {
							f.write("else");
						},
						(true, None) => panic!(),
					}
					first = false;
					f.writeln(":");
					f.indent(|f| tree_func(f, body));
				}
			},
			TreeInsn::Switch(e, cs) => {
				f.write("switch ");
				val(f, I::Expr(e));
				f.writeln(":");
				f.indent(|f| {
					for (v, body) in cs {
						match v {
							Some(v) => val(f, I::u16(v)),
							None => f.write("default"),
						}
						f.writeln(" =>");
						f.indent(|f| tree_func(f, body));
					}
				});
			},
			TreeInsn::While(e, body) => {
				f.write("while ");
				val(f, I::Expr(e));
				f.writeln(":");
				f.indent(|f| tree_func(f, body));
			},
			TreeInsn::Break => {
				f.write("break");
				f.line();
			},
			TreeInsn::Continue => {
				f.write("continue");
				f.line();
			},
			TreeInsn::Insn(i) => {
				insn(f, i);
				f.line();
			},
		}
	}
}

fn insn(f: &mut Context, i: &Insn) {
	f.write(i.name());
	for &a in i.args().iter() {
		f.write(" ");
		val(f, a);
	}
}

fn label(f: &mut Context, l: &Label) {
	f.write(&format!("L{}", l.0));
}

fn val(f: &mut Context, a: I) {
	match a {
		// I::i8(v)  => f.write(&format!("{v}")),
		I::i16(v) => f.write(&format!("{v}")),
		I::i32(v) => f.write(&format!("{v}")),
		I::u8(v)  => f.write(&format!("{v}")),
		I::u16(v) => f.write(&format!("{v}")),
		I::u32(v) => f.write(&format!("{v}")),
		I::String(v) => f.write(&format!("{v:?}")),

		I::Flag(v) => f.write(&format!("flag[{}]", v.0)),
		I::Attr(v) => f.write(&format!("system[{}]", v.0)),
		I::Var(v) => f.write(&format!("var[{}]", v.0)),
		I::Global(v) => f.write(&format!("global[{}]", v.0)),
		I::CharAttr(v) => { val(f, I::CharId(&v.0)); f.write(&format!(":{}", v.1)) },

		I::SystemFlags(v) => f.write(&format!("0x{:08X}", v.0)),
		I::CharFlags(v)   => f.write(&format!("0x{:04X}", v.0)),
		I::QuestFlags(v)  => f.write(&format!("0x{:02X}", v.0)),
		I::ObjectFlags(v) => f.write(&format!("0x{:04X}", v.0)),
		I::Color(v)       => f.write(&format!("#{:08X}", v.0)),

		I::Member(v)   => f.write(&format!("{v:?}")),
		I::CharId(v)   => f.write(&format!("{v:?}")),
		I::BattleId(v) => f.write(&format!("{v:?}")),
		I::BgmId(v)    => f.write(&format!("{v:?}")),
		I::ItemId(v)   => f.write(&format!("{v:?}")),
		I::MagicId(v)  => f.write(&format!("{v:?}")),
		I::QuestId(v)  => f.write(&format!("{v:?}")),
		I::ShopId(v)   => f.write(&format!("{v:?}")),
		I::SoundId(v)  => f.write(&format!("{v:?}")),
		I::TownId(v)   => f.write(&format!("{v:?}")),

		I::EntranceId(v) => f.write(&format!("EntranceId({v})")),
		I::ForkId(v)   => f.write(&format!("ForkId({v})")),
		I::MenuId(v)   => f.write(&format!("MenuId({v})")),
		I::SelectId(v) => f.write(&format!("SelectId({v})")),
		I::ObjectId(v) => f.write(&format!("ObjectId({v})")),
		I::VisId(v)    => f.write(&format!("VisId({v})")),
		I::EffId(v)    => f.write(&format!("EffId({v})")),
		I::ChcpId(v)   => f.write(&format!("ChcpId({v})")),

		I::Expr(v) => expr(f, v),
		I::Fork(v) => {
			f.writeln(":");
			f.indent(|f| {
				for i in v {
					insn(f, i);
					f.line();
				}
			})
		},
		I::FuncRef(v) => {
			if v.0 != 0 {
				f.write(&format!("{}", v.0))
			}
			f.write(&format!(":{}", v.1))
		},

		I::TextTitle(_) if f.blind => f.write("\"…\""),
		I::TextTitle(v) => f.write(&format!("{v:?}")),
		I::Text(_) if f.blind => f.write("{…}"),
		I::Text(v) => text(f, v),
		I::MenuItem(_) if f.blind => f.write("\"…\""),
		I::MenuItem(v) => f.write(&format!("{v:?}")),

		I::Menu(v) => {
			f.writeln("[");
			f.indent(|f| {
				for (i, line) in v.iter().enumerate() {
					val(f, I::u32(&(i as u32)));
					f.write(" => ");
					val(f, I::MenuItem(line));
					f.line();
				}
			});
			f.write("]");
		},

		I::Angle(v)   => f.write(&format!("{v}°")),
		I::Angle32(v) => f.write(&format!("{v}°₃₂")),
		I::Speed(v)   => f.write(&format!("{v}mm/s")),
		I::Time(v)    => f.write(&format!("{v}ms")),

		I::Pos2(Pos2(x,z))   => f.write(&format!("({x}, -, {z})")),
		I::Pos3(Pos3(x,y,z)) => f.write(&format!("({x}, {y}, {z})")),
		// I::RelPos3(Pos3(x,y,z)) => f.write(&format!("({x:+}, {y:+}, {z:+})")),

		I::Emote(v) => f.write(&format!("{v:?}")),
		I::MemberAttr(v) => f.write(&format!("{v:?}")),
		I::QuestTask(v) => f.write(&format!("{v:?}")),
		I::Animation(v) => f.write(&format!("{v:?}")),

		I::QuestList(v)        => f.write(&format!("{v:?}")),
		I::MandatoryMembers(v) => f.write(&format!("{v:?}")),
		I::OptionalMembers(v)  => f.write(&format!("{v:?}")),
		I::TcMembers(v)        => f.write(&format!("{v:016b}")),
		I::NpcBattleCombatants(v) => f.write(&format!("{v:?}")),

		I::AviFileRef(v)   => f.write(&format!("{v:?}")),
		I::EffFileRef(v)   => f.write(&format!("{v:?}")),
		I::MapFileRef(v)   => f.write(&format!("{v:?}")),
		I::OpFileRef(v)    => f.write(&format!("{v:?}")),
		I::ScenaFileRef(v) => f.write(&format!("{v:?}")),
		I::VisFileRef(v)   => f.write(&format!("{v:?}")),
	}
}

fn expr(f: &mut Context, e: &Expr) {
	expr_prio(f, e, 0)
}

fn expr_prio(f: &mut Context, e: &Expr, prio: u8) {
	match e {
		Expr::Const(v)    => val(f, I::u32(v)),
		Expr::Flag(v)     => val(f, I::Flag(v)),
		Expr::Var(v)      => val(f, I::Var(v)),
		Expr::Attr(v)     => val(f, I::Attr(v)),
		Expr::CharAttr(v) => val(f, I::CharAttr(v)),
		Expr::Rand        => f.write("Rand"),
		Expr::Global(v)   => val(f, I::Global(v)),

		Expr::Binop(op, a, b) => {
			let (text, prio2) = binop(*op);
			if prio2 < prio { f.write("("); }
			expr_prio(f, a, prio2);
			f.write(" ");
			f.write(text);
			f.write(" ");
			expr_prio(f, b, prio2+1);
			if prio2 < prio { f.write(")"); }
		},
		Expr::Unop(op, e) => {
			let (text, is_assign) = unop(*op);
			if is_assign {
				f.write(text);
				f.write(" ");
				expr_prio(f, e, 0);
			} else {
				f.write(text);
				expr_prio(f, e, 100);
			}
		},
		Expr::Insn(i) => insn(f, i),
	}
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

fn text(f: &mut Context, v: &Text) {
	f.write("{");
	f.indent += 1;
	f.line();
	for v in v.iter() {
		match v {
			TextSegment::String(s) => {
				f.write(&s.replace('{', "{{").replace('}', "{}"))
			},
			TextSegment::Line => {
				f.line()
			},
			TextSegment::Wait => {
				f.write("{wait}")
			},
			TextSegment::Page => {
				f.indent -= 1;
				f.line();
				f.write("} {");
				f.line();
				f.indent += 1;
			},
			TextSegment::Color(n) => {
				f.write(&format!("{{color {n}}}"));
			},
			TextSegment::Line2 => {
				f.write("\\");
				f.line()
			},
			TextSegment::Item(n) => {
				f.write("{item ");
				val(f, I::ItemId(n));
				f.write("}");
			},
			TextSegment::Byte(n) => {
				f.write(&format!("{{0x{n:02X}}}"))
			},
		}
	}
	f.line();
	f.indent -= 1;
	f.write("}");
}
