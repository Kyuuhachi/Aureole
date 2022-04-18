mod archive;

fn main() -> anyhow::Result<()> {
	for i in (0..=0x16).chain([0x19,0x1B,0x1C]) {
		let arch = archive::Archive::new("data/fc/", i)?;
		println!("{:?}", arch.entries[0])
	}
	Ok(())
}
