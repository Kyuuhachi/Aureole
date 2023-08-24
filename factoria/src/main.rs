#![feature(try_blocks)]

use clap::Parser;
mod util;
mod grid;

mod list;
mod extract;
mod add;

#[derive(Debug, Clone, Parser)]
#[command(args_conflicts_with_subcommands = true, disable_help_subcommand = true)]
struct Cli {
	#[clap(subcommand)]
	command: Option<Command>,
	#[clap(flatten)]
	extract: Option<extract::Extract>,
}

#[derive(Debug, Clone, clap::Subcommand)]
enum Command {
	/// Extract files from archives [default]
	Extract(extract::Extract),
	/// List files in archives [ls]
	#[clap(alias = "ls")]
	List(list::List),
	/// Add files to archives
	Add(add::Add),
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
	use tracing_error::ErrorLayer;
	use tracing_subscriber::prelude::*;
	use tracing_subscriber::{fmt, EnvFilter};

	let fmt_layer = fmt::layer().with_target(false);
	let filter_layer = EnvFilter::try_from_default_env()
		.or_else(|_| EnvFilter::try_new("info"))?;

	tracing_subscriber::registry()
		.with(filter_layer)
		.with(fmt_layer)
		.with(ErrorLayer::default())
		.init();

	eyre_span::install()?;

	let cli = Cli::parse();
	let command = cli.command.or(cli.extract.map(Command::Extract)).expect("no command");
	match command {
		Command::Extract(cmd) => extract::run(&cmd)?,
		Command::List(cmd) => list::run(&cmd)?,
		Command::Add(cmd) => add::run(&cmd)?,
		Command::Remove => todo!(),
		Command::Defrag => todo!(),
		Command::Index => todo!(),
		Command::Create => todo!(),
	}
	Ok(())
}
