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

type Backtrace = Box<std::backtrace::Backtrace>;

#[derive(Debug, thiserror::Error)]
pub enum LookupError {
	#[error("failed to look up {name:?}")]
	Name { name: String, backtrace: Backtrace },

	#[error("failed to look up {index:08X}")]
	Index { index: u32, backtrace: Backtrace },
}

impl std::convert::From<&str> for LookupError {
	fn from(name: &str) -> Self {
		Self::Name {
			name: name.into(),
			backtrace: std::backtrace::Backtrace::capture().into(),
		}
	}
}

impl std::convert::From<u32> for LookupError {
	fn from(index: u32) -> Self {
		Self::Index {
			index,
			backtrace: std::backtrace::Backtrace::capture().into(),
		}
	}
}

pub trait GameDataImpl {
	fn name(&self, index: u32) -> Result<&str, LookupError>;
	fn index(&self, name: &str) -> Result<u32, LookupError>;
}

impl GameDataImpl for Vec<Box<dyn GameDataImpl>> {
	fn name(&self, index: u32) -> Result<&str, LookupError> {
		self.iter()
			.find_map(|a| a.name(index).ok())
			.ok_or_else(|| index.into())
	}

	fn index(&self, name: &str) -> Result<u32, LookupError> {
		self.iter()
			.find_map(|a| a.index(name).ok())
			.ok_or_else(|| name.into())
	}
}

impl<A, B> GameDataImpl for (A, B) where A: GameDataImpl, B: GameDataImpl {
	fn name(&self, index: u32) -> Result<&str, LookupError> {
		self.0.name(index).or_else(|_| self.1.name(index))
	}

	fn index(&self, name: &str) -> Result<u32, LookupError> {
		self.0.index(name).or_else(|_| self.1.index(name))
	}
}

pub struct SkyGameData<T: GameDataImpl>(pub u16, pub T);

impl<T: GameDataImpl> GameDataImpl for SkyGameData<T> {
	fn name(&self, index: u32) -> Result<&str, LookupError> {
		if index >> 16 == self.0 as u32 {
			self.1.name(index & 0xFFFF)
		} else {
			Err(index.into())
		}
	}

	fn index(&self, name: &str) -> Result<u32, LookupError> {
		Ok(self.1.index(name)? | ((self.0 as u32) << 16))
	}
}
