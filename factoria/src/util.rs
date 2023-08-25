use std::path::Path;

use clap::builder::TypedValueParser;

#[tracing::instrument(fields(path=%path.display()))]
pub fn mmap(path: &Path) -> eyre::Result<memmap2::Mmap> {
	let file = std::fs::File::open(path)?;
	Ok(unsafe { memmap2::Mmap::map(&file)? })
}

#[tracing::instrument(fields(path=%path.display()))]
pub fn mmap_mut(path: &Path) -> eyre::Result<memmap2::MmapMut> {
	let file = std::fs::File::options().read(true).write(true).open(path)?;
	Ok(unsafe { memmap2::MmapMut::map_mut(&file)? })
}

pub fn glob_parser() -> impl clap::builder::TypedValueParser<Value=globset::Glob> {
	clap::builder::StringValueParser::new().try_map(|glob| {
		globset::GlobBuilder::new(&glob)
			.case_insensitive(true)
			.backslash_escape(true)
			.empty_alternates(true)
			.literal_separator(false)
			.build()
	})
}
