use std::path::PathBuf;

use clap::{Parser, ValueHint};
mod util;
mod list;

#[derive(Debug, Clone, Parser)]
struct Cli {
	#[clap(subcommand)]
	command: Command,
}

#[derive(Debug, Clone, clap::Args)]
struct Extract {
	/// Directory to place resulting files in.
	///
	/// If unspecified, a directory will be created adjacent to the dir file, with the .dir suffix removed.
	#[clap(long, short, value_hint = ValueHint::DirPath)]
	output: Option<PathBuf>,
	#[clap(long, default_value_t = true)]
	no_decompress: bool,

	/// The .dir file to extract.
	#[clap(value_hint = ValueHint::FilePath)]
	dir_file: PathBuf,

	/// Globs of filenames to extract.
	///
	/// If unspecified, extracts all files.
	names: Vec<String>,
}

#[derive(Debug, Clone, clap::Args)]
struct MakeIndex {
	/// The directory to generate the indexes from.
	/// Should be either the root directory for the PC games (containing the .dir/.dat files),
	/// or the data/data_sc/data_3rd directory for Evolution.
	#[clap(value_hint = ValueHint::DirPath)]
	dir: PathBuf,
	#[clap(long, short, value_hint = ValueHint::DirPath)]
	output: PathBuf,
}

#[derive(Debug, Clone, clap::Subcommand)]
enum Command {
	#[clap(visible_alias = "ls")]
	List(list::List),
	#[clap(visible_alias = "x")]
	Extract(Extract),
	#[clap(visible_alias = "index")]
	MakeIndex(MakeIndex),
}

fn main() -> eyre::Result<()> {
	let cli = Cli::parse();
	match cli.command {
		Command::List(cmd) => list::run(&cmd)?,
		Command::Extract(cmd) => todo!(),
		Command::MakeIndex(cmd) => todo!(),
	}
	Ok(())
}
