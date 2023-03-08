use themelios::types::Game;
use themelios_archive::lookup::Lookup;

#[derive(Clone, Copy, Debug)]
enum Space {
	None,
	Space,
	Newline,
}

pub struct Context<'a> {
	pub game: Game,
	pub decompile: bool,
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
			indent: 0,
			space: Space::None,
			lookup: lookup.unwrap_or_else(|| crate::util::default_lookup(game)),
			out: String::new(),
		}
	}

	pub fn flat(mut self) -> Self {
		self.decompile = false;
		self
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
