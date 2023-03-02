use std::path::Path;
use crate::lookup::ED6Lookup;

impl ED6Lookup {
	pub fn from_vita(dir: impl AsRef<Path>) -> std::io::Result<ED6Lookup> {
		let chcp = |s1| move |s: &str| {
			if s.ends_with('p') {
				format!("{s1}/{s}._cp")
			} else {
				format!("{s1}/{s}._ch")
			}
		};

		let suf = |suf| move |s: &str| {
			format!("{s}.{suf}")
		};

		let mut x = [(); 64].map(|_| Vec::new());

		let dir = dir.as_ref();
		let name = dir.file_name().unwrap_or_default();
		let mut o = 0;
		x[o+0x06] = txt(dir.join("chr/apl_pt.txt"), chcp("apl"))?;
		x[o+0x07] = txt(dir.join("chr/npl_pt.txt"), chcp("npl"))?;
		x[o+0x09] = txt(dir.join("chr/mons_pt.txt"), chcp("mons"))?;
		if name == "data_sc" || name == "data_3rd" {
			o = 0x20;
			x[o+0x06] = txt(dir.join("chr/apl2_pt.txt"), chcp("apl2"))?;
			x[o+0x07] = txt(dir.join("chr/npl2_pt.txt"), chcp("npl2"))?;
			x[o+0x09] = txt(dir.join("chr/mons2_pt.txt"), chcp("mons2"))?;
		}

		if name == "data" {
			x[o+0x01] = txt(dir.join("scenario/0/map.txt"), suf("_sn"))?;
		} else if name == "data_sc" {
			x[o+0x01] = txt(dir.join("scenario/1/map.txt"), suf("_sn"))?;
		} else if name == "data_3rd" {
			x[o+0x01] = txt(dir.join("scenario/2/map.txt"), suf("_sn"))?;
		}

		x[o+0x08] = txt(dir.join("minimap/_minimap.txt"), suf("_ch"))?;

		// There is also visual/dt[02]4.txt (which is a binary file), but I don't know if there are any file refs there
		// And also system/chrpt[12].txt, not sure what that is for

		Ok(ED6Lookup::new(x))
	}
}

fn txt(path: std::path::PathBuf, format: impl Fn(&str) -> String) -> std::io::Result<Vec<String>> {
	use std::io::BufRead;
	let file = std::fs::File::open(path)?;
	let mut lines = Vec::new();
	for line in std::io::BufReader::new(file).lines() {
		lines.push(format(&line?.to_lowercase()));
	}
	Ok(lines)
}
