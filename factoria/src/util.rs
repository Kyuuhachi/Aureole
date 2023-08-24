use std::path::Path;

use clap::builder::TypedValueParser;

pub fn mmap(path: &Path) -> std::io::Result<memmap2::Mmap> {
	let file = std::fs::File::open(path)?;
	unsafe { memmap2::Mmap::map(&file) }
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
