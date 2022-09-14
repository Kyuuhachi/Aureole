use clap::StructOpt;
use ed6::archive::Archives;
use snafu::prelude::*;

mod extract;

#[derive(Debug, Clone, clap::Parser)]
struct Cli {
	#[clap(subcommand)]
	command: Command,
}

#[derive(Debug, Clone, clap::Subcommand)]
enum Command {
	Extract(extract::Command),
	Test {
		#[clap(required(true))]
		dirs: Vec<String>,
	},
}

fn main() {
	let cli = Cli::parse();
	if let Err(e) = run(cli.command) {
		report(e);
		std::process::exit(1);
	}
}

fn report<E>(e: E) where E: std::error::Error + snafu::ErrorCompat + 'static {
	eprintln!("{e}\n");

	let env_backtrace = std::env::var("RUST_BACKTRACE").unwrap_or_default();
	let env_lib_backtrace = std::env::var("RUST_LIB_BACKTRACE").unwrap_or_default();
	if env_lib_backtrace == "1" || (env_backtrace == "1" && env_lib_backtrace != "0") {
		if let Some(backtrace) = snafu::ErrorCompat::backtrace(&e) {
			eprintln!("Backtrace:");
			eprintln!("{:?}", backtrace);
		}
	}
}

#[derive(Debug, snafu::Snafu)]
pub enum Error {
	#[snafu(display("{message}\n{source}"))]
	Archive { message: String, #[snafu(backtrace)] source: ed6::archive::Error },

	#[snafu(whatever, display("{}", source.as_ref().map_or(message.into(), |source| format!("{message}\n{source}"))))]
	Whatever {
		#[snafu(source(from(Box<dyn std::error::Error>, Some)))]
		source: Option<Box<dyn std::error::Error>>,
		message: String,
		backtrace: snafu::Backtrace,
	},
}

fn run(command: Command) -> Result<(), Error> {
	match command {
		Command::Extract(command) => extract::run(command)?,
		Command::Test { dirs } => run_test(dirs)?,
	}
	Ok(())
}

fn run_test(dirs: Vec<String>) -> Result<(), Error> {
	for name in dirs {
		let arcs = Archives::new(&name).with_context(|_| ArchiveSnafu { message: format!("Couldn't read data/{name}") })?;
		if let Err(e) = run_test_on(&arcs) {
			report(e);
		}
	}
	Ok(())
}

fn run_test_on(arc: &Archives) -> Result<(), ed6::tables::Error> {
	ed6::tables::town::load(arc)?;
	Ok(())
}
