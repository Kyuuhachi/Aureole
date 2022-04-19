mod ed6;
mod decompress;
mod util;

use hamu::{read::In, dump::{Dump, DumpSpec}};

fn main() -> anyhow::Result<()> {
	let arch = ed6::Archives::new("data/fc");
	let data = arch.get_compressed_by_name(0x2, *b"T_MAGIC ._DT")?.1;
	println!("{:#?}", ed6::magic::Magic::read(&mut In::new(&data)));

	// for i in (0..=0x16).chain([0x19,0x1B,0x1C]) {
	// 	let arch = archive::Archive::new("data/fc/", i)?;
	// 	for (j, e) in arch.entries().iter().enumerate() {
	// 		println!("{i:02X} {j:4} {:?}", e);
	// 	}
	// }
	Ok(())
}
