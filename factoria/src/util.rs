use std::path::Path;

pub fn mmap(path: &Path) -> std::io::Result<memmap2::Mmap> {
	let file = std::fs::File::open(path)?;
	unsafe { memmap2::Mmap::map(&file) }
}
