mod ed6 {
	use std::path::Path;
	use themelios::scena::code::InstructionSet;
	use themelios::scena::code::decompile::fixup_eddec;
	use crate::util::*;
	use themelios::gamedata::{Lookup, GameData};

	macro_rules! test {
		($a:item) => {
			#[test_case::test_case(InstructionSet::Fc,    &*FC, Strict, "../data/fc.extract/01/", "._sn"; "fc")]
			#[test_case::test_case(InstructionSet::FcEvo, &*FC, Strict, "../data/vita/extract/fc/gamedata/data/data/scenario/0/", ".bin"; "fc_evo")]
			#[test_case::test_case(InstructionSet::Sc,    &*SC, Strict, "../data/sc.extract/21/", "._sn"; "sc")]
			#[test_case::test_case(InstructionSet::ScEvo, &*SC, Strict, "../data/vita/extract/sc/gamedata/data/data_sc/scenario/1/", ".bin"; "sc_evo")]
			#[test_case::test_case(InstructionSet::Tc,    &*TC, Strict, "../data/3rd.extract/21/", "._sn"; "tc")]
			#[test_case::test_case(InstructionSet::TcEvo, &*TC, Strict, "../data/vita/extract/3rd/gamedata/data/data_3rd/scenario/2/", ".bin"; "tc_evo")]
			$a
		}
	}

	test! {
	#[test_case::test_case(InstructionSet::Fc,    &*FC, Lenient, "../data/fc-voice/scena/", "._SN"; "fc_voice")]
	#[test_case::test_case(InstructionSet::Sc,    &*SC, Lenient, "../data/sc-voice/scena/", "._SN"; "sc_voice")]
	#[test_case::test_case(InstructionSet::Tc,    &*TC, Lenient, "../data/3rd-voice/scena/", "._SN"; "tc_voice")]
	fn roundtrip(iset: InstructionSet, lookup: &dyn Lookup, strict: Strictness, scenapath: &str, suffix: &str) -> Result<(), Error> {
		let game = GameData { iset, lookup, kai: false };
		let mut failed = false;

		let mut paths = std::fs::read_dir(scenapath)?
			.map(|r| r.unwrap())
			.collect::<Vec<_>>();
		paths.sort_by_key(|dir| dir.path());

		for file in paths {
			let path = file.path();
			let name = path.file_name().unwrap().to_str().unwrap();
			if !name.ends_with(suffix) {
				continue
			}

			let data = std::fs::read(&path)?;

			if let Err(err) = check_roundtrip(strict, &data, |a| themelios::scena::ed6::read(&game, a), |a| themelios::scena::ed6::write(&game, a)) {
				println!("{name}: {err:?}");
				failed = true;
			};
		}

		assert!(!failed);
		Ok(())
	}
	}

	test! {
	fn decompile(iset: InstructionSet, lookup: &dyn Lookup, _strict: Strictness, scenapath: &str, suffix: &str) -> Result<(), Error> {
		let game = GameData { iset, lookup, kai: false };
		let mut failed = false;

		let mut paths = std::fs::read_dir(scenapath)?
			.map(|r| r.unwrap())
			.collect::<Vec<_>>();
		paths.sort_by_key(|dir| dir.path());

		for file in paths {
			let path = file.path();
			let name = path.file_name().unwrap().to_str().unwrap();
			if !name.ends_with(suffix) {
				continue
			}

			let data = std::fs::read(&path)?;

			let scena = themelios::scena::ed6::read(&game, &data)?;
			for (i, func) in scena.functions.iter().enumerate() {
				let decomp = themelios::scena::code::decompile::decompile(func).map_err(|e| anyhow::anyhow!("{name}:{i}: {e}"))?;
				let recomp = themelios::scena::code::decompile::recompile(&decomp).map_err(|e| anyhow::anyhow!("{name}:{i}: {e}"))?;
				if &recomp != func {
					println!("{name}:{i}: incorrect recompile");
					//
					// let mut ctx = super::text::Context::new().blind();
					// ctx.indent += 1;
					// super::text::flat_func(&mut ctx, func);
					// print!("{}", ctx.output);
					// println!("\n======\n");
					//
					// let mut ctx = super::text::Context::new().blind();
					// ctx.indent += 1;
					// super::text::tree_func(&mut ctx, &decomp);
					// print!("{}", ctx.output);
					// println!("\n======\n");
					//
					// let mut ctx = super::text::Context::new().blind();
					// ctx.indent += 1;
					// super::text::flat_func(&mut ctx, &recomp);
					// println!("{}", ctx.output);

					failed = true;
				}
			}
		}

		assert!(!failed);

		Ok(())
	}
	}

	#[test_case::test_case(InstructionSet::Fc, &*FC, "../data/fc.extract/01/", "../data/fc-voice/scena/";  "fc")]
	#[test_case::test_case(InstructionSet::Sc, &*SC, "../data/sc.extract/21/", "../data/sc-voice/scena/";  "sc")]
	#[test_case::test_case(InstructionSet::Tc, &*TC, "../data/3rd.extract/21/","../data/3rd-voice/scena/"; "tc")]
	fn eddec(iset: InstructionSet, lookup: &dyn Lookup, vanilla: impl AsRef<Path>, voice: impl AsRef<Path>) -> Result<(), Error> {
		let game = GameData { iset, lookup, kai: false };
		let mut failed = false;

		let mut paths = std::fs::read_dir(voice)?
			.map(|r| r.unwrap())
			.collect::<Vec<_>>();
		paths.sort_by_key(|dir| dir.path());
		for file in paths {
			let vpath = file.path();
			let vname = vpath.file_name().unwrap().to_str().unwrap();
			let name = vname.replace(' ', "").to_lowercase();
			let path = vanilla.as_ref().join(&name);

			if !path.exists() {
				println!("{} does not exist (from {})", path.display(), vpath.display());
				continue;
			}
			let data = std::fs::read(path)?;
			let vdata = std::fs::read(vpath)?;
			let scena = themelios::scena::ed6::read(&game, &data)?;
			let vscena = match themelios::scena::ed6::read(&game, &vdata) {
				Ok(a) => a,
				Err(err) =>  {
					println!("{name}: {err:?}");
					failed = true;
					continue;
				}
			};

			for (i, (func, vfunc)) in scena.functions.iter().zip(vscena.functions.iter()).enumerate() {
				if let Some(vfunc2) = fixup_eddec(func, vfunc) {
					let decomp = themelios::scena::code::decompile::decompile(&vfunc2).map_err(|e| anyhow::anyhow!("{name}:{i}: {e}"));
					let decomp = match decomp {
						Ok(d) => d,
						Err(e) => {
							println!("{name}:{i}: failed to decompile: {e}");
							// let mut ctx = super::text::Context::new().blind();
							// ctx.indent += 1;
							// super::text::flat_func(&mut ctx, func);
							// print!("{}", ctx.output);
							// println!("\n======\n");
							//
							// let mut ctx = super::text::Context::new().blind();
							// ctx.indent += 1;
							// super::text::flat_func(&mut ctx, vfunc);
							// print!("{}", ctx.output);
							// println!("\n======\n");
							//
							// let mut ctx = super::text::Context::new().blind();
							// ctx.indent += 1;
							// super::text::flat_func(&mut ctx, &vfunc2);
							// print!("{}", ctx.output);
							continue
						}
					};
					let recomp = themelios::scena::code::decompile::recompile(&decomp).map_err(|e| anyhow::anyhow!("{name}:{i}: {e}"))?;
					if recomp != vfunc2 {
						println!("{name}:{i}: incorrect recompile");

						// let mut ctx = super::text::Context::new().blind();
						// ctx.indent += 1;
						// super::text::flat_func(&mut ctx, func);
						// print!("{}", ctx.output);
						// println!("\n======\n");
						//
						// let mut ctx = super::text::Context::new().blind();
						// ctx.indent += 1;
						// super::text::flat_func(&mut ctx, &vfunc2);
						// print!("{}", ctx.output);
						// println!("\n======\n");
						//
						// let mut ctx = super::text::Context::new().blind();
						// ctx.indent += 1;
						// super::text::flat_func(&mut ctx, &recomp);
						// println!("{}", ctx.output);

						failed = true;
					}
				}
			}
		}

		assert!(!failed);

		Ok(())
	}
}

mod ed7 {
	use crate::util::*;
	use themelios::gamedata::GameData;

	macro_rules! test {
		($a:item) => {
			#[test_case::test_case(&GameData::ZERO, Lenient, &[], "../data/zero-gf/data/scena", ".bin"; "zero_gf_jp")]
			#[test_case::test_case(&GameData::ZERO, Lenient, &[], "../data/zero-gf/data_en/scena", ".bin"; "zero_gf_en")]
			#[test_case::test_case(&GameData::ZERO_KAI, Strict, &["c1440.bin"], "../data/zero/data/scena", ".bin"; "zero_nisa_jp")]
			#[test_case::test_case(&GameData::ZERO_KAI, Strict, &[], "../data/zero/data/scena_us", ".bin"; "zero_nisa_en")]
			#[test_case::test_case(&GameData::ZERO_EVO, Strict, &["c1440.bin"], "../data/vita/extract/zero/data1/data/scena", ".bin"; "zero_evo")]
			#[test_case::test_case(&GameData::AO, Strict, &[], "../data/ao-psp/PSP_GAME/USRDIR/data/scena", ".bin"; "ao_psp")]
			#[test_case::test_case(&GameData::AO_EVO, Strict, &[], "../data/vita/extract/ao/data1/data/scena", ".bin"; "ao_evo")]
			#[test_case::test_case(&GameData::AO, Lenient, &[], "../data/ao-gf/data_en/scena", ".bin"; "ao_gf_en")]
			$a
		}
	}

	test! {
	fn roundtrip(game: &GameData, strict: Strictness, except: &[&str], scenapath: &str, suffix: &str) -> Result<(), Error> {
		let mut failed = false;

		let mut paths = std::fs::read_dir(scenapath)?
			.map(|r| r.unwrap())
			.collect::<Vec<_>>();
		paths.sort_by_key(|dir| dir.path());

		for file in paths {
			let path = file.path();
			let name = path.file_name().unwrap().to_str().unwrap();
			if !name.ends_with(suffix) {
				continue
			}

			let data = std::fs::read(&path)?;

			let strict = match (strict, except.iter().any(|a| *a == name)) {
				(Strict, true) => Lenient,
				(Lenient, true) => Strict,
				(a, _) => a,
			};

			if let Err(err) = check_roundtrip(strict, &data, |a| themelios::scena::ed7::read(game, a), |a| themelios::scena::ed7::write(game, a)) {
				println!("{name}: {err:?}");
				failed = true;
			}
		}

		assert!(!failed);
		Ok(())
	}
	}
}
