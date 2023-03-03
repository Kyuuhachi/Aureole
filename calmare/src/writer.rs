use std::io::{Write, Result};

use themelios::types::Game;
use themelios_archive::lookup::{Lookup, ED7Lookup};

#[derive(Clone, Copy, Debug)]
enum Space {
	None,
	Space,
	Newline,
}

pub struct Context<'a> {
	pub game: Game,
	pub decompile: bool, //  but then I'd have to reexport all the writing functions and that's a pain
	indent: usize,
	space: Space,
	pub lookup: &'a dyn Lookup,
	out: Box<dyn Write + 'a>,
}

impl<'a> Context<'a> {
	pub fn new(game: Game, lookup: Option<&'a dyn Lookup>, out: impl Write + 'a) -> Self {
		Self {
			game,
			decompile: true,
			indent: 0,
			space: Space::None,
			lookup: lookup.unwrap_or_else(|| default_lookup(game)),
			out: Box::new(out),
		}
	}

	pub fn flat(mut self) -> Self {
		self.decompile = false;
		self
	}
}

fn default_lookup(game: Game) -> &'static dyn Lookup {
	use Game::*;
	use themelios_archive_prebuilt as pb;
	match game {
		Fc | FcKai => &*pb::FC,
		FcEvo => &*pb::FC_EVO,
		Sc | ScKai => &*pb::SC,
		ScEvo => &*pb::SC_EVO,
		Tc | TcKai => &*pb::TC,
		TcEvo => &*pb::TC_EVO,
		Zero | ZeroEvo | ZeroKai |
		Ao | AoEvo | AoKai => &ED7Lookup
	}
}

impl<'a> Context<'a> {
	fn put_space(&mut self) -> Result<()> {
		match self.space {
			Space::None => {}
			Space::Space => {
				write!(&mut self.out, " ")?;
			}
			Space::Newline => {
				for _ in 0..self.indent {
					write!(&mut self.out, "\t")?;
				}
			}
		}
		self.space = Space::None;
		Ok(())
	}

	pub fn space(&mut self) -> Result<&mut Self> {
		// Cannot fail, but let's Result it for consistency.
		self.space = Space::Space;
		Ok(self)
	}

	pub fn no_space(&mut self) -> Result<&mut Self> {
		self.space = Space::None;
		Ok(self)
	}

	pub fn kw(&mut self, arg: &str) -> Result<&mut Self> {
		self.put_space()?;
		write!(&mut self.out, "{arg}")?;
		self.space()?;
		Ok(self)
	}

	pub fn pre(&mut self, arg: &str) -> Result<&mut Self> {
		self.put_space()?;
		write!(&mut self.out, "{arg}")?;
		Ok(self)
	}

	pub fn suf(&mut self, arg: &str) -> Result<&mut Self> {
		write!(&mut self.out, "{arg}")?;
		self.space()?;
		Ok(self)
	}

	pub fn line(&mut self) -> Result<&mut Self> {
		writeln!(&mut self.out)?;
		self.space = Space::Newline;
		Ok(self)
	}

	pub fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> Result<()> {
		self.put_space()?;
		self.out.write_fmt(args)
	}

	pub fn indent<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> T {
		self.indent += 1;
		let v = f(self);
		self.indent -= 1;
		v
	}
}
