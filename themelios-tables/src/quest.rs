use std::collections::BTreeSet;

use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use themelios_scena::gamedata::GameData;
use themelios_scena::text::Text;
use themelios_scena::types::{QuestId, Flag};
use themelios_scena::util::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ED6Quest {
	pub id: QuestId,
	pub section: u16,
	pub index: u16,
	pub bp: u16,
	pub mira: u16,
	pub flags: [Flag; 3],
	pub name: String,
	pub desc: Text,
	pub steps: Vec<Text>,
}

pub fn read_ed6(_game: &GameData, data: &[u8]) -> Result<Vec<ED6Quest>, ReadError> {
	let mut f = Bytes::new(data);
	let n = f.clone().u16()? / 2;
	let mut table = Vec::new();

	for _ in 0..n {
		let mut g = f.ptr()?;

		let id = QuestId(g.u16()?);
		g.check_u16(0)?;

		let section = g.u16()?;
		let index = g.u16()?;
		let bp = g.u16()?;
		let mira = g.u16()?;
		let flags = array(|| Ok(Flag(g.u16()?))).strict()?;

		let name = g.ptr()?.string()?;
		let desc = Text::read(&mut g.ptr()?)?;
		let mut steps = Vec::new();
		for _ in 0..16 {
			steps.push(Text::read(&mut g.ptr()?)?);
		}

		table.push(ED6Quest { id, section, index, bp, mira, flags, name, desc, steps });
	}

	Ok(table)
}

pub fn write_ed6(_game: &GameData, table: &[ED6Quest]) -> Result<Vec<u8>, WriteError> {
	let mut f = OutBytes::new();
	let mut g = OutBytes::new();

	for &ED6Quest { id, section, index, bp, mira, flags, ref name, ref desc, ref steps } in table {
		f.delay_u16(g.here());

		g.u16(id.0);
		g.u16(0);
		g.u16(section);
		g.u16(index);
		g.u16(bp);
		g.u16(mira);
		g.u16(flags[0].0);
		g.u16(flags[1].0);
		g.u16(flags[2].0);

		let mut h = OutBytes::new();

		g.delay_u16(h.here());
		h.string(name)?;
		g.delay_u16(h.here());
		Text::write(&mut h, desc)?;
		for step in steps {
			g.delay_u16(h.here());
			Text::write(&mut h, step)?;
		}

		g.append(h);
	}

	f.append(g);
	Ok(f.finish()?)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ED7Quest {
	pub id: QuestId,
	pub section: u8,
	pub mira: u16,
	pub bp: u8,
	pub unk1: u8,
	pub flags: [Flag; 2],
	pub name: String,
	pub client: String,
	pub desc: Text,
	pub steps: Vec<Text>,
}

pub fn read_ed7(_game: &GameData, data: &[u8]) -> Result<Vec<ED7Quest>, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));
	let mut table = Vec::new();
	let mut step_ptrs = Vec::new();

	let mut all_ptrs = BTreeSet::new();
	all_ptrs.insert(f.len());
	let f_ = f.clone();
	let mut ptr = |p: u32| {
		all_ptrs.insert(p as usize);
		f_.clone().at(p as usize)
	};

	loop {
		let id = QuestId(f.u8()?.into());
		let section = f.u8()?;
		let mira = f.u16()?;
		let bp = f.u8()?;
		let unk1 = f.u8()?;
		f.check_u16(0)?;
		let flags = [Flag(f.u16()?), Flag(f.u16()?)];
		let name = ptr(f.u32()?)?.string()?;
		let client = ptr(f.u32()?)?.string()?;
		let desc = Text::read(&mut ptr(f.u32()?)?)?;
		step_ptrs.push(ptr(f.u32()?)?);
		table.push(ED7Quest {
			id,
			section,
			mira,
			bp,
			unk1,
			flags,
			name,
			client,
			desc,
			steps: Vec::new(),
		});
		if id == QuestId(0xFF) {
			break
		}
	}
	for (q, mut g) in table.iter_mut().zip(step_ptrs) {
		let end = *all_ptrs.range(g.pos()+1..).next().unwrap();
		while g.pos() + 4 <= end {
			q.steps.push(Text::read(&mut g.ptr32()?)?);
		}
	}
	Ok(table)
}

pub fn write_ed7(_game: &GameData, table: &[ED7Quest]) -> Result<Vec<u8>, WriteError> {
	let mut f = OutBytes::new();
	let mut g = OutBytes::new();
	let mut h = OutBytes::new();
	for q in table {
		f.u8(cast(q.id.0)?);
		f.u8(q.section);
		f.u16(q.mira);
		f.u8(q.bp);
		f.u8(q.unk1);
		f.u16(0);
		f.u16(q.flags[0].0);
		f.u16(q.flags[1].0);
		f.delay_u32(g.here()); g.string(&q.name)?;
		f.delay_u32(g.here()); g.string(&q.client)?;
		f.delay_u32(g.here()); Text::write(&mut g, &q.desc)?;
		f.delay_u32(h.here());
		for task in &q.steps {
			h.delay_u32(g.here());
			Text::write(&mut g, task)?;
		}
	}
	f.append(g);
	f.append(h);
	Ok(f.finish()?)
}
