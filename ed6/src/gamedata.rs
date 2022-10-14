use std::io;

#[derive(derive_more::Deref)]
pub struct GameData {
	#[deref]
	data: Box<dyn GameDataImpl + Sync>,
	insn_set: InstructionSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstructionSet {
	Fc, FcEvo,
	Sc, ScEvo,
	Tc, TcEvo, // It's called 3rd, I know, but that's not a valid identifier
	Zero, ZeroEvo,
	Azure, AzureEvo,
}

impl GameData {
	pub fn new(data: impl GameDataImpl + Sync + 'static, insn_set: InstructionSet) -> Self {
		Self { data: Box::new(data), insn_set }
	}

	pub fn insn_set(&self) -> InstructionSet {
		self.insn_set
	}
}

pub trait GameDataImpl {
	fn name(&self, index: u32) -> io::Result<&str>;
	fn index(&self, name: &str) -> io::Result<u32>;
	fn get(&self, name: &str) -> io::Result<&[u8]>;
	// This is not a great abstraction, but it's the best I can come up with right now
	fn get_decomp(&self, name: &str) -> io::Result<Vec<u8>>;
	fn list(&self) -> Box<dyn Iterator<Item=&str> + '_>;
}

impl GameDataImpl for Vec<Box<dyn GameDataImpl>> {
	fn name(&self, index: u32) -> io::Result<&str> {
		self.iter()
			.find_map(|a| a.name(index).ok())
			.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("{index:08X}")))
	}

	fn index(&self, name: &str) -> io::Result<u32> {
		self.iter()
			.find_map(|a| a.index(name).ok())
			.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, name.to_owned()))
	}

	fn get(&self, name: &str) -> io::Result<&[u8]> {
		self.iter()
			.find_map(|a| a.get(name).ok())
			.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, name.to_owned()))
	}

	fn get_decomp(&self, name: &str) -> io::Result<Vec<u8>> {
		self.iter()
			.find_map(|a| a.get_decomp(name).ok())
			.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, name.to_owned()))
	}

	fn list(&self) -> Box<dyn Iterator<Item=&str> + '_> {
		let mut seen = std::collections::HashSet::new();
		Box::new(
			self.iter()
			.flat_map(|a| a.list())
			.filter(move |a| seen.insert(*a))
		)
	}
}

impl<A, B> GameDataImpl for (A, B) where A: GameDataImpl, B: GameDataImpl {
	fn name(&self, index: u32) -> io::Result<&str> {
		self.0.name(index).or_else(|_| self.1.name(index))
	}

	fn index(&self, name: &str) -> io::Result<u32> {
		self.0.index(name).or_else(|_| self.1.index(name))
	}

	fn get(&self, name: &str) -> io::Result<&[u8]> {
		self.0.get(name).or_else(|_| self.1.get(name))
	}

	fn get_decomp(&self, name: &str) -> io::Result<Vec<u8>> {
		self.0.get_decomp(name).or_else(|_| self.1.get_decomp(name))
	}

	fn list(&self) -> Box<dyn Iterator<Item=&str> + '_> {
		let mut seen = std::collections::HashSet::new();
		Box::new(self.0.list().chain(self.1.list()).filter(move |a| seen.insert(*a)))
	}
}

pub struct SkyGameData<T: GameDataImpl>(pub u16, pub T);

impl<T: GameDataImpl> GameDataImpl for SkyGameData<T> {
	fn name(&self, index: u32) -> io::Result<&str> {
		if index >> 16 == self.0 as u32 {
			self.1.name(index & 0xFFFF)
		} else {
			Err(io::ErrorKind::NotFound.into())
		}
	}

	fn index(&self, name: &str) -> io::Result<u32> {
		Ok(self.1.index(name)? | ((self.0 as u32) << 16))
	}

	fn get(&self, name: &str) -> io::Result<&[u8]> {
		self.1.get(name)
	}

	fn get_decomp(&self, name: &str) -> io::Result<Vec<u8>> {
		self.1.get_decomp(name)
	}

	fn list(&self) -> Box<dyn Iterator<Item=&str> + '_> {
		self.1.list()
	}
}
