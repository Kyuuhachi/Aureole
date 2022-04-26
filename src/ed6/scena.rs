use std::collections::{BTreeSet, BTreeMap};

use choubun::Node;
use kaiseki::ed6::{scena::*, code::*};
use kaiseki::Text;

pub fn render(scena: &Scena, asm: bool) -> choubun::Node {
	let Scena {
		dir, fname, town, bgm,
		entry_func,
		includes,
		ch, cp,
		npcs, monsters, triggers, objects,
		camera_angles,
		functions,
	} = scena;

	choubun::document(|doc| {
		doc.root.attr("lang", "en");
		let name = format!("{}/{}", dir.decode(), fname.decode());
		doc.head.node("title", |a| a.text(&name));
		doc.head.node("style", |a| a.raw(r#"
			.code { tab-size: 4; }
			.code .indent::before { position: absolute; content: "⟩"; }

			.code              { color: #FF0000; }
			.code .indent      { color: #CFCFCF; }
			.code .empty-block { color: #AFAFAF; }

			.code .syntax      { color: #7F007F; }
			.code .keyword     { color: #7F007F; font-weight: bold; }
			.code .case        { color: #007F00; }
			.code .label       { color: #007F00; }
			.code .insn        { color: #000000; }
			.code .expr-op     { color: #3F3F00; }

			.code .int         { color: #007F3F; }
			.code .flag        { color: #0000FF; }
			.code .var         { color: #3F007F; }
			.code .attr        { color: #3F7F7F; }
			.code .char        { color: #7F3F00; }
			.code .char-attr   { color: #7F7F00; }

			.code .time        { color: #00AF3F; }
			.code .speed       { color: #00AF3F; }
			.code .angle       { color: #00AF3F; }
			.code .color       { color: #000000; }
		"#));

		doc.body.node("h1", |a| a.text(format!("{} (town: {}, bgm: {})", &name, town, bgm)));

		doc.body.node("div", |a| {
			a.indent();
			a.attr("id", "chcp");
			a.node("select", |a| {
				a.indent();
				a.attr("id", "ch");
				for ch in ch {
					a.node("option", |a| a.text(format!("{:?}", ch)));
				}
			});
			a.node("select", |a| {
				a.indent();
				a.attr("id", "cp");
				for cp in cp {
					a.node("option", |a| a.text(format!("{:?}", cp)));
				}
			});
		});

		doc.body.node("h2", |a| a.text("NPCs"));
		doc.body.node("ol", |a| {
			a.indent();
			a.attr("start", "0");
			for npc in npcs {
				a.node("li", |a| a.text(format!("{:?}", npc)));
			}
		});

		doc.body.node("h2", |a| a.text("Monsters"));
		doc.body.node("ol", |a| {
			a.indent();
			a.attr("start", npcs.len().to_string());
			for monster in monsters {
				a.node("li", |a| a.text(format!("{:?}", monster)));
			}
		});

		doc.body.node("h2", |a| a.text("Triggers"));
		doc.body.node("ol", |a| {
			a.indent();
			a.attr("start", "0");
			for trigger in triggers {
				a.node("li", |a| a.text(format!("{:?}", trigger)));
			}
		});

		doc.body.node("h2", |a| a.text("Object"));
		doc.body.node("ol", |a| {
			a.indent();
			a.attr("start", "0");
			for object in objects {
				a.node("li", |a| a.text(format!("{:?}", object)));
			}
		});

		doc.body.node("h2", |a| a.text("Camera angles (?)"));
		doc.body.node("ol", |a| {
			a.indent();
			a.attr("start", "0");
			for camera_angle in camera_angles {
				a.node("li", |a| a.text(format!("{:?}", camera_angle)));
			}
		});

		doc.body.node("h2", |a| a.text("Code"));
		for (i, func) in functions.iter().enumerate() {
			doc.body.node("h3", |a| a.text(format!("Function {}", i)));
			let render = RenderCode {
				raw: asm,
			};
			if asm {
				doc.body.node_class("pre", "code asm", |a| render.asm(a, func));
			} else {
				match decompile(func) {
					Ok(code) => {
						doc.body.node_class("pre", "code", |a| render.code(a, 0, &code));
					},
					Err(e) => {
						tracing::error!("{:?}", e);
						doc.body.node_class("div", "decompile-error", |a| {
							a.text("Decompilation failed. This is a bug.");
						});
						doc.body.node_class("pre", "code asm", |a| render.asm(a, func));
					},
				}
			}
		}
	})
}

#[extend::ext]
impl Node {
	fn node_class<A>(&mut self, name: &str, class: &str, body: impl FnOnce(&mut Node) -> A) -> A {
		self.node(name, |a| {
			a.class(class);
			body(a)
		})
	}

	fn span<A>(&mut self, class: &str, body: impl FnOnce(&mut Node) -> A) -> A {
		self.node_class("span", class, body)
	}

	fn span_text(&mut self, class: &str, text: impl ToString) {
		self.span(class, |a| a.text(text));
	}
}

struct RenderCode {
	raw: bool,
}

impl RenderCode {
	fn asm(&self, a: &mut Node, asm: &Asm) {
		let mut labels = BTreeSet::<usize>::new();
		for (_, insn) in &asm.code {
			match insn {
				FlowInsn::If(_, target) => {
					labels.insert(*target);
				}
				FlowInsn::Goto(target) => {
					labels.insert(*target);
				}
				FlowInsn::Switch(_, branches, default) => {
					labels.extend(branches.iter().map(|a| a.1));
					labels.insert(*default);
				}
				FlowInsn::Insn(_) => {}
			}
		}

		let labels: BTreeMap<usize, String> =
			labels.into_iter()
			.enumerate()
			.map(|(i, a)| (a, format!("L{}", i)))
			.collect();

		for (addr, insn) in &asm.code {
			if let Some(label) = labels.get(addr) {
				a.span_text("label", label);
				a.span_text("syntax", ":");
				a.text("\n");
			}
			a.text("  ");

			match insn {
				FlowInsn::If(expr, target) => {
					a.span_text("keyword", "UNLESS");
					a.text(" ");
					self.expr(a, expr);
					a.text(" ");
					a.span_text("keyword", "GOTO");
					a.text(" ");
					a.span_text("label", &labels[target]);
				}

				FlowInsn::Goto(target) => {
					a.span_text("keyword", "GOTO");
					a.text(" ");
					a.span_text("label", &labels[target]);
				}

				FlowInsn::Switch(expr, branches, default) => {
					a.span_text("keyword", "SWITCH");
					a.text(" ");
					self.expr(a, expr);
					a.text(" ");
					a.span_text("syntax", "[");
					for (case, target) in branches {
						a.span_text("case", case);
						a.span_text("syntax", ":");
						a.text(" ");
						a.span_text("label", &labels[target]);
						a.span_text("syntax", ",");
						a.text(" ");
					}
					a.span_text("keyword", "default");
					a.span_text("syntax", ":");
					a.text(" ");
					a.span_text("label", &labels[default]);
					a.span_text("syntax", "]");
				}

				FlowInsn::Insn(insn) => {
					self.insn(a, insn);
				}
			}
			a.text("\n");
		}
	}

	fn code(&self, a: &mut Node, indent: usize, code: &[Stmt]) {
		fn line<A>(a: &mut Node, indent: usize, body: impl FnOnce(&mut Node) -> A) -> A {
			for _ in 0..indent {
				a.span_text("indent", "\t");
			}
			let v = body(a);
			a.text("\n");
			v
		}
		if code.is_empty() {
			line(a, indent, |a| a.span_text("empty-block", "(empty)"));
		}
		for stmt in code {
			match stmt {
				Stmt::If(cases) => {
					line(a, indent, |a| {
						a.span_text("keyword", "IF");
					});
					for (expr, body) in cases {
						line(a, indent+1, |a| {
							match expr {
								Some(expr) => self.expr(a, expr),
								None => a.span_text("keyword", "ELSE"),
							}
							a.text(" ");
							a.span_text("syntax", "=>");
						});
						self.code(a, indent+2, body);
					}
				}

				Stmt::Switch(expr, cases) => {
					line(a, indent, |a| {
						a.span_text("keyword", "SWITCH");
						a.text(" ");
						self.expr(a, expr);
					});
					for (cases, body) in cases {
						line(a, indent+1, |a| {
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
						self.code(a, indent+2, body);
					}
				}

				Stmt::While(expr, body) => {
					line(a, indent, |a| {
						a.span_text("keyword", "WHILE");
						a.text(" ");
						self.expr(a, expr);
					});
					self.code(a, indent+1, body);
				}

				Stmt::Break => {
					line(a, indent, |a| {
						a.span_text("keyword", "BREAK");
					});
				}

				Stmt::Insn(insn) => {
					line(a, indent, |a| {
						self.insn(a, insn);
					});
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
				if prio2 < prio || self.raw { a.span_text("syntax", "("); }
				self.expr_inner(a, l, prio2);
				a.text(" ");
				a.span_text("expr-op", text);
				a.text(" ");
				self.expr_inner(a, r, prio2+1);
				if prio2 < prio || self.raw { a.span_text("syntax", ")"); }
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
				let mut r = self.visitor(a, "Flag");
				r.flag(flag);
			}
			Expr::Var(var) => {
				let mut r = self.visitor(a, "Var");
				r.var(var);
			}
			Expr::Attr(attr) => {
				let mut r = self.visitor(a, "Attr");
				r.attr(attr);
			}
			Expr::CharAttr(char, attr) => {
				let mut r = self.visitor(a, "CharAttr");
				r.char(char);
				r.char_attr(attr);
			},
			Expr::Rand => {
				self.visitor(a, "Rand");
			}
		}
	}

	fn visitor<'a, 'b>(&'a self, a: &'b mut Node, name: &'static str) -> InsnRenderer<'a, 'b> {
		a.span_text("insn", name);
		InsnRenderer { context: self, node: a }
	}

	fn insn(&self, a: &mut Node, insn: &Insn) {
		let mut vis = self.visitor(a, insn.name());
		insn.visit(&mut vis);
	}
}

struct InsnRenderer<'a, 'b> {
	context: &'a RenderCode,
	node: &'b mut Node,
}

// This might seem a little convoluted right now, but it's necessary for when I add proper traversal
impl InsnRenderer<'_, '_> {
}

impl InsnVisitor for InsnRenderer<'_, '_> {
	fn u8(&mut self, v: &u8) { self.node.text(" "); self.node.span_text("int", v); }
	fn u16(&mut self, v: &u16) { self.node.text(" "); self.node.span_text("int", v); }
	fn u32(&mut self, v: &u32) { self.node.text(" "); self.node.span_text("int", v); }

	fn i8(&mut self, v: &i8) { self.node.text(" "); self.node.span_text("int", v); }
	fn i16(&mut self, v: &i16) { self.node.text(" "); self.node.span_text("int", v); }
	fn i32(&mut self, v: &i32) { self.node.text(" "); self.node.span_text("int", v); }

	fn func_ref(&mut self, v: &FuncRef) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn file_ref(&mut self, v: &FileRef) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }

	fn pos2(&mut self, v: &Pos2) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn pos3(&mut self, v: &Pos3) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn relative(&mut self, v: &Pos3) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }

	fn time(&mut self, v: &u32) { self.node.text(" "); self.node.span_text("time", format!("{}ms", v)); }
	fn speed(&mut self, v: &u32) { self.node.text(" "); self.node.span_text("speed", format!("{}mm/s", v)); }
	fn angle(&mut self, v: &u16) { self.node.text(" "); self.node.span_text("angle", format!("{}°", v)); }
	fn color(&mut self, v: &u32) { self.node.text(" "); self.node.span_text("color", format!("#{:08X}", v)); }

	fn time16(&mut self, v: &u16) { self.node.text(" "); self.node.span_text("time", format!("{}ms", v)); }
	fn angle32(&mut self, v: &i32) { self.node.text(" "); self.node.span_text("angle", format!("{}m°", v)); }

	fn battle(&mut self, v: &u16) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn town(&mut self, v: &u16) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn bgmtbl(&mut self, v: &u8) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn quest(&mut self, v: &u16) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn sound(&mut self, v: &u16) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn item(&mut self, v: &u16) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn flag(&mut self, v: &u16) {
		self.node.text(" ");
		self.node.span_text("flag", v);
	}

	fn fork(&mut self, v: &[Insn]) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn expr(&mut self, v: &Expr) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn string(&mut self, v: &str) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn text(&mut self, v: &Text) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn menu(&mut self, v: &[String]) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn emote(&mut self, v: &(u8, u8, u32, u8)) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }

	fn flags(&mut self, v: &u32) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn quest_flag(&mut self, v: &u8) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn char_flags(&mut self, v: &u16) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn quest_task(&mut self, v: &u16) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn member(&mut self, v: &u8) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }

	fn var(&mut self, v: &u16) {
		self.node.text(" ");
		self.node.span_text("var", v);
	}

	fn attr(&mut self, v: &u8) {
		self.node.text(" ");
		self.node.span_text("attr", v);
	}

	fn char_attr(&mut self, v: &u8) {
		self.node.span_text("char-attr", format!(":{}", v));
	}


	fn char(&mut self, v: &u16) {
		self.node.text(" ");
		self.node.span_text("char", v);
	}

	fn chcp(&mut self, v: &u16) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn fork_id(&mut self, v: &u8) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn menu_id(&mut self, v: &u16) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
	fn object(&mut self, v: &u16) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }

	fn data(&mut self, v: &[u8]) { self.node.text(" "); self.node.span_text("unknown", format!("{:?}", v)); }
}
