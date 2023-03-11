use hamu::read::le::*;
use hamu::write::le::*;
use crate::types::*;

#[doc(inline)]
pub use themelios_scena::code;
pub mod ed6;
pub mod ed7;

pub mod decompile;

trait ReadStreamExt2: ReadStream {
	fn pos2(&mut self) -> Result<Pos2, Self::Error> {
		Ok(Pos2(self.i32()?, self.i32()?))
	}

	fn pos3(&mut self) -> Result<Pos3, Self::Error> {
		Ok(Pos3(self.i32()?, self.i32()?, self.i32()?))
	}
}
impl<T: ReadStream> ReadStreamExt2 for T {}

trait WriteStreamExt2: WriteStream {
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
impl<T: WriteStream> WriteStreamExt2 for T {}
