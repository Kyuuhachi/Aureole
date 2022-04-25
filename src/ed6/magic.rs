use kaiseki::ed6::magic::*;

pub fn render(magics: &Vec<Magic<String>>) -> choubun::Node {
	choubun::document(|doc| {
		doc.root.attr("lang", "en");
		doc.head.node("title", |a| a.text("Arts list"));
		doc.head.node("style", |a| a.raw(r#"
			table { border-collapse: collapse; }
			tr { border-bottom: 1px solid #7F7F7F3F; }
			td { padding-left: 1ex; padding-right: 1ex; }
			[title] { text-decoration: underline dotted; }

			.el-none   { background-color: #7F7F7F3F; }
			.el-earth  { background-color: #CC88553F; }
			.el-water  { background-color: #3366FF3F; }
			.el-fire   { background-color: #FF33333F; }
			.el-wind   { background-color: #3399333F; }
			.el-time   { background-color: #6677883F; }
			.el-space  { background-color: #FFEE773F; }
			.el-mirage { background-color: #CCCCCC3F; }
		"#));

		doc.body.node("table", |a| {
			a.indent();
			for Magic {
				id, name, desc,
				flags, element,
				target, effect1, effect2,
				target_p1, target_p2,
				warmup, cooldown, cost, sort,
				effect_p1, effect_p2, effect_p3, effect_p4,
			} in magics {
				a.node("tr", |a| {
					a.indent();
					a.class(&format!("el-{}", element.to_string().to_lowercase()));

					a.node("td", |a| {
						a.class("name");
						a.node("small", |a| a.text(id.to_string()));
						a.text(" ");
						a.node("span", |a| {
							if !desc.trim().is_empty() {
								a.attr("title", desc.replace("\\n", "\n"));
							}
							a.text(name);
						});
					});

					a.node("td", |a| {
						a.node("span", |a| a.text(sort.to_string()));
					});

					a.node("td", |a| {
						a.class("target");
						a.text(format!("{target}({target_p1}, {target_p2})"));
					});

					if (*effect2, *effect_p3, *effect_p4) == (MagicEffect::None, 0, 0) {
						a.node("td", |a| {
							a.class("effect");
							a.attr("colspan", "2");
							a.text(format!("{effect1}({effect_p1}, {effect_p2})"));
						});
					} else if *effect2 == MagicEffect::None {
						a.node("td", |a| {
							a.class("effect");
							a.attr("colspan", "2");
							a.text(format!("{effect1}({effect_p1}, {effect_p2}, {effect_p3}, {effect_p4})"));
						});
					} else {
						a.node("td", |a| {
							a.class("effect");
							a.text(format!("{effect1}({effect_p1}, {effect_p2})"));
						});
						a.node("td", |a| {
							a.class("effect");
							a.text(format!("{effect2}({effect_p3}, {effect_p4})"));
						});
					}

					a.node("td", |a| {
						a.class("flags");
						for (i, c) in "涙本..敵死良友あい砲.う呪え.".chars().enumerate() {
							let i = match MagicFlags::from_bits(1<<i) {
								Some(i) => i,
								None => continue
							};
							if flags.contains(i) {
								a.node("span", |a| {
									a.attr("title", format!("{:?}", i));
									a.text(format!("{}", c));
								});
							} else {
								a.text("・");
							}
						}
					});

					a.node("td", |a| {
						a.class("cost");
						if *cost == 0 {
							// empty
						} else if flags.contains(MagicFlags::Magic) {
							a.text(format!("{} EP", cost));
						} else {
							a.text(format!("{} CP", cost));
						}
					});

					a.node("td", |a| {
						a.class("time");
						if *warmup != 0 {
							a.text(format!("{warmup}+{cooldown} T"));
						} else if *cooldown != 0 {
							a.text(format!("{cooldown} T"));
						}
					});
				});
			}
		});
	})
}
