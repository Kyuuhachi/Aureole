use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::archive::Archives;
use crate::util::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bgm { id: u32, name: String, loops: bool }

pub fn read(_arcs: &Archives, t_town: &[u8]) -> Result<Vec<Bgm>, ReadError> {
	let mut f = Coverage::new(Bytes::new(t_town));
	let mut bgmtbl = Vec::with_capacity(f.remaining() / 16);
	while f.remaining() > 0 {
		let id = f.u32()?;
		let name = f.sized_string::<8>()?;
		let loops = match f.u32()? {
			0 => false,
			1 => true,
			n => Err(cast_error::<bool>(n, "out of range integral type conversion attempted"))?,
		};
		bgmtbl.push(Bgm { id, name, loops });
	}
	f.assert_covered()?;
	Ok(bgmtbl)
}

pub fn write(_arcs: &Archives, bgmtbl: &[Bgm]) -> Result<Vec<u8>, WriteError> {
	let mut out = Out::<()>::new();
	for bgm in bgmtbl {
		out.u32(bgm.id);
		out.sized_string::<8>(&bgm.name)?;
		out.u32(bgm.loops.into());
	}
	Ok(out.finish()?)
}

#[cfg(test)]
mod test {
	use crate::archive::Archives;
	use crate::util::test::*;

	#[test_case::test_case(&FC; "fc")]
	fn roundtrip(arc: &Archives) -> Result<(), Error> {
		check_roundtrip(arc, "t_bgmtbl._dt", super::read, super::write)
	}
}
