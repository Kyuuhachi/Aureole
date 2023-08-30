use std::path::{Path, PathBuf};
use std::io::{self, BufWriter, Write};

use clap::ValueHint;
use serde::Serialize;
use serde_json::Value;
use eyre_span::emit;

use themelios_archive::dirdat::{self, DirEntry, Name};

#[derive(Debug, Clone, clap::Args)]
#[command(arg_required_else_help = true)]
/// Produces a json file listing all the files in an archive.
///
/// Combined with the `extract` command, this is enough for `create` to recreate an identical archive.
///
/// Note that while this writes file ids with eight hex digits, only the lower four are used when reconstructing.
pub struct Command {
	/// Do not attempt to infer compression mode.
	///
	/// Useful when extracted with `extract -C`.
	#[clap(short='C', long)]
	compressed: bool,

	/// Directory to place the resulting json file in.
	///
	/// This is *not* the path of the actual file, for consistency with the `extract` command.
	/// As a special case, if this is `-`, the json is written to stdout.
	#[clap(long, short, value_hint = ValueHint::DirPath)]
	output: Option<PathBuf>,

	/// The .dir files to create indexes for
	#[clap(value_hint = ValueHint::FilePath, required = true)]
	dir_file: Vec<PathBuf>,
}

pub fn run(cmd: &Command) -> eyre::Result<()> {
	for dir_file in &cmd.dir_file {
		emit(index(cmd, dir_file));
	}
	Ok(())
}

#[tracing::instrument(skip_all, fields(path=%dir_file.display(), out))]
fn index(cmd: &Command, dir_file: &Path) -> eyre::Result<()> {
	let dir = dirdat::read_dir(&std::fs::read(dir_file)?)?;
	let dat = if !cmd.compressed {
		Some(crate::util::mmap(&dir_file.with_extension("dat"))?)
	} else {
		None
	};
	let dat = dat.as_deref();
	let archive_number = crate::list::get_archive_number(dir_file);

	let json = dir.iter().enumerate().map(|(id, ent)| {
		let _span = tracing::debug_span!("index_file", id=%format_args!("{id:04X}"), name=%ent.name).entered();
		let mut key = String::from("0x");
		if let Some(archive_number) = archive_number {
			key.push_str(&format!("{:04X}", archive_number));
		}
		key.push_str(&format!("{:04X}", id));

		(key, index_file(ent, dir_file, dat))
	}).collect::<Value>();

	let out = if cmd.output.as_ref().is_some_and(|a| a == Path::new("-")) {
		tracing::Span::current().record("out", tracing::field::display("stdout"));
		Box::new(std::io::stdout().lock()) as Box<dyn Write>
	} else {
		let out = cmd.output.as_ref()
			.map_or_else(|| dir_file.parent().unwrap(), |v| v.as_path())
			.join(dir_file.file_name().unwrap())
			.with_extension("json");

		std::fs::create_dir_all(out.parent().unwrap())?;
		tracing::Span::current().record("out", tracing::field::display(out.display()));
		Box::new(std::fs::File::create(out)?)
	};

	let mut out = BufWriter::new(out);
	let mut ser = serde_json::Serializer::with_formatter(&mut out, MyFormatter::new(1));
	json.serialize(&mut ser)?;
	out.write_all(b"\n")?;
	out.flush()?;

	tracing::info!("done");

	Ok(())
}

fn index_file(m: &DirEntry, dir_file: &Path, dat: Option<&[u8]>) -> Value {
	if m.name == Name::default() {
		Value::Null
	} else {
		let mut o = serde_json::Map::new();

		if m.timestamp == 0 {
			o.insert("path".into(), Value::Null);
			o.insert("name".into(), m.name.to_string().into());
		} else {
			o.insert("path".into(), format!("{}/{}", dir_file.file_stem().unwrap().to_string_lossy(), m.name).into());
			let comp = dat.and_then(|a| a.get(m.offset..m.offset+m.size)).and_then(bzip::compression_info_ed6);
			if let Some(comp) = comp {
				match comp.1.unwrap_or_default() {
					bzip::CompressMode::Mode1 => o.insert("compress".into(), 1u8.into()),
					bzip::CompressMode::Mode2 => o.insert("compress".into(), 2u8.into()),
				};
			}
		}

		if m.reserved_size != m.size {
			o.insert("reserve".into(), m.reserved_size.into());
		}
		if m.unk1 != 0 {
			o.insert("unknown1".into(), m.unk1.into());
		}
		if m.unk2 != 0 {
			o.insert("unknown2".into(), m.unk2.into());
		}

		// size, timestamp. and offset are all derived from the file

		o.into()
	}
}

struct MyFormatter {
	level: usize,
	indent_to: usize,
	has_value: bool,
}

impl MyFormatter {
	pub fn new(depth: usize) -> Self {
		Self {
			level: 0,
			indent_to: depth,
			has_value: false,
		}
	}
}

impl serde_json::ser::Formatter for MyFormatter {
	#[inline]
	fn begin_array<W: Write + ?Sized>(&mut self, writer: &mut W) -> io::Result<()> {
		self.level += 1;
		self.has_value = false;
		writer.write_all(b"[")
	}

	#[inline]
	fn end_array<W: Write + ?Sized>(&mut self, writer: &mut W) -> io::Result<()> {
		if self.has_value {
			indent(writer, self.level - 1, self.indent_to - 1)?;
		}
		self.level -= 1;
		writer.write_all(b"]")
	}

	#[inline]
	fn begin_array_value<W: Write + ?Sized>(&mut self, writer: &mut W, first: bool) -> io::Result<()> {
		if !first {
			writer.write_all(b",")?;
		}
		indent(writer, self.level, self.indent_to)?;
		Ok(())
	}

	#[inline]
	fn end_array_value<W: Write + ?Sized>(&mut self, _writer: &mut W) -> io::Result<()> {
		self.has_value = true;
		Ok(())
	}

	#[inline]
	fn begin_object<W: Write + ?Sized>(&mut self, writer: &mut W) -> io::Result<()> {
		self.level += 1;
		self.has_value = false;
		writer.write_all(b"{")
	}

	#[inline]
	fn end_object<W: Write + ?Sized>(&mut self, writer: &mut W) -> io::Result<()> {
		if self.has_value {
			indent(writer, self.level - 1, self.indent_to - 1)?;
		}
		self.level -= 1;
		writer.write_all(b"}")
	}

	#[inline]
	fn begin_object_key<W: Write + ?Sized>(&mut self, writer: &mut W, first: bool) -> io::Result<()> {
		if !first {
			writer.write_all(b",")?;
		}
		indent(writer, self.level, self.indent_to)?;
		Ok(())
	}

	#[inline]
	fn begin_object_value<W: Write + ?Sized>(&mut self, writer: &mut W) -> io::Result<()> {
		writer.write_all(b": ")
	}

	#[inline]
	fn end_object_value<W: Write + ?Sized>(&mut self, _writer: &mut W) -> io::Result<()> {
		self.has_value = true;
		Ok(())
	}
}

fn indent<W: Write + ?Sized>(wr: &mut W, n: usize, m: usize) -> io::Result<()> {
	if n <= m {
		wr.write_all(b"\n")?;
		for _ in 0..n {
			wr.write_all(b"\t")?;
		}
	} else {
		wr.write_all(b" ")?;
	}
	Ok(())
}
