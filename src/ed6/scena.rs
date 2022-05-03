use std::borrow::Cow;
use std::collections::{BTreeSet, BTreeMap};

use choubun::Node;
use kaiseki::ed6::{scena::*, Archives};
use kaiseki::util::{Text, TextSegment};

use crate::app::Tables;

#[tracing::instrument(skip(scena, archives, tables))]
pub fn render(scena: &Scena, archives: &Archives, tables: &Tables, raw: bool) -> choubun::Node {
	ScenaRenderer { scena, archives, tables, raw }.render()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharKind {
	Party,
	Npc,
	Monster,
	Self_,
	Member,
	Unknown,
}

struct ScenaRenderer<'a> {
	scena: &'a Scena,
	archives: &'a Archives,
	tables: &'a Tables,
	raw: bool,
}

impl ScenaRenderer<'_> {
	fn render(&self) -> Node {
		choubun::document(|doc| {
			let name = format!("{}/{}", self.scena.dir.decode(), self.scena.fname.decode());
			doc.head.node("title", |a| a.text(&name));
			doc.head.node("link", |a| {
				a.attr("rel", "stylesheet");
				a.attr("href", "/assets/style.css"); // XXX url
			});

			doc.body.node("h1", |a| a.text(format!("{} (town: {}, bgm: {})", &name, self.scena.town, self.scena.bgm)));

			doc.body.node("div", |a| {
				a.indent();
				a.attr("id", "chcp");
				a.node("select", |a| {
					a.indent();
					a.attr("id", "ch");
					for &ch in &self.scena.ch {
						a.node("option", |a| a.text(self.file_name(ch)));
					}
				});
				a.node("select", |a| {
					a.indent();
					a.attr("id", "cp");
					for &cp in &self.scena.cp {
						a.node("option", |a| a.text(self.file_name(cp)));
					}
				});
			});

			doc.body.node("h2", |a| a.text("NPCs"));
			doc.body.node("ol", |a| {
				a.indent();
				a.attr("start", 8usize);
				for npc in &self.scena.npcs {
					a.node("li", |a| a.text(format!("{:?}", npc)));
				}
			});

			doc.body.node("h2", |a| a.text("Monsters"));
			doc.body.node("ol", |a| {
				a.indent();
				a.attr("start", 8usize+self.scena.npcs.len());
				for monster in &self.scena.monsters {
					a.node("li", |a| a.text(format!("{:?}", monster)));
				}
			});

			doc.body.node("h2", |a| a.text("Triggers"));
			doc.body.node("ol", |a| {
				a.indent();
				a.attr("start", 0usize);
				for trigger in &self.scena.triggers {
					a.node("li", |a| a.text(format!("{:?}", trigger)));
				}
			});

			doc.body.node("h2", |a| a.text("Object"));
			doc.body.node("ol", |a| {
				a.indent();
				a.attr("start", 0usize);
				for object in &self.scena.objects {
					a.node("li", |a| a.text(format!("{:?}", object)));
				}
			});

			doc.body.node("h2", |a| a.text("Camera angles (?)"));
			doc.body.node("ol", |a| {
				a.indent();
				a.attr("start", 0usize);
				for camera_angle in &self.scena.camera_angles {
					a.node("li", |a| a.text(format!("{:?}", camera_angle)));
				}
			});

			doc.body.node("h2", |a| a.text("Code"));
			let decompile_span = tracing::info_span!("decompile");
			for (i, func) in self.scena.functions.iter().enumerate() {
				doc.body.node("h3", |a| a.text(format!("Function {}", i)));
				let render = CodeRenderer { inner: self, indent: 0 };
				if self.raw {
					doc.body.node_class("pre", "code asm", |a| render.asm(a, func));
				} else {
					match decompile_span.in_scope(|| decompile(func)) {
						Ok(code) => {
							doc.body.node_class("pre", "code", |a| render.code(a, &code));
						},
						Err(e) => {
							tracing::error!("{:#}", e);
							doc.body.node_class("div", "decompile-error", |a| {
								a.text(e.to_string());
							});
							doc.body.node_class("pre", "code asm", |a| render.asm(a, func));
						},
					}
				}
			}
		})
	}

	fn file_name(&self, FileRef(arch, index): FileRef) -> String {
		if let Ok(file) = self.archives.get(arch as u8, index as usize) {
			let name = file.0.name.decode();
			let (prefix, suffix) = name.split_once('.').unwrap_or((&name, ""));
			let prefix = prefix.trim_end_matches(|a| a == ' ');
			let suffix = suffix.trim_start_matches(|a| a == '_');
			format!("{:02}/{}.{}", arch, prefix, suffix)
		} else {
			format!("{:02}/<{}>", arch, index)
		}
	}

	fn member_name(&self, id: usize) -> Cow<str> {
		let member_names = &["Estelle", "Joshua", "Scherazard", "Olivier", "Kloe", "Agate", "Tita", "Zin"];
		if id < member_names.len() {
			Cow::Borrowed(member_names[id])
		} else {
			Cow::Owned(format!("[unknown {}]", id))
		}
	}

	fn char_name(&self, id: usize) -> (CharKind, Cow<str>) {
		let npc_start = 8;
		let monster_start = npc_start + self.scena.npcs.len();
		let monster_end = monster_start + self.scena.monsters.len();
		let member_start = 0x101;
		
		fn get_name<T>(idx: usize, items: &[T], f: impl Fn(&T) -> &str) -> Cow<str> {
			let name = f(&items[idx]);
			let mut dups = items.iter().enumerate().filter(|a| f(a.1) == name);
			if dups.clone().count() == 1 {
				name.into()
			} else {
				let dup_idx = dups.position(|a| a.0 == idx).unwrap();
				format!("{} [{}]", name, dup_idx+1).into()
			}
		}

		if id == 0 {
			(CharKind::Party, "[lead]".into())
		} else if (1..4).contains(&id) {
			(CharKind::Party, format!("[party {}]", id+1).into())
		} else if (npc_start..monster_start).contains(&id) {
			(CharKind::Npc, get_name(id-npc_start, &self.scena.npcs, |a| &*a.name))
		} else if (monster_start..monster_end).contains(&id) {
			(CharKind::Monster, get_name(id-monster_start, &self.scena.monsters, |a| &*a.name))
		} else if id == 0xFE {
			(CharKind::Self_, "self".into())
		} else if id >= member_start {
			(CharKind::Member, self.member_name(id - member_start))
		} else {
			(CharKind::Unknown, format!("[unknown {}]", id).into())
		}
	}
}

#[extend::ext]
impl Node {
	fn node_class(&mut self, name: &str, class: &str, body: impl FnOnce(&mut Node)) {
		self.node(name, |a| {
			a.class(class);
			body(a);
		})
	}

	fn span(&mut self, class: &str, body: impl FnOnce(&mut Node)) {
		self.node_class("span", class, body)
	}

	fn span_text(&mut self, class: &str, text: impl ToString) {
		self.span(class, |a| a.text(text));
	}
}

struct CodeRenderer<'a> {
	inner: &'a ScenaRenderer<'a>,
	indent: u32,
}

impl<'a> CodeRenderer<'a> {
	fn indent(&self) -> Self {
		CodeRenderer { inner: self.inner, indent: self.indent + 1 }
	}

	fn line(&self, a: &mut Node, body: impl Fn(&mut Node)) {
		a.node("div", |a| {
			a.class("code-line");
			for _ in 0..self.indent {
				a.span_text("indent", "\t");
			}
			body(a);
		})
	}

	fn asm(&self, a: &mut Node, asm: &Asm) {
		let mut labels = BTreeSet::<usize>::new();
		for (_, insn) in &asm.code {
			insn.labels(|a| { labels.insert(a); });
		}

		let labels: BTreeMap<usize, String> =
			labels.into_iter()
			.enumerate()
			.map(|(i, a)| (a, format!("L{}", i)))
			.collect();

		let render_label = |a: &mut Node, addr: usize| {
			a.span("label", |a| {
				a.attr("title", addr);
				a.text(&labels[&addr]);
			});
		};

		for (addr, insn) in &asm.code {
			if labels.contains_key(addr) {
				self.line(a, |a| {
					render_label(a, *addr);
					a.span_text("syntax", ":");
				});
			}

			match insn {
				FlowInsn::Unless(expr, target) => self.line(a, |a| {
					a.text("  ");
					a.span_text("keyword", "UNLESS");
					a.text(" ");
					self.expr(a, expr);
					a.text(" ");
					a.span_text("keyword", "GOTO");
					a.text(" ");
					render_label(a, *target);
				}),

				FlowInsn::Goto(target) => self.line(a, |a| {
					a.text("  ");
					a.span_text("keyword", "GOTO");
					a.text(" ");
					render_label(a, *target);
				}),

				FlowInsn::Switch(expr, branches, default) => self.line(a, |a| {
					a.text("  ");
					a.span_text("keyword", "SWITCH");
					a.text(" ");
					self.expr(a, expr);
					a.text(" ");
					a.span_text("syntax", "[");
					for (case, target) in branches {
						a.span_text("case", case);
						a.span_text("syntax", ":");
						a.text(" ");
						render_label(a, *target);
						a.span_text("syntax", ",");
						a.text(" ");
					}
					a.span_text("keyword", "default");
					a.span_text("syntax", ":");
					a.text(" ");
					render_label(a, *default);
					a.span_text("syntax", "]");
				}),

				FlowInsn::Insn(insn) => self.line(a, |a| {
					a.text("  ");
					self.insn(a, insn);
				}),
			}
		}
	}

	fn code(&self, a: &mut Node, code: &[Stmt]) {
		if code.is_empty() {
			self.line(a, |a| a.span_text("empty-block", "(empty)"));
		}

		for stmt in code {
			match stmt {
				Stmt::If(cases) => {
					self.line(a, |a| a.span_text("keyword", "IF"));
					a.node("div", |_|{});

					let inner = self.indent();
					for (expr, body) in cases {
						inner.line(a, |a| {
							match expr {
								Some(expr) => self.expr(a, expr),
								None => a.span_text("keyword", "ELSE"),
							}
							a.text(" ");
							a.span_text("syntax", "=>");
						});

						inner.indent().code(a, body);
					}
				}

				Stmt::Switch(expr, cases) => {
					self.line(a, |a| {
						a.span_text("keyword", "SWITCH");
						a.text(" ");
						self.expr(a, expr);
					});

					let inner = self.indent();
					for (cases, body) in cases {
						inner.line(a, |a| {
							let mut first = true;
							for case in cases {
								if !first {
									a.span_text("syntax", ",");
									a.text(" ");
								}
								first = false;
								match case {
									Some(case) => a.span_text("case", case),
									None => a.span_text("keyword", "default"),
								}
							}
							a.text(" ");
							a.span_text("syntax", "=>");
						});

						inner.indent().code(a, body);
					}
				}

				Stmt::While(expr, body) => {
					self.line(a, |a| {
						a.span_text("keyword", "WHILE");
						a.text(" ");
						self.expr(a, expr);
					});

					self.indent().code(a, body);
				}

				Stmt::Break => {
					self.line(a, |a| a.span_text("keyword", "BREAK"));
				}

				Stmt::Insn(insn) => {
					self.line(a, |a| self.insn(a, insn));
				}
			}
		}
	}

	fn expr(&self, a: &mut Node, expr: &Expr) {
		self.expr_inner(a, expr, 0)
	}

	fn expr_inner(&self, a: &mut Node, expr: &Expr, prio: u8) {
		match expr {
			Expr::Const(v) => {
				a.span_text("int", v);
			}

			Expr::Binop(op, l, r) => {
				let (text, prio2) = match op {
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
				};
				if prio2 < prio || self.inner.raw { a.span_text("syntax", "("); }
				self.expr_inner(a, l, prio2);
				a.text(" ");
				a.span_text("expr-op", text);
				a.text(" ");
				self.expr_inner(a, r, prio2+1);
				if prio2 < prio || self.inner.raw { a.span_text("syntax", ")"); }
			}

			Expr::Unop(op, v) => {
				let (text, is_assign) = match op {
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
				};
				a.span_text("expr-op", text);
				if is_assign {
					a.text(" ");
					self.expr_inner(a, v, 0);
				} else {
					self.expr_inner(a, v, 100);
				}
			}

			Expr::Exec(insn) => {
				self.insn(a, insn);
			}
			Expr::Flag(flag) => {
				self.insn_parts(a, "Flag", &[InsnArg::flag(flag)]);
			}
			Expr::Var(var) => {
				self.insn_parts(a, "Var", &[InsnArg::var(var)]);
			}
			Expr::Attr(attr) => {
				self.insn_parts(a, "Attr", &[InsnArg::attr(attr)]);
			}
			Expr::CharAttr(char, attr) => {
				self.insn_parts(a, "CharAttr", &[InsnArg::char(char), InsnArg::char_attr(attr)]);
			}
			Expr::Rand => {
				self.insn_parts(a, "Rand", &[]);
			}
		}
	}

	fn insn(&self, a: &mut Node, insn: &Insn) {
		let (name, args) = insn.parts();
		self.insn_parts(a, name, &args);
	}

	fn insn_parts(&self, a: &mut Node, name: &str, args: &[InsnArg]) {
		a.span_text("insn", name);
		let inner = self.indent();
		for arg in args.iter() {
			inner.arg(a, *arg)
		}
	}

	fn arg(&self, a: &mut Node, arg: InsnArg) {
		match arg {
			InsnArg::u8(v)  => { a.text(" "); a.span_text("int", v); }
			InsnArg::u16(v) => { a.text(" "); a.span_text("int", v); }
			InsnArg::u32(v) => { a.text(" "); a.span_text("int", v); }
			InsnArg::i8(v)  => { a.text(" "); a.span_text("int", v); }
			InsnArg::i16(v) => { a.text(" "); a.span_text("int", v); }
			InsnArg::i32(v) => { a.text(" "); a.span_text("int", v); }

			InsnArg::scena_file(v) => {
				a.text(" ");
				let text = self.inner.file_name(*v);
				if text.get(2..3) == Some("/") && text.ends_with(".SN") {
					a.node("a", |a| {
						a.class("file-ref");
						a.attr("href", &text[3..text.len()-3]); // XXX url
						a.text(text);
					});
				} else {
					a.span_text("file-ref", text);
				}
			}

			InsnArg::map_file(v) => {
				a.text(" ");
				a.span_text("file-ref", self.inner.file_name(*v));
			}
			InsnArg::vis_file(v) => {
				a.text(" ");
				a.span_text("file-ref", self.inner.file_name(*v));
			}
			InsnArg::eff_file(v) => {
				a.text(" ");
				a.span_text("file-ref", v);
			}
			InsnArg::op_file(v) => {
				a.text(" ");
				a.span_text("file-ref", v);
			}
			InsnArg::avi_file(v) => {
				a.text(" ");
				a.span_text("file-ref", v);
			}

			InsnArg::pos2(v) => { a.text(" "); a.span_text("unknown", format!("{:?}", v)); }
			InsnArg::pos3(v) => { a.text(" "); a.span_text("unknown", format!("{:?}", v)); }
			InsnArg::relative(v) => { a.text(" "); a.span_text("unknown", format!("relative{:?}", v)); }

			InsnArg::time(v) => { a.text(" "); a.span_text("time", format!("{}ms", v)); }
			InsnArg::speed(v) => { a.text(" "); a.span_text("speed", format!("{}mm/s", v)); }
			InsnArg::angle(v) => { a.text(" "); a.span_text("angle", format!("{}°", v)); }
			InsnArg::color(v) => {
				a.text(" ");
				a.span("color", |a| {
					a.attr("style", format!("--splat-color: #{:06X}; --splat-alpha: {}", v&0xFFFFFF, (v>>24) as f32 / 255.0));
					a.node_class("svg", "color-splat", |a| a.node("use", |a| a.attr("href", "/assets/color-splat.svg#splat"))); // XXX url
					a.text(format!("#{:08X}", v));
				});
			}

			InsnArg::time16(v) => { a.text(" "); a.span_text("time", format!("{}ms", v)); }
			InsnArg::angle32(v) => { a.text(" "); a.span_text("angle", format!("{}m°", v)); }

			InsnArg::battle(v) => { a.text(" "); a.span_text("unknown", format!("{:?}", v)); }
			InsnArg::town(v) => { a.text(" "); a.span_text("unknown", format!("{:?}", v)); }
			InsnArg::bgmtbl(v) => { a.text(" "); a.span_text("unknown", format!("{:?}", v)); }
			InsnArg::quest(v) => { a.text(" "); a.span_text("unknown", format!("{:?}", v)); }
			InsnArg::sound(v) => { a.text(" "); a.span_text("unknown", format!("{:?}", v)); }
			InsnArg::item(v) => { a.text(" "); a.span_text("unknown", format!("{:?}", v)); }
			InsnArg::flag(v) => {
				a.text(" ");
				a.span_text("flag", v);
			}
			InsnArg::shop(v) => { a.text(" "); a.span_text("unknown", format!("{:?}", v)); }
			InsnArg::magic(v) => {
				a.text(" ");
				let magic = self.inner.tables.magic.get(*v as usize);
				let name: Cow<str> = magic.map_or(Cow::Owned(format!("[unknown {}]", v)), |a| Cow::Borrowed(&a.name));
				let kind = if let Some(magic) = magic {
					#[allow(clippy::zero_prefixed_literal)]
					match magic.base.id {
						000..=009 => Cow::Borrowed("unknown"),
						010..=149 => Cow::Owned(magic.base.element.to_string().to_lowercase()),
						150..=229 => Cow::Borrowed("craft"),
						230..=299 => Cow::Borrowed("scraft"),
						300..     => Cow::Borrowed("unknown"),
					}
				} else {
					Cow::Borrowed("unknown")
				};
				self.named(a, "magic", *v as usize, &name, Some(&kind));
			}

			InsnArg::fork(v) => {
				if self.inner.raw {
					a.text(" ");
					a.span_text("syntax", "[");
				}

				for insn in v {
					self.line(a, |a| self.insn(a, insn));
				}

				if self.inner.raw {
					self.line(a, |a| a.span_text("syntax", "]"));
				}
			}

			InsnArg::expr(v) => {
				a.text(" ");
				self.expr(a, v);
			}

			InsnArg::text_title(v) => {
				a.text(" ");
				a.span_text("text-title", v);
			}
			InsnArg::text(v) => {
				a.text(" ");
				a.node("div", |a| {
					a.attr("role", "list");
					a.class("block talk");
					a.attr("style", format!("--indent: {}", self.indent));
					self.text(a, v);
				});
			}

			InsnArg::menu(v) => {
				a.node("div", |a| {
					a.attr("role", "list");
					a.class("block menu");
					a.attr("style", format!("--indent: {}", self.indent));
					for (idx, line) in v.iter().enumerate() {
						a.node("div", |a| {
							a.class("menu-row");
							a.span_text("menu-idx", format!("({})", idx));
							a.text(" ");
							a.span("menu-label", |a| {
								a.attr("role", "listitem");
								a.text(line);
							});
						});
					}
				});
			}

			InsnArg::quests(v) => {
				for q in v {
					self.arg(a, InsnArg::quest(q))
				}
			}

			InsnArg::emote((n, m, time)) => {
				a.text(" ");
				a.span("emote", |a| {
					a.attr("style", format!("--emote-start: {}; --emote-end: {}; --emote-time: {}ms", n, m+1, time));
					a.text(format!("{}..={}", n, m));
				});
				self.arg(a, InsnArg::time(time));
			}

			InsnArg::flags(v)      => { a.text(" "); a.span_text("unknown", format!("0x{:08X}", v)); }
			InsnArg::quest_flag(v) => { a.text(" "); a.span_text("unknown", format!("0x{:02X}", v)); }
			InsnArg::char_flags(v) => { a.text(" "); a.span_text("unknown", format!("0x{:04X}", v)); }
			InsnArg::quest_task(v) => { a.text(" "); a.span_text("unknown", format!("0x{:04X}", v)); }

			InsnArg::sepith_element(v) => {
				a.text(" ");
				let (kind, name) = match v {
					// Not sure about these indices
					0 => (Some("earth"),  Cow::Borrowed("earth")),
					1 => (Some("water"),  Cow::Borrowed("water")),
					2 => (Some("fire"),   Cow::Borrowed("fire")),
					3 => (Some("wind"),   Cow::Borrowed("wind")),
					4 => (Some("time"),   Cow::Borrowed("time")),
					5 => (Some("space"),  Cow::Borrowed("space")),
					6 => (Some("mirage"), Cow::Borrowed("mirage")),
					_ => (None, Cow::Owned(format!("[unknown {}]", v))),
				};
				a.span("sepith-element", |a| {
					if let Some(kind) = kind {
						a.class(&format!("sepith-element-{kind}"));
					}
					if self.inner.raw {
						a.attr("title", format!("sepith-element {v}"));
					}
					a.text(name);
				});
			}

			InsnArg::var(v) => {
				a.text(" ");
				a.span_text("var", v);
			}

			InsnArg::attr(v) => {
				a.text(" ");
				a.span_text("attr", v);
			}

			InsnArg::char_attr(v) => {
				a.text(":");
				let name = match *v {
					1 => Cow::Borrowed("x"),
					2 => Cow::Borrowed("y"),
					3 => Cow::Borrowed("z"),
					4 => Cow::Borrowed("angle"),
					_ => Cow::Owned(format!("[unknown {}]", v)),
				};
				self.named(a, "char-attr", *v as usize, &name, None);
			}

			InsnArg::member_attr(v) => {
				a.text(":");
				let name = match *v {
					0 => Cow::Borrowed("level"),
					5 => Cow::Borrowed("cp"),
					254 => Cow::Borrowed("full_heal"),
					_ => Cow::Owned(format!("[unknown {}]", v)),
				};
				self.named(a, "member-attr", *v as usize, &name, None);
			}

			InsnArg::char(v) => {
				a.text(" ");

				let (kind, name) = self.inner.char_name(*v as usize);
				let kind = match kind {
					CharKind::Party => "party",
					CharKind::Npc => "npc",
					CharKind::Monster => "monster",
					CharKind::Self_ => "self",
					CharKind::Member => "member",
					CharKind::Unknown => "unknown",
				};
				self.named(a, "char", *v as usize, &name, Some(kind));
			}

			InsnArg::member(v) => {
				a.text(" ");
				let name = self.inner.member_name(*v as usize);
				self.named(a, "member", *v as usize, &name, None);
			}

			InsnArg::chcp(v) => { a.text(" "); a.span_text("unknown", format!("{:?}", v)); }
			InsnArg::fork_id(v) => { a.text(" "); a.span_text("unknown", format!("{:?}", v)); }
			InsnArg::menu_id(v) => { a.text(" "); a.span_text("unknown", format!("{:?}", v)); }
			InsnArg::object(v) => { a.text(" "); a.span_text("unknown", format!("{:?}", v)); }
			InsnArg::func_ref(v) => { a.text(" "); a.span_text("unknown", format!("{:?}", v)); }

			InsnArg::data(v) => { a.text(" "); a.span_text("unknown", format!("{:?}", v)); }
		}
	}

	fn named(&self, a: &mut Node, class: &str, v: usize, name: &str, kind: Option<&str>) {
		a.span(class, |a| {
			if let Some(kind) = kind {
				a.class(&format!("{class}-{kind}"));
			}

			if self.inner.raw {
				a.attr("title", match kind {
					Some(kind) => format!("{class}-{kind} {v}"),
					None => format!("{class} {v}"),
				});
			}
			a.text(name);
		});
	}

	fn text(&self, a: &mut Node, v: &Text) {
		let mut color = 0;
		let mut size = 2;
		let mut face = None;
		for page in v.0.split(|s| s == &TextSegment::Page) {
			let body = choubun::node("div", |a| {
				a.class("talk-text");
				let mut iter = page.iter().peekable();
				while let Some(item) = iter.next() {
					match item {
						TextSegment::Page => unreachable!(),
						TextSegment::String(s) => {
							let (s, ruby) = if let Some(TextSegment::Ruby(width, rt)) = iter.peek() {
								iter.next();
								let mut w = 0;
								let split_pos = s.char_indices().rfind(|(_, ch)| {
									match unicode_width::UnicodeWidthChar::width_cjk(*ch) {
										Some(cw) => {
											w += cw;
											w >= *width as usize
										}
										None => true
									}
								}).map_or(0, |a| a.0);
								(&s[..split_pos], Some((&s[split_pos..], rt)))
							} else {
								(&s[..], None)
							};

							if !s.is_empty() {
								a.node("span", |a| {
									a.class(&format!("text text-color-{color}, text-size-{size}"));
									a.text(s)
								});
							}
							if let Some((rb, rt)) = ruby {
								a.node("ruby", |a| {
									a.class(&format!("text text-color-{color}, text-size-{size}"));
									a.text(rb);
									a.node("rp", |a| a.text("（"));
									a.node("rt", |a| a.text(rt));
									a.node("rp", |a| a.text("）"));
								});
							}
						}
						TextSegment::Wait => {},
						TextSegment::Speed(_) => {},
						TextSegment::Pos(_) => {},
						TextSegment::Color(c) => color = *c,
						TextSegment::Size(s) => size = *s,
						TextSegment::Face(f) => face = Some(*f),
						TextSegment::Ruby(_, text) => {
							a.node("ruby", |a| a.node("rt", |a| a.text(text)))
						}
						item => a.span_text("text-unknown", format!("{:?}", item)),
					}
				}
			});

			a.node("div", |a| {
				a.class("talk-page");
				if let Some(face) = face {
					a.node("div", |a| {
						a.class("talk-face-wrapper");
						a.leaf("img", |a| {
							a.class("talk-face-visible");
							a.attr("loading", "lazy");
							a.attr("src", format!("/fc/face/{face}.png"));
						});
						a.leaf("img", |a| {
							a.class("talk-face-click");
							a.attr("loading", "lazy");
							a.attr("src", format!("/fc/face/{face}.png"));
						});
					});
				}
				a.add_node(body);
			})
		}
	}
}
