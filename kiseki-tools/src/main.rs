use std::path::PathBuf;
use std::fs;
use std::io::Write as _;

use clap::{StructOpt, IntoApp};
use kaiseki::ed6::Archive;

#[derive(Debug, Clone, clap::Parser)]
struct Cli {
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

fn main() -> eyre::Result<()> {
	let cli = Cli::parse();
	let mut cmd = Cli::command();
	match cli.command {
		Command::Extract { force, dirfile, outdir } => {
			if !dirfile.is_file() || dirfile.extension().filter(|a| a == &"dir").is_none() {
				cmd.error(clap::ErrorKind::ValueValidation, "Input must be a .dir file").exit();
			}
			let datfile = dirfile.with_extension("dat");
			if !datfile.is_file() {
				cmd.error(clap::ErrorKind::ValueValidation, ".dat file not found").exit();
			}

			if outdir.exists() {
				if force {
					fs::remove_dir_all(&outdir)?;
				} else {
					cmd.error(clap::ErrorKind::ValueValidation, "Output directory already exists (use -f to overwrite)").exit();
				}
			}

			extract(&dirfile, &datfile, &outdir)?;
		},

		Command::ExtractAll { force, indir, outdir } => {
			if !indir.is_dir() {
				cmd.error(clap::ErrorKind::ValueValidation, "Invalid input directory").exit();
			}

			for a in fs::read_dir(&indir)? {
				let a = a?;
				let dirfile = a.path();
				if dirfile.extension().filter(|a| a == &"dir").is_some() {
					let datfile = dirfile.with_extension("dat");
					if !datfile.is_file() {
						cmd.error(clap::ErrorKind::ValueValidation, ".dat file not found").exit();
					}

					let outdir = outdir.join(dirfile.file_stem().unwrap());

					if outdir.exists() {
						if force {
							fs::remove_dir_all(&outdir)?;
						} else {
							cmd.error(clap::ErrorKind::ValueValidation, "Output directory already exists (use -f to overwrite)").exit();
						}
					}

					extract(&dirfile, &datfile, &outdir)?;
				}
			}
		},
	}

	Ok(())
}

fn extract(dirfile: &PathBuf, datfile: &PathBuf, outdir: &PathBuf) -> eyre::Result<()> {
	fs::create_dir_all(&outdir)?;

	let arch = Archive::from_dir_dat(&dirfile, &datfile)?;
	let mut index = fs::File::create(outdir.join("index"))?;

	for e in arch.entries() {
		let outfile = outdir.join(e.display_name());
		let data = arch.get(e.index).unwrap().1;
		println!("Extracting {} ({} → {})", outfile.display(), data.len(), e.size);

		let note = if e.timestamp == 0 {
			" e "
		} else {
			let time = filetime::FileTime::from_unix_time(e.timestamp as i64, 0);
			if data.len() == e.size {
				fs::write(&outfile, data)?;
				filetime::set_file_mtime(&outfile, time)?;
				"   "
			} else {
				match kaiseki::decompress::decompress(data) {
					Ok(udata) => {
						fs::write(&outfile, udata)?;
						filetime::set_file_mtime(&outfile, time)?;
						"(C)"
					}
					Err(e) => {
						println!("  Decompression failed: {}", e);
						fs::write(&outfile, data)?;
						filetime::set_file_mtime(&outfile, time)?;
						"(?)"
					}
				}
			}
		};

		writeln!(index, "{:4} {} {:?} ({} → {})", e.index, note, e.name, data.len(), e.size)?;
	}

	Ok(())
}
