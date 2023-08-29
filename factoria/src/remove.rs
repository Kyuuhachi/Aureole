use std::path::PathBuf;

use clap::ValueHint;

use eyre_span::emit;
use themelios_archive::dirdat::{self, DirEntry, Name};

#[derive(Debug, Clone, clap::Args)]
#[command(arg_required_else_help = true)]
/// Deletes one or more files from an archive file.
///
/// Note however that while the data is zeroed out, the space it previously occupied
/// remains. Use `factoria rebuild` to remove this.
///
/// Falcom's archives have many files that are nothing but a filename. By default,
/// this command replicates this behavior: the -f flag overrides this behavior and
/// removes the filename as well.
pub struct Command {
	/// Do not keep the filename in the dir file.
	#[clap(short, long)]
	force: bool,

	/// .dir file to insert into
	#[clap(value_hint = ValueHint::FilePath, required = true)]
	dir_file: PathBuf,

	/// Files to delete
	#[clap(required = true)]
	file: Vec<String>,
}

#[tracing::instrument(skip_all, fields(path=%cmd.dir_file.display()))]
pub fn run(cmd: &Command) -> eyre::Result<()> {
	let mut dir = dirdat::read_dir(&std::fs::read(&cmd.dir_file)?)?;
	let mut dat = crate::util::mmap_mut(&cmd.dir_file.with_extension("dat"))?;

	eyre::ensure!(dat[0..8] == *b"LB DAT\x1A\0", "invalid dat file");

	for file in &cmd.file {
		emit(remove(cmd, &mut dir, &mut dat, file));
	}

	std::fs::write(&cmd.dir_file, dirdat::write_dir(&dir))?;

	Ok(())
}

#[tracing::instrument(skip_all, fields(file=%file))]
fn remove(cmd: &Command, dir: &mut [DirEntry], dat: &mut [u8], file: &str) -> eyre::Result<()> {
	let name = Name::try_from(file)?;

	let Some(id) = dir.iter().position(|e| e.name == name) else {
		eyre::bail!("not found in archive");
	};

	let ent = &mut dir[id];
	
	if ent.timestamp == 0 && !cmd.force {
		eyre::bail!("file is already soft-deleted (use -f to hard delete)");
	}

	dat[ent.offset..][..ent.archived_size.max(ent.compressed_size)].fill(0);

	*ent = DirEntry {
		name,
		unk1: 0,
		compressed_size: 0,
		unk3: 0,
		archived_size: 0,
		timestamp: 0,
		offset: ent.offset,
	};

	if cmd.force {
		*ent = DirEntry::default();
		dat[16+id*4..][..4].fill(0);
	}

	tracing::info!("removed {} at {:04X}", name, id);

	Ok(())
}
