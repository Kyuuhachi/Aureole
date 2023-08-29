use std::fs::File;
use std::io::{prelude::*, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bzip::CompressMode;
use clap::ValueHint;
use clap::builder::TypedValueParser;

use eyre_span::emit;
use themelios_archive::dirdat::{self, DirEntry, Name};

#[derive(Debug, Clone, clap::Args)]
#[command(arg_required_else_help = true)]
/// Rebuilds a new archive with the same contents as an existing one, pruning unused space.
///
/// This is useful after a sequence of add/remove operations.
pub struct Command {
	/// Change number of allocated file entries.
	#[clap(short='n', long)]
	reserve: Option<usize>,

	/// Remove all reserved space from files.
	#[clap(short, long)]
	pack: bool,

	/// Where to place the resulting .dir.
	///
	/// .dat file will be placed next to the .dir.
	#[clap(long, short, value_hint = ValueHint::AnyPath)]
	output: Option<PathBuf>,

	/// The .dir files to rebuild
	#[clap(value_hint = ValueHint::FilePath, required = true)]
	dir_file: Vec<PathBuf>,
}

pub fn run(cmd: &Command) -> eyre::Result<()> {
	for dir_file in &cmd.dir_file {
		emit(rebuild(cmd, dir_file));
	}
	Ok(())
}

#[tracing::instrument(skip_all, fields(path=%dir_file.display(), out))]
fn rebuild(cmd: &Command, dir_file: &Path) -> eyre::Result<()> {
	let mut dir = dirdat::read_dir(&std::fs::read(dir_file)?)?;
	let dat = crate::util::mmap(&dir_file.with_extension("dat"))?;

	let out_dir = match &cmd.output {
		None => dir_file.to_path_buf(),
		Some(f) if f.is_dir() || cmd.dir_file.len() > 1 => f.join(dir_file.file_name().unwrap()),
		Some(f) => f.to_path_buf(),
	};

	tracing::Span::current().record("out", tracing::field::display(out_dir.display()));

	std::fs::create_dir_all(out_dir.parent().unwrap())?;

	let expected_size = cmd.reserve.unwrap_or(dir.len());
	while dir.len() < expected_size {
		dir.push(DirEntry::default())
	}
	while dir.len() > expected_size {
		let last = dir.pop().unwrap();
		if last != DirEntry::default() {
			eyre::bail!("cannot trim capacity: {:04X} is non-empty", dir.len())
		}
	}

	let mut out_dat = std::fs::File::create(out_dir.with_extension("dat.tmp"))?;
	out_dat.write_all(b"LB DAT\x1A\0")?;
	out_dat.write_all(&u64::to_le_bytes(dir.len() as u64))?;
	for _ in 0..=dir.len() {
		out_dat.write_all(&u32::to_le_bytes(0))?;
	}

	for (id, ent) in dir.iter_mut().enumerate() {
		let _span = tracing::debug_span!("rebuild_file", id=%format_args!("{id:04X}"), name=%ent.name).entered();
		if ent.name == Name::default() {
			*ent = DirEntry::default();
		} else {
			if cmd.pack {
				ent.reserved_size = ent.size;
			}
			let size = ent.size.max(ent.reserved_size);
			let Some(data) = dat.get(ent.offset..ent.offset+size) else {
				eyre::bail!("invalid range")
			};
			let pos = out_dat.seek(SeekFrom::End(0))?;
			out_dat.write_all(data)?;
			let pos2 = out_dat.seek(SeekFrom::End(0))?;
			out_dat.seek(SeekFrom::Start(16 + 4 * id as u64))?;
			out_dat.write_all(&u32::to_le_bytes(pos as u32))?;
			out_dat.write_all(&u32::to_le_bytes(pos2 as u32))?;
			ent.offset = pos as usize;
		}
	}

	std::fs::rename(out_dir.with_extension("dat.tmp"), out_dir.with_extension("dat"))?;
	std::fs::write(&out_dir, dirdat::write_dir(&dir))?;
	
	tracing::info!("rebuilt");

	Ok(())
}
