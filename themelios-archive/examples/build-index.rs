use std::{path::PathBuf, fs::File, io::Write};

use themelios_archive::ED6Lookup;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let x = vec![
		("fc",      ED6Lookup::for_pc("./data/fc")?),
		("sc",      ED6Lookup::for_pc("./data/sc")?),
		("3rd",     ED6Lookup::for_pc("./data/3rd")?),
		("fc-evo",  ED6Lookup::for_vita("./data/fc-evo/data")?),
		("sc-evo",  ED6Lookup::for_vita("./data/sc-evo/data_sc")?),
		("3rd-evo", ED6Lookup::for_vita("./data/3rd-evo/data_3rd")?),
	];

	let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("index");

	let names = x.iter().flat_map(|a| a.1.names()).flatten().collect::<Vec<_>>();
	let dict = zstd::dict::from_samples(&names, 256)?;
	File::create(dir.join("dict"))?.write_all(&dict)?;
	println!("dict");
	let dict = zstd::dict::EncoderDictionary::copy(&dict, -21);
	for (name, val) in x {
		let ed6i = val.write_ed6i()?;
		let mut file = File::create(dir.join(format!("{name}.ed6i.zst")))?;
		let mut enc = zstd::stream::Encoder::with_prepared_dictionary(&mut file, &dict)?;
		enc.set_pledged_src_size(Some(ed6i.len() as u64))?;
		enc.write_all(&ed6i)?;
		enc.finish()?;
		println!("{name}.ed6i.zst");
	}
	Ok(())
}
