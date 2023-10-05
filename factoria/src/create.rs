use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::io::{prelude::*, SeekFrom};
use std::time::SystemTime;

use clap::ValueHint;
use indicatif::ProgressIterator;
use serde::de::{self, Deserialize};
use eyre_span::emit;

use themelios_archive::dirdat::{self, DirEntry, Name};

#[derive(Debug, Clone, clap::Args)]
#[command(arg_required_else_help = true)]
pub struct Command {
	/// Directory to place resulting .dir/.dat in
	#[clap(long, short, value_hint = ValueHint::DirPath)]
	output: Option<PathBuf>,

	/// The .json indexes to reconstruct
	#[clap(value_hint = ValueHint::FilePath, required = true)]
	json_file: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct FileId(u16);

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(remote = "Entry")]
struct Entry {
	path: Option<PathBuf>,
	name: Option<String>,
	#[serde(default, deserialize_with="parse_compress_mode")]
	compress: Option<bzip::CompressMode>,
	reserve: Option<usize>,
	#[serde(default)]
	unknown1: u32,
	#[serde(default)]
	unknown2: usize,
}

pub fn run(cmd: &Command) -> eyre::Result<()> {
	for json_file in &cmd.json_file {
		emit(create(cmd, json_file));
	}
	Ok(())
}

#[tracing::instrument(skip_all, fields(path=%json_file.display(), out))]
fn create(cmd: &Command, json_file: &Path) -> eyre::Result<()> {
	let json: BTreeMap<FileId, Option<Entry>>
		= serde_json::from_reader(std::fs::File::open(json_file)?)?;

	let out_dir = cmd.output.as_ref()
		.map_or_else(|| json_file.parent().unwrap(), |v| v.as_path())
		.join(json_file.file_name().unwrap())
		.with_extension("dir");

	tracing::Span::current().record("out", tracing::field::display(out_dir.display()));
	std::fs::create_dir_all(out_dir.parent().unwrap())?;

	let size = json.last_key_value().map(|a| a.0.0 + 1).unwrap_or_default() as usize;
	let mut entries = vec![None; size];
	for (k, v) in json {
		entries[k.0 as usize] = v
	}

	// TODO lots of duplicated code between here and rebuild

	let mut out_dat = std::fs::File::create(out_dir.with_extension("dat.tmp"))?;
	out_dat.write_all(b"LB DAT\x1A\0")?;
	out_dat.write_all(&u64::to_le_bytes(size as u64))?;
	for _ in 0..=size {
		out_dat.write_all(&u32::to_le_bytes(0))?;
	}

	let mut dir = Vec::with_capacity(size);
	let style = indicatif::ProgressStyle::with_template("{bar} {prefix} {pos}/{len}").unwrap()
		.progress_chars("█🮆🮅🮄▀🮃🮂▔ ");
	let ind = indicatif::ProgressBar::new(entries.iter().filter(|a| a.is_some()).count() as _)
		.with_style(style)
		.with_prefix(out_dir.display().to_string());
	for (id, e) in entries.into_iter().progress_with(ind.clone()).enumerate() {
		let mut ent = DirEntry::default();
		if let Some(e) = e {
			let name = match &e {
				Entry { name: Some(name), .. } => name.as_str(),
				Entry { path: Some(path), .. } => path.file_name().unwrap().to_str().unwrap(),
				_ => unreachable!()
			};
			let _span = tracing::info_span!("file", name=%name, path=tracing::field::Empty).entered();
			ent.name = Name::try_from(name)?;
			ent.unk1 = e.unknown1;
			ent.unk2 = e.unknown2;

			let pos = out_dat.seek(SeekFrom::End(0))?;
			ent.offset = pos as usize;

			if let Some(path) = &e.path {
				let path = json_file.parent().unwrap().join(path);
				_span.record("path", tracing::field::display(path.display()));

				let data = std::fs::read(&path)?;
				let mut data = match e.compress {
					Some(method) => bzip::compress_ed6_to_vec(&data, method),
					None => data,
				};
				ent.size = data.len();
				ent.reserved_size = e.reserve.unwrap_or(data.len());

				while data.len() < e.reserve.unwrap_or(0) {
					data.push(0);
				}
				out_dat.write_all(&data)?;

				let timestamp = std::fs::metadata(path)?
					.modified()
					.unwrap_or_else(|_| SystemTime::now());
				ent.timestamp = timestamp.duration_since(SystemTime::UNIX_EPOCH)?.as_secs() as u32;
			}

			let pos2 = out_dat.seek(SeekFrom::End(0))?;
			out_dat.seek(SeekFrom::Start(16 + 4 * id as u64))?;
			out_dat.write_all(&u32::to_le_bytes(pos as u32))?;
			out_dat.write_all(&u32::to_le_bytes(pos2 as u32))?;
		}
		dir.push(ent)
	}
	ind.abandon();

	std::fs::rename(out_dir.with_extension("dat.tmp"), out_dir.with_extension("dat"))?;
	std::fs::write(&out_dir, dirdat::write_dir(&dir))?;
	
	tracing::info!("created");

	Ok(())
}

fn parse_compress_mode<'de, D: serde::Deserializer<'de>>(des: D) -> Result<Option<bzip::CompressMode>, D::Error> {
	match <Option<u8>>::deserialize(des)? {
		Some(1) => Ok(Some(bzip::CompressMode::Mode1)),
		Some(2) => Ok(Some(bzip::CompressMode::Mode2)),
		None => Ok(None),
		Some(v) => Err(de::Error::invalid_value(
			de::Unexpected::Unsigned(v as _),
			&"1, 2, or null"),
		),
	}
}

impl std::str::FromStr for Entry {
	type Err = std::convert::Infallible;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Entry {
			path: Some(PathBuf::from(s)),
			name: None,
			compress: None,
			reserve: None,
			unknown1: 0,
			unknown2: 0,
		})
	}
}

impl<'de> Deserialize<'de> for Entry {
	fn deserialize<D: de::Deserializer<'de>>(des: D) -> Result<Self, D::Error> {
		struct V;
		impl<'de> de::Visitor<'de> for V {
			type Value = Entry;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				formatter.write_str("string or map")
			}

			fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
				std::str::FromStr::from_str(value).map_err(de::Error::custom)
			}

			fn visit_map<M: de::MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
				Entry::deserialize(de::value::MapAccessDeserializer::new(map))
			}
		}

		let v = des.deserialize_any(V)?;
		if v.path.is_none() && v.name.is_none() {
			return Err(de::Error::custom("at least one of `path` and `name` must be present"))
		}
		Ok(v)
	}
}

impl<'de> Deserialize<'de> for FileId {
	fn deserialize<D: de::Deserializer<'de>>(des: D) -> Result<Self, D::Error> {
		let s = String::deserialize(des)?;
		let err = || de::Error::invalid_value(
			de::Unexpected::Str(&s),
			&"a hexadecimal number",
		);

		let s = s.strip_prefix("0x").ok_or_else(err)?;
		let v = u32::from_str_radix(s, 16).map_err(|_| err())?;
		Ok(FileId(v as u16))
	}
}
