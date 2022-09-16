use std::{io::{self, Write}, path::{PathBuf, Path}};
use ed6::decompress;

/// Decompress a file or stdin into stdout
#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	#[clap(value_hint=clap::ValueHint::FilePath)]
	path: Option<PathBuf>,
}

pub fn run(Command {path}: Command) -> Result<(), io::Error> {
	let path = path.as_deref().unwrap_or_else(|| Path::new("-"));
	let mut input: Box<dyn io::Read> = if path == Path::new("-") {
		Box::new(io::stdin().lock())
	} else {
		Box::new(std::fs::File::open(&path)?)
	};

	let mut output = io::stdout().lock();
	for chunk in decompress::decompress_stream(&mut input) {
		output.write_all(&chunk?)?;
	}
	Ok(())
}
