use choubun::Node;
use kaiseki::ed6::{scena::*, code::*};

pub fn render(Scena {
	dir, fname, town, bgm,
	entry_func,
	includes,
	ch, cp,
	npcs, monsters, triggers, objects,
	camera_angles,
	code,
}: &Scena) -> choubun::Node {
	choubun::document(|doc| {
		doc.root.attr("lang", "en");
		let name = format!("{}/{}", dir.decode(), fname.decode());
		doc.head.node("title", |a| a.text(&name));

		doc.body.node("h1", |a| a.text(format!("{} (town: {}, bgm: {})", &name, town, bgm)));

		doc.body.node("div", |a| {
			a.indent();
			a.attr("id", "chcp");
			a.node("select", |a| {
				a.indent();
				a.attr("id", "ch");
				for ch in ch {
					a.node("option", |a| a.text(format!("{:?}", ch)))
				}
			});
			a.node("select", |a| {
				a.indent();
				a.attr("id", "cp");
				for cp in cp {
					a.node("option", |a| a.text(format!("{:?}", cp)))
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
		for (i, func) in code.iter().enumerate() {
			doc.body.node("h3", |a| a.text(format!("Function {}", i)));
			match decompile(func) {
				Ok(code) => {
					doc.body.node("pre", |a| {
						render_code(a, 0, &code);
					});
				},
				Err(_) => todo!(),
			}
		}
	})
}

#[extend::ext]
impl Node {
	fn span<A>(&mut self, class: &str, body: impl FnOnce(&mut Node) -> A) -> A {
		self.node("span", |a| {
			a.class(class);
			body(a)
		})
	}

	fn span_text(&mut self, class: &str, text: impl ToString) {
		self.span(class, |a| a.text(text))
	}

	fn line<A>(&mut self, indent: usize, body: impl FnOnce(&mut Node) -> A) -> A {
		for _ in 0..indent {
			self.span_text("indent", "\t");
		}
		let v = body(self);
		self.text("\n");
		v
	}
}


fn render_code(a: &mut Node, indent: usize, code: &[Stmt]) {
	for stmt in code {
		a.node("span", |a| {
			a.class("stmt");
			match stmt {
				Stmt::If(cases) => {
					a.line(indent, |a| {
						a.span_text("keyword", "IF");
					});
					for (expr, body) in cases {
						a.line(indent+1, |a| {
							match expr {
								Some(expr) => render_expr(a, expr),
								None => a.span_text("keyword", "ELSE"),
							}
							a.text(" ");
							a.span_text("keyword", "=>");
						});
						render_code(a, indent+2, body);
					}
				}

				Stmt::Switch(expr, cases) => {
					a.line(indent, |a| {
						a.span_text("keyword", "SWITCH");
						a.text(" ");
						render_expr(a, expr);
					});
					for (cases, body) in cases {
						a.line(indent+1, |a| {
							a.span_text("case", format!("{:?}", cases));
							a.text(" ");
							a.span_text("keyword", "=>");
						});
						render_code(a, indent+2, body);
					}
				}

				Stmt::While(expr, body) => {
					a.line(indent, |a| {
						a.span_text("keyword", "WHILE");
						a.text(" ");
						render_expr(a, expr);
					});
					render_code(a, indent+1, body);
				}

				Stmt::Break => {
					a.line(indent, |a| {
						a.span_text("keyword", "BREAK");
					});
				}

				Stmt::Insn(insn) => {
					a.line(indent, |a| {
						a.span("insn", |a| {
							a.text(format!("{:?}", insn));
						});
					});
				}
			}
		});
	}
}

fn render_expr(a: &mut Node, expr: &Expr) {
	a.span("expr", |a| {
		a.text(format!("{:?}", expr));
	});
}
