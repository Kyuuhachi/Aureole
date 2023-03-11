#![allow(clippy::collapsible_else_if)]
use std::io::{Read, Write};
use std::path::{PathBuf, Path};

use calmare::parse::diag::Level;
use clap::{Parser, ValueHint};
use themelios::lookup::Lookup;
use themelios::types::Game;

#[derive(Debug, Clone, Parser)]
struct Cli {
	/// Where to place the output.
	///
	/// If unspecified, output will be placed next to the input file.
	#[clap(long, short, value_hint = ValueHint::FilePath)]
	output: Option<PathBuf>,

	/// Force compile mode.
	#[clap(long, short)]
	compile: bool,

	/// Force decompile mode.
	///
	/// If neither --compile nor --decompile is set, Calmare will try to guess which mode is wanted.
	#[clap(long, short, conflicts_with = "compile")]
	decompile: bool,

	/// Game to decompile as.
	///
	/// There is no indicator in the binary files which game it belongs to, so unless specified,
	/// this will be determined heuristically, which may lead to incorrect decompilation.
	///
	/// Has no effect for compilation.
	#[clap(long, short, hide_possible_values = true)]
	game: Option<CliGame>,

	/// The file to process.
	///
	/// Can be `-` to read from stdin.
	#[clap(required = true, value_hint = ValueHint::FilePath)]
	file: PathBuf,
}

// Feels like I'm implementing this mapping way too often. Gotta do something about that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum CliGame {
	#[value(name = "fc")] Fc,
	#[value(name = "fc_e")] FcEvo,
	#[value(name = "fc_k")] FcKai,
	#[value(name = "sc")] Sc,
	#[value(name = "sc_e")] ScEvo,
	#[value(name = "sc_k")] ScKai,
	#[value(name = "tc")] Tc,
	#[value(name = "tc_e")] TcEvo,
	#[value(name = "tc_k")] TcKai,

	#[value(name = "zero")] Zero,
	#[value(name = "zero_e")] ZeroEvo,
	#[value(name = "zero_k")] ZeroKai,
	#[value(name = "ao")] Ao,
	#[value(name = "ao_e")] AoEvo,
	#[value(name = "ao_k")] AoKai,
}

#[cfg(target_os = "windows")]
fn windows_wait() {
	let process_count: u32 = unsafe {
		windows_sys::Win32::System::Console::GetConsoleProcessList([0].as_mut_ptr(), 1)
	};

	if process_count == 1 {
		println!("\nPress any key to exit...");
		let _ = std::io::stdin().read(&mut []);
	}
}

#[cfg(not(target_os = "windows"))]
fn windows_wait() { }

fn main() -> eyre::Result<()> {
	main_inner().map_err(|e| {
		windows_wait();
		e
	})
}

fn main_inner() -> eyre::Result<()> {
	let cli = match Cli::try_parse() {
		Ok(cli) => cli,
		Err(e) => {
			e.print()?;
			windows_wait();
			std::process::exit(2);
		},
	};
	let mut buf = Vec::new();
	get_input(&cli.file)?.read_to_end(&mut buf)?;

	let src = if cli.decompile {
		None
	} else {
		let v = std::str::from_utf8(&buf);
		let is_text = cli.compile || v.map_or_else(|e| e.valid_up_to() >= 256.min(buf.len()), |s| !s.contains('\0'));
		is_text.then_some(v)
	};

	let lookup = None;

	if let Some(src) = src {
		let src = src?;
		let (val, diags) = calmare::parse(src, lookup);
		let filename = if cli.file.as_os_str() == "-" {
			"<stdin>".into()
		} else {
			cli.file.as_os_str().to_string_lossy()
		};
		print_diags(&filename, src, &diags);
		let Some((game, val)) = val else {
			eyre::bail!("failed with {} errors", diags.iter().filter(|a| a.is_fatal()).count())
		};

		match val {
			calmare::Content::ED6Scena(s) => {
				let suffix = if matches!(game, Game::Fc|Game::Sc|Game::Tc) {
					"_sn"
				} else {
					"bin"
				};
				let data = themelios::scena::ed6::write(game, &s)?;
				get_output(cli.output.as_deref(), &cli.file, suffix)?
					.write_all(&data)?;
			}
			calmare::Content::ED7Scena(s) => {
				let data = themelios::scena::ed7::write(game, &s)?;
				get_output(cli.output.as_deref(), &cli.file, "bin")?
					.write_all(&data)?;
			}
		}

		if !diags.is_empty() {
			windows_wait();
		}
	} else {
		let src = write_scena(cli.game, &buf, lookup)?;
		get_output(cli.output.as_deref(), &cli.file, "clm")?
			.write_all(src.as_bytes())?;
	}

	Ok(())
}

fn write_scena(game: Option<CliGame>, buf: &[u8], lookup: Option<&dyn Lookup>) -> eyre::Result<String> {
	match game {
		Some(game) => {
			let game = cli_game(game);
			let c = if game.is_ed7() {
				calmare::Content::ED7Scena(themelios::scena::ed7::read(game, buf)?)
			} else {
				calmare::Content::ED6Scena(themelios::scena::ed6::read(game, buf)?)
			};
			Ok(calmare::to_string(game, &c, lookup))
		},
		None => {
			for game in [
				Game::Fc, Game::Sc, Game::Tc, Game::ZeroKai, Game::AoKai, // Pc
				Game::FcEvo, Game::ScEvo, Game::TcEvo, Game::ZeroEvo, Game::AoEvo, // Evo
				Game::Zero, Game::Ao, // Geofront
			] {
				if game.is_ed7() {
					if let Ok(scena) = themelios::scena::ed7::read(game, buf) {
						return Ok(calmare::to_string(game, &calmare::Content::ED7Scena(scena), lookup))
					}
				} else {
					if let Ok(scena) = themelios::scena::ed6::read(game, buf) {
						return Ok(calmare::to_string(game, &calmare::Content::ED6Scena(scena), lookup))
					}
				}
			}
			eyre::bail!("could not parse script; specify --game for more details")
		}
	}
}

fn cli_game(e: CliGame) -> Game {
	match e {
		CliGame::Fc      => Game::Fc,
		CliGame::FcEvo   => Game::FcEvo,
		CliGame::FcKai   => Game::FcKai,
		CliGame::Sc      => Game::Sc,
		CliGame::ScEvo   => Game::ScEvo,
		CliGame::ScKai   => Game::ScKai,
		CliGame::Tc      => Game::Tc,
		CliGame::TcEvo   => Game::TcEvo,
		CliGame::TcKai   => Game::TcKai,
		CliGame::Zero    => Game::Zero,
		CliGame::ZeroEvo => Game::ZeroEvo,
		CliGame::ZeroKai => Game::ZeroKai,
		CliGame::Ao      => Game::Ao,
		CliGame::AoEvo   => Game::AoEvo,
		CliGame::AoKai   => Game::AoKai,
	}
}

fn get_input(input: &Path) -> std::io::Result<Box<dyn Read>> {
	if input.as_os_str() == "-" {
		Ok(Box::new(std::io::stdin()))
	} else {
		Ok(Box::new(std::fs::File::open(input)?))
	}
}

fn get_output(output: Option<&Path>, input: &Path, suffix: &str) -> std::io::Result<Box<dyn Write>> {
	if let Some(output) = output {
		if output.as_os_str() == "-" {
			Ok(Box::new(std::io::stdout()))
		} else {
			Ok(Box::new(std::fs::File::create(output)?))
		}
	} else {
		if input.as_os_str() == "-" {
			Ok(Box::new(std::io::stdout()))
		} else {
			Ok(Box::new(std::fs::File::create(input.with_extension(suffix))?))
		}
	}
}

pub fn print_diags(filename: &str, source: &str, diags: &[calmare::parse::Diag]) {
	use codespan_reporting::diagnostic::{Diagnostic, Label};
	use codespan_reporting::files::SimpleFiles;
	use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

	let writer = StandardStream::stderr(ColorChoice::Auto);
	let config = codespan_reporting::term::Config::default();
	let mut files = SimpleFiles::new();
	let file_id = files.add(filename, source);

	let mut diags = diags.to_owned();
	diags.sort_by_key(|a| (a.text.0.start, a.text.0.end));

	for d in diags {
		let mut l = vec![
			Label::primary(file_id, d.text.0.as_range()).with_message(&d.text.1),
		];
		for n in &d.notes {
			l.push(Label::secondary(file_id, n.0.as_range()).with_message(&n.1));
		}
		let d = match d.level {
			Level::Error => Diagnostic::error(),
			Level::Warning => Diagnostic::warning(),
			Level::Info => Diagnostic::help(),
		};
		let d = d.with_labels(l);
		codespan_reporting::term::emit(&mut writer.lock(), &config, &files, &d).unwrap();
	}
}
