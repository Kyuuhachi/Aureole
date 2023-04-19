use std::{path::{PathBuf, Path}, borrow::Cow};

use clap::ValueHint;
use themelios_archive::dirdat::{self, DirEntry};
use unicode_width::UnicodeWidthStr;

use crate::util::mmap;

#[derive(Debug, Clone, clap::Args)]
pub struct List {
	#[clap(short, long)]
	long: bool,
	#[clap(short='H', long)] // I'd rather have -h, but Clap claims that for itself
	human_readable: bool,
	#[clap(short='1')]
	one_per_line: bool,

	/// The .dir file to inspect.
	#[clap(value_hint = ValueHint::FilePath, required = true)]
	dir_file: Vec<PathBuf>,
}

struct Cell<'a> {
	width: usize,
	right: bool,
	format: Option<&'a str>,
	text: Cow<'a, str>,
}

impl<'a> Cell<'a> {
	fn new(text: impl Into<Cow<'a, str>>) -> Self {
		let text = text.into();
		Cell { width: text.width(), right: false, format: None, text }
	}

	fn format(mut self, format: &'a str) -> Self {
		self.format = Some(format);
		self
	}

	fn right(mut self) -> Self {
		self.right = true;
		self
	}
}

impl std::fmt::Display for Cell<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.text)
	}
}

struct Table<'a> {
	columns: usize,
	pad: usize,
	cells: &'a [Cell<'a>]
}

impl Table<'_> {
	fn column_widths(&self) -> impl Iterator<Item=usize> + '_ {
		(0..self.columns).map(|c| {
			self.cells.iter()
				.skip(c)
				.step_by(self.columns)
				.map(|a| a.width + if c == self.columns-1 { 0 } else { self.pad })
				.max().unwrap_or(0)
		})
	}
}

impl std::fmt::Display for Table<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let widths = self.column_widths().collect::<Vec<_>>();
		for (i, c) in self.cells.iter().enumerate() {
			if c.right {
				for _ in c.width..widths[i%self.columns]-self.pad {
					write!(f, " ")?;
				}
			}
			if let Some(fmt) = c.format {
				write!(f, "\x1B[{}m{}\x1B[m", fmt, c.text)?;
			} else {
				write!(f, "{}", c.text)?;
			}
			if (i+1) == self.cells.len() || (i+1) % self.columns == 0 {
				writeln!(f)?;
			} else {
				if !c.right {
					for _ in c.width..widths[i%self.columns]-self.pad {
						write!(f, " ")?;
					}
				}
				for _ in 0..self.pad {
					write!(f, " ")?;
				}
			}
		}
		Ok(())
	}
}

pub fn list(cmd: &List) -> eyre::Result<()> {
	for (idx, dir_file) in cmd.dir_file.iter().enumerate() {
		if cmd.dir_file.len() != 1 {
			println!("{}:", dir_file.display());
		}

		list_one(cmd, dir_file)?;

		if idx + 1 != cmd.dir_file.len() {
			println!();
		}
	}
	Ok(())
}

fn list_one(cmd: &List, dir_file: &Path) -> eyre::Result<()> {
	let archive_number = get_archive_number(dir_file);
	let entries = dirdat::read_dir(&mmap(dir_file)?)?;
	// TODO sort order: index (default), name, timestamp
	if cmd.long {
		let mut cells = Vec::new();
		for (i, e) in entries.iter().enumerate() {
			if let Some(arch) = archive_number {
				cells.push(Cell::new(format!("0x{:04X}{:04X}", arch, i)));
			} else {
				cells.push(Cell::new(format!("0x{:04X}", i)));
			}
			cells.push(Cell::new(e.unk1.to_string()).right());
			cells.push(Cell::new(size(cmd, e.unk3)).right());
			if e.compressed_size == e.archived_size {
				cells.push(Cell::new(size(cmd, e.compressed_size)).right());
			} else {
				cells.push(Cell::new(format!("{} ({})", size(cmd, e.compressed_size), size(cmd, e.archived_size))).right());
			}
			if e.timestamp == 0 {
				cells.push(Cell::new("---- -- -- --:--:--").format("2"));
			} else {
				let ts = chrono::NaiveDateTime::from_timestamp_opt(e.timestamp as i64, 0).unwrap();
				cells.push(Cell::new(ts.to_string()));
			}
			cells.push(format_name(e));
		}
		print!("{}", Table {
			columns: 6,
			pad: 1,
			cells: &cells,
		});
	} else {
		let names = entries.iter().map(format_name).collect::<Vec<_>>();
		if !cmd.one_per_line && let Some((width, _)) = term_size::dimensions_stdout() {
			let mut table = Table {
				columns: 0,
				pad: 2,
				cells: &names,
			};
			for columns in (1..width/3+2).rev() {
				table.columns = columns;
				if table.column_widths().sum::<usize>() <= width {
					print!("{}", table);
					return Ok(())
				}
			}
		}
		for e in names {
			println!("{}", e);
		}
	}
	Ok(())
}

fn size(cmd: &List, mut size: usize) -> String {
	if cmd.human_readable {
		if size < 768 {
			return format!("{}", size)
		}
		for unit in ["K", "M", "G", "T", "P", "E", "Z"] {
			if size < 768*1024 {
				return format!("{:3.1}{unit}", size as f32 / 1024.)
			}
			size /= 1024
		}
		return format!("{:3.1}Y", size as f32 / 1024.)
	}
	size.to_string()
}

fn get_archive_number(path: &Path) -> Option<u8> {
	let name = path
		.file_name()?
		.to_str()?
		.strip_prefix("ED6_DT")?
		.strip_suffix(".dir")?;
	u8::from_str_radix(name, 16).ok()
}

fn format_name(e: &DirEntry) -> Cell {
	if e.timestamp == 0 {
		Cell::new(&e.name).format("2")
	} else {
		Cell::new(&e.name)
	}
}
