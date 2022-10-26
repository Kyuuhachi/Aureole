use std::{
	io::{self, prelude::*},
	path::PathBuf,
	ffi::OsStr,
};
use clap::StructOpt;

#[derive(Debug, Clone, clap::Parser)]
#[clap(name = "beryl", version, setting(clap::AppSettings::DeriveDisplayOrder))]
struct Cli {
	#[clap(long, short, parse(try_from_str = parse_size), default_value="0", hide_default_value=true)]
	start: usize,

	#[clap(long, short, parse(try_from_str = parse_size), conflicts_with_all = &["length", "lines"])]
	end: Option<usize>,

	#[clap(long, short, parse(try_from_str = parse_size))]
	length: Option<usize>,

	#[clap(long, short='L')]
	lines: Option<usize>,

	#[clap(long, short, parse(try_from_str = parse_size))]
	width: Option<usize>,

	#[clap(long, short='n', value_name="DIGITS")]
	num_width: Option<usize>,

	#[clap(long, short='N')]
	no_blank: bool,

	#[clap(long, short='1')]
	one_line: bool,

	#[clap(long)]
	gray: bool,

	#[clap(long, short='E', default_value="ascii")]
	encoding: String,

	#[clap(long, short='v')]
	print_name: bool,

	#[clap(value_hint=clap::ValueHint::FilePath)]
	files: Vec<PathBuf>,
}

fn parse_size(s: &str) -> Result<usize, std::num::ParseIntError> {
	if s.starts_with("0x") {
		usize::from_str_radix(s.trim_start_matches("0x"), 16)
	} else if s.starts_with("0o") {
		usize::from_str_radix(s.trim_start_matches("0o"), 8)
	} else if s.starts_with("0b") {
		usize::from_str_radix(s.trim_start_matches("0b"), 2)
	} else {
		s.parse::<usize>()
	}
}

fn main() -> io::Result<()> {
	let cli = Cli::parse();

	let preview: Option<Box<beryl::PreviewFn>> = if cli.encoding.to_ascii_lowercase() == "none" {
		None
	} else if cli.encoding.to_ascii_lowercase() == "ascii" {
		Some(Box::new(beryl::preview::ascii))
	} else if let Some(encoding) = encoding_rs::Encoding::for_label_no_replacement(cli.encoding.as_bytes()) {
		Some(Box::new(beryl::preview::encoding(encoding)))
	} else {
		panic!("Invalid encoding");
	};

	let files = if cli.files.is_empty() {
		vec![PathBuf::from("-")]
	} else {
		cli.files
	};

	for path in files {
		let size;
		let mut file: Box<dyn io::Read> = if path == OsStr::new("-") {
			size = 0;
			Box::new(io::stdin())
		} else {
			let file = std::fs::File::open(&path)?;
			size = file.metadata()?.len() as usize;
			Box::new(file)
		};

		io::copy(&mut file.by_ref().take(cli.start as u64), &mut io::sink())?;

		let mut dump = beryl::Dump::new(&mut file, cli.start);
		dump = dump.num_width_from(size);
		dump = dump.preview_option(preview.as_deref());
		if let Some(v) = cli.end    { dump = dump.end(v); }
		if let Some(v) = cli.lines  { dump = dump.lines(v); }
		if let Some(v) = cli.length { dump = dump.bytes(v); }
		if let Some(v) = cli.width  { dump = dump.width(v); }
		if let Some(v) = cli.num_width { dump = dump.num_width(v); }
		if cli.no_blank { dump = dump.newline(false); }
		if cli.one_line { dump = dump.oneline(); }
		if cli.gray { dump = dump.color(beryl::color::gray); }

		if cli.print_name {
			println!("{}", path.display());
		}
		dump.to_stdout();
	}
	Ok(())
}
