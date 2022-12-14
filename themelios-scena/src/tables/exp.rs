use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::util::*;

pub fn read(data: &[u8]) -> Result<Vec<u32>, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));
	let mut table = Vec::with_capacity(f.remaining() / 4);
	while f.remaining() > 0 {
		table.push(f.u32()?);
	}
	f.assert_covered()?;
	Ok(table)
}

pub fn write(table: &[u32]) -> Result<Vec<u8>, WriteError> {
	let mut out = OutBytes::new();
	for &item in table {
		out.u32(item);
	}
	Ok(out.finish()?)
}
