use eyre::Result;
use hamu::read::{In, Le};
use crate::util::{self, ByteString, InExt};
use super::code::{FileRef, FuncRef, Pos3, Code, CodeParser};

#[derive(Debug, Clone)]
pub struct Npc {
	pub name: String,
	pub pos: Pos3,
	pub angle: u16,
	pub ch: (u16, u16), // First entry seems to always be zero. Probably include index, just like for functions
	pub cp: (u16, u16),
	pub flags: u16,
	pub func1: FuncRef, // I think one of them is idle and one is speak
	pub func2: FuncRef,
}

#[derive(Debug, Clone)]
pub struct Monster {
	pub name: String,
	pub pos: Pos3,
	pub angle: u16,
	pub _1: u16, // this is likely related to chcp, but not sure
	pub flags: u16,
	pub _2: i32, // Is this maybe a funcref? It's always -1
	pub battle: u16, // T_BATTLE index
	pub flag: u16, // set when defeated
	pub _3: u16,
}

#[derive(Debug, Clone)]
pub struct Trigger {
	pub pos1: Pos3,
	pub pos2: Pos3,
	pub flags: u16,
	pub func: FuncRef,
	pub _1: u16,
}

#[derive(Debug, Clone)]
pub struct Object {
	pub pos: Pos3,
	pub radius: u32,
	pub bubble_pos: Pos3,
	pub flags: u16,
	pub func: FuncRef,
	pub _1: u16,
}

// I'm not 100% sure, but I *think* this one has to do with camera
#[derive(Debug, Clone)]
pub struct CameraAngle {
	pub pos: Pos3,
	pub _1: u32,
	pub _2: u32,
	pub _3: u32,
	pub _4: i32,
	pub _5: u32,
	pub _6: u32,
	pub _7: u32,
	pub _8: u32,
	pub _9: u32,
	pub _10: u32,
	pub _11: u32,
	pub _12: [u16; 8],
}

#[derive(Debug, Clone)]
pub struct Scena {
	pub dir: ByteString<10>,
	pub fname: ByteString<14>,
	pub town: u16, // T_TOWN index
	pub bgm: u16, // T_BGMTBL index
	pub entry_func: FuncRef,
	pub includes: [FileRef; 8],
	pub ch: Vec<FileRef>,
	pub cp: Vec<FileRef>,
	pub npcs: Vec<Npc>,
	pub monsters: Vec<Monster>,
	pub triggers: Vec<Trigger>,
	pub objects: Vec<Object>,
	pub camera_angles: Vec<CameraAngle>,
	pub code: Vec<Code>,
}

#[extend::ext(name=InExtForScena)]
impl In<'_> {
	fn file_ref(&mut self) -> hamu::read::Result<FileRef> {
		Ok(FileRef(self.u16()?, self.u16()?))
	}

	fn func_ref(&mut self) -> hamu::read::Result<FuncRef> {
		Ok(FuncRef(self.u16()?, self.u16()?))
	}

	fn pos3(&mut self) -> hamu::read::Result<Pos3> {
		Ok(Pos3(self.i32()?, self.i32()?, self.i32()?))
	}
}

pub fn read(i: &[u8]) -> Result<Scena> {
	let mut i = In::new(i);

	let dir = i.bytestring::<10>()?;
	let fname = i.bytestring::<14>()?;
	let town = i.u16()?;
	let bgm = i.u16()?; // T_BGMTBL index
	let entry_func = i.func_ref()?;
	let includes = [
		i.file_ref()?, i.file_ref()?, i.file_ref()?, i.file_ref()?,
		i.file_ref()?, i.file_ref()?, i.file_ref()?, i.file_ref()?,
	];
	i.check_u16(0)?;

	let head_end = i.clone().u16()? as usize;

	let ch       = (i.ptr_u16()?, i.u16()?);
	let cp       = (i.ptr_u16()?, i.u16()?);
	let npcs     = (i.ptr_u16()?, i.u16()?);
	let monsters = (i.ptr_u16()?, i.u16()?);
	let triggers = (i.ptr_u16()?, i.u16()?);
	let objects  = (i.ptr_u16()?, i.u16()?);

	let mut strings = i.ptr_u16()?;
	let code_start = i.u16()? as usize;
	i.check_u16(0)?;
	let code_end = i.clone().u16()? as usize;
	let func_table = (i.ptr_u16()?, i.u16()? / 2);

	eyre::ensure!(strings.str()? == "@FileName", stringify!(strings.str()? == "@FileName"));

	let ch = chcp_list(ch)?;
	let cp = chcp_list(cp)?;

	let npcs = list(npcs, |i| Ok(Npc {
		name: strings.str()?,
		pos: i.pos3()?,
		angle: i.u16()?,
		ch: (i.u16()?, i.u16()?),
		cp: (i.u16()?, i.u16()?),
		flags: i.u16()?,
		func1: i.func_ref()?,
		func2: i.func_ref()?,
	}))?;

	let monsters = list(monsters, |i| Ok(Monster {
		name: strings.str()?,
		pos: i.pos3()?,
		angle: i.u16()?,
		_1: i.u16()?,
		flags: i.u16()?,
		_2: i.i32()?,
		battle: i.u16()?,
		flag: i.u16()?,
		_3: i.u16()?,
	}))?;

	let triggers = list(triggers, |i| Ok(Trigger {
		pos1: i.pos3()?,
		pos2: i.pos3()?,
		flags: i.u16()?,
		func: i.func_ref()?,
		_1: i.u16()?,
	}))?;

	let objects = list(objects, |i| Ok(Object {
		pos: i.pos3()?,
		radius: i.u32()?,
		bubble_pos: i.pos3()?,
		flags: i.u16()?,
		func: i.func_ref()?,
		_1: i.u16()?,
	}))?;

	let func_table = list(func_table, |i| Ok(i.u16()? as usize))?;
	eyre::ensure!(func_table.is_empty() || func_table[0] == code_start,
		"Unexpected func table: {:X?} does not start with {:X?}", func_table, code_start);

	let mut camera_angles = Vec::new();
	while i.pos() < head_end {
		camera_angles.push(CameraAngle {
			pos: i.pos3()?,
			_1: i.u32()?,
			_2: i.u32()?,
			_3: i.u32()?,
			_4: i.i32()?,
			_5: i.u32()?,
			_6: i.u32()?,
			_7: i.u32()?,
			_8: i.u32()?,
			_9: i.u32()?,
			_10: i.u32()?,
			_11: i.u32()?,
			_12: [i.u16()?, i.u16()?, i.u16()?, i.u16()?, i.u16()?, i.u16()?, i.u16()?, i.u16()?]
		});
	}

	let mut codeparser = CodeParser::new(i.clone());
	let code = util::multiple(&i, &func_table, code_end, |i, len| {
		codeparser.seek(i.pos())?;
		Ok(codeparser.read_func(i.pos() + len)?)
	})?;

	i.dump_uncovered(|a| a.to_stderr())?;

	Ok(Scena {
		dir, fname,
		town, bgm,
		entry_func,
		includes,
		ch, cp,
		npcs, monsters,
		triggers, objects,
		camera_angles,
		code,
	})
}

fn chcp_list((mut i, count): (In, u16)) -> Result<Vec<FileRef>> {
	let mut out = Vec::with_capacity(count as usize);
	for _ in 0..count {
		out.push(i.file_ref()?);
	}
	i.check_u8(0xFF)?; // Nope, no idea what this is for
	Ok(out)
}

fn list<A>((mut i, count): (In, u16), mut f: impl FnMut(&mut In) -> Result<A>) -> Result<Vec<A>> {
	let mut out = Vec::with_capacity(count as usize);
	for _ in 0..count {
		out.push(f(&mut i)?);
	}
	Ok(out)
}
