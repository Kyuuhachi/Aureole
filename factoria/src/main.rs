#![feature(try_blocks)]

use clap::Parser;
use eyre_span::emit;

mod util;
mod grid;

mod extract;
mod list;
mod add;
mod remove;
mod rebuild;
mod index;
mod create;

#[derive(Debug, Clone, Parser)]
#[command(args_conflicts_with_subcommands = true, disable_help_subcommand = true)]
struct Cli {
	#[clap(subcommand)]
	command: Option<Command>,
	#[clap(flatten)]
	extract: Option<extract::Command>,
}

#[derive(Debug, Clone, clap::Subcommand)]
enum Command {
	/// Extract files from archives [default]
	Extract(extract::Command),
	/// List files in archives [ls]
	#[clap(alias = "ls")]
	List(list::Command),
	/// Add or replaces files to archives
	Add(add::Command),
	/// Delete files from archives [rm]
	#[clap(alias = "rm")]
	Remove(remove::Command),
	/// Clear out unused data from archives
	Rebuild(rebuild::Command),
	/// Create a json index file for an archive
	Index(index::Command),
	/// Create an brand new archive from scratch a json index file
	Create(create::Command),
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
		Command::Extract(cmd) => emit(extract::run(&cmd)),
		Command::List(cmd) => emit(list::run(&cmd)),
		Command::Add(cmd) => emit(add::run(&cmd)),
		Command::Remove(cmd) => emit(remove::run(&cmd)),
		Command::Rebuild(cmd) => emit(rebuild::run(&cmd)),
		Command::Index(cmd) => emit(index::run(&cmd)),
		Command::Create(cmd) => emit(create::run(&cmd)),
	};
	Ok(())
}
