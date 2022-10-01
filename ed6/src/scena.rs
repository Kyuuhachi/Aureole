use hamu::read::coverage::Coverage;
use hamu::read::le::*;
use hamu::write::le::*;
use crate::archive::Archives;
use crate::tables::bgmtbl::BgmId;
use crate::tables::btlset::BattleId;
use crate::tables::town::TownId;
use crate::util::*;

pub mod code;

#[derive(Clone, Copy, PartialEq, Eq)]
#[derive(derive_more::DebugCustom)]
#[debug(fmt = "FuncRef({_0}, {_1})")]
pub struct FuncRef(pub u16, pub u16);

#[derive(Clone, Copy, PartialEq, Eq)]
#[derive(derive_more::DebugCustom)]
#[debug(fmt = "Pos2({_0}, {_1})")]
pub struct Pos2(pub i32, pub i32);

#[derive(Clone, Copy, PartialEq, Eq)]
#[derive(derive_more::DebugCustom)]
#[debug(fmt = "Pos3({_0}, {_1}, {_2})")]
pub struct Pos3(pub i32, pub i32, pub i32);

newtype!(Flag, u16);
newtype!(Var, u16);
newtype!(Attr, u8);
newtype!(CharId, u16);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[derive(derive_more::DebugCustom)]
#[debug(fmt = "CharAttr({_0:?}, {_1})")]
pub struct CharAttr(pub CharId, pub u8);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[derive(derive_more::DebugCustom)]
#[debug(fmt = "Emote({_0:?}, {_1})")]
pub struct Emote(pub u8, pub u8, pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Npc {
	pub name: String,
	pub pos: Pos3,
	pub angle: u16,
	pub ch: (u16, u16), // First entry seems to always be zero. Probably include index, just like for functions.
	pub cp: (u16, u16),
	pub flags: u16,
	pub init: FuncRef,
	pub talk: FuncRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monster {
	pub name: String,
	pub pos: Pos3,
	pub angle: u16,
	pub _1: u16, // This looks like a chcp index, but npcs have 4×u16 while this only has 1×u16?
	pub flags: u16,
	pub _2: i32, // Always -1
	pub battle: BattleId,
	pub flag: Flag, // set when defeated
	pub _3: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trigger {
	pub pos1: Pos3,
	pub pos2: Pos3,
	pub flags: u16,
	pub func: FuncRef,
	pub _1: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Object {
	pub pos: Pos3,
	pub radius: u32,
	pub bubble_pos: Pos3,
	pub flags: u16,
	pub func: FuncRef,
	pub _1: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraAngle {
	// According to some debug strings, the camera has
	// CAM_X, CAM_Y, CAM_Z,
	// CAM_WX, CAM_WY, CAM_WZ,
	// CAM_AX, CAM_AY, CAM_AZ,
	// CAM_DEG, CAM_ZOM, CAM_PER, CAM_VZDEF
	pub pos: Pos3,
	pub _1: u16,
	pub angle: u16,
	pub pos2: Pos3,
	pub pos3: Pos3,
	pub zoom: i32,
	pub fov: i32,
	pub angle1: i16,
	pub angle2: i16,
	pub angle3: i16,
	pub _2: u16,
	pub _3: u16,
	pub _4: u16,
	pub _5: u16,
	pub _6: u32,
	pub _7: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scena {
	pub dir: String,
	pub fname: String,
	pub town: TownId,
	pub bgm: BgmId,
	pub entry_func: FuncRef, // Other funcrefs are (-1, -1) when null, but this one is (0, -1).
	pub includes: Vec<String>,
	pub ch: Vec<String>,
	pub cp: Vec<String>,
	pub npcs: Vec<Npc>,
	pub monsters: Vec<Monster>,
	pub triggers: Vec<Trigger>,
	pub objects: Vec<Object>,
	pub camera_angles: Vec<CameraAngle>,
	pub functions: Vec<Vec<u8>>,
}

pub trait InExt2<'a>: In<'a> {
	fn func_ref(&mut self) -> Result<FuncRef, ReadError> {
		Ok(FuncRef(self.u16()?, self.u16()?))
	}

	fn pos2(&mut self) -> Result<Pos2, ReadError> {
		Ok(Pos2(self.i32()?, self.i32()?))
	}

	fn pos3(&mut self) -> Result<Pos3, ReadError> {
		Ok(Pos3(self.i32()?, self.i32()?, self.i32()?))
	}
}
impl<'a, T: In<'a>> InExt2<'a> for T {}

pub trait OutExt2: Out {
	fn func_ref(&mut self, fr: FuncRef) {
		self.u16(fr.0);
		self.u16(fr.1);
	}

	fn pos2(&mut self, p: Pos2) {
		self.i32(p.0);
		self.i32(p.1);
	}

	fn pos3(&mut self, p: Pos3) {
		self.i32(p.0);
		self.i32(p.1);
		self.i32(p.2);
	}
}
impl<T: Out> OutExt2 for T {}

pub fn read(arc: &Archives, data: &[u8]) -> Result<Scena, ReadError> {
	let mut f = Coverage::new(Bytes::new(data));

	let dir = f.sized_string::<10>()?;
	let fname = f.sized_string::<14>()?;
	let town = TownId(f.u16()?);
	let bgm = BgmId(f.u8()?);
	f.check_u8(0)?;
	let entry_func = f.func_ref()?;
	let includes = f.multiple::<8, _>(&[0xFF;4], |g| Ok(arc.name(g.array()?)?.to_owned()))?;
	f.check_u16(0)?;

	let head_end = f.clone().u16()? as usize;

	let ch       = (f.ptr()?, f.u16()?);
	let cp       = (f.ptr()?, f.u16()?);
	let npcs     = (f.ptr()?, f.u16()?);
	let monsters = (f.ptr()?, f.u16()?);
	let triggers = (f.ptr()?, f.u16()?);
	let objects  = (f.ptr()?, f.u16()?);

	let mut strings = f.ptr()?;

	let code_start = f.u16()? as usize;
	f.check_u16(0)?;
	let func_table = (f.ptr()?, f.u16()? / 2);
	let code_end = func_table.0.pos();

	ensure!(strings.string()? == "@FileName", "expected @FileName");

	let (mut g, n) = ch;
	let ch = list(n as usize, || Ok(arc.name(g.array()?)?.to_owned())).strict()?;
	g.check_u8(0xFF)?;

	let (mut g, n) = cp;
	let cp = list(n as usize, || Ok(arc.name(g.array()?)?.to_owned())).strict()?;
	g.check_u8(0xFF)?;

	let (mut g, n) = npcs;
	let npcs = list(n as usize, || Ok(Npc {
		name: strings.string()?,
		pos: g.pos3()?,
		angle: g.u16()?,
		ch: (g.u16()?, g.u16()?),
		cp: (g.u16()?, g.u16()?),
		flags: g.u16()?,
		init: g.func_ref()?,
		talk: g.func_ref()?,
	})).strict()?;

	let (mut g, n) = monsters;
	let monsters = list(n as usize, || Ok(Monster {
		name: strings.string()?,
		pos: g.pos3()?,
		angle: g.u16()?,
		_1: g.u16()?,
		flags: g.u16()?,
		_2: g.i32()?,
		battle: BattleId(g.u16()?),
		flag: Flag(g.u16()?),
		_3: g.u16()?,
	})).strict()?;

	let (mut g, n) = triggers;
	let triggers = list(n as usize, || Ok(Trigger {
		pos1: g.pos3()?,
		pos2: g.pos3()?,
		flags: g.u16()?,
		func: g.func_ref()?,
		_1: g.u16()?,
	})).strict()?;

	let (mut g, n) = objects;
	let objects = list(n as usize, || Ok(Object {
		pos: g.pos3()?,
		radius: g.u32()?,
		bubble_pos: g.pos3()?,
		flags: g.u16()?,
		func: g.func_ref()?,
		_1: g.u16()?,
	})).strict()?;

	let (mut g, n) = func_table;
	let func_table = list(n as usize, || Ok(g.u16()? as usize)).strict()?;
	ensure!(func_table.is_empty() || func_table[0] == code_start,
		"Unexpected func table: {func_table:X?} does not start with {code_start:X?}"
	);

	let mut camera_angles = Vec::new();
	while f.pos() < head_end {
		camera_angles.push(CameraAngle {
			pos: f.pos3()?,
			_1: f.u16()?,
			angle: f.u16()?,
			pos2: f.pos3()?,
			pos3: f.pos3()?,
			zoom: f.i32()?,
			fov: f.i32()?,
			angle1: f.i16()?,
			angle2: f.i16()?,
			angle3: f.i16()?,
			_2: f.u16()?,
			_3: f.u16()?,
			_4: f.u16()?,
			_5: f.u16()?,
			_6: f.u32()?,
			_7: f.u16()?,
		});
	}
	ensure!(f.pos() == head_end, "overshot with camera angles");

	let mut functions = Vec::with_capacity(func_table.len());
	let starts = func_table.iter().copied();
	let ends = func_table.iter().copied().skip(1).chain(std::iter::once(code_end));
	for (start, end) in starts.zip(ends) {
		let code = code::read_func(&mut f.clone().at(start)?, arc, end)?;
		// println!("{:#?}", code);
		let mut g = f.clone().at(start)?;
		let slice = g.slice(end-start)?;
		functions.push(slice.to_owned());
	}

	f.assert_covered()?;

	Ok(Scena {
		dir, fname,
		town, bgm,
		entry_func,
		includes,
		ch, cp,
		npcs, monsters,
		triggers, objects,
		camera_angles,
		functions,
	})
}

pub fn write(arc: &Archives, scena: &Scena) -> Result<Vec<u8>, WriteError> {
	let &Scena {
		ref dir,
		ref fname,
		town,
		bgm,
		entry_func,
		ref includes,
		ref ch,
		ref cp,
		ref npcs,
		ref monsters,
		ref triggers,
		ref objects,
		ref camera_angles,
		ref functions,
	} = scena;
	let mut f = OutBytes::new();
	let mut g = OutBytes::new();
	let mut func_table = OutBytes::new();
	let mut strings = OutBytes::new();
	let mut count = Count::new();

	f.sized_string::<10>(dir)?;
	f.sized_string::<14>(fname)?;
	f.u16(town.0);
	f.u8(bgm.0);
	f.u8(0);
	f.func_ref(entry_func);
	f.multiple::<8, _>(&[0xFF; 4], includes, |g, a| { g.array(arc.index(a)?); Ok(()) }).strict()?;
	f.u16(0);

	let l_ch = count.next();
	f.delay_u16(l_ch);
	f.u16(cast(ch.len())?);

	let l_cp = count.next();
	f.delay_u16(l_cp);
	f.u16(cast(cp.len())?);

	let l_npcs = count.next();
	f.delay_u16(l_npcs);
	f.u16(cast(npcs.len())?);

	let l_monsters = count.next();
	f.delay_u16(l_monsters);
	f.u16(cast(monsters.len())?);

	let l_triggers = count.next();
	f.delay_u16(l_triggers);
	f.u16(cast(triggers.len())?);

	let l_objects = count.next();
	f.delay_u16(l_objects);
	f.u16(cast(objects.len())?);

	let l_strings = count.next();
	f.delay_u16(l_strings);
	strings.label(l_strings);
	strings.string("@FileName")?;

	let l_code_start = count.next();
	f.delay_u16(l_code_start);
	f.u16(0);
	let l_func_table = count.next();
	f.delay_u16(l_func_table);
	f.u16(cast(functions.len() * 2)?);

	g.label(l_ch);
	for ch in ch { g.array(arc.index(ch)?); }
	g.u8(0xFF);

	g.label(l_cp);
	for cp in cp { g.array(arc.index(cp)?); }
	g.u8(0xFF);

	g.label(l_npcs);
	for &Npc { ref name, pos, angle, ch, cp, flags, init, talk } in npcs {
		strings.string(name)?;
		g.pos3(pos);
		g.u16(angle);
		g.u16(ch.0); g.u16(ch.1);
		g.u16(cp.0); g.u16(cp.1);
		g.u16(flags);
		g.func_ref(init);
		g.func_ref(talk);
	}

	g.label(l_monsters);
	for &Monster { ref name, pos, angle, _1, flags, _2, battle, flag, _3 } in monsters {
		strings.string(name)?;
		g.pos3(pos);
		g.u16(angle);
		g.u16(_1);
		g.u16(flags);
		g.i32(_2);
		g.u16(battle.0);
		g.u16(flag.0);
		g.u16(_3);
	}

	g.label(l_triggers);
	for &Trigger { pos1, pos2, flags, func, _1 } in triggers {
		g.pos3(pos1);
		g.pos3(pos2);
		g.u16(flags);
		g.func_ref(func);
		g.u16(_1);
	}

	g.label(l_objects);
	for &Object { pos, radius, bubble_pos, flags, func, _1 } in objects {
		g.pos3(pos);
		g.u32(radius);
		g.pos3(bubble_pos);
		g.u16(flags);
		g.func_ref(func);
		g.u16(_1);
	}

	func_table.label(l_func_table);
	g.label(l_code_start);
	for func in functions {
		let l_func = count.next();
		func_table.delay_u16(l_func);
		g.label(l_func);
		g.slice(func);
	}

	for &CameraAngle { pos, _1, angle, pos2, pos3, zoom, fov, angle1, angle2, angle3, _2, _3, _4, _5, _6, _7 } in camera_angles {
		f.pos3(pos);
		f.u16(_1);
		f.u16(angle);
		f.pos3(pos2);
		f.pos3(pos3);
		f.i32(zoom);
		f.i32(fov);
		f.i16(angle1);
		f.i16(angle2);
		f.i16(angle3);
		f.u16(_2);
		f.u16(_3);
		f.u16(_4);
		f.u16(_5);
		f.u32(_6);
		f.u16(_7);
	}

	Ok(f.concat(g).concat(func_table).concat(strings).finish()?)
}

#[cfg(test)]
mod test {
	use crate::archive::Archives;
	use crate::util::test::*;

	#[test_case::test_case(&FC, 0x01; "fc")]
	fn roundtrip(arc: &Archives, scena_archive: u16) -> Result<(), Error> {
		let mut failed = false;

		for e in arc.archive(scena_archive)?.entries() {
			if e.is_empty() { continue }
			if let Err(err) = check_roundtrip_strict(arc, &e.name, super::read, super::write) {
				// println!("{}: {err:#?}", &e.name);
				println!("{}: {err}", &e.name);
				failed = true;
			};
		}

		assert!(!failed);
		Ok(())
	}
}
