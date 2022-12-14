#![cfg(test)]
#![feature(error_generic_member_access, provide_any)]

mod util;
mod scena;

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

	mod quest {
		use crate::util::test::*;

		#[test_case::test_case(&FC; "fc")]
		fn roundtrip(arc: &crate::archive::Archives) -> Result<(), Error> {
			check_roundtrip(&arc.get_decomp("t_quest._dt").unwrap(), super::read, super::write)?;
			Ok(())
		}
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
