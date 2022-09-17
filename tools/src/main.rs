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

fn main() -> Result<(), eyre::Report> {
	color_eyre::config::HookBuilder::default()
		.add_frame_filter(Box::new(|frames| {
			if let Some(a) = frames.iter().rposition(|f| matches!(&f.filename, Some(a) if a.starts_with(env!("CARGO_MANIFEST_DIR")))) {
				frames.truncate(a+2)
			}
		})).install()?;

	let cli = Cli::parse();
	match cli.command {
		Command::Extract(command) => extract::run(command)?,
		Command::Decompress(command) => decompress::run(command)?,
	}
	Ok(())
}
