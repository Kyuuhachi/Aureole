//! Provides conversion between 32-bit file ids and filenames.
pub use themelios_archive::lookup::*;

#[cfg(feature = "indexes")]
/// Returns the default [`Lookup`] for the given game.
///
/// This should be all that's needed unless any mods add new files to the archives.
pub fn default_for(game: crate::types::Game) -> &'static dyn Lookup {
	use std::io::Read;
	use zstd::dict::DecoderDictionary as DD;

	fn load(bytes: &[u8]) -> ED6Lookup {
		let mut dec = zstd::Decoder::with_prepared_dictionary(bytes, &DICT).unwrap();
		let mut data = Vec::new();
		dec.read_to_end(&mut data).unwrap();
		ED6Lookup::read_ed6i(&data).unwrap()
	}

	lazy_static::lazy_static! {
		static ref DICT: DD<'static> = DD::copy(include_bytes!("../index/dict"));
		pub static ref FC:     ED6Lookup = load(include_bytes!("../index/fc.ed6i.zst"));
		pub static ref SC:     ED6Lookup = load(include_bytes!("../index/sc.ed6i.zst"));
		pub static ref TC:     ED6Lookup = load(include_bytes!("../index/3rd.ed6i.zst"));
		pub static ref FC_EVO: ED6Lookup = load(include_bytes!("../index/fc-evo.ed6i.zst"));
		pub static ref SC_EVO: ED6Lookup = load(include_bytes!("../index/sc-evo.ed6i.zst"));
		pub static ref TC_EVO: ED6Lookup = load(include_bytes!("../index/3rd-evo.ed6i.zst"));
	}

	use crate::types::Game::*;
	match game {
		Fc | FcKai => &*FC,
		FcEvo => &*FC_EVO,
		Sc | ScKai => &*SC,
		ScEvo => &*SC_EVO,
		Tc | TcKai => &*TC,
		TcEvo => &*TC_EVO,
		Zero | ZeroEvo | ZeroKai |
		Ao | AoEvo | AoKai => &ED7Lookup
	}
}
