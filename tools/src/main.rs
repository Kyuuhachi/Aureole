use clap::StructOpt;

mod extract;
mod decompress;

#[derive(Debug, Clone, clap::Parser)]
struct Cli {
	#[clap(subcommand)]
	command: Command,
}

#[derive(Debug, Clone, clap::Subcommand)]
enum Command {
	Extract(extract::Command),
	Decompress(decompress::Command),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let cli = Cli::parse();
	match cli.command {
		Command::Extract(command) => extract::run(command)?,
		Command::Decompress(command) => decompress::run(command)?,
	}
	Ok(())
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
