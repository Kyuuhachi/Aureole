use super::*;

ed6_macros::bytecode! {
	(iset: InstructionSet, lookup: &dyn Lookup)
	#[games(iset => InstructionSet::{Fc, FcEvo, Sc})]
	[
		skip!(1), // null
		Return(), // [return]
		skip!(3), // control flow
		Call(func_ref() -> FuncRef), // [call]
		NewScene(file_ref(lookup) -> String alias ScenaFileRef, u8, u8, u8, u8), // [new_scene] (last two args are unaccounted for)
		skip!(1),
		Sleep(u32 alias Time), // [delay]
		SystemFlagsSet(u32 as SystemFlags), // [set_system_flag]
		SystemFlagsUnset(u32 as SystemFlags), // [reset_system_flag]
		FadeOut(u32 alias Time, u32 as Color, u8), // [fade_out]
		FadeIn(u32 alias Time, u32 as Color), // [fade_in]
		FadeWait(), // [fade_wait]
		CrossFade(u32 alias Time), // [cross_fade]
		Battle(u16 as BattleId, u16, u16, u16, u8, u16, i8),
		ExitSetEnabled(u8 alias ExitId, u8),
		Fog(u8, u8, u8, u32, u32, u32), // First three are color; TODO parse it as one. Last is always 0.
		_12(i32, i32, u32),
		PlaceSetName(u16 as TownId),
		Sc_14(u32, u32, u32, u32, u8),
		Sc_15(u32),
		Map(match {
			0x00 => Hide(),
			0x01 => Show(),
			0x02 => Set(i32, Pos2, file_ref(lookup) -> String alias MapFileRef),
		}),
		#[game(Fc, Sc)] Save(),
		#[game(FcEvo)] SaveEvo(u8),
		Sc_18(u8, u8, u8),
		EventBegin(u8), // [event_begin]
		EventEnd(u8), // [event_end]
		_1B(u16, u16),
		_1C(u16, u16),
		BgmPlay(u8 as BgmId), // [play_bgm]
		BgmResume(), // [resume_bgm]
		BgmVolume(u8, u32 alias Time), // [volume_bgm]
		BgmStop(u32 alias Time), // [stop_bgm]
		BgmWait(), // [wait_bgm]
		SoundPlay(u16 as SoundId, u8, u8), // [sound]
		SoundStop(u16 as SoundId),
		SoundLoop(u16 as SoundId, u8),
		_Sound25(u16 as SoundId, Pos3, u32, u32, u8, u32),
		SoundLoad(u16 as SoundId), // [sound_load]
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
		PartyAdd(u8 as Member, u8, u8_not_in_fc(iset) -> u8), // [join_party]
		PartyRemove(u8 as Member, u8), // [separate_party]
		ScPartyClear(),
		_Party30(u8 as Member),
		PartySetAttr(u8 as Member, u8 as MemberAttr, u16), // [set_status]
		skip!(2),
		PartyAddArt(u8 as Member, u16 as MagicId),
		PartyAddCraft(u8 as Member, u16 as MagicId),
		PartyAddSCraft(u8 as Member, u16 as MagicId),
		PartySetSlot(u8 as Member, u8, party_set_slot(_1) -> i8),
		SepithAdd(u8 as Element alias SepithElement, u16),
		SepithRemove(u8 as Element alias SepithElement, u16),
		MiraAdd(u16), // [get_gold]
		MiraSub(u16),
		BpAdd(u16),
		skip!(1), // I have a guess what this is, but it doesn't exist in any scripts
		ItemAdd(u16 as ItemId, u16),
		ItemRemove(u16 as ItemId, u16), // [release_item]
		ItemHas(u16 as ItemId, u8_not_in_fc(iset) -> u8), // or is it ItemGetCount?
		PartyEquip(u8 as Member, u16 as ItemId, party_equip_slot(iset, _1) -> i8),
		PartyPosition(u8 as Member),
		ForkFunc(u16 as CharId, u8 alias ForkId, func_ref() -> FuncRef), // [execute]
		ForkQuit(u16 as CharId, u8 alias ForkId), // [terminate]
		Fork(u16 as CharId, u8 alias ForkId, u8, fork(iset, lookup) -> Vec<Insn> alias Fork), // [preset]? In t0311, only used with a single instruction inside
		ForkLoop(u16 as CharId, u8 alias ForkId, u8, fork_loop(iset, lookup) -> Vec<Insn> alias Fork),
		ForkAwait(u16 as CharId, u8 alias ForkId, u8), // [wait_terminate]
		NextFrame(), // [next_frame]
		Event(func_ref() -> FuncRef), // [event] Not sure how this differs from Call
		_Char4A(u16 as CharId, u8), // Argument is almost always 255, but sometimes 0, and in a single case 1
		_Char4B(u16 as CharId, u8),
		skip!(1),
		Var(u16 as Var, expr(iset, lookup) -> Expr),
		skip!(1),
		Attr(u8 as Attr, expr(iset, lookup) -> Expr), // [system[n]]
		skip!(1),
		CharAttr(char_attr() -> CharAttr, expr(iset, lookup) -> Expr),
		TextStart(u16 as CharId), // [talk_start]
		TextEnd(u16 as CharId), // [talk_end]
		TextMessage(text() -> Text), // [mes]
		skip!(1),
		TextClose(u8), // [mes_close]
		Sc_57(u32, u16, text() -> Text),
		TextWait(), // [wait_prompt]
		_59(), // Always directly after a TextReset 1, and exists in all but one such case. I suspect that one is a bug.
		TextSetPos(i16, i16, i16, i16), // [mes_pos]
		TextTalk(u16 as CharId, text() -> Text), // [popup]
		TextTalkNamed(u16 as CharId, String alias TextTitle, text() -> Text), // [popup2]
		Menu(u16 alias MenuId, i16, i16, u8, menu() -> Vec<String> alias Menu), // [menu] (the u8 is a bool)
		MenuWait(u16 as Var), // [wait_menu]
		MenuClose(u16 alias MenuId), // [menu_close]
		TextSetName(String alias TextTitle), // [name]
		CharName2(u16 as CharId), // [name2]
		Emote(u16 as CharId, i32, u32 alias Time, emote() -> Emote, u8), // [emotion] mostly used through macros such as EMO_BIKKURI3()
		EmoteStop(u16 as CharId), // [emotion_close]
		_64(u8 as u16 alias ObjectId, u16),
		_65(u8 as u16 alias ObjectId, u16),
		CamChangeAxis(u16), // [camera_change_axis] 0 CAMERA_ABSOLUTE_MODE, 1 CAMERA_RELATIVE_MODE
		CamMove(i32, i32, i32, u32 alias Time), // [camera_move]
		skip!(1),
		CamLookChar(u16 as CharId, u32 alias Time), // [camera_look_chr]
		_Char6A(u16 as CharId),
		CamZoom(i32, u32 alias Time), // [camera_zoom]
		CamRotate(i32 alias Angle32, u32 alias Time), // [camera_rotate]
		CamLookPos(Pos3, u32 alias Time), // [camera_look_at]
		CamPers(u32, u32 alias Time), // [camera_pers]
		ObjFrame(u16 alias ObjectId, u32), // [mapobj_frame]
		ObjPlay(u16 alias ObjectId, u32), // [mapobj_play]
		ObjFlagsSet(u16 alias ObjectId, u16 as ObjectFlags), // [mapobj_set_flag]
		ObjFlagsUnset(u16 alias ObjectId, u16 as ObjectFlags), // [mapobj_reset_flag]
		_Obj73(u16 alias ObjectId),
		_74(u16, u32, u16),
		_75(u8, u32, u8),
		_76(u16, u32, u16, Pos3, u8, u8),
		MapColor(u32 as Color /*24*/, u32 alias Time), // [map_color]
		_78(u8, u16),
		_79(u8, u16),
		_7A(u8, u16),
		_7B(),
		Shake(u32, u32, u32, u32 alias Time), // [quake]
		Sc_7D(u8, u8, u8, u8, u8, u16),
		_7E(i16, i16, u16, u8, u32),
		EffLoad(u8, String alias EffFileRef),
		EffPlay(u8, u8, i16, Pos3, u16, u16, u16, u32, u32, u32, u16, u32, u32, u32, u32),
		EffPlay2(u16, u8, String alias EffFileRef, Pos3, u16, u16, u16, u32, u32, u32, u32),
		_82(u16),
		Achievement(u8, u8),
		_84(u8),
		_85(u16),
		CharSetBase    (u16 as CharId, u16), // [set_chr_base]
		CharSetPattern (u16 as CharId, u16), // [set_chr_ptn]
		CharSetPos     (u16 as CharId, Pos3, i16 alias Angle), // [set_pos]
		CharSetPos2    (u16 as CharId, Pos3, i16 alias Angle),
		CharLookAtChar (u16 as CharId, u16 as CharId, u16 alias Time16), // [look_to]
		CharLookAtPos  (u16 as CharId, Pos2, u16 alias Time16),
		CharTurn       (u16 as CharId, i16 alias Angle, u16 alias Time16), // [turn_to]
		CharIdle       (u16 as CharId, Pos2, Pos2, u32 alias Speed),
		CharWalkToPos  (u16 as CharId, Pos3, u32 alias Speed, u8), // [walk_to]
		CharWalkToPos2 (u16 as CharId, Pos3, u32 alias Speed, u8),
		_Char90        (u16 as CharId, i32, i32, i32, u32, u8),
		_Char91        (u16 as CharId, i32, i32, i32, i32, u8),
		CharWalkToChar (u16 as CharId, u16 as CharId, u32, u32 alias Speed, u8), // [walk_to_chr]
		CharWalkToChar2(u16 as CharId, u16 as CharId, u32, u32 alias Speed, u8),
		_94        (u8, u16 as CharId, i16 alias Angle, u32, u32 alias Speed, u8),
		CharJump       (u16 as CharId, i32, i32, i32, u32, u32 alias Speed), // [jump]
		_Char96        (u16 as CharId, Pos3, i32, i32),
		_Char97        (u16 as CharId, Pos2, i32 alias Angle32, u32, u16), // used with pigeons
		Sc_Char98(match {
			0 => _0(u16 as CharId),
			1 => _1(Pos3),
			2 => _2(u16 as CharId, u32, u8),
		}),
		CharAnimation  (u16 as CharId, u8, u8, u32 alias Time), // [chr_anime]
		CharFlagsSet   (u16 as CharId, u16 as CharFlags), // [set_state]
		CharFlagsUnset (u16 as CharId, u16 as CharFlags), // [reset_state]
		_Char9C        (u16 as CharId, u16), // always 32
		_Char9D        (u16 as CharId, u16),
		CharShake      (u16 as CharId, u32, u32, u32, u32),
		CharColor      (u16 as CharId, u32 as Color, u32 alias Time),
		Sc_A0(u8,u8,u8,u8,u8,u8,u8,u8,u8),
		CharAttachObj  (u16 as CharId, u16 alias ObjectId),
		FlagSet(u16 as Flag), // [set_flag]
		FlagUnset(u16 as Flag), // [reset_flag]
		skip!(1),
		FlagAwaitUnset(u16 as Flag), // [wait_flag_false]
		FlagAwaitSet(u16 as Flag), // [wait_flag_true]
		skip!(2),
		ShopOpen(u8 as ShopId),
		skip!(2),
		RecipeLearn(u16), // TODO check type
		ImageShow(file_ref(lookup) -> String alias VisFileRef, u16, u16, u32 alias Time), // [portrait_open]
		ImageHide(u32 alias Time), // [portrait_close]
		QuestSubmit(u8 as ShopId, u16 as QuestId),
		_ObjB0(u16 alias ObjectId, u8), // Used along with 6F, 70, and 73 during T0700#11
		OpLoad(String alias OpFileRef),
		_B2(u8, u8, u16),
		Video(match {
			0x00 => Play(String alias AviFileRef, u32_not_in_fc(iset) -> u32), // [movie(MOVIE_START)]
			0x01 => End(u8, u32_not_in_fc(iset) -> u32), // [movie(MOVIE_END)]
		}),
		ReturnToTitle(u8),

		#[game(Fc,FcEvo)] PartySlot(u8 as Member, u8, u8),
		#[game(Fc,FcEvo)] Fc_B6(u8),
		#[game(Fc,FcEvo)] Fc_B7(u8 as Member, u8, u8), // Related to PartyAdd
		#[game(Fc,FcEvo)] Fc_B8(u8 as Member), // Related to PartyRemove
		#[game(Fc,FcEvo)] ReadBook(u16 as ItemId, u16),
		#[game(Fc,FcEvo)] PartyHasSpell(u8 as Member, u16 as MagicId),
		#[game(Fc,FcEvo)] PartyHasSlot(u8 as Member, u8),
		#[game(Fc)] skip!(34),
		#[game(Fc)] SaveClearData(),
		#[game(Fc)] skip!(33),

		#[game(FcEvo)] skip!(10),
		#[game(FcEvo)] EvoVisLoad(u8 alias VisId, u16, u16, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u32 as Color, u8, u8, String),
		#[game(FcEvo)] EvoVisColor(u8 alias VisId, u8, u32 as Color, u32, u32, u32),
		#[game(FcEvo)] EvoVisDispose(u8, u8 alias VisId, u8),
		#[game(FcEvo)] skip!(19),
		#[game(FcEvo)] Evo_DC(),
		#[game(FcEvo)] Evo_DD(),
		#[game(FcEvo)] EvoClearSaveData(),
		#[game(FcEvo)] skip!(2),
		#[game(FcEvo)] Evo_E1(u8 as u16 alias ObjectId, Pos3),
		#[game(FcEvo)] skip!(2),
		#[game(FcEvo)] EvoCtp(String), // Refers to /data/map2/{}.ctp
		#[game(FcEvo)] EvoVoiceLine(u16), // [pop_msg]
		#[game(FcEvo)] Evo_E6(text() -> Text),
		#[game(FcEvo)] Evo_E7(u8, u8),
		#[game(FcEvo)] skip!(24),

		#[game(Sc)] Sc_B5(u8),
		#[game(Sc)] Sc_B6(u8, u8, u8),
		#[game(Sc)] Sc_B7(u8),
		#[game(Sc)] Sc_B8(u32),
		#[game(Sc)] ScPartyHasSpell(u8 as Member, u16 as MagicId),
		#[game(Sc)] skip!(1),
		#[game(Sc)] ScSetPortrait(u8 as Member, u8, u32),
		#[game(Sc)] skip!(1),
		#[game(Sc)] Sc_BD(),
		#[game(Sc)] Sc_BE(u8,u8,u8,u8, u16, u16, u8, i32,i32,i32,i32,i32,i32),
		#[game(Sc)] Sc_BF(u32, u16),
		#[game(Sc)] ScMinigame(u8, i32,i32,i32,i32,i32,i32,i32,i32), // 11 roulette, 12 slots, 13 blackjack, 15 poker, 17 broken shooting minigame , 19 menu with st/eq/orb
		#[game(Sc)] Sc_C1(u16, u32),
		#[game(Sc)] Sc_C2(),
		#[game(Sc)] skip!(1),
		#[game(Sc)] ScScreenInvert(u8, u32),
		#[game(Sc)] ScVisLoad(u8 alias VisId, u16,u16,u16,u16, u16,u16,u16,u16, u16,u16,u16,u16, u32 as Color, u8, String),
		#[game(Sc)] ScVisColor(u8 alias VisId, u8, u32 as Color, i32, u32),
		#[game(Sc)] ScVisDispose(u8, u8 alias VisId, u8),
		#[game(Sc)] Sc_C8(u32, String, u8, u16),
		#[game(Sc)] ScPartySelect(u16, sc_party_select_mandatory() -> [Option<Member>; 4] alias MandatoryMembers, sc_party_select_optional() -> Vec<Member> alias OptionalMembers),
		#[game(Sc)] Sc_CA(u16, u32),
		#[game(Sc)] Sc_CharInSlot(u8),
		#[game(Sc)] Sc_Select(match {
			0 => New(u8,u8,u8,u8,u8,u8),
			1 => Add(u8, String),
			2 => Show(u8),
			3 => _3(u8, u8),
		}),
		#[game(Sc)] Sc_CD(u16 as CharId),
		#[game(Sc)] Sc_ExprUnk(u8, expr(iset, lookup) -> Expr),
		#[game(Sc)] Sc_CF(u16 as CharId, u8, String),
		#[game(Sc)] Sc_D0(u32, u32),
		#[game(Sc)] Sc_D1(u16 as CharId, i32, i32, i32, i32),
		#[game(Sc)] Sc_D2(file_ref(lookup) -> String, file_ref(lookup) -> String, u8),
		#[game(Sc)] Sc_D3(u8),
		#[game(Sc)] skip!(1),
		#[game(Sc)] ScPartyIsEquipped(u8 as Member, u16, u16 as ItemId, u8,u8,u8),
		#[game(Sc)] Sc_D6(u8),
		#[game(Sc)] Sc_D7(u8,u8,u8,u8,u8,u8,u8),
		#[game(Sc)] Sc_D8(u8, u16),
		#[game(Sc)] Sc_D9(match {
			0 => _0(String),
			1 => _1(),
		}),
		#[game(Sc)] Sc_DA(),
		#[game(Sc)] Sc_DB(),
		#[game(Sc)] Sc_DC(),
		#[game(Sc)] Sc_DD(),
		#[game(Sc)] Sc_DE(String),
		#[game(Sc)] skip!(1),
		#[game(Sc)] Sc_E0(u8, Pos3),
		#[game(Sc)] skip!(2),
		#[game(Sc)] Sc_E3(u32),
		#[game(Sc)] skip!(1),
		#[game(Sc)] Sc_E5(u16 as CharId, u8),
		#[game(Sc)] Sc_E6(u8), // related to RAM saving, according to debug script
		#[game(Sc)] Sc_E7(u8, String, u8,u8,u8,u8,u8),
		#[game(Sc)] Sc_E8(u32),
		#[game(Sc)] Sc_E9(u8), // related to RAM saving
		#[game(Sc)] Sc_EA(u32),
		#[game(Sc)] Sc_EB(u16),
		#[game(Sc)] skip!(20),
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
	pub(super) fn read<'a>(f: &mut impl In<'a>, iset: InstructionSet, lookup: &dyn Lookup) -> Result<Vec<Insn>, ReadError> {
		let len = f.u8()? as usize;
		let pos = f.pos();
		let mut insns = Vec::new();
		while f.pos() < pos+len {
			insns.push(Insn::read(f, iset, lookup)?);
		}
		ensure!(f.pos() == pos+len, "overshot while reading fork");
		if len > 0 {
			f.check_u8(0)?;
		}
		Ok(insns)
	}

	pub(super) fn write(f: &mut impl OutDelay, iset: InstructionSet, lookup: &dyn Lookup, v: &[Insn]) -> Result<(), WriteError> {
		let (l1, l1_) = HLabel::new();
		let (l2, l2_) = HLabel::new();
		f.delay(move |l| Ok(u8::to_le_bytes(hamu::write::cast_usize(l(l2)? - l(l1)?)?)));
		f.label(l1_);
		for i in v {
			Insn::write(f, iset, lookup, i)?;
		}
		f.label(l2_);
		if !v.is_empty() {
			f.u8(0);
		}
		Ok(())
	}
}

mod fork_loop {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, iset: InstructionSet, lookup: &dyn Lookup) -> Result<Vec<Insn>, ReadError> {
		let len = f.u8()? as usize;
		let pos = f.pos();
		let mut insns = Vec::new();
		while f.pos() < pos+len {
			insns.push(Insn::read(f, iset, lookup)?);
		}
		ensure!(f.pos() == pos+len, "overshot while reading fork loop");
		ensure!(read_raw_insn(f, iset, lookup)? == RawIInsn::Insn(Insn::NextFrame()), "invalid loop");
		ensure!(read_raw_insn(f, iset, lookup)? == RawIInsn::Goto(pos), "invalid loop");
		Ok(insns)
	}

	pub(super) fn write(f: &mut impl OutDelay, iset: InstructionSet, lookup: &dyn Lookup, v: &[Insn]) -> Result<(), WriteError> {
		let (l1, l1_) = HLabel::new();
		let (l2, l2_) = HLabel::new();
		let l1c = l1.clone();
		f.delay(|l| Ok(u8::to_le_bytes(hamu::write::cast_usize(l(l2)? - l(l1)?)?)));
		f.label(l1_);
		for i in v {
			Insn::write(f, iset, lookup, i)?;
		}
		f.label(l2_);
		write_raw_insn(f, iset, lookup, RawOInsn::Insn(&Insn::NextFrame()))?;
		write_raw_insn(f, iset, lookup, RawOInsn::Goto(l1c))?;
		Ok(())
	}
}

mod party_equip_slot {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, iset: InstructionSet, arg1: &ItemId) -> Result<i8, ReadError> {
		if !(600..800).contains(&arg1.0) && matches!(iset, InstructionSet::Fc|InstructionSet::FcEvo) {
			Ok(-1)
		} else {
			Ok(f.i8()?)
		}
	}

	pub(super) fn write(f: &mut impl Out, iset: InstructionSet, arg1: &ItemId, v: &i8) -> Result<(), WriteError> {
		if !(600..800).contains(&arg1.0) && matches!(iset, InstructionSet::Fc|InstructionSet::FcEvo) {
			ensure!(*v == -1, "invalid PartyEquipSlot");
		} else {
			f.i8(*v);
		}
		Ok(())
	}
}

// I'm fairly sure this logic is wrong; check t4403:17 for an example
mod party_set_slot {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, arg1: &u8) -> Result<i8, ReadError> {
		if *arg1 == 0xFF {
			Ok(-1)
		} else {
			Ok(f.i8()?)
		}
	}

	pub(super) fn write(f: &mut impl Out, arg1: &u8, v: &i8) -> Result<(), WriteError> {
		if *arg1 == 0xFF {
			ensure!(*v == -1, "invalid PartySetSlot");
		} else {
			f.i8(*v);
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
	pub(super) fn read<'a>(f: &mut impl In<'a>, lookup: &dyn Lookup) -> Result<String, ReadError> {
		Ok(lookup.name(f.u32()?)?.to_owned())
	}

	pub(super) fn write(f: &mut impl Out, lookup: &dyn Lookup, v: &str) -> Result<(), WriteError> {
		f.u32(lookup.index(v)?);
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

mod u8_not_in_fc {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, iset: InstructionSet) -> Result<u8, ReadError> {
		match iset {
			InstructionSet::Fc | InstructionSet::FcEvo => Ok(0),
			_ => Ok(f.u8()?)
		}
	}

	pub(super) fn write(f: &mut impl Out, iset: InstructionSet, v: &u8) -> Result<(), WriteError> {
		match iset {
			InstructionSet::Fc | InstructionSet::FcEvo => ensure!(*v == 0, "{v} must be 0"),
			_ => f.u8(*v)
		}
		Ok(())
	}
}

mod u32_not_in_fc {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, iset: InstructionSet) -> Result<u32, ReadError> {
		match iset {
			InstructionSet::Fc | InstructionSet::FcEvo => Ok(0),
			_ => Ok(f.u32()?)
		}
	}

	pub(super) fn write(f: &mut impl Out, iset: InstructionSet, v: &u32) -> Result<(), WriteError> {
		match iset {
			InstructionSet::Fc | InstructionSet::FcEvo => ensure!(*v == 0, "{v} must be 0"),
			_ => f.u32(*v)
		}
		Ok(())
	}
}

mod sc_party_select_mandatory {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<[Option<Member>; 4], ReadError> {
		f.multiple_loose::<4, _>(&[0xFF,0], |g| Ok(Member(cast(g.u16()?)?)))
	}

	pub(super) fn write(f: &mut impl Out, v: &[Option<Member>; 4]) -> Result<(), WriteError> {
		f.multiple_loose::<4, _>(&[0xFF,0], v, |g, a| { g.u16(a.0.into()); Ok(()) })
	}
}

mod sc_party_select_optional {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Vec<Member>, ReadError> {
		let mut quests = Vec::new();
		loop {
			match f.u16()? {
				0xFFFF => break,
				q => quests.push(Member(cast(q)?))
			}
		}
		Ok(quests)
	}

	pub(super) fn write(f: &mut impl Out, v: &Vec<Member>) -> Result<(), WriteError> {
		for &i in v {
			f.u16(i.0.into());
		}
		f.u16(0xFFFF);
		Ok(())
	}
}
