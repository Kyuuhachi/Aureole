use std::time::{Instant, Duration};

use themelios::{archive::Archives, gamedata::GameData, scena::code::InstructionSet};

pub use anyhow::Error;

// Sync wrapper. It's not safe at all.
pub struct SW<T>(T);

impl <T> std::ops::Deref for SW<T> {
	type Target = T;

	fn deref(&self) -> &T {
		&self.0
	}
}

// SAFETY: None.
unsafe impl <T> Sync for SW<T> {}

lazy_static::lazy_static! {
	pub static ref FC: Archives = Archives::new("../data/fc").unwrap();
	pub static ref SC: Archives = Archives::new("../data/sc").unwrap();
	pub static ref TC: Archives = Archives::new("../data/3rd").unwrap();

	pub static ref GD_FC: SW<GameData<'static>> = SW(GameData { iset: InstructionSet::Fc, lookup: &*FC, kai: false });
	pub static ref GD_SC: SW<GameData<'static>> = SW(GameData { iset: InstructionSet::Sc, lookup: &*SC, kai: false });
	pub static ref GD_TC: SW<GameData<'static>> = SW(GameData { iset: InstructionSet::Tc, lookup: &*TC, kai: false });

	pub static ref GD_FC_EVO: SW<GameData<'static>> = SW(GameData { iset: InstructionSet::FcEvo, lookup: &*FC, kai: false });
	pub static ref GD_SC_EVO: SW<GameData<'static>> = SW(GameData { iset: InstructionSet::ScEvo, lookup: &*SC, kai: false });
	pub static ref GD_TC_EVO: SW<GameData<'static>> = SW(GameData { iset: InstructionSet::TcEvo, lookup: &*TC, kai: false });
}

pub fn check_equal<T: PartialEq + std::fmt::Debug>(a: &T, b: &T) -> Result<(), Error> {
	if a != b {
		let a = format!("{:#?}", a);
		let b = format!("{:#?}", b);
		let diff = similar::TextDiff::configure().diff_lines(&a, &b);

		for (i, hunk) in diff.unified_diff().iter_hunks().enumerate() {
			if i > 0 {
				println!("\x1B[34m…\x1B[39m");
			}
			for change in hunk.iter_changes() {
				match change.tag() {
					similar::ChangeTag::Delete => print!("\x1B[31m-{change}\x1B[39m"),
					similar::ChangeTag::Insert => print!("\x1B[32m+{change}\x1B[39m"),
					similar::ChangeTag::Equal => print!(" {change}"),
				};
			}
		}
		return Err(anyhow::anyhow!("{} differs", std::any::type_name::<T>()))
	}
	Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strictness {
	Lenient,
	Strict,
}
pub use Strictness::*;

pub fn check_roundtrip<T, RE, WE>(
	strict: Strictness,
	data: &[u8],
	read: impl Fn(&[u8]) -> Result<T, RE>,
	write: impl Fn(&T) -> Result<Vec<u8>, WE>,
) -> Result<T, anyhow::Error> where
	T: PartialEq + std::fmt::Debug,
	anyhow::Error: From<RE>,
	anyhow::Error: From<WE>,
{
	let val = read(data)?;
	let data2 = write(&val)?;
	if data != data2 {
		let val2 = read(&data2)?;
		check_equal(&val, &val2)?;
		if strict == Lenient {
			return Ok(val)
		}

		let deadline = Instant::now() + Duration::from_secs(1);
		let diff = similar::capture_diff_slices_deadline(similar::Algorithm::Patience, data, &data2, Some(deadline));

		for chunk in diff {
			match chunk {
				similar::DiffOp::Equal { old_index, new_index, len } => {
					println!(
						"{:04X?} = {:04X?}",
						old_index..old_index+len,
						new_index..new_index+len,
					);
				}
				similar::DiffOp::Delete { old_index, old_len, new_index } => {
					println!(
						"{:04X?} ⇒ {:04X?}..---- ({:02X?} ⇒ [])",
						old_index..old_index+old_len,
						new_index,
						&data[old_index..old_index+old_len],
					);
				}
				similar::DiffOp::Insert { old_index, new_index, new_len } => {
					println!(
						"{:04X?}..---- ⇐ {:04X?} ([] ⇐ {:02X?})",
						old_index,
						new_index..new_index+new_len,
						&data2[new_index..new_index+new_len],
					);
				}
				similar::DiffOp::Replace { old_index, old_len, new_index, new_len } => {
					println!(
						"{:04X?} ≠ {:04X?} ({:02X?} ≠ {:02X?})",
						old_index..old_index+old_len,
						new_index..new_index+new_len,
						&data[old_index..old_index+old_len],
						&data2[new_index..new_index+new_len],
					);
				}
			}
		}
		return Err(anyhow::anyhow!("{} bytes differ", std::any::type_name::<T>()).into())
	}
	Ok(val)
}
