mod archive;
mod decompress;

use hamu::dump::{Dump, DumpSpec};

fn main() -> anyhow::Result<()> {
	let arch = archive::Archive::new("data/fc", 0x1C)?;
	let (entry, data) = arch.get(4)?;
	println!("{:?}", entry);
	hamu::read::In::new(&data).edump(&DumpSpec::new())?;

	// for i in (0..=0x16).chain([0x19,0x1B,0x1C]) {
	// 	let arch = archive::Archive::new("data/fc/", i)?;
	// 	for (j, e) in arch.entries().iter().enumerate() {
	// 		println!("{i:X} {j} {:?}", e);
	// 	}
	// }
	Ok(())
}
