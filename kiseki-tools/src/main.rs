use std::path::PathBuf;
use std::fs;
use std::io::Write as _;

use clap::{StructOpt, IntoApp};
use kaiseki::ed6::Archive;

#[derive(Debug, Clone, clap::Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
	#[clap(subcommand)]
	command: Command,
}

#[derive(Debug, Clone, clap::Subcommand)]
enum Command {
	Extract {
		/// Overwrite the output directory if it already exists.
		#[clap(short, long)]
		force: bool,

		/// .dir file to read from. Corresponding .dat file must also exist.
		#[clap(value_hint=clap::ValueHint::FilePath)]
		dirfile: PathBuf,
		/// Directory to write files to.
		#[clap(value_hint=clap::ValueHint::DirPath)]
		outdir: PathBuf,
	},
}

fn main() -> eyre::Result<()> {
	let cli = Cli::parse();
	let mut cmd = Cli::command();
	match cli.command {
		Command::Extract { force, dirfile, outdir } => {
			if !dirfile.is_file() || dirfile.extension().filter(|a| "dir" == *a).is_none() {
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
	}

	Ok(())
}

fn extract(dirfile: &PathBuf, datfile: &PathBuf, outdir: &PathBuf) -> eyre::Result<()> {
	fs::create_dir_all(&outdir)?;

	let arch = Archive::from_dir_dat(&dirfile, &datfile)?;
	let mut index = fs::File::create(outdir.join("index"))?;

	for e in arch.entries() {
		let outfile = outdir.join(e.display_name());
		println!("Extracting {}", outfile.display());
		let note = if e.timestamp == 0 {
			" e "
		} else {
			let time = filetime::FileTime::from_unix_time(e.timestamp as i64, 0);
			let data = arch.get(e.index).unwrap().1;
			if data.len() == e.size {
				fs::write(&outfile, data)?;
				filetime::set_file_mtime(&outfile, time)?;
				"   "
			} else {
				let data = kaiseki::decompress::decompress(data).unwrap();
				fs::write(&outfile, data)?;
				filetime::set_file_mtime(&outfile, time)?;
				"(C)"
			}
		};

		writeln!(index, "{:4} {} {:?}", e.index, note, e.name)?;
	}

	Ok(())
}
