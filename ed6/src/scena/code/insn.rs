use super::*;

ed6_macros::bytecode! {
	|arc: &GameData|
	#[games(arc.insn_set() => crate::gamedata::InstructionSet::{Fc, FcEvo})]
	[
		skip!(1), // null
		Return(),
		skip!(3), // control flow
		Call(func_ref() -> FuncRef),
		NewScene(file_ref(arc) -> String alias ScenaFileRef, u8, u8, u8, u8),
		skip!(1),
		Sleep(u32 alias Time),
		FlagsSet(u32 as Flags),
		FlagsUnset(u32 as Flags),
		FadeOn(u32 alias Time, u32 as Color, u8),
		FadeOff(u32 alias Time, u32 as Color),
		_0D(),
		Blur(u32 alias Time),
		Battle(u16 as BattleId, u16, u16, u16, u8, u16, i8),
		ExitSetEnabled(u8 alias ExitId, u8),
		Fog(u8, u8, u8, u32, u32, u32), // First three are color; TODO parse it as one. Last is always 0.
		_12(i32, i32, u32),
		PlaceSetName(u16 as TownId),
		skip!(2),
		Map(match {
			0x00 => Hide(),
			0x01 => Show(),
			0x02 => Set(i32, Pos2, file_ref(arc) -> String alias MapFileRef),
		}),
		Save(),
		skip!(1),
		EventBegin(u8),
		EventEnd(u8),
		_1B(u16, u16),
		_1C(u16, u16),
		BgmPlay(u8 as BgmId),
		_1E(),
		BgmSetVolume(u8, u32 alias Time),
		BgmStop(u32 alias Time),
		BgmFadeWait(),
		SoundPlay(u16 as SoundId, u8, u8),
		SoundStop(u16 as SoundId),
		SoundLoop(u16 as SoundId, u8),
		_Sound25(u16 as SoundId, Pos3, u32, u32, u8, u32),
		_Sound26(u16 as SoundId),
		skip!(1),
		Quest(u16 as QuestId, match {
			0x01 => TaskSet(u16 alias QuestTask),
			0x02 => TaskUnset(u16 alias QuestTask),
			0x03 => FlagsSet(u8 as QuestFlags),
			0x04 => FlagsUnset(u8 as QuestFlags),
		}),
		Quest(u16 as QuestId, match {
			0x00 => FlagsGet(u8 as QuestFlags),
			0x01 => TaskGet(u16 alias QuestTask),
		}),
		QuestList(quest_list() -> Vec<QuestId> alias QuestList),
		QuestBonusBp(u16 as QuestId, u16),
		QuestBonusMira(u16 as QuestId, u16),
		PartyAdd(u8 as Member, u8),
		PartyRemove(u8 as Member, u8),
		skip!(1),
		_Party30(u8),
		PartySetAttr(u8 as Member, u8 as MemberAttr, u16),
		skip!(2),
		PartyAddArt(u8 as Member, u16 as MagicId),
		PartyAddCraft(u8 as Member, u16 as MagicId),
		PartyAddSCraft(u8 as Member, u16 as MagicId),
		PartySet(u8 as Member, u8, u8),
		SepithAdd(u8 as Element alias SepithElement, u16),
		SepithRemove(u8 as Element alias SepithElement, u16),
		MiraAdd(u16),
		MiraSub(u16),
		BpAdd(u16),
		skip!(1), // I have a guess what this is, but it doesn't exist in any scripts
		ItemAdd(u16 as ItemId, u16),
		ItemRemove(u16 as ItemId, u16),
		ItemHas(u16 as ItemId), // or is it ItemGetCount?
		PartyEquip(u8 as Member, u16 as ItemId, party_equip_slot(_1) -> i8),
		PartyPosition(u8 as Member),
		CharForkFunc(u16 as CharId, u8 alias ForkId, func_ref() -> FuncRef),
		CharForkQuit(u16 as CharId, u8 alias ForkId),
		CharFork(u16 as CharId, u8 alias ForkId, u8, fork(arc) -> Vec<Insn> alias Fork),
		CharForkLoop(u16 as CharId, u8 alias ForkId, u8, fork_loop(arc) -> Vec<Insn> alias Fork),
		CharForkAwait(u16 as CharId, u8 alias ForkId, u8),
		Yield(), // Used in tight loops, probably wait until next frame
		Event(func_ref() -> FuncRef), // Not sure how this differs from Call
		_Char4A(u16 as CharId, u8), // Argument is almost always 255, but sometimes 0, and in a single case 1
		_Char4B(u16 as CharId, u8),
		skip!(1),
		Var(u16 as Var, expr(arc) -> Expr),
		skip!(1),
		Attr(u8 as Attr, expr(arc) -> Expr),
		skip!(1),
		CharAttr(char_attr() -> CharAttr, expr(arc) -> Expr),
		TextStart(u16 as CharId),
		TextEnd(u16 as CharId),
		TextMessage(text() -> Text),
		skip!(1),
		TextReset(u8),
		skip!(1),
		TextWait(),
		_59(), // Always directly after a TextReset 1, and exists in all but one such case. I suspect that one is a bug.
		TextSetPos(i16, i16, i16, i16),
		TextTalk(u16 as CharId, text() -> Text),
		TextTalkNamed(u16 as CharId, String alias TextTitle, text() -> Text),
		Menu(u16 alias MenuId, i16, i16, u8, menu() -> Vec<String> alias Menu),
		MenuWait(u16 as Var),
		MenuClose(u16 alias MenuId),
		TextSetName(String alias TextTitle),
		_61(u16 as CharId),
		Emote(u16 as CharId, i32, u32 alias Time, emote() -> Emote, u8),
		EmoteStop(u16 as CharId),
		_64(u8, u16), // I suspect these two are ObjectId, but ObjectId is u16?
		_65(u8, u16),
		CamFollow(u16), // A bool
		CamOffset(i32, i32, i32, u32 alias Time),
		skip!(1),
		CamLookAt(u16 as CharId, u32 alias Time),
		_Char6A(u16 as CharId),
		CamDistance(i32, u32 alias Time),
		CamAngle(i32 alias Angle32, u32 alias Time),
		CamPos(Pos3, u32 alias Time),
		_Cam6E(u8, u8, u8, u8, u32 alias Time),
		_Obj6F(u16 alias ObjectId, u32),
		_Obj70(u16 alias ObjectId, u32),
		_Obj71(u16 alias ObjectId, u16),
		_Obj72(u16 alias ObjectId, u16),
		_Obj73(u16 alias ObjectId),
		_74(u16, u32, u16),
		_75(u8, u32, u8),
		_76(u16, u32, u16, Pos3, u8, u8),
		_77(u32 as Color, u32 alias Time),
		_78(u8, u16),
		_79(u8, u16),
		_7A(u8, u16),
		_7B(),
		Shake(u32, u32, u32, u32 alias Time),
		skip!(1),
		_7E(i16, i16, u16, u8, u32),
		EffLoad(u8, String alias EffFileRef),
		EffPlay(u8, u8, i16, Pos3, u16, u16, u16, u32, u32, u32, u16, u32, u32, u32, u32),
		EffPlay2(u16, u8, String alias EffFileRef, Pos3, u16, u16, u16, u32, u32, u32, u32),
		_82(u16),
		Achievement(u8, u8),
		_84(u8),
		_85(u16),
		CharSetChcp   (u16 as CharId, u16 alias ChcpId),
		CharSetFrame  (u16 as CharId, u16),
		CharSetPos    (u16 as CharId, Pos3, u16 alias Angle),
		CharSetPos2   (u16 as CharId, Pos3, u16 alias Angle),
		CharLookAt    (u16 as CharId, u16 as CharId, u16 alias Time16),
		CharLookAtPos (u16 as CharId, Pos2, u16 alias Time16),
		CharSetAngle  (u16 as CharId, u16 alias Angle, u16 alias Time16),
		CharIdle      (u16 as CharId, Pos2, Pos2, u32 alias Speed),
		CharWalkTo    (u16 as CharId, Pos3, u32 alias Speed, u8),
		CharWalkTo2   (u16 as CharId, Pos3, u32 alias Speed, u8), // how are these two different?
		DontGoThere   (u16 as CharId, i32, i32, i32, u32, u8),
		_Char91       (u16 as CharId, i32, i32, i32, i32, u8),
		_Char92       (u16 as CharId, u16 as CharId, u32, u32 alias Speed, u8),
		_Char93       (u16 as CharId, u16 as CharId, u32, u32 alias Speed, u8),
		_94       (u8, u16 as CharId, u16 alias Angle, u32, u32 alias Speed, u8),
		CharJump      (u16 as CharId, i32, i32, i32, u32, u32),
		_Char96       (u16 as CharId, Pos3, i32, i32),
		_Char97       (u16 as CharId, Pos2, i32 alias Angle32, u32, u16), // used with pigeons
		skip!(1),
		CharAnimation (u16 as CharId, u8, u8, u32 alias Time),
		CharFlagsSet  (u16 as CharId, u16 as CharFlags),
		CharFlagsUnset(u16 as CharId, u16 as CharFlags),
		_Char9C       (u16 as CharId, u16), // always 32
		_Char9D       (u16 as CharId, u16),
		CharShake     (u16 as CharId, u32, u32, u32, u32),
		CharColor     (u16 as CharId, u32 as Color, u32 alias Time),
		skip!(1),
		_CharA1(u16 as CharId, u16),
		FlagSet(u16 as Flag),
		FlagUnset(u16 as Flag),
		skip!(1),
		FlagAwaitUnset(u16 as Flag),
		FlagAwaitSet(u16 as Flag),
		skip!(2),
		ShopOpen(u8 as ShopId),
		skip!(2),
		RecipeLearn(u16), // TODO check type
		ImageShow(file_ref(arc) -> String alias VisFileRef, u16, u16, u32 alias Time),
		ImageHide(u32 alias Time),
		QuestSubmit(u8 as ShopId, u16 as QuestId),
		_ObjB0(u16 alias ObjectId, u8), // Used along with 6F, 70, and 73 during T0700#11
		OpLoad(String alias OpFileRef),
		_B2(u8, u8, u16),
		Video(match {
			0x00 => _00(String alias AviFileRef),
			0x01 => _01(u8),
		}),
		ReturnToTitle(u8),
		// fc only region
		#[game(Fc)]
		PartySlot(u8 as Member, u8, u8),
		#[game(Fc)]
		_B6(u8),
		#[game(Fc)]
		_B7(u8 as Member, u8, u8), // Related to PartyAdd
		#[game(Fc)]
		_B8(u8 as Member), // Related to PartyRemove
		#[game(Fc)]
		ReadBook(u16 as ItemId, u16),
		#[game(Fc)]
		PartyHasSpell(u8 as Member, u16 as MagicId),
		#[game(Fc)]
		PartyHasSlot(u8 as Member, u8),
		#[game(Fc)]
		skip!(34),
		#[game(Fc)]
		SaveClearData(),
		#[game(Fc)]
		skip!(33),
	]
}

mod quest_list {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Vec<QuestId>, ReadError> {
		let mut quests = Vec::new();
		loop {
			match f.u16()? {
				0xFFFF => break,
				q => quests.push(QuestId(q))
			}
		}
		Ok(quests)
	}

	pub(super) fn write(f: &mut impl Out, v: &Vec<QuestId>) -> Result<(), WriteError> {
		for &i in v {
			f.u16(i.0);
		}
		f.u16(0xFFFF);
		Ok(())
	}
}

mod fork {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, arc: &GameData) -> Result<Vec<Insn>, ReadError> {
		let len = f.u8()? as usize;
		let pos = f.pos();
		let mut insns = Vec::new();
		while f.pos() < pos+len {
			insns.push(Insn::read(f, arc)?);
		}
		ensure!(f.pos() == pos+len, "overshot while reading fork");
		f.check_u8(0)?;
		Ok(insns)
	}

	pub(super) fn write(f: &mut impl OutDelay, arc: &GameData, v: &[Insn]) -> Result<(), WriteError> {
		let (l1, l1_) = HLabel::new();
		let (l2, l2_) = HLabel::new();
		f.delay(move |l| Ok(u8::to_le_bytes(hamu::write::cast_usize(l(l2)? - l(l1)?)?)));
		f.label(l1_);
		for i in v {
			Insn::write(f, arc, i)?;
		}
		f.label(l2_);
		f.u8(0);
		Ok(())
	}
}

mod fork_loop {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, arc: &GameData) -> Result<Vec<Insn>, ReadError> {
		let len = f.u8()? as usize;
		let pos = f.pos();
		let mut insns = Vec::new();
		while f.pos() < pos+len {
			insns.push(Insn::read(f, arc)?);
		}
		ensure!(f.pos() == pos+len, "overshot while reading fork loop");
		ensure!(read_raw_insn(f, arc)? == RawIInsn::Insn(Insn::Yield()), "invalid loop");
		ensure!(read_raw_insn(f, arc)? == RawIInsn::Goto(pos), "invalid loop");
		Ok(insns)
	}

	pub(super) fn write(f: &mut impl OutDelay, arc: &GameData, v: &[Insn]) -> Result<(), WriteError> {
		let (l1, l1_) = HLabel::new();
		let (l2, l2_) = HLabel::new();
		let l1c = l1.clone();
		f.delay(|l| Ok(u8::to_le_bytes(hamu::write::cast_usize(l(l2)? - l(l1)?)?)));
		f.label(l1_);
		for i in v {
			Insn::write(f, arc, i)?;
		}
		f.label(l2_);
		write_raw_insn(f, arc, RawOInsn::Insn(&Insn::Yield()))?;
		write_raw_insn(f, arc, RawOInsn::Goto(l1c))?;
		Ok(())
	}
}

mod party_equip_slot {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, arg1: &ItemId) -> Result<i8, ReadError> {
		if (600..800).contains(&arg1.0) {
			Ok(f.i8()?)
		} else {
			Ok(-1)
		}
	}

	pub(super) fn write(f: &mut impl Out, arg1: &ItemId, v: &i8) -> Result<(), WriteError> {
		if (600..800).contains(&arg1.0) {
			f.i8(*v);
		} else {
			ensure!(*v == -1, "invalid PartyEquipSlot");
		}
		Ok(())
	}
}

mod menu {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Vec<String>, ReadError> {
		Ok(f.string()?.split_terminator('\x01').map(|a| a.to_owned()).collect())
	}

	pub(super) fn write(f: &mut impl Out, v: &[String]) -> Result<(), WriteError> {
		let mut s = String::new();
		for line in v {
			s.push_str(line.as_str());
			s.push('\x01');
		}
		f.string(&s)?;
		Ok(())
	}
}

mod emote {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Emote, ReadError> {
		let a = f.u8()?;
		let b = f.u8()?;
		let c = f.u32()?;
		Ok(Emote(a, b, c))
	}

	pub(super) fn write(f: &mut impl Out, &Emote(a, b, c): &Emote) -> Result<(), WriteError> {
		f.u8(a);
		f.u8(b);
		f.u32(c);
		Ok(())
	}
}

pub(super) mod char_attr {
	use super::*;
	pub fn read<'a>(f: &mut impl In<'a>) -> Result<CharAttr, ReadError> {
		let a = CharId(f.u16()?);
		let b = f.u8()?;
		Ok(CharAttr(a, b))
	}

	pub fn write(f: &mut impl Out, &CharAttr(a, b): &CharAttr) -> Result<(), WriteError> {
		f.u16(a.0);
		f.u8(b);
		Ok(())
	}
}

mod file_ref {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, arc: &GameData) -> Result<String, ReadError> {
		Ok(arc.name(f.u32()?)?.to_owned())
	}

	pub(super) fn write(f: &mut impl Out, arc: &GameData, v: &str) -> Result<(), WriteError> {
		f.u32(arc.index(v)?);
		Ok(())
	}
}

mod func_ref {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<FuncRef, ReadError> {
		let a = f.u8()? as u16;
		let b = f.u16()?;
		Ok(FuncRef(a, b))
	}

	pub(super) fn write(f: &mut impl Out, &FuncRef(a, b): &FuncRef) -> Result<(), WriteError> {
		f.u8(cast(a)?);
		f.u16(b);
		Ok(())
	}
}

mod text {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Text, ReadError> {
		crate::text::Text::read(f)
	}

	pub(super) fn write(f: &mut impl Out, v: &Text) -> Result<(), WriteError> {
		crate::text::Text::write(f, v)
	}
}
