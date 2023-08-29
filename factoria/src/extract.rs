use std::borrow::Cow;
use std::path::{PathBuf, Path};

use clap::ValueHint;

use eyre_span::emit;
use indicatif::ParallelProgressIterator;
use rayon::prelude::*;
use themelios_archive::dirdat;

use crate::util::mmap;

#[derive(Debug, Clone, clap::Args)]
#[command(arg_required_else_help = true)]
/// Extracts files from an archive into a directory.
///
/// Files will be placed in a directory with the name of the archive.
pub struct Command {
	/// Directory to place resulting subdirectory in.
	///
	/// If unspecified, place the subdirectory next to the .dir file.
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

pub fn run(cmd: &Command) -> eyre::Result<()> {
	for dir_file in &cmd.dir_file {
		emit(extract(cmd, dir_file));
	}
	Ok(())
}

#[tracing::instrument(skip_all, fields(path=%dir_file.display()))]
fn extract(cmd: &Command, dir_file: &Path) -> eyre::Result<()> {
	let dir_entries = dirdat::read_dir(&std::fs::read(dir_file)?)?;
	let dat = mmap(&dir_file.with_extension("dat"))?;

	let outdir = cmd.output.as_ref()
		.map_or_else(|| dir_file.parent().unwrap(), |v| v.as_path())
		.join(dir_file.file_stem().unwrap());

	std::fs::create_dir_all(&outdir)?;

	let mut globset = globset::GlobSetBuilder::new();
	for glob in &cmd.glob {
		globset.add(glob.clone());
	}
	let globset = globset.build()?;

	let dir_entries = dir_entries.into_iter()
		.filter(|e| e.name != dirdat::Name::default())
		.filter(|e| cmd.all || e.timestamp != 0)
		.filter(|e| globset.is_empty() || globset.is_match(e.name.to_string()))
		.collect::<Vec<_>>();

	let span = tracing::Span::current();
	let style = indicatif::ProgressStyle::with_template("{bar} {prefix} {pos}/{len}").unwrap()
		.progress_chars("█🮆🮅🮄▀🮃🮂▔ ");
	let ind = indicatif::ProgressBar::new(dir_entries.len() as _)
		.with_style(style)
		.with_prefix(dir_file.display().to_string());
	dir_entries.par_iter().progress_with(ind.clone()).for_each(|e| {
		emit(try {
			let _span = tracing::info_span!(parent: &span, "extract_file", name=%e.name).entered();
			let outfile = &outdir.join(e.name.to_string());
			let Some(rawdata) = dat.get(e.offset..e.offset+e.compressed_size) else {
				tracing::error!("invalid range");
				return
			};
			let data = if !cmd.compressed && bzip::compression_info_ed6(rawdata).is_some() {
				Cow::Owned(bzip::decompress_ed6_from_slice(rawdata)?)
			} else {
				Cow::Borrowed(rawdata)
			};

			std::fs::write(outfile, data)?;
			filetime::set_file_mtime(outfile, filetime::FileTime::from_unix_time(e.timestamp as _, 0))?;
		});
	});
	ind.abandon();

	Ok(())
}
