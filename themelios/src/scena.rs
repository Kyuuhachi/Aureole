use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _};
use crate::types::*;

#[doc(inline)]
pub use themelios_scena::code;
pub mod ed6;
pub mod ed7;

pub mod decompile;

#[extend::ext(name = ReaderExt)]
impl Reader<'_> {
	fn pos2(&mut self) -> Result<Pos2, gospel::read::Error> {
		Ok(Pos2(self.i32()?, self.i32()?))
	}

	fn pos3(&mut self) -> Result<Pos3, gospel::read::Error> {
		Ok(Pos3(self.i32()?, self.i32()?, self.i32()?))
	}
}

#[extend::ext]
impl Writer {
	fn pos2(&mut self, p: Pos2) {
		self.i32(p.0);
		self.i32(p.1);
	}

	fn pos3(&mut self, p: Pos3) {
		self.i32(p.0);
		self.i32(p.1);
		self.i32(p.2);
	}
}
