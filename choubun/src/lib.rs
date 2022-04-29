use std::fmt;

use linked_hash_map::LinkedHashMap;

#[derive(Clone)]
enum Item {
	Node(Node),
	Leaf(Leaf),
	Text(String),
	Raw(String),
}

impl fmt::Debug for Item {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Node(v) => { fmt::Debug::fmt(v, f)?; }
			Self::Leaf(v) => { fmt::Debug::fmt(v, f)?; }
			Self::Text(v) => { write!(f, "Text(")?;fmt::Debug::fmt(v, f)?; write!(f, ")")?; }
			Self::Raw(v)  => { write!(f, "Raw(")?; fmt::Debug::fmt(v, f)?; write!(f, ")")?; }
		}
		Ok(())
	}
}

#[derive(Debug, Clone)]
pub struct Leaf {
	name: String,
	attrs: LinkedHashMap<String, String>,
}

#[derive(Clone, Default)]
pub struct Body(Vec<Item>);

impl fmt::Debug for Body {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(&self.0, f)
	}
}

#[derive(Clone)]
pub struct Node {
	leaf: Leaf,
	indent: bool,
	body: Body,
}

impl fmt::Debug for Node {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Node")
			.field("name", &self.leaf.name)
			.field("attrs", &self.leaf.attrs)
			.field("indent", &self.indent)
			.field("body", &self.body)
			.finish()
	}
}

impl Leaf {
	fn new(name: &str) -> Leaf {
		Leaf {
			name: name.to_owned(),
			attrs: LinkedHashMap::new(),
		}
	}

	pub fn attr(&mut self, name: &str, value: impl ToString) {
		self.attrs.insert(name.to_owned(), value.to_string());
	}

	pub fn class(&mut self, class: &str) {
		if let Some(v) = self.attrs.get_mut("class") {
			v.push(' ');
			v.push_str(class);
		} else {
			self.attr("class", class);
		}
	}
}

impl Node {
	fn new(name: &str) -> Node {
		Node {
			indent: false,
			leaf: Leaf::new(name),
			body: Body::default(),
		}
	}

	pub fn indent(&mut self) {
		self.indent = true;
	}

	pub fn attr(&mut self, name: &str, value: impl ToString) {
		self.leaf.attr(name, value);
	}

	pub fn class(&mut self, class: &str) {
		self.leaf.class(class);
	}

	pub fn node(&mut self, name: &str, body: impl FnOnce(&mut Node)) {
		self.body.node(name, body)
	}

	pub fn leaf(&mut self, name: &str, body: impl FnOnce(&mut Leaf)) {
		self.body.leaf(name, body)
	}

	pub fn text(&mut self, text: impl ToString) {
		self.body.text(text)
	}

	pub fn raw(&mut self, text: &str) {
		self.body.raw(text)
	}
}

impl Body {
	pub fn node(&mut self, name: &str, body: impl FnOnce(&mut Node)) {
		let mut node = Node::new(name);
		body(&mut node);
		self.0.push(Item::Node(node));
	}

	pub fn leaf(&mut self, name: &str, body: impl FnOnce(&mut Leaf)) {
		let mut node = Leaf::new(name);
		body(&mut node);
		self.0.push(Item::Leaf(node));
	}

	pub fn text(&mut self, text: impl ToString) {
		self.0.push(Item::Text(text.to_string()));
	}

	pub fn raw(&mut self, text: &str) {
		self.0.push(Item::Raw(text.to_owned()));
	}
}

pub fn node(name: &str, body: impl FnOnce(&mut Node)) -> Node {
	let mut node = Node::new(name);
	body(&mut node);
	node
}

#[derive(Debug, Clone)]
pub struct Document {
	pub root: Leaf,
	pub head: Node,
	pub body: Node,
}

impl Document {
	fn new() -> Document {
		Document {
			root: Leaf::new("html"),
			head: node("head", |a| {
				a.indent();
				a.leaf("meta", |a| a.attr("charset", "utf-8"));
			}),
			body: node("body", |a| {
				a.indent();
			}),
		}
	}

	fn into_node(self) -> Node {
		Node {
			leaf: self.root,
			indent: true,
			body: Body(vec![
				Item::Node(self.head),
				Item::Node(self.body),
			]),
		}
	}
}

pub fn document(body: impl FnOnce(&mut Document)) -> Node {
	let mut doc = Document::new();
	body(&mut doc);
	doc.into_node()
}

impl Node {
	pub fn render(&self, out: &mut impl fmt::Write) -> fmt::Result {
		writeln!(out, "<!DOCTYPE html>")?;
		self.render_fragment(out, 0)
	}

	pub fn render_to_string(&self) -> String {
		let mut out = String::new();
		self.render(&mut out).unwrap();
		out
	}

	pub fn render_fragment(&self, out: &mut impl fmt::Write, indent: usize) -> fmt::Result {
		self.leaf.render_fragment(out, false)?;
		self.body.render_fragment(out, self.indent, indent+1)?;
		if self.indent {
			write!(out, "\n{}", "\t".repeat(indent))?;
		}
		write!(out, "</{}>", self.leaf.name)?;
		Ok(())
	}
}

impl Leaf {
	fn render_fragment(&self, out: &mut impl fmt::Write, slash: bool) -> fmt::Result {
		write!(out, "<{}", self.name)?;
		for (k, v) in &self.attrs {
			write!(out, " {k}=\"")?;
			escape(out, v)?;
			write!(out, "\"")?;
		}
		if slash {
			write!(out, " />")?;
		} else {
			write!(out, ">")?;
		}
		Ok(())
	}
}

impl Body {
	fn render_fragment<W: fmt::Write>(&self, out: &mut W, do_indent: bool, indent: usize) -> fmt::Result {
		for item in &self.0 {
			if do_indent {
				write!(out, "\n{}", "\t".repeat(indent))?;
			}
			match item {
				Item::Node(v) => v.render_fragment(out, indent)?,
				Item::Leaf(v) => v.render_fragment(out, true)?,
				Item::Text(v) => escape(out, v)?,
				Item::Raw(v)  => write!(out, "{}", v)?,
			}
		}
		Ok(())
	}
}

fn escape<W: fmt::Write>(out: &mut W, str: &str) -> fmt::Result {
	for c in str.chars() {
		match c {
			'&' => write!(out, "&amp;")?,
			'<' => write!(out, "&lt;")?,
			'>' => write!(out, "&gt;")?,
			'"' => write!(out, "&quot;")?,
			c => write!(out, "{}", c)?,
		}
	}
	Ok(())
}
