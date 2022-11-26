type Backtrace = Box<std::backtrace::Backtrace>;

#[derive(thiserror::Error)]
pub enum LookupError {
	#[error("failed to look up {name:?}")]
	Name { name: String, backtrace: Backtrace },

	#[error("failed to look up 0x{index:08X}")]
	Index { index: u32, backtrace: Backtrace },
}

impl std::fmt::Debug for LookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Name { name, backtrace } => f.debug_struct("Name").field("name", name).field("backtrace", backtrace).finish(),
            Self::Index { index, backtrace } => f.debug_struct("Index").field("index", &format_args!("0x{:08X}", index)).field("backtrace", backtrace).finish(),
        }
    }
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

pub trait Lookup {
	fn name(&self, index: u32) -> Result<String, LookupError>;
	fn index(&self, name: &str) -> Result<u32, LookupError>;
}

impl Lookup for Vec<Box<dyn Lookup>> {
	fn name(&self, index: u32) -> Result<String, LookupError> {
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

impl<A, B> Lookup for (A, B) where A: Lookup, B: Lookup {
	fn name(&self, index: u32) -> Result<String, LookupError> {
		self.0.name(index).or_else(|_| self.1.name(index))
	}

	fn index(&self, name: &str) -> Result<u32, LookupError> {
		self.0.index(name).or_else(|_| self.1.index(name))
	}
}

pub struct SkyGameData<T: Lookup>(pub u16, pub T);

impl<T: Lookup> Lookup for SkyGameData<T> {
	fn name(&self, index: u32) -> Result<String, LookupError> {
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

impl Lookup for crate::archive::Archives {
	fn name(&self, a: u32) -> Result<String, LookupError> {
		self.name(a).map(str::to_owned).ok_or_else(|| a.into())
	}

	fn index(&self, name: &str) -> Result<u32, LookupError> {
		self.index(name).ok_or_else(|| name.into())
	}
}

#[derive(Debug, Clone, Copy)]
pub struct ED7Lookup;

impl Lookup for ED7Lookup {
	fn name(&self, index: u32) -> Result<String, LookupError> {
		match (index & 0xFF000000) >> 24 {
			0x00 => {
				let a = (index & 0xF00000) >> 20;
				let b = index & 0x0FFFFF;
				let prefix = match a {
					7 => "chr",
					8 => "apl",
					9 => "monster",
					_ => return Err(index.into())
				};
				Ok(format!("{prefix}/ch{b:05x}.itc"))
			}

			0x21 => {
				let a = (index & 0xF00000) >> 20;
				let b = (index & 0x0FFFF0) >> 4;
				let c = index & 0x00000F;
				let prefix = "0atcrme".chars().nth(a as usize).ok_or(index)?;
				if c == 0 {
					Ok(format!("scena/{prefix}{b:04x}.bin"))
				} else {
					Ok(format!("scena/{prefix}{b:04x}_{c:01x}.bin"))
				}
			}

			0x30 => {
				let a = (index & 0xF00000) >> 20;
				let b = index & 0x0FFFFF;
				let prefix = match a {
					0 => "ms",
					1 => "as",
					2 => "bs",
					_ => return Err(index.into())
				};
				Ok(format!("battle/dat/{prefix}{b:05x}.dat"))
			}

			_ => Err(index.into())
		}
	}

	fn index(&self, name: &str) -> Result<u32, LookupError> {
		fn inner(name: &str) -> Option<u32> {
			if let Some(name) = name.strip_prefix("chr/ch") {
				let name = name.strip_suffix(".itc")?;
				let b = u32::from_str_radix(name, 16).ok()?;
				return Some(0x00700000 | b)
			}
			if let Some(name) = name.strip_prefix("apl/ch") {
				let name = name.strip_suffix(".itc")?;
				let b = u32::from_str_radix(name, 16).ok()?;
				return Some(0x00800000 | b)
			}
			if let Some(name) = name.strip_prefix("monster/ch") {
				let name = name.strip_suffix(".itc")?;
				let b = u32::from_str_radix(name, 16).ok()?;
				return Some(0x00900000 | b)
			}

			if let Some(name) = name.strip_prefix("scena/") {
				let name = name.strip_suffix(".bin")?;
				let mut iter = name.chars();
				let a = iter.next()?;
				let name = iter.as_str();
				let a = "0atcrme".chars().position(|x| x == a)? as u32;
				let (b, c) = if let Some((b, c)) = name.split_once('_') {
					let b = u32::from_str_radix(b, 16).ok()?;
					let c = u32::from_str_radix(c, 16).ok()?;
					(b, c)
				} else {
					let b = u32::from_str_radix(name, 16).ok()?;
					(b, 0)
				};
				return Some(0x21000000 | a << 20 | b << 4 | c)
			}

			if let Some(name) = name.strip_prefix("battle/dat/") {
				if let Some(name) = name.strip_prefix("ms") {
					let name = name.strip_suffix(".dat")?;
					let b = u32::from_str_radix(name, 16).ok()?;
					return Some(0x30000000 | b)
				}
				if let Some(name) = name.strip_prefix("as") {
					let name = name.strip_suffix(".dat")?;
					let b = u32::from_str_radix(name, 16).ok()?;
					return Some(0x30100000 | b)
				}
				if let Some(name) = name.strip_prefix("bs") {
					let name = name.strip_suffix(".dat")?;
					let b = u32::from_str_radix(name, 16).ok()?;
					return Some(0x30200000 | b)
				}
			}

			None
		}
		inner(name).ok_or_else(|| name.into())
	}
}
