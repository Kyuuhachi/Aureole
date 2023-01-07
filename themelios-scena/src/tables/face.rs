use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::gamedata::Lookup;
use crate::util::*;

newtype!(FaceId, u16);

pub fn read(lookup: &dyn Lookup, t_face: &[u8]) -> Result<Vec<String>, ReadError> {
	let mut f = Coverage::new(Reader::new(t_face));
	let mut faces = Vec::with_capacity(f.remaining() / 4);
	while f.remaining() > 0 {
		faces.push(lookup.name(f.u32()?)?.to_owned())
	}
	f.assert_covered()?;
	Ok(faces)
}

pub fn write(lookup: &dyn Lookup, names: &Vec<String>) -> Result<Vec<u8>, WriteError> {
	let mut out = Writer::new();
	for name in names {
		out.u32(lookup.index(name.as_ref()).unwrap())
	}
	Ok(out.finish()?)
}
