use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::archive::Archives;

pub fn read(_arcs: &Archives, t_face: &[u8]) -> Result<Vec<String>, super::ReadError> {
	let mut f = Coverage::new(Bytes::new(t_face));
	let mut faces = Vec::with_capacity(f.remaining() / 4);
	while f.remaining() > 0 {
		faces.push(_arcs.name(f.array()?)?.to_owned())
	}
	f.assert_covered()?;
	Ok(faces)
}

pub fn write(_arcs: &Archives, names: &[impl AsRef<str>]) -> Result<Vec<u8>, super::WriteError> {
	let mut out = Out::<()>::new();
	for name in names {
		out.array(_arcs.index(name.as_ref()).unwrap())
	}
	Ok(out.finish()?)
}

#[cfg(test)]
mod test {
	use crate::archive::Archives;
	use super::super::test::*;

	#[test_case::test_case(&FC; "fc")]
	fn roundtrip(arc: &Archives) -> Result<(), Error> {
		let t_face = arc.get_decomp("t_face._dt")?;
		let face = super::read(arc, &t_face)?;
		let t_face_ = super::write(arc, &face)?;
		let face_ = super::read(arc, &t_face_)?;
		check_equal(&face, &face_)
	}
}
