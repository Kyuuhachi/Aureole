#![cfg(test)]
#![feature(error_generic_member_access, provide_any)]

mod util;
mod scena;

use std::path::Path;

use themelios::gamedata::GameData;
use themelios::tables;
use util::*;

#[cfg(feature="not_used")]
mod todo {
	#[macro_export]
	macro_rules! __simple_roundtrip {
		($name:literal) => {
			#[test_case::test_case(&$crate::util::test::FC; "fc")]
			fn roundtrip(arc: &$crate::archive::Archives) -> Result<(), $crate::util::test::Error> {
				$crate::util::test::check_roundtrip_strict(
					&arc.get_decomp($name).unwrap(),
					super::read,
					|a| super::write(a),
				)?;
				Ok(())
			}
		};
	}
	pub use __simple_roundtrip as simple_roundtrip;

	#[macro_export]
	macro_rules! __simple_roundtrip_arc {
		($name:literal) => {
			#[test_case::test_case(&$crate::util::test::FC; "fc")]
			fn roundtrip(arc: &$crate::archive::Archives) -> Result<(), $crate::util::test::Error> {
				$crate::util::test::check_roundtrip_strict(
					&arc.get_decomp($name).unwrap(),
					|a| super::read(arc, a),
					|a| super::write(arc, a),
				)?;
				Ok(())
			}
		};
	}
	pub use __simple_roundtrip_arc as simple_roundtrip_arc;

	mod cook2 {
		crate::util::test::simple_roundtrip!("t_cook2._dt");
	}

	mod item {
		use crate::util::test::*;

		#[test_case::test_case(&FC; "fc")]
		fn roundtrip(arc: &crate::archive::Archives) -> Result<(), Error> {
			let t_item = arc.get_decomp("t_item._dt").unwrap();
			let t_item2 = arc.get_decomp("t_item2._dt").unwrap();
			let items = super::read(&t_item, &t_item2)?;
			let (t_item_, t_item2_) = super::write(&items)?;
			let items2 = super::read(&t_item_, &t_item2_)?;
			check_equal(&items, &items2)?;
			Ok(())
		}
	}

	mod name {
		crate::util::test::simple_roundtrip_arc!("t_name._dt");
	}

	mod orb {
		crate::util::test::simple_roundtrip!("t_orb._dt");
	}

	mod bgmtbl {
		use crate::util::test::*;

		#[test_case::test_case(&FC; "fc")]
		fn roundtrip(arc: &crate::archive::Archives) -> Result<(), Error> {
			check_roundtrip(&arc.get_decomp("t_bgmtbl._dt").unwrap(), super::read, super::write)?;
			Ok(())
		}
	}

	mod status {
		crate::util::test::simple_roundtrip!("t_status._dt");
	}

	mod exp {
		crate::util::test::simple_roundtrip!("t_exp._dt");
	}

	mod face {
		crate::util::test::simple_roundtrip_arc!("t_face._dt");
	}

	mod town {
		use crate::util::test::*;

		#[test_case::test_case(&FC; "fc")]
		fn roundtrip(arc: &crate::archive::Archives) -> Result<(), Error> {
			check_roundtrip(&arc.get_decomp("t_town._dt").unwrap(), super::read, |a| super::write(a))?;
			Ok(())
		}
	}

	mod btlset {
		use crate::util::test::*;

		#[test_case::test_case(&FC; "fc")]
		fn parse(arc: &crate::archive::Archives) -> Result<(), Error> {
			let data = arc.get_decomp("t_btlset._dt").unwrap();
			let _parsed = super::read(arc, &data)?;
			Ok(())
		}
	}

	mod se {
		crate::util::test::simple_roundtrip_arc!("t_se._dt");
	}

	mod world {
		crate::util::test::simple_roundtrip_arc!("t_world._dt");
	}
}


#[test_case::test_case(&GD_FC, "../data/fc.extract/02/t_quest._dt"; "fc")]
#[test_case::test_case(&GD_SC, "../data/sc.extract/22/t_quest._dt"; "sc")]
#[test_case::test_case(&GD_SC, "../data/sc.extract/22/t_quest1._dt"; "sc1")]
#[test_case::test_case(&GD_SC, "../data/sc.extract/22/t_quest2._dt"; "sc2")]
#[test_case::test_case(&GD_TC, "../data/3rd.extract/22/t_quest._dt"; "tc")]

#[test_case::test_case(&GD_FC_EVO, "../data/vita/extract/fc/gamedata/data/data/text/t_quest._dt"; "fc_evo")]
#[test_case::test_case(&GD_SC_EVO, "../data/vita/extract/sc/gamedata/data/data_sc/text/t_quest._dt"; "sc_evo")]
#[test_case::test_case(&GD_SC_EVO, "../data/vita/extract/sc/gamedata/data/data_sc/text/t_quest1._dt"; "sc_evo1")]
#[test_case::test_case(&GD_SC_EVO, "../data/vita/extract/sc/gamedata/data/data_sc/text/t_quest2._dt"; "sc_evo2")]
#[test_case::test_case(&GD_TC_EVO, "../data/vita/extract/3rd/gamedata/data/data_3rd/text/t_quest._dt"; "tc_evo")]

fn quest_ed6(game: &GameData, path: impl AsRef<Path>) -> Result<(), Error> {
	check_roundtrip(
		Lenient,
		&std::fs::read(path)?,
		|a| tables::quest::read_ed6(game, a),
		|a| tables::quest::write_ed6(game, a),
	)?;
	Ok(())
}

#[test_case::test_case(GameData::ZERO,     "../data/zero-gf/data/text/t_quest._dt"; "zero_gf_jp")]
#[test_case::test_case(GameData::ZERO,     "../data/zero-gf/data_en/text/t_quest._dt"; "zero_gf_en")]
#[test_case::test_case(GameData::ZERO_KAI, "../data/zero/data/text/t_quest._dt"; "zero_nisa_jp")]
#[test_case::test_case(GameData::ZERO_KAI, "../data/zero/data/text_us/t_quest._dt"; "zero_nisa_en")]
#[test_case::test_case(GameData::ZERO_EVO, "../data/vita/extract/zero/data/data/text/t_quest._dt"; "zero_evo")]

#[test_case::test_case(GameData::AO,     "../data/ao-psp/PSP_GAME/USRDIR/data/text/t_quest._dt"; "ao_psp")]
// #[test_case::test_case(GameData::AO,     "../data/ao-gf/data/text/t_quest._dt"; "ao_gf_cn")]
#[test_case::test_case(GameData::AO,     "../data/ao-gf/data_en/text/t_quest._dt"; "ao_gf_en")]
#[test_case::test_case(GameData::AO_EVO, "../data/vita/extract/ao/data/data/text/t_quest._dt"; "ao_evo")]

fn quest_ed7(game: &GameData, path: impl AsRef<Path>) -> Result<(), Error> {
	check_roundtrip(
		Lenient,
		&std::fs::read(path)?,
		|a| tables::quest::read_ed7(game, a),
		|a| tables::quest::write_ed7(game, a),
	)?;
	Ok(())
}


#[test_case::test_case(GameData::ZERO,     Lenient, "../data/zero-gf/data/text/t_name._dt"; "zero_gf_jp")]
#[test_case::test_case(GameData::ZERO,     Strict, "../data/zero-gf/data_en/text/t_name._dt"; "zero_gf_en")]
#[test_case::test_case(GameData::ZERO_KAI, Strict, "../data/zero/data/text/t_name._dt"; "zero_nisa_jp")]
#[test_case::test_case(GameData::ZERO_KAI, Strict, "../data/zero/data/text_us/t_name._dt"; "zero_nisa_en")]
#[test_case::test_case(GameData::ZERO_EVO, Strict, "../data/vita/extract/zero/data/data/text/t_name._dt"; "zero_evo")]

#[test_case::test_case(GameData::AO,     Strict, "../data/ao-psp/PSP_GAME/USRDIR/data/text/t_name._dt"; "ao_psp")]
// #[test_case::test_case(GameData::AO,     Strict, "../data/ao-gf/data/text/t_name._dt"; "ao_gf_cn")]
#[test_case::test_case(GameData::AO,     Strict, "../data/ao-gf/data_en/text/t_name._dt"; "ao_gf_en")]
#[test_case::test_case(GameData::AO_EVO, Strict, "../data/vita/extract/ao/data/data/text/t_name._dt"; "ao_evo")]

fn name_ed7(game: &GameData, strict: Strictness, path: impl AsRef<Path>) -> Result<(), Error> {
	check_roundtrip(
		strict,
		&std::fs::read(path)?,
		|a| tables::name::read_ed7(game, a),
		|a| tables::name::write_ed7(game, a),
	)?;
	Ok(())
}
