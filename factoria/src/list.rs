use std::path::{PathBuf, Path};

use clap::ValueHint;

use bzip::CompressMode;
use themelios_archive::dirdat::{self, DirEntry};

use crate::util::mmap;
use crate::grid::{Grid, Cell, Orientation};

#[derive(Debug, Clone, clap::Args)]
#[command(arg_required_else_help = true)]
pub struct Command {
	/// Include zero-sized files
	#[clap(short, long)]
	all: bool,
	/// Filter which files to include
	#[clap(short, long, value_parser = crate::util::glob_parser())]
	glob: Vec<globset::Glob>,

	/// Show a detailed view with one file per line
	#[clap(short, long)]
	long: bool,
	/// Show one file per line
	#[clap(short='1', long, overrides_with("long"))]
	oneline: bool,
	/// Show several files per line
	#[clap(short='G', long, overrides_with("oneline"))]
	grid: bool,
	/// Draws grid left to right, not downwards
	#[clap(short='x', long)]
	across: bool,

	/// Specify sort order
	#[clap(short, long, default_value="id", require_equals(true), num_args=0..=1, default_missing_value="name")]
	sort: SortColumn,
	/// Reverse sort order
	#[clap(short, long)]
	reverse: bool,

	/// Use binary prefixes instead of SI for file sizes
	#[clap(short, long)]
	binary: bool,
	/// Display raw number of bytes for file sizes
	#[clap(short='B', long, overrides_with("binary"))]
	bytes: bool,
	/// Show (decompressed) file size in short modes
	#[clap(short='S', long)]
	size: bool,
	/// Do not attempt to estimate decompressed size
	#[clap(short='C', long)]
	compressed: bool,
	/// Show compressed size in addition to decompressed size
	#[clap(short='c', long)]
	compressed_size: bool,

	/// Show file id in short modes (always shown in -l)
	#[clap(short, long)]
	id: bool,

	/// Show unix timestamp instead of ISO format
	#[clap(short, long)]
	unix: bool,

	/// The .dir file(s) to inspect.
	#[clap(value_hint = ValueHint::FilePath, required = true)]
	dir_file: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, clap::ValueEnum)]
pub enum SortColumn {
	Id,
	Name,
	Size,
	CSize,
	Time,
	Ext,
}

#[derive(Debug)]
pub struct Entry {
	dirent: DirEntry,
	index: u16,
	decompressed_size: Option<usize>,
	compression_mode: Option<bzip::CompressMode>,
}

impl std::ops::Deref for Entry {
	type Target = DirEntry;

	fn deref(&self) -> &Self::Target {
		&self.dirent
	}
}

pub fn run(cmd: &Command) -> eyre::Result<()> {
	for (idx, dir_file) in cmd.dir_file.iter().enumerate() {
		if cmd.dir_file.len() != 1 {
			println!("{}:", dir_file.display());
		}

		if let Err(e) = list_one(cmd, dir_file) {
			eprintln!("{e}");
		};

		if idx + 1 != cmd.dir_file.len() {
			println!();
		}
	}
	Ok(())
}

fn list_one(cmd: &Command, dir_file: &Path) -> eyre::Result<()> {
	let archive_number = get_archive_number(dir_file);
	let entries = get_entries(cmd, dir_file)?;

	let orientation = if cmd.across {
		Orientation::Horizontal
	} else {
		Orientation::Vertical
	};

	if cmd.long {
		let mut cells = Vec::new();

		for e in &entries {
			format_entry_long(cmd, archive_number, e, &mut cells);
		}

		let width = if cmd.grid {
			term_size::dimensions_stdout().map_or(0, |a| a.0)
		} else  {
			0
		};
		let group = 7;
		print!("{}", Grid::best_fit(width, orientation, group, &cells, " "));
	} else {
		let mut cells = Vec::new();

		for e in &entries {
			format_entry_short(cmd, archive_number, e, &mut cells);
		}

		let group = usize::from(cmd.size) + usize::from(cmd.id) + 1;
		let width = if cmd.oneline {
			0
		} else if let Some(dim) = term_size::dimensions_stdout() {
			dim.0
		} else {
			0
		};

		print!("{}", Grid::best_fit(width, orientation, group, &cells, " "));
	}
	Ok(())
}

fn format_entry_short(cmd: &Command, archive_number: Option<u8>, e: &Entry, cells: &mut Vec<Cell>) {
	// ls puts inode before size, but since id is fixed size it looks better in the middle
	if cmd.size {
		cells.push(Cell::right(format_size(cmd, e)));
	}
	if cmd.id {
		let mut s = String::new();
		s.push_str("\x1B[2m");
		if let Some(archive_number) = archive_number {
			s.push_str(&format!("{:02X}", archive_number));
		}
		s.push_str("\x1B[m");
		s.push_str(&format!("{:04X}", e.index));
		cells.push(Cell::left(s));
	}
	cells.push(Cell::left(format_name(cmd, e)));
}

fn format_entry_long(cmd: &Command, archive_number: Option<u8>, e: &Entry, cells: &mut Vec<Cell>) {
	let mut s = String::new();
	s.push_str("\x1B[2m");
	if let Some(archive_number) = archive_number {
		s.push_str(&format!("{:04X}", archive_number));
	}
	s.push_str("\x1B[m");
	s.push_str(&format!("{:04X}", e.index));
	cells.push(Cell::left(s));

	cells.push(Cell::right(e.unk1.to_string()));

	if e.unk3 == e.compressed_size {
		cells.push(Cell::right("-".into()));
	} else {
		cells.push(Cell::right(format_size2(cmd, e.unk3)));
	}

	if e.archived_size == e.compressed_size {
		cells.push(Cell::right("-".into()));
	} else {
		cells.push(Cell::right(format_size2(cmd, e.archived_size)));
	}

	cells.push(Cell::right(format_size(cmd, e)));

	if cmd.unix {
		cells.push(Cell::right(e.timestamp.to_string()));
	} else if e.timestamp == 0 {
		cells.push(Cell::right("---- -- -- --:--:--".into()));
	} else {
		let ts = chrono::NaiveDateTime::from_timestamp_opt(e.timestamp as i64, 0).unwrap();
		cells.push(Cell::right(ts.to_string()));
	}

	cells.push(Cell::left(format_name(cmd, e)));
}

fn format_name(_cmd: &Command, e: &Entry) -> String {
	let mut s = String::new();
	let name = e.name.to_string();
	let ext = name.split_once('.').map_or("", |a| a.1);
	if let Some(color) = get_color(ext) {
		s.push_str(&format!("\x1B[38;5;{color}m"))
	}
	if e.timestamp == 0 {
		s.push_str("\x1B[2m");
	}
	s.push_str(&name);
	s.push_str("\x1B[m");
	s
}

fn format_size(cmd: &Command, e: &Entry) -> String {
	let mut s = String::new();
	if cmd.compressed_size && e.decompressed_size.is_some() {
		s.push_str(&format_size2(cmd, e.compressed_size));
	}
	match e.compression_mode {
		Some(CompressMode::Mode1) => s.push('⇒'),
		Some(CompressMode::Mode2) => s.push('→'),
		None if e.decompressed_size.is_some() => s.push('⇢'),
		None => {},
	}
	s.push_str(&format_size2(cmd, e.decompressed_size.unwrap_or(e.compressed_size)));
	s
}

fn format_size2(cmd: &Command, size: usize) -> String {
	if cmd.bytes {
		size.to_string()
	} else {
		use number_prefix::NumberPrefix as NP;
		let n = if cmd.binary {
			NP::binary(size as f64)
		} else {
			NP::decimal(size as f64)
		};
		match n {
			NP::Standalone(n) => n.round().to_string(),
			NP::Prefixed(p, n) => format!("{}{}", n.round(), p.symbol()),
		}
	}
}

fn get_color(ext: &str) -> Option<u8> {
	// General policy: files that are likely to appear in the same or adjacent archive should have different colors
	Some(match ext {
		"_ch" => 5, // image
		"_cp" => 2, // sprite
		"_ds" => 3, // dds file
		"_da" => 1, // font
		"_dt" => 3, // table or ani script
		"_op" => 4, // object placement
		"_en" => 2, // entrance placement
		"_x2" => 4, // model
		"_x3" => 4, // model
		"_ef" => 2, // effect
		"_ep" => 1, // effect placement
		"_sn" => 2, // scena script
		"_hd" => 5, // shadow mesh
		"_mh" => 5, // battlefield shape
		"wav" => 6, // sound
		"_ct" => 5, // collision mesh
		"_lm" => 6, // lightmap
		"_cl" => 2, // ?
		"_vs" => 1, // shader
		"_sy" => 6, // battle face

		// There exist .dm and .{blank}, but those can be uncolored
		_ => return None
	})
}

fn get_entries(cmd: &Command, dir_file: &Path) -> eyre::Result<Vec<Entry>> {
	let mut globset = globset::GlobSetBuilder::new();
	for glob in &cmd.glob {
		globset.add(glob.clone());
	}
	let globset = globset.build()?;

	let mut entries = dirdat::read_dir(&std::fs::read(dir_file)?)?
		.into_iter()
		.filter(|e| cmd.all || e.timestamp != 0)
		.filter(|e| globset.is_empty() || globset.is_match(e.name.to_string()))
		.enumerate()
		.map(|(index, dirent)| Entry {
			dirent,
			index: index as u16,
			decompressed_size: None,
			compression_mode: None,
		})
		.collect::<Vec<_>>();

	if !cmd.compressed && (cmd.size || cmd.long || cmd.sort == SortColumn::Size) {
		if let Ok(dat) = mmap(&dir_file.with_extension("dat")) {
			for m in &mut entries {
				let info = m.range()
					.and_then(|r| dat.get(r))
					.and_then(bzip::compression_info_ed6);
				if let Some(info) = info {
					m.decompressed_size = Some(info.0);
					m.compression_mode = info.1;
				}
			}
		}
	}

	match cmd.sort {
		SortColumn::Id => {},
		SortColumn::Name => entries.sort_by_key(|e| e.name.to_string()),
		SortColumn::Size => entries.sort_by_key(|e| e.decompressed_size.unwrap_or(e.compressed_size)),
		SortColumn::CSize => entries.sort_by_key(|e| e.compressed_size),
		SortColumn::Time => entries.sort_by_key(|e| e.timestamp),
		SortColumn::Ext => entries.sort_by(|a, b| {
			let a = a.name.to_string();
			let b = b.name.to_string();
			let a = (a.split_once('.').map(|a| a.1), &a);
			let b = (b.split_once('.').map(|b| b.1), &b);
			a.cmp(&b)
		}),
	}

	if cmd.reverse {
		entries.reverse();
	}

	Ok(entries)
}

fn get_archive_number(path: &Path) -> Option<u8> {
	let name = path
		.file_name()?
		.to_str()?
		.strip_prefix("ED6_DT")?
		.strip_suffix(".dir")?;
	u8::from_str_radix(name, 16).ok()
}
