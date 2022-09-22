use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::archive::Archives;
use crate::util::*;

pub fn read(_arcs: &Archives, data: &[u8]) -> Result<Vec<u32>, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));
	let mut list = Vec::with_capacity(f.remaining() / 4);
	while f.remaining() > 0 {
		list.push(f.u32()?);
	}
	f.assert_covered()?;
	Ok(list)
}

pub fn write(_arcs: &Archives, list: &Vec<u32>) -> Result<Vec<u8>, WriteError> {
	let mut out = Out::<()>::new();
	for item in list {
		out.u32(*item);
	}
	Ok(out.finish()?)
}

#[cfg(test)]
mod test {
	use crate::archive::Archives;
	use crate::util::test::*;

	#[test_case::test_case(&FC; "fc")]
	fn roundtrip(arc: &Archives) -> Result<(), Error> {
		check_roundtrip(arc, "t_exp._dt", super::read, super::write)?;
		Ok(())
	}
}
