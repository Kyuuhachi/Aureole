use std::{
	path::{Path, PathBuf},
	fs::{self, File},
	io::Write as _, ffi::OsStr,
};
use clap::StructOpt;
use indicatif::{ProgressBar, MultiProgress, ProgressStyle};
use snafu::prelude::*;
use ed6::archive::{Archive, Archives};

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
	/// Each `<indir>/ED6_DTxx.dir` will be extracted to `<outdir>/xx`.
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
			eprintln!("{}", backtrace);
		}
	}
}

#[derive(Debug, snafu::Snafu)]
pub enum Error {
	#[snafu(display("{message}\n{source}"))]
	Archive { message: String, #[snafu(backtrace)] source: ed6::archive::ArchiveError },

	#[snafu(whatever, display("{}",
		if let Some(source) = &source {
			format!("{message}\n{source}")
		} else {
			message.to_string()
		}
	))]
	Whatever {
		#[snafu(source(from(Box<dyn std::error::Error>, Some)))]
		source: Option<Box<dyn std::error::Error>>,
		message: String,
		backtrace: snafu::Backtrace,
	},
}

fn run(command: Command) -> Result<(), Error> {
	let style = ProgressStyle::with_template("{wide_bar} {msg:>12} ({bytes}/{total_bytes})").unwrap();
	match command {
		Command::Extract { force, dirfile, outdir } => {
			snafu::ensure_whatever!(
				dirfile.extension() == Some(OsStr::new("dir")),
				"{} is not a .dir file", dirfile.display(),
			);
			let dir = File::open(&dirfile)
				.with_whatever_context(|_| format!("could not open {}", dirfile.display()))?;
			let datfile = dirfile.with_extension("dat");
			let dat = File::open(&datfile)
				.with_whatever_context(|_| format!("could not open {}", datfile.display()))?;
			let arc = Archive::from_dir_dat(&dir, &dat)
				.with_context(|_| ArchiveSnafu { message: format!("could not read archive from {}", dirfile.display()) })?;
			let bar = ProgressBar::new(0).with_style(style);
			extract(force, &arc, &outdir, bar, None)?;
		}

		Command::ExtractAll { force, indir, outdir } => {
			let arcs = Archives::new(&indir)
				.with_context(|_| ArchiveSnafu { message: format!("could not read archives from {}", indir.display()) })?;
			snafu::ensure_whatever!(
				arcs.archives().next().is_some(),
				"no archives in {}", indir.display(),
			);
			let mut arcs = arcs.archives().collect::<Vec<_>>();
			arcs.sort_by_key(|a| a.0);
			let mpb = MultiProgress::new();

			let total_size = arcs.iter().flat_map(|a| a.1.entries()).map(|a| a.len() as u64).sum();
			let outerbar = ProgressBar::new(total_size)
				.with_style(ProgressStyle::with_template("{elapsed_precise} ({percent}%) {wide_bar} {msg}").unwrap());
			mpb.add(outerbar.clone());

			for (i, arc) in arcs {
				outerbar.set_message(format!("ED6_DT{i:02X}"));

				let bar = ProgressBar::new(0).with_style(style.clone());
				mpb.add(bar.clone());
				extract(force, arc, &outdir.join(format!("{i:02X}")), bar, Some(outerbar.clone()))?;
			}
		}
	}

	Ok(())
}

fn extract(force: bool, arc: &Archive, outdir: &Path, bar: ProgressBar, outerbar: Option<ProgressBar>) -> Result<(), Error> {
	if outdir.exists() {
		if force {
			fs::remove_dir_all(&outdir)
				.with_whatever_context(|_| format!("failed to remove {}", outdir.display()))?;
		} else {
			eprintln!("output directory {} already exists (use -f to overwrite)", outdir.display());
			return Ok(())
		}
	}

	fs::create_dir_all(&outdir)
		.with_whatever_context(|_| format!("failed to create output directory {}", outdir.display()))?;

	let mut index = fs::File::create(outdir.join("index"))
		.with_whatever_context(|_| format!("failed to create index {}", outdir.join("index").display()))?;

	bar.set_length(arc.entries().iter().map(|e| e.len() as u64).sum());
	for e in arc.entries() {
		bar.set_message(e.name.to_owned());
		let (rawlen, outlen) = if &e.name == "/_______.___" {
			continue
		} else if e.timestamp == 0 {
			(0, None)
		} else {
			let outfile = outdir.join(&e.name);
			let raw = arc.get(&e.name).unwrap();

			fs::write(&outfile, &raw)
				.with_whatever_context(|_| format!("failed to write output file {}", outfile.display()))?;
			filetime::set_file_mtime(&outfile, filetime::FileTime::from_unix_time(e.timestamp as i64, 0))
				.with_whatever_context(|_| format!("failed to set mtime on {}", outfile.display()))?;

			let decomp = ed6::decompress::decompress(raw).ok();
			if let Some(decomp) = &decomp {
				let outfile2 = outdir.join(&format!("{}.dec", e.name));
				fs::write(&outfile2, &decomp)
					.with_whatever_context(|_| format!("failed to write output file {}", outfile.display()))?;
				filetime::set_file_mtime(&outfile2, filetime::FileTime::from_unix_time(e.timestamp as i64, 0))
					.with_whatever_context(|_| format!("failed to set mtime on {}", outfile.display()))?;
			}

			(raw.len(), decomp.map(|a| a.len()))
		};

		let lenstr = if let Some(outlen) = outlen {
			format!("{} → {}", rawlen, outlen)
		} else {
			format!("{}", rawlen)
		};

		writeln!(index, "{:4} {:12} {} ({lenstr}; {} {})",
			e.index, e.name,
			chrono::NaiveDateTime::from_timestamp(e.timestamp as i64, 0),
			e.unk1, e.unk2,
		).whatever_context("failed to write to index")?;

		bar.inc(e.len() as u64);
		if let Some(outerbar) = &outerbar {
			outerbar.inc(e.len() as u64);
		}
	}

	Ok(())
}
