use std::collections::HashMap;

pub trait Lookup {
	fn name(&self, index: u32) -> Option<String>;
	fn index(&self, name: &str) -> Option<u32>;
}

#[derive(Debug, Clone, Copy)]
pub struct NullLookup;

impl Lookup for NullLookup {
	fn name(&self, _index: u32) -> Option<String> {
		None
	}

	fn index(&self, _name: &str) -> Option<u32> {
		None
	}
}

#[derive(Clone)]
pub struct ED6Lookup {
	name: [Vec<String>; 64],
	index: HashMap<String, u32>,
}

impl ED6Lookup {
	pub fn new(name: [Vec<String>; 64]) -> Self {
		let mut index = HashMap::new();
		for (n, x) in name.iter().enumerate() {
			for (i, v) in x.iter().enumerate() {
				index.insert(v.clone(), (n << 16) as u32 | i as u32);
			}
		}
		Self { name, index }
	}
}

impl std::fmt::Debug for ED6Lookup {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let mut d = f.debug_struct("ED6Lookup");
		for (n, x) in self.name.iter().enumerate() {
			if !x.is_empty() {
				d.field(&format!("ED6_DT{n:02X}"), &format_args!("{} entries", x.len()));
			}
		}
		d.finish()
	}
}

impl Lookup for ED6Lookup {
	fn name(&self, index: u32) -> Option<String> {
		let (arch, index) = (index >> 16, index & 0xFFFF);
		Some(self.name.get(arch as usize)?.get(index as usize)?.clone())
	}

	fn index(&self, name: &str) -> Option<u32> {
		Some(*self.index.get(name)?)
	}
}

#[derive(Debug, Clone, Copy)]
pub struct ED7Lookup;

impl Lookup for ED7Lookup {
	fn name(&self, index: u32) -> Option<String> {
		match (index & 0xFF000000) >> 24 {
			0x00 => {
				let a = (index & 0xF00000) >> 20;
				let b = index & 0x0FFFFF;
				let prefix = match a {
					7 => "chr",
					8 => "apl",
					9 => "monster",
					_ => return None,
				};
				Some(format!("{prefix}/ch{b:05x}.itc"))
			}

			0x21 => {
				let a = (index & 0xF00000) >> 20;
				let b = (index & 0x0FFFF0) >> 4;
				let c = index & 0x00000F;
				let prefix = "0atcrmeb".chars().nth(a as usize)?;
				if c == 0 {
					Some(format!("scena/{prefix}{b:04x}.bin"))
				} else {
					Some(format!("scena/{prefix}{b:04x}_{c:01x}.bin"))
				}
			}

			0x30 => {
				let a = (index & 0xF00000) >> 20;
				let b = index & 0x0FFFFF;
				let prefix = match a {
					0 => "ms",
					1 => "as",
					2 => "bs",
					_ => return None,
				};
				Some(format!("{prefix}{b:05x}.dat"))
			}

			_ => None,
		}
	}

	fn index(&self, name: &str) -> Option<u32> {
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
			let a = "0atcrmeb".chars().position(|x| x == a)? as u32;
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

		None
	}
}

