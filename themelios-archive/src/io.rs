use hamu::write::le::*;
use hamu::read::le::*;
use crate::lookup::ED6Lookup;

impl ED6Lookup {
	pub fn read(data: &[u8]) -> Result<Self, hamu::read::Error> {
		let mut f = Reader::new(data);
		f.check(b"ED6I")?;
		let mut x = [(); 64].map(|_| Vec::new());
		for i in x.iter_mut() {
			let n = f.u16()?;
			i.reserve(n as usize);
			for _ in 0..n {
				let len = f.u8()? as usize;
				let a = f.error_state();
				let s = f.vec(len)?;
				let s = String::from_utf8(s)
					.map_err(|e| Reader::to_error(a, Box::new(e)))?;
				i.push(s);
			}
		}
		Ok(Self::new(x))
	}

	pub fn write(&self) -> Result<Vec<u8>, hamu::write::Error> {
		let mut f = Writer::new();
		f.slice(b"ED6I");
		for i in &self.name {
			f.u16(i.len() as u16);
			for s in i {
				f.u8(s.len() as u8);
				f.slice(s.as_bytes());
			}
		}
		f.finish()
	}
}
