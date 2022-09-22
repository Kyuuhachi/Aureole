use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::archive::Archives;
use crate::util::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[derive(derive_more::From, derive_more::Into)]
pub struct SoundId(u16);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sound {
	pub id: SoundId,
	pub unk: u16,
	pub file: String,
	pub flag1: bool,
	pub flag2: bool,
}

pub fn read(_arcs: &Archives, data: &[u8]) -> Result<Vec<Sound>, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));
	let mut list = Vec::with_capacity(f.remaining() / 12);
	while f.remaining() > 12 {
		let id = f.u16()?.into();
		let unk = f.u16()?;
		let file = _arcs.name(f.array()?)?.to_owned();
		let flag1 = cast_bool(f.u16()?)?;
		let flag2 = cast_bool(f.u16()?)?;
		list.push(Sound { id, unk, file, flag1, flag2 });
	}

	f.check_u16(0xFFFF)?;
	f.check_u16(0x0001)?;
	f.check_u32(0)?;
	f.check_u16(0)?;
	f.check_u16(0)?;

	f.assert_covered()?;
	Ok(list)
}

pub fn write(_arcs: &Archives, list: &[Sound]) -> Result<Vec<u8>, WriteError> {
	let mut out = Out::<()>::new();
	for &Sound { id, unk, ref file, flag1, flag2 } in list {
		out.u16(id.into());
		out.u16(unk);
		out.array(_arcs.index(file).unwrap());
		out.u16(flag1.into());
		out.u16(flag2.into());
	}

	out.u16(0xFFFF);
	out.u16(0x0001);
	out.u32(0);
	out.u16(0);
	out.u16(0);

	Ok(out.finish()?)
}

#[cfg(test)]
mod test {
	use crate::archive::Archives;
	use crate::util::test::*;

	#[test_case::test_case(&FC; "fc")]
	fn roundtrip(arc: &Archives) -> Result<(), Error> {
		check_roundtrip(arc, "t_se._dt", super::read, super::write)?;
		Ok(())
	}
}
