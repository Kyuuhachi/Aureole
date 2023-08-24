use std::{path::{PathBuf, Path}, borrow::Cow};

use clap::ValueHint;

use eyre_span::emit;
use themelios_archive::dirdat;

use crate::util::mmap;

#[derive(Debug, Clone, clap::Args)]
#[command(arg_required_else_help = true)]
pub struct Extract {
	/// Directory to place resulting files in.
	///
	/// If unspecified, a directory will be created adjacent to the dir file, with the .dir suffix removed.
	#[clap(long, short, value_hint = ValueHint::DirPath)]
	output: Option<PathBuf>,
	/// Include zero-sized files
	#[clap(short, long)]
	all: bool,
	/// Filter which files to include
	#[clap(short, long, value_parser = crate::util::glob_parser())]
	glob: Vec<globset::Glob>,
	/// Do not attempt to decompress files.
	#[clap(short='C', long)]
	compressed: bool,

	/// The .dir file(s) to extract.
	#[clap(value_hint = ValueHint::FilePath, required = true)]
	dir_file: Vec<PathBuf>,
}

pub fn run(cmd: &Extract) -> eyre::Result<()> {
	for dir_file in &cmd.dir_file {
		emit(extract(cmd, dir_file));
	}
	Ok(())
}

#[tracing::instrument(skip_all, fields(path=%dir_file.display()))]
fn extract(cmd: &Extract, dir_file: &Path) -> eyre::Result<()> {
	let dir = mmap(dir_file)?;
	let dir_entries = dirdat::read_dir(&dir)?;
	let dat = mmap(&dir_file.with_extension("dat"))?;
	let dat_entries = dirdat::read_dat(&dat)?;
	eyre::ensure!(dir_entries.capacity() == dat_entries.capacity(), "mismatched dat file (capacity, {} != {})", dir_entries.capacity(), dat_entries.capacity());

	let outdir = match &cmd.output {
		Some(v) if cmd.dir_file.len() > 1 => v.join(dir_file.file_stem().unwrap()),
		Some(v) => v.clone(),
		None => dir_file.parent().unwrap().join(dir_file.file_stem().unwrap()),
	};

	std::fs::create_dir_all(&outdir)?;

	let mut globset = globset::GlobSetBuilder::new();
	for glob in &cmd.glob {
		globset.add(glob.clone());
	}
	let globset = globset.build()?;

	for e in dir_entries {
		emit(try {
			let _span = tracing::info_span!("extract_file", name=%e.name).entered();
			let outfile = outdir.join(&e.name);
			if e.timestamp == 0 && !cmd.all {
				tracing::debug!("empty");
				continue;
			}
			if !globset.is_empty() && !globset.is_match(&e.name) {
				tracing::debug!("filtered");
				continue;
			}

			let Some(rawdata) = dat.get(e.offset..e.offset+e.compressed_size) else {
				eyre::bail!("{}: mismatched dat file (bounds)", outfile.display());
			};
			let data = if !cmd.compressed && bzip::compression_info_ed6(rawdata).is_some() {
				Cow::Owned(bzip::decompress_ed6_from_slice(rawdata)?)
			} else {
				Cow::Borrowed(rawdata)
			};

			std::fs::write(&outfile, data)?;

			// This slows it down significantly
			// tracing::info!(file = %outfile.display(), "done")
		});
	}

	Ok(())
}
