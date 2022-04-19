mod ed6;
mod decompress;
mod util;

use hamu::read::In;

fn main() -> anyhow::Result<()> {
	let arch = ed6::Archives::new("data/fc");
	let data = arch.get_compressed_by_name(0x2, *b"T_MAGIC ._DT")?.1;
	println!("{:#?}", ed6::magic::Magic::read(&mut In::new(&data)));
	Ok(())
}
