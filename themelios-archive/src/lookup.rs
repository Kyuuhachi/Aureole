//! Contains everything related to file id lookups.
//!
//! Everything here is reexported at the crate root, so there's little reason to interact with this module directly.
mod ed6;
pub use ed6::ED6Lookup;

/// The main lookup trait.
pub trait Lookup {
	/// Given a file id, return the corresponding filename, if any.
	fn name(&self, index: u32) -> Option<String>;
	/// Given a filename, return the corresponding file id, if any.
	fn index(&self, name: &str) -> Option<u32>;
}

/// A dummy object that does not perform any lookups.
#[derive(Debug, Clone)]
pub struct NullLookup;

impl Lookup for NullLookup {
	fn name(&self, _index: u32) -> Option<String> {
		None
	}

	fn index(&self, _name: &str) -> Option<u32> {
		None
	}
}

/// Lookup for ED7: *Trails from Zero* and *Trails to Azure*.
///
/// These games encode the filenames directly into the file ids, so this does not require any additional data.
#[derive(Debug, Clone)]
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
