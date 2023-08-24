use std::path::PathBuf;

use clap::{Parser, ValueHint};
mod util;
mod list;
mod grid;

#[derive(Debug, Clone, Parser)]
#[command(args_conflicts_with_subcommands = true, disable_help_subcommand = true)]
struct Cli {
	#[clap(subcommand)]
	command: Option<Command>,
	#[clap(flatten)]
	extract: Option<Extract>,
}

#[derive(Debug, Clone, clap::Args)]
#[command(arg_required_else_help = true)]
struct Extract {
	/// Directory to place resulting files in.
	///
	/// If unspecified, a directory will be created adjacent to the dir file, with the .dir suffix removed.
	#[clap(long, short, value_hint = ValueHint::DirPath)]
	output: Option<PathBuf>,
	/// Do not attempt to decompress files.
	#[clap(short='C', long)]
	compressed: bool,

	/// The .dir file(s) to extract.
	#[clap(value_hint = ValueHint::FilePath, required = true)]
	dir_file: Vec<PathBuf>,
}

#[derive(Debug, Clone, clap::Subcommand)]
enum Command {
	/// Extract files from archives [default]
	Extract(Extract),
	/// List files in archives [ls]
	#[clap(alias = "ls")]
	List(list::List),
	/// Add files to archives (TBI)
	Add,
	/// Delete files from archives (TBI) [rm]
	#[clap(alias = "rm")]
	Remove,
	/// Clear out unused data from archives (TBI)
	Defrag,
	/// Create a json index file for an archive (TBI)
	Index,
	/// Create an archive from a json index file (TBI)
	Create,
}

fn main() -> eyre::Result<()> {
	let cli = Cli::parse();
	let command = cli.command.or(cli.extract.map(Command::Extract)).expect("no command");
	println!("{:?}", command);
	match command {
		Command::Extract(cmd) => todo!(),
		Command::List(cmd) => list::run(&cmd)?,
		Command::Add => todo!(),
		Command::Remove => todo!(),
		Command::Defrag => todo!(),
		Command::Index => todo!(),
		Command::Create => todo!(),
	}
	Ok(())
}
