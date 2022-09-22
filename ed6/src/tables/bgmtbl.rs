use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::archive::Archives;
use crate::util::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[derive(derive_more::From, derive_more::Into)]
pub struct BgmId(u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bgm {
	pub id: BgmId,
	pub name: String,
	pub loops: bool,
}

pub fn read(_arcs: &Archives, t_town: &[u8]) -> Result<Vec<Bgm>, ReadError> {
	let mut f = Coverage::new(Bytes::new(t_town));
	let mut bgmtbl = Vec::with_capacity(f.remaining() / 16);
	while f.remaining() > 16 {
		let id = f.u32()?.into();
		let name = f.sized_string::<8>()?;
		let loops = cast_bool(f.u32()?)?;
		bgmtbl.push(Bgm { id, name, loops });
	}
	f.check_u32(0)?;
	f.check(b"ED6999\0\0")?;
	f.check_u32(0)?;

	f.assert_covered()?;
	Ok(bgmtbl)
}

pub fn write(_arcs: &Archives, bgmtbl: &[Bgm]) -> Result<Vec<u8>, WriteError> {
	let mut out = Out::<()>::new();
	for bgm in bgmtbl {
		out.u32(bgm.id.into());
		out.sized_string::<8>(&bgm.name)?;
		out.u32(bgm.loops.into());
	}
	out.u32(0);
	out.slice(b"ED6999\0\0");
	out.u32(0);
	Ok(out.finish()?)
}

#[cfg(test)]
mod test {
	use crate::archive::Archives;
	use crate::util::test::*;

	#[test_case::test_case(&FC; "fc")]
	fn roundtrip(arc: &Archives) -> Result<(), Error> {
		check_roundtrip(arc, "t_bgmtbl._dt", super::read, super::write)?;
		Ok(())
	}
}
