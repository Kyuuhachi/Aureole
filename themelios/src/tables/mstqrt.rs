use std::collections::BTreeMap;

use gospel::read::{Reader, Le as _};
use gospel::write::{Writer, Le as _};
use crate::types::*;
use themelios_common::util::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MstQrt {
	pub stats: Stats,
	pub eff: [u8; 4], // union of [u8; 4] and [u16; 2]
	pub art: MagicId,
	pub desc: [String; 6],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stats {
	pub hp: u16,
	pub ep: u16, // must be multiple of 10
	pub str: u16, // -''-
	pub def: u16, // -''-
	pub ats: u16, // -''-
	pub adf: u16, // -''-
	pub spd: u16,
}

impl MstQrt {
	pub fn read(data: &[u8]) -> Result<Vec<[MstQrt; 5]>, ReadError> {
		let f = &mut Reader::new(data);
		let g = &mut f.clone().at(110 * 20)?;
		let mut strings = Vec::new();
		let s = g.clone().u16()? as usize;
		while g.pos() < s {
			strings.push(g.ptr16()?.string()?);
		}

		let mut table = Vec::new();
		for _ in 0..22 {
			table.push(std::array::try_from_fn(|_| {
				let stats = Stats {
					hp:  f.u16()?,
					ep:  f.u8()? as u16 * 10,
					str: f.u8()? as u16 * 10,
					def: f.u8()? as u16 * 10,
					ats: f.u8()? as u16 * 10,
					adf: f.u8()? as u16 * 10,
					spd: f.u8()? as u16,
				};
				let eff = f.array::<4>()?;
				let art = MagicId(f.u16()?);
				let desc = f.array::<6>()?.map(|a| strings[a as usize].clone());
				Ok(MstQrt {
					stats,
					eff,
					art,
					desc,
				})
			}).strict()?);
		}
		Ok(table)
	}

	pub fn write(table: &[[MstQrt; 5]]) -> Result<Vec<u8>, WriteError> {
		let mut f = Writer::new();
		let mut g = Writer::new();
		let mut h = Writer::new();

		g.delay16(h.here());
		h.string("")?;

		let mut nstrings = 0;

		for mq in table {
			let mut w = mq.each_ref().try_map(|mq| {
				let mut g = Writer::new();
				g.u16(mq.stats.hp);
				g.u8(div(mq.stats.ep)?);
				g.u8(div(mq.stats.str)?);
				g.u8(div(mq.stats.def)?);
				g.u8(div(mq.stats.ats)?);
				g.u8(div(mq.stats.adf)?);
				g.u8(cast(mq.stats.spd)?);
				g.array(mq.eff);
				g.u16(mq.art.0);
				Ok(g)
			}).strict()?;

			for j in 0..6 {
				let mut prev = None;
				for i in 0..5 {
					let cur = &mq[i].desc[j];
					let n = match prev.filter(|(a, _)| *a == cur) {
						_ if cur.is_empty() => 0,
						Some((_, i)) => i,
						None => {
							g.delay16(h.here());
							h.string(cur)?;
							nstrings += 1;
							nstrings
						}
					};
					prev = Some((cur, n));
					w[i].u8(n);
				}
			}

			for w in w {
				f.append(w)
			}
		}

		f.append(g);
		f.append(h);
		Ok(f.finish()?)
	}
}

fn div(val: u16) -> Result<u8, WriteError> {
	ensure!(val % 10 == 0);
	Ok(cast(val / 10)?)
}

pub fn show(strs: &[&str], eff: u32) -> String {
	let [a,b,c,d] = eff.to_le_bytes();
	let values = [
		"ERROR".into(),
		u8::from_le_bytes([a]).to_string(),
		u8::from_le_bytes([b]).to_string(),
		u16::from_le_bytes([a, b]).to_string(),
		(u16::from_le_bytes([a, b]) as f32 / 100.).to_string(),
		u8::from_le_bytes([c]).to_string(),
		u8::from_le_bytes([d]).to_string(),
		u16::from_le_bytes([c, d]).to_string(),
		(u16::from_le_bytes([c, d]) as f32 / 100.).to_string(),
	];
	let s = strs.iter().copied().filter(|a| !a.is_empty()).collect::<Vec<_>>().join("\n");
	let mut s2 = String::new();
	let mut iter = s.chars();
	while let Some(c) = iter.next() {
		if c == '#' {
			let c = iter.next().unwrap();
			let n = c.to_digit(10).unwrap();
			s2.push_str(&values[n as usize].to_string());
			assert_eq!(iter.next(), Some('Q'));
		} else {
			s2.push(c)
		}
	}
	s2
}

#[test]
fn test_ao() -> Result<(), Box<dyn std::error::Error>> {
	for s in [
		"../data/ao/data/text_us/t_mstqrt._dt",
		"../data/ao/data/text/t_mstqrt._dt",
	] {
		let b = std::fs::read(s)?;
		let mq = MstQrt::read(&b)?;
		let b2 = MstQrt::write(&mq)?;
		println!("{:#.20X}", gospel_dump::dump(&Reader::new(&b)));
		println!("{:#.20X}", gospel_dump::dump(&Reader::new(&b2)));
		ensure!(b == b2);
	}
	Ok(())
}
