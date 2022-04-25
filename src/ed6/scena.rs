use std::collections::{BTreeSet, BTreeMap};

use choubun::Node;
use kaiseki::ed6::{scena::*, code::*};

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
			.syntax { color: purple }
			.label { color: green }
			.keyword { font-weight: bold; }
			.code { tab-size: 4; }
			.indent::before { position: absolute; content: "⟩"; opacity: 0.25; }
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
			if asm {
				doc.body.node_class("pre", "code asm", |a| render_asm(a, func));
			} else {
				match decompile(func) {
					Ok(code) => {
						doc.body.node_class("pre", "code", |a| render_code(a, 0, &code));
					},
					Err(e) => {
						tracing::error!("{:?}", e);
						doc.body.node_class("div", "decompile-error", |a| {
							a.text("Decompilation failed. This is a bug.");
						});
						doc.body.node_class("pre", "code asm", |a| render_asm(a, func));
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

fn render_asm(a: &mut Node, asm: &Asm) {
	let labels = find_labels(asm);
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
				render_expr(a, expr);
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
				render_expr(a, expr);
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
				a.span("insn", |a| {
					a.text(format!("{:?}", insn));
				});
			}
		}
		a.text("\n");
	}
}

fn find_labels(asm: &Asm) -> BTreeSet<usize> {
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
	labels
}

fn render_code(a: &mut Node, indent: usize, code: &[Stmt]) {
	fn line<A>(a: &mut Node, indent: usize, body: impl FnOnce(&mut Node) -> A) -> A {
		for _ in 0..indent {
			a.span_text("indent", "\t");
		}
		let v = body(a);
		a.text("\n");
		v
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
							Some(expr) => render_expr(a, expr),
							None => a.span_text("keyword", "ELSE"),
						}
						a.text(" ");
						a.span_text("syntax", "=>");
					});
					render_code(a, indent+2, body);
				}
			}

			Stmt::Switch(expr, cases) => {
				line(a, indent, |a| {
					a.span_text("keyword", "SWITCH");
					a.text(" ");
					render_expr(a, expr);
				});
				for (cases, body) in cases {
					line(a, indent+1, |a| {
						a.span_text("case", format!("{:?}", cases));
						a.text(" ");
						a.span_text("syntax", "=>");
					});
					render_code(a, indent+2, body);
				}
			}

			Stmt::While(expr, body) => {
				line(a, indent, |a| {
					a.span_text("keyword", "WHILE");
					a.text(" ");
					render_expr(a, expr);
				});
				render_code(a, indent+1, body);
			}

			Stmt::Break => {
				line(a, indent, |a| {
					a.span_text("keyword", "BREAK");
				});
			}

			Stmt::Insn(insn) => {
				line(a, indent, |a| {
					a.span("insn", |a| {
						a.text(format!("{:?}", insn));
					});
				});
			}
		}
	}
}

fn render_expr(a: &mut Node, expr: &Expr) {
	a.span("expr", |a| {
		a.text(format!("{:?}", expr));
	});
}