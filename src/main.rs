mod ed6;
mod decompress;

use std::{fs::File, io::Write};

use hamu::dump::{Dump, DumpSpec};

fn main() -> anyhow::Result<()> {
	let arch = ed6::Archive::new("data/fc", 0)?;
	let (entry, data) = arch.get_raw(44)?;
	println!("{:?}", entry);
	File::create("wipe00.dds")?.write_all(data)?;
	let (entry, data) = arch.get_raw(45)?;
	println!("{:?}", entry);
	File::create("wipe01.dds")?.write_all(data)?;
	let (entry, data) = arch.get_raw(46)?;
	println!("{:?}", entry);
	File::create("wipe02.dds")?.write_all(data)?;
	// hamu::read::In::new(&data).edump(&DumpSpec::new())?;

	// for i in (0..=0x16).chain([0x19,0x1B,0x1C]) {
	// 	let arch = archive::Archive::new("data/fc/", i)?;
	// 	for (j, e) in arch.entries().iter().enumerate() {
	// 		println!("{i:02X} {j:4} {:?}", e);
	// 	}
	// }
	Ok(())
}
