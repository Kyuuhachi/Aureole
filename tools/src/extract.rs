use std::{
	path::{Path, PathBuf},
	fs::{self, File},
	io::Write as _, ffi::OsStr,
};
use indicatif::{ProgressBar, MultiProgress, ProgressStyle};
use ed6::archive::{Archive, Archives};
use eyre::*;

/// Extract one or several .dir/dat archives.
#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// Overwrite the output directory/ies if they already exist.
	#[clap(short, long)]
	force: bool,

	/// .dir file or directory to read from.
	///
	/// If a file, extract its contents to <outdir>/.
	/// If a directory, extract each .dir file inside to <outdir>/nn/.
	#[clap(value_hint=clap::ValueHint::FilePath)]
	infile: PathBuf,

	/// Directory to write extracted files to.
	#[clap(value_hint=clap::ValueHint::DirPath)]
	outdir: PathBuf,
}

const BAR_STYLE: &str = "{wide_bar} {msg:>12} ({bytes}/{total_bytes})";
const TOTAL_BAR_STYLE: &str = "{elapsed_precise} ({percent}%) {wide_bar} {msg}";

pub fn run(Command { force, infile, outdir }: Command) -> Result<(), Report> {
	let meta = infile.metadata()
		.with_context(|| format!("could stat open {}", infile.display()))?;
	if meta.is_file() {
		ensure!(
			infile.extension() == Some(OsStr::new("dir")),
			"{} is not a .dir file", infile.display(),
		);

		let dir = File::open(&infile)
			.with_context(|| format!("could not open {}", infile.display()))?;
		let datfile = infile.with_extension("dat");
		let dat = File::open(&datfile)
			.with_context(|| format!("could not open {}", datfile.display()))?;
		let arc = Archive::from_dir_dat(&dir, &dat)
			.with_context(|| format!("could not read archive from {}", infile.display()))?;

		let bar = ProgressBar::new(0)
			.with_style(ProgressStyle::with_template(BAR_STYLE).unwrap());
		extract(force, &arc, &outdir, bar, None)?;
	} else if meta.is_dir() {
		let arcs = Archives::new(&infile)
			.with_context(|| format!("could not read archives from {}", infile.display()))?;
		ensure!(
			arcs.archives().next().is_some(),
			"no archives in {}", infile.display(),
		);
		let mut arcs = arcs.archives().collect::<Vec<_>>();
		arcs.sort_by_key(|a| a.0);

		let mpb = MultiProgress::new();

		let total_size = arcs.iter().flat_map(|a| a.1.entries()).map(|a| a.len() as u64).sum();
		let outerbar = ProgressBar::new(total_size)
			.with_style(ProgressStyle::with_template(TOTAL_BAR_STYLE).unwrap());
		mpb.add(outerbar.clone());

		for (i, arc) in arcs {
			outerbar.set_message(format!("ED6_DT{i:02X}"));

			let bar = ProgressBar::new(0)
				.with_style(ProgressStyle::with_template(BAR_STYLE).unwrap());
			mpb.add(bar.clone());
			extract(force, arc, &outdir.join(format!("{i:02X}")), bar, Some(outerbar.clone()))?;
		}
	} else {
		bail!("cannot handle {}", infile.display());
	}

	Ok(())
}

fn extract(force: bool, arc: &Archive, outdir: &Path, bar: ProgressBar, outerbar: Option<ProgressBar>) -> Result<(), Error> {
	if outdir.exists() {
		if force {
			fs::remove_dir_all(outdir)
				.with_context(|| format!("failed to remove {}", outdir.display()))?;
		} else {
			eprintln!("output directory {} already exists (use -f to overwrite)", outdir.display());
			return Ok(())
		}
	}

	fs::create_dir_all(outdir)
		.with_context(|| format!("failed to create output directory {}", outdir.display()))?;

	let mut index = fs::File::create(outdir.join("index"))
		.with_context(|| format!("failed to create index {}", outdir.join("index").display()))?;

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

			fs::write(&outfile, raw)
				.with_context(|| format!("failed to write output file {}", outfile.display()))?;
			filetime::set_file_mtime(&outfile, filetime::FileTime::from_unix_time(e.timestamp as i64, 0))
				.with_context(|| format!("failed to set mtime on {}", outfile.display()))?;

			let decomp = ed6::decompress::decompress(raw).ok();
			if let Some(decomp) = &decomp {
				let outfile2 = outdir.join(&format!("{}.dec", e.name));
				fs::write(&outfile2, decomp)
					.with_context(|| format!("failed to write output file {}", outfile.display()))?;
				filetime::set_file_mtime(&outfile2, filetime::FileTime::from_unix_time(e.timestamp as i64, 0))
					.with_context(|| format!("failed to set mtime on {}", outfile.display()))?;
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
		).context("failed to write to index")?;

		bar.inc(e.len() as u64);
		if let Some(outerbar) = &outerbar {
			outerbar.inc(e.len() as u64);
		}
	}

	Ok(())
}
