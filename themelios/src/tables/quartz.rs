use std::collections::BTreeMap;

use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _};
use crate::types::*;
use themelios_common::util::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quartz {
	pub id: u16,
	pub element: u16,
	pub cost: [u16; 7],
	pub value: [u16; 7],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MQuartz {
	pub id: u16,
	pub element: u16,
	pub value: [[u8; 7]; 5],
}

			// #[cfg(test)] print!("{:#1.32X}", gospel_dump::dump(g));

impl Quartz {
	pub fn read_ed6(data: &[u8]) -> Result<Vec<Quartz>, ReadError> {
		let f = &mut Reader::new(data);
		let end = f.clone().ptr16()?;
		let mut table = Vec::new();
		while f.pos() < end.pos() {
			let g = &mut f.ptr16()?;
			let id = g.u16()?;
			let element = g.u16()?;
			let cost = std::array::try_from_fn(|_| g.u16())?;
			let value = std::array::try_from_fn(|_| g.u16())?;
			table.push(Quartz { id, element, cost, value });
		}
		Ok(table)
	}

	pub fn read_zero(data: &[u8]) -> Result<Vec<Quartz>, ReadError> {
		let f = &mut Reader::new(data);
		Self::read_ed7(f, f.len())
	}

	fn read_ed7(f: &mut Reader, end: usize) -> Result<Vec<Quartz>, ReadError> {
		let mut table = Vec::new();
		while f.pos() < end {
			let id = f.u16()?;
			let element = f.u16()? - 1;
			f.check_u16(0)?;
			let cost = std::array::try_from_fn(|_| f.u16())?;
			f.check_u8(0)?;
			let value = f.array()?.map(|a| a as u16);
			table.push(Quartz { id, element, cost, value });
		}
		Ok(table)
	}

	pub fn read_ao(data: &[u8]) -> Result<(Vec<Quartz>, Vec<MQuartz>), ReadError> {
		let f = &mut Reader::new(data);
		let g = &mut f.ptr16()?;

		let quartz = Self::read_ed7(f, g.pos())?;

		let mut mq = Vec::new();
		while f.pos() < f.len() {
			let id = f.u16()?;
			let element = f.u16()? - 1;
			let value = std::array::try_from_fn(|_| {
				f.check_u8(0)?;
				f.array()
			})?;
			mq.push(MQuartz { id, element, value });
		}

		Ok((quartz, mq))
	}

	pub fn write_ed6(table: &[Quartz]) -> Result<Vec<u8>, WriteError> {
		let mut f = Writer::new();
		let mut g = Writer::new();
		for q in table {
			f.delay16(g.here());
			g.u16(q.id);
			g.u16(q.element);
			for i in q.cost { g.u16(i) }
			for i in q.value { g.u16(i) }
		}
		f.append(g);
		Ok(f.finish()?)
	}

	pub fn write_zero(table: &[Quartz]) -> Result<Vec<u8>, WriteError> {
		let mut f = Writer::new();
		Ok(f.finish()?)
	}

	pub fn write_ao(quart: &[Quartz], mq: &[MQuartz]) -> Result<Vec<u8>, WriteError> {
		let mut f = Writer::new();
		Ok(f.finish()?)
	}
}

#[test]
fn test_ed6() -> Result<(), Box<dyn std::error::Error>> {
	for s in [
		"../data/fc.extract/02/t_quartz._dt",
		"../data/sc.extract/22/t_quartz._dt",
		"../data/3rd.extract/22/t_quartz._dt",
	] {
		let b = std::fs::read(s)?;
		let mq = Quartz::read_ed6(&b)?;
		let b2 = Quartz::write_ed6(&mq)?;
		println!("{:#X}", gospel_dump::dump(&Reader::new(&b)));
		println!("{:#X}", gospel_dump::dump(&Reader::new(&b2)));
		ensure!(b == b2);
	}
	Ok(())
}

// #[test]
// fn test_zero() -> Result<(), Box<dyn std::error::Error>> {
// 	let s = "../data/zero/data/text/t_quartz._dt";
// 	let b = std::fs::read(s)?;
// 	let mq = Quartz::read_zero(&b)?;
// 	let b2 = Quartz::write_zero(&mq)?;
// 	ensure!(b != b2);
// 	Ok(())
// }
//
// #[test]
// fn test_ao() -> Result<(), Box<dyn std::error::Error>> {
// 	for s in [
// 		"../data/ao/data/text/t_quartz._dt",
// 		"../data/ao/data/text_us/t_quartz._dt",
// 	] {
// 		let b = std::fs::read(s)?;
// 		let mq = Quartz::read_ao(&b)?;
// 		let b2 = Quartz::write_ao(&mq)?;
// 		ensure!(b != b2);
// 	}
// 	Ok(())
// }
