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
			doc.body.node("pre", |a| {
				for (addr, op) in func {
					a.text(format!("{} {:?}\n", addr, op));
				}
			});
		}
	})
}

