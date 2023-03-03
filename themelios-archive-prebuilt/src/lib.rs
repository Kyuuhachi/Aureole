use std::io::Read;

use themelios_archive::ED6Lookup;

fn load(bytes: &[u8]) -> ED6Lookup {
	let mut dec = zstd::Decoder::with_prepared_dictionary(bytes, &DICT).unwrap();
	let mut data = Vec::new();
	dec.read_to_end(&mut data).unwrap();
	ED6Lookup::read_ed6i(&data).unwrap()
}

use zstd::dict::DecoderDictionary as DD;
lazy_static::lazy_static! {
	static ref DICT: DD<'static> = DD::copy(include_bytes!("../index/dict"));
	pub static ref FC:     ED6Lookup = load(include_bytes!("../index/fc.ed6i.zst"));
	pub static ref SC:     ED6Lookup = load(include_bytes!("../index/sc.ed6i.zst"));
	pub static ref TC:     ED6Lookup = load(include_bytes!("../index/3rd.ed6i.zst"));
	pub static ref FC_EVO: ED6Lookup = load(include_bytes!("../index/fc-evo.ed6i.zst"));
	pub static ref SC_EVO: ED6Lookup = load(include_bytes!("../index/sc-evo.ed6i.zst"));
	pub static ref TC_EVO: ED6Lookup = load(include_bytes!("../index/3rd-evo.ed6i.zst"));
}
