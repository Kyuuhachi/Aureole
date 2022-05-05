use std::path::{Path, PathBuf};
use std::borrow::Cow;
use std::fs;
use std::io::Write as _;

use clap::StructOpt;
use snafu::prelude::*;
use kaiseki::ed6::Archive;

#[derive(Debug, Clone, clap::Parser)]
struct Cli {
	#[clap(flatten)]
	verbose: clap_verbosity_flag::Verbosity<clap_verbosity_flag::InfoLevel>,

	#[clap(subcommand)]
	command: Command,
}

#[derive(Debug, Clone, clap::Subcommand)]
enum Command {
	/// Extract a single .dir/.dat archive.
	Extract {
		/// Overwrite the output directory if it already exists.
		#[clap(short, long)]
		force: bool,

		/// .dir file to read from. Corresponding .dat file must also exist.
		#[clap(value_hint=clap::ValueHint::FilePath)]
		dirfile: PathBuf,
		/// Directory to write extracted files to.
		#[clap(value_hint=clap::ValueHint::DirPath)]
		outdir: PathBuf,
	},

	/// Extract multiple .dir/.dat archives from a directory.
	///
	/// Each `<indir>/{file}.dir` will be extracted to `<outdir>/{file}`.
	ExtractAll {
		/// Overwrite the output directories if they already exist.
		#[clap(short, long)]
		force: bool,

		/// Directory containing .dir files to be extracted.
		#[clap(value_hint=clap::ValueHint::DirPath)]
		indir: PathBuf,
		/// Superdirectory to write extracted directories to.
		#[clap(value_hint=clap::ValueHint::DirPath)]
		outdir: PathBuf,
	},
}

fn main() {
	let cli = Cli::parse();
	env_logger::Builder::new()
		.filter_level(cli.verbose.log_level_filter())
		.init();

	if let Err(e) = run(cli.command) {
		eprintln!("{}", e);
		std::process::exit(1);
	}
}

fn run(command: Command) -> Result<(), snafu::Whatever> {
	match command {
		Command::Extract { force, dirfile, outdir } => {
			extract(force, &dirfile, &outdir)?;
		},

		Command::ExtractAll { force, indir, outdir } => {
			for a in fs::read_dir(&indir)
					.with_whatever_context(|_| format!("failed to read input directory {}", indir.display()))? {
				let a = a.whatever_context("failed to read directory entry")?;
				let dirfile = a.path();
				if dirfile.extension().filter(|a| a == &"dir").is_some() {
					extract(force, &dirfile, &outdir.join(dirfile.file_stem().unwrap()))?;
				}
			}
		},
	}

	Ok(())
}

fn extract(force: bool, dirfile: &Path, outdir: &Path) -> Result<(), snafu::Whatever> {
	ensure_whatever!(dirfile.is_file() && dirfile.extension().filter(|a| a == &"dir").is_some(),
		"dirfile {} must be a .dir file", dirfile.display(),
	);
	let datfile = dirfile.with_extension("dat");
	ensure_whatever!(datfile.is_file(),
		"datfile {} not found", datfile.display(),
	);

	if outdir.exists() {
		if force {
			fs::remove_dir_all(&outdir)
				.with_whatever_context(|_| format!("failed to remove {}", outdir.display()))?;
		} else {
			whatever!("output directory {} already exists (use -f to overwrite)", outdir.display());
		}
	}

	log::info!("Extracting archive {} to {}", dirfile.display(), outdir.display());

	let arch = Archive::from_dir_dat(&dirfile, &datfile)
		.with_whatever_context(|_| format!("failed to read archive {}", dirfile.display()))?;

	fs::create_dir_all(&outdir)
		.with_whatever_context(|_| format!("failed to create output directory {}", outdir.display()))?;

	let mut index = fs::File::create(outdir.join("index"))
		.with_whatever_context(|_| format!("failed to create index {}", outdir.join("index").display()))?;

	for e in arch.entries() {
		let outfile = outdir.join(e.display_name());
		let raw = arch.get(e.index).unwrap().1;
		log::debug!("{} ({} → {})", outfile.display(), raw.len(), e.size);

		let (note, data) = if e.timestamp == 0 {
			(" e ", None)
		} else if raw.len() == e.size {
			// I'm not sure about this heuristic; the size is very unreliable
			("   ", Some(Cow::Borrowed(raw)))
		} else {
			match kaiseki::decompress::decompress(raw) {
				Ok(decompressed) => {
					("(C)", Some(Cow::Owned(decompressed)))
				}
				Err(e) => {
					log::warn!("{}: decompression failed: {}", outfile.display(), e);
					("(?)", Some(Cow::Borrowed(raw)))
				}
			}
		};

		if let Some(data) = &data {
			fs::write(&outfile, data)
				.with_whatever_context(|_| format!("failed to write output file {}", outfile.display()))?;
			filetime::set_file_mtime(&outfile, filetime::FileTime::from_unix_time(e.timestamp as i64, 0))
				.with_whatever_context(|_| format!("failed to set mtime on {}", outfile.display()))?;
		}

		writeln!(index, "{:4} {} {:?} ({} → {}; {})", e.index, note, e.name, raw.len(), data.map_or(0, |a| a.len()), e.size)
			.whatever_context("failed to write to index")?;
	}

	Ok(())
}
