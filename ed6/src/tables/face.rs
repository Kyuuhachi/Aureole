use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::gamedata::GameData;
use crate::util::*;

newtype!(FaceId, u16);

pub fn read(_arcs: &GameData, t_face: &[u8]) -> Result<Vec<String>, ReadError> {
	let mut f = Coverage::new(Bytes::new(t_face));
	let mut faces = Vec::with_capacity(f.remaining() / 4);
	while f.remaining() > 0 {
		faces.push(_arcs.name(f.array()?)?.to_owned())
	}
	f.assert_covered()?;
	Ok(faces)
}

pub fn write(_arcs: &GameData, names: &Vec<String>) -> Result<Vec<u8>, WriteError> {
	let mut out = OutBytes::new();
	for name in names {
		out.array(_arcs.index(name.as_ref()).unwrap())
	}
	Ok(out.finish()?)
}

#[cfg(test)]
mod test {
	use crate::gamedata::GameData;
	use crate::util::test::*;

	#[test_case::test_case(&FC; "fc")]
	fn roundtrip(arc: &GameData) -> Result<(), Error> {
		check_roundtrip_strict(arc, "t_face._dt", super::read, super::write)?;
		Ok(())
	}
}
