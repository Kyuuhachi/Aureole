use themelios::types::Game;
use themelios::lookup::Lookup;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Space {
	None,
	Space,
	Newline,
}

pub struct Context<'a> {
	pub game: Game,
	pub decompile: bool,
	pub has_warn: bool,
	indent: usize,
	space: Space,
	pub lookup: &'a dyn Lookup,
	out: String,
}

impl<'a> Context<'a> {
	pub fn new(game: Game, lookup: Option<&'a dyn Lookup>) -> Self {
		Self {
			game,
			decompile: true,
			has_warn: false,
			indent: 0,
			space: Space::None,
			lookup: lookup.unwrap_or_else(|| themelios::lookup::default_for(game)),
			out: String::new(),
		}
	}

	pub fn flat(mut self) -> Self {
		self.decompile = false;
		self
	}

	pub fn warn(&mut self) {
		self.has_warn = true;
	}

	pub fn finish(self) -> String {
		self.out
	}
}

impl<'a> Context<'a> {
	fn put_space(&mut self) {
		match self.space {
			Space::None => {}
			Space::Space => {
				self.out.push(' ');
			}
			Space::Newline => {
				for _ in 0..self.indent {
					self.out.push('\t');
				}
			}
		}
		self.space = Space::None;
	}

	pub fn space(&mut self) -> &mut Self {
		self.space = Space::Space;
		self
	}

	pub fn no_space(&mut self) -> &mut Self {
		self.space = Space::None;
		self
	}

	pub fn kw(&mut self, arg: &str) -> &mut Self {
		self.put_space();
		self.out.push_str(arg);
		self.space();
		self
	}

	pub fn pre(&mut self, arg: &str) -> &mut Self {
		self.put_space();
		self.out.push_str(arg);
		self
	}

	pub fn suf(&mut self, arg: &str) -> &mut Self {
		self.out.push_str(arg);
		self.space();
		self
	}

	pub fn line(&mut self) -> &mut Self {
		self.out.push('\n');
		self.space = Space::Newline;
		self
	}

	pub fn is_line(&self) -> bool {
		self.space == Space::Newline
	}

	pub fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) {
		self.put_space();
		std::fmt::Write::write_fmt(&mut self.out, args).unwrap();
	}

	pub fn indent<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
		self.indent += 1;
		let v = f(self);
		self.indent -= 1;
		v
	}
}
