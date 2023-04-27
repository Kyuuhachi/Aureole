#![feature(let_chains)]
#![feature(decl_macro)]

use themelios_common::{types, util};
use gospel::read::{Reader, Le as _};
use themelios_common::util::*;

pub mod insn;

fn strings(f: &mut Reader) -> Result<Vec<String>, ReadError> {
	let mut strings = Vec::new();
	loop {
		let s = f.string()?;
		if s.is_empty() {
			break;
		}
		strings.push(s);
	}
	Ok(strings)
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Addr(pub usize);

themelios_common::impl_from_into!(Addr(usize));
impl std::fmt::Debug for Addr {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "@{:04X}", &self.0)
	}
}

pub mod ed6 {
	use std::collections::BTreeSet;

	use themelios_common::util::*;
	use gospel::read::{Reader, Le as _};
	use crate::types::*;
	use crate::Addr;

	pub struct Ani {
		pub chips: Vec<(FileId, FileId)>,
		pub models: Vec<String>,
		pub bones: Option<(u8, Vec<String>)>,
		pub sprite_offsets: [(u8,u8); 8],
		pub funcs: Vec<Addr>,
		pub insns: Vec<(Addr, crate::insn::Insn)>,
	}

	pub fn read_monster(game: Game, data: &[u8]) -> Result<Ani, ReadError> {
		let mut f = Reader::new(data);
		let f_func_table = f.ptr16()?;
		let f_sprite_offsets = f.ptr16()?;
		let f_bones = f.ptr16()?;

		let mut chips = Vec::new();
		loop {
			match f.u32()? {
				0xFFFFFFFF => break,
				a => chips.push((FileId(a), FileId(f.u32()?)))
			}
		}

		let models = super::strings(&mut f)?;

		let bones = if f_bones.pos() != 0 {
			ensure!(f.pos() == f_bones.pos());
			let x = f.u8()?;
			let bones = super::strings(&mut f)?;
			Some((x, bones))
		} else { None };

		let mut funcs = Vec::new();
		ensure!(f.pos() == f_func_table.pos());
		while f.pos() < f_sprite_offsets.pos() {
			funcs.push(Addr(f.u16()? as usize));
		}

		ensure!(f.pos() == f_sprite_offsets.pos());
		let sprite_offsets = array::<8, _>(|| Ok((f.u8()?, f.u8()?))).strict()?;

		let mut insns = Vec::new();
		while !f.is_empty() {
			let p = f.pos();
			let i = crate::insn::Insn::read(&mut f, game)?;
			insns.push((Addr(p), i));
		}

		let mut xs = BTreeSet::from_iter(funcs.iter().copied());
		for i in &insns {
			macro run {
				([$(($ident:ident $(($_n:ident $($ty:tt)*))*))*]) => {
					match &i.1 {
						$(crate::insn::Insn::$ident($($_n),*) => {
							$(run!($_n $($ty)*);)*
						})*
					}
				},
				($v:ident Addr) => { xs.insert(*$v) },
				($i:ident $($t:tt)*) => {}
			}
			crate::insn::introspect!(run);
		}

		println!("chips: {:?}", chips);
		println!("bones: {:?}", bones);
		println!("offsets: {:?}", sprite_offsets);
		println!("funcs: {:?}", funcs);
		for (i, p) in &insns {
			if xs.remove(i) {
				print!("  {i:?} ");
			} else {
				print!("        ");
			}
			println!("{p:?}");
		}

		if !xs.is_empty() {
			println!("ERROR: {xs:?}");
		}

		Ok(Ani {
			chips,
			models,
			bones,
			sprite_offsets,
			funcs,
			insns,
		})
	}

	#[test]
	fn test() -> Result<(), Box<dyn std::error::Error>> {
		let mut i = std::fs::read_dir("../data/fc.extract/10/")?.collect::<Result<Vec<_>, _>>()?;
		i.sort_by_key(|a| a.path());
		println!("FC");
		for file in i {
			let p = file.path();
			let n = p.file_name().unwrap().to_str().unwrap();
			if n.starts_with("as") && !n.starts_with("asmag") && !n.starts_with("asitem") {
				println!("\n{n}");
				read_monster(Game::Fc, &std::fs::read(p)?)?;
			}
		}

		println!("SC");
		let mut i = std::fs::read_dir("../data/sc.extract/30/")?.collect::<Result<Vec<_>, _>>()?;
		i.sort_by_key(|a| a.path());
		for file in i {
			let p = file.path();
			let n = p.file_name().unwrap().to_str().unwrap();
			if n.starts_with("as") && !n.starts_with("asmag") && !n.starts_with("asitem") {
				println!("\n{n}");
				read_monster(Game::Sc, &std::fs::read(p)?)?;
			}
		}

		println!("3rd");
		let mut i = std::fs::read_dir("../data/3rd.extract/30/")?.collect::<Result<Vec<_>, _>>()?;
		i.sort_by_key(|a| a.path());
		for file in i {
			let p = file.path();
			let n = p.file_name().unwrap().to_str().unwrap();
			if n.starts_with("as") && !n.starts_with("asmag") && !n.starts_with("asitem") {
				println!("\n{n}");
				read_monster(Game::Tc, &std::fs::read(p)?)?;
			}
		}

		Ok(())
	}
}
