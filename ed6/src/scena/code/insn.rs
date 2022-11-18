use super::*;

ed6_macros::bytecode! {
	(iset: InstructionSet, lookup: &dyn Lookup)
	#[games(iset => InstructionSet::{Fc, FcEvo, Sc, ScEvo, Tc, TcEvo})]
	[
		skip!(1), // null
		Return(), // [return]
		skip!(3), // control flow
		Call(func_ref() -> FuncRef), // [call]

		/// Technically the last argument, which is always 7, is a separate hcf instruction.
		/// But they are never used separately, so since treating it separately would just bloat the output, it's treated as a constant for now.
		///
		/// Official name is `new_scene`.
		NewScene(file_ref(lookup) -> String alias ScenaFileRef, u8, u8, u8, u8),

		// This is some kind of hcf instruction: like [`NextFrame`](Self::NextFrame), but does not advance the instruction pointer.
		skip!(1),

		Sleep(u32 alias Time), // [delay]
		SystemFlagsSet(u32 as SystemFlags), // [set_system_flag]
		SystemFlagsUnset(u32 as SystemFlags), // [reset_system_flag]

		FadeOut(u32 alias Time, u32 as Color, u8), // [fade_out]
		FadeIn(u32 alias Time, u32 as Color), // [fade_in]
		FadeWait(), // [fade_wait]
		CrossFade(u32 alias Time), // [cross_fade]

		Battle(u32 as u16 as BattleId, u16, u16, u8, u16, i8), // is this last one a CharId? Used a few times in FC's prologue where it clearly refers to npc/monsters, and 0xFF everywhere else

		/// Sets whether an entrance (or rather exit), defined in the accompanying `._en` file, is enabled.
		/// Specifically, it sets the 0x0001 flag.
		/// I think `1` sets the exit as enabled, `0` as disabled. But I could be misreading it.
		EntranceSetDisabled(u8 alias EntranceId, u8),

		/// I have not been able to verify this one, the asm is complex.
		///
		/// Arguments are `D3DRS_FOGCOLOR` (24 bit, so color ignored), `D3DRS_FOGSTART`, `D3DRS_FOGEND`, `D3DRS_RANGEFOGENABLE`, `D3DRS_FOGDENSITY`.
		/// But since `D3DRS_FOGVERTEXMODE` is hardcoded to `D3DFOG_LINEAR` (at least in FC), the third parameter is never used.
		Fog(color24() -> Color, i32, i32, i32),

		/// Something related to fog.
		/// If I'm reading the assembly correctly, if arg1 is `0f`, then it is set to `32f`. arg2 is similarly defaulted to `130f`.
		///
		/// The third arg is an index to something, but it is unclear what.
		_12(i32, i32, u32),

		PlaceSetName(u16 as TownId),

		#[game(Fc, FcEvo)] skip!(1), // Unused one-byte instruction that calls an unknown function
		#[game(Fc, FcEvo)] skip!(1), // One-byte nop
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_14(u32, u32 as Color, u32, u32, u8),
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_15(u32),

		Map(match {
			0x00 => Hide(),
			0x01 => Show(),
			0x02 => Set(i32, Pos2, file_ref(lookup) -> String alias MapFileRef),
		}),
		#[game(Fc, Sc, Tc)] Save(),
		#[game(FcEvo, ScEvo, TcEvo)] EvoSave(u8),
		#[game(Fc, FcEvo)] skip!(1), // two-byte nop
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_18(u8, u8, u8),

		/// Performs a variety of setup when initializing a talk or cutscene.
		///
		/// The argument is a bitmask, but the meanings are unknown.
		///
		/// Official name is `event_begin`.
		EventBegin(u8),

		/// Undoes the setup performed by [`EventBegin`](Self::EventBegin).
		///
		/// At least that's what it intuitively should do, but it has at least one flag not supported by `EventBegin`, and the assembly code is very long.
		///
		/// Official name is `event_end`.
		EventEnd(u8),

		// Can't tell what these two are doing
		_1B(u8, u8, u16),
		_1C(u8, u8, u16),

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
			0x03 => FlagsUnset(u8 as QuestFlags),
			0x04 => FlagsSet(u8 as QuestFlags),
		}),
		Quest(u16 as QuestId, match {
			0x00 => FlagsGet(u8 as QuestFlags),
			0x01 => TaskGet(u16 alias QuestTask),
		}),
		QuestList(quest_list() -> Vec<QuestId> alias QuestList),
		QuestBonusBp(u16 as QuestId, u16),
		QuestBonusMira(u16 as QuestId, u16),

		PartyAdd(u8 as Member, u8, { IS::Fc|IS::FcEvo => const 0u8, _ => u8 }), // [join_party]
		PartyRemove(u8 as Member, u8), // [separate_party]
		ScPartyClear(),
		_Party30(u8 as Member),
		PartySetAttr(u8 as Member, u8 as MemberAttr, u16), // [set_status]
		skip!(2),
		PartyAddArt(u8 as Member, u16 as MagicId),
		PartyAddCraft(u8 as Member, u16 as MagicId),
		PartyAddSCraft(u8 as Member, u16 as MagicId),
		PartySetSlot(u8 as Member, u8, party_set_slot(iset, _1) -> i8), // merged with FC's PartyUnlockSlot

		SepithAdd(u8 as Element alias SepithElement, u16),
		SepithRemove(u8 as Element alias SepithElement, u16),
		MiraAdd(u16), // [get_gold]
		MiraSub(u16),
		BpAdd(u16),
		BpSub(u16),
		ItemAdd(u16 as ItemId, u16),
		ItemRemove(u16 as ItemId, u16), // [release_item]
		ItemHas(u16 as ItemId, { IS::Fc|IS::FcEvo => const 0u8, _ => u8 }), // or is it ItemGetCount?

		PartyEquip(u8 as Member, u16 as ItemId, party_equip_slot(iset, _1) -> i8),
		PartyPosition(u8 as Member),

		ForkFunc(u16 as CharId, u8 alias ForkId, func_ref() -> FuncRef), // [execute]
		ForkQuit(u16 as CharId, u8 alias ForkId), // [terminate]
		Fork(u16 as CharId, u8 alias ForkId, u8, fork(iset, lookup) -> Vec<Insn> alias Fork), // [preset]? In t0311, only used with a single instruction inside
		ForkLoop(u16 as CharId, u8 alias ForkId, u8, fork_loop(iset, lookup) -> Vec<Insn> alias Fork),
		ForkWait(u16 as CharId, u8 alias ForkId, u8), // [wait_terminate]
		NextFrame(), // [next_frame]

		Event(func_ref() -> FuncRef), // [event] Not sure how this differs from Call

		_Char4A(u16 as CharId, u8), // Argument is almost always 255, but sometimes 0, and in a single case 1
		_Char4B(u16 as CharId, u8),

		skip!(1), // {asm} one-byte nop
		Var(u16 as Var, expr(iset, lookup) -> Expr),
		skip!(1), // {asm} one-byte nop
		Attr(u8 as Attr, expr(iset, lookup) -> Expr), // [system[n]]
		skip!(1), // {asm} one-byte nop
		CharAttr(char_attr() -> CharAttr, expr(iset, lookup) -> Expr),

		TextStart(u16 as CharId), // [talk_start]
		TextEnd(u16 as CharId), // [talk_end]
		TextMessage(text() -> Text), // [mes]
		skip!(1), // {asm} same as NextFrame
		TextClose(u8), // [mes_close]
		/// Exists in FC too, but does not appear to do anything.
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

		Emote(u16 as CharId, i32, i32, emote() -> Emote, u8), // [emotion] mostly used through macros such as EMO_BIKKURI3(). Third argument is height.
		EmoteStop(u16 as CharId), // [emotion_close]

		/// These two seem to use the scp idx for something?
		_64(u8 as u16 alias ObjectId, u16), // What's the difference between this and ObjFlagsSet?
		_65(u8 as u16 alias ObjectId, u16),

		CamChangeAxis(u16), // [camera_change_axis] 0 CAMERA_ABSOLUTE_MODE, 1 CAMERA_RELATIVE_MODE
		CamMove(i32, i32, i32, u32 alias Time), // [camera_move]
		_Cam68(u8),
		CamLookChar(u16 as CharId, u32 alias Time), // [camera_look_chr]
		_Char6A(u16 as CharId),
		CamZoom(i32, u32 alias Time), // [camera_zoom]
		CamRotate(i32 alias Angle32, u32 alias Time), // [camera_rotate]
		CamLookPos(Pos3, u32 alias Time), // [camera_look_at]
		CamPers(u32, u32 alias Time), // [camera_pers]

		ObjFrame(u16 alias ObjectId, u32), // [mapobj_frame]
		ObjPlay(u16 alias ObjectId, u32), // [mapobj_play]
		#[game(Fc, FcEvo, Sc, ScEvo)] ObjFlagsSet(u16 alias ObjectId, u16 as ObjectFlags), // [mapobj_set_flag]
		#[game(Fc, FcEvo, Sc, ScEvo)] ObjFlagsUnset(u16 alias ObjectId, u16 as ObjectFlags), // [mapobj_reset_flag]
		#[deprecated]
		#[game(Tc, TcEvo)] TcObjFlagsSet(u8 as u16 alias ObjectId, u16 as ObjectFlags, u16), // [mapobj_set_flag]
		#[deprecated]
		#[game(Tc, TcEvo)] TcObjFlagsUnset(u8 as u16 alias ObjectId, u16 as ObjectFlags, u16), // [mapobj_reset_flag]
		ObjWait(u16 alias ObjectId),

		_74(u16, u32, u16),
		_75(u8 as u16 alias ObjectId, u32, u8),
		_76(u16, u32, u16, i32, i32, i32, u8, u8),
		MapColor(u32 as Color, u32 alias Time), // [map_color]
		_78(u8, u8, u8),
		_79(u8 as u16 alias ObjectId, u16),
		_7A(u8 as u16 alias ObjectId, u16),
		_7B(),
		Shake(u32, u32, u32, u32 alias Time), // [quake]
		#[game(Fc, FcEvo)] skip!(1), // {asm} two-byte nop
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_7D(match {
			0 => _0(u16 as CharId, u16, u16),
			1 => _1(u16 as CharId, u16, u16), // args always zero; always paired with a _0 except when the char is 254
		}),
		_7E(i16, i16, u16, u8, u32),
		EffLoad(u8, String alias EffFileRef),
		EffPlay(
			u8 alias EffId, u8,
			u16 as CharId, Pos3, // source
			i16, i16, i16,
			u32, u32, u32, // scale?
			u16 as CharId, Pos3, // target
			u32 alias Time, // period (0 if one-shot)
		),
		EffPlay2(
			u8 alias EffId, u8,
			u8 as u16 alias ObjectId, String, Pos3, // source
			i16, i16, i16,
			u32, u32, u32, // scale
			u32 alias Time, // period (0 if one-shot)
		),
		_82(u8 alias EffId, u8),
		#[game(Fc, FcEvo)] FcAchievement(u8, u8),
		#[game(Sc, ScEvo, Tc, TcEvo)] _83(u8, u8), // might have to do with EffPlay
		_84(u8),
		_85(u8, u8),
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
		_94        (u8, u16 as CharId, i16 alias Angle, i32, u32 alias Speed, u8),
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
		Sc_A0          (u16 as CharId, u32 as Color, u8,u8,u8),
		CharAttachObj  (u16 as CharId, u16 alias ObjectId),
		FlagSet(u16 as Flag), // [set_flag]
		FlagUnset(u16 as Flag), // [reset_flag]

		skip!(1), // {asm} 3-byte nop

		/// Waits until the flag is true.
		///
		/// Equivalent to
		/// ```text
		/// while !flag[n]:
		///   NextFrame
		/// ```
		///
		/// Official name is `flag_wait_false`.
		FlagWaitSet(u16 as Flag),

		/// Waits until the flag is true.
		///
		/// Equivalent to
		/// ```text
		/// while flag[n]:
		///   NextFrame
		/// ```
		///
		/// Official name is `flag_wait_true`.
		FlagWaitUnset(u16 as Flag),

		/// Waits until the variable has the given value.
		///
		/// Equivalent to
		/// ```text
		/// while var[n] != val:
		///   NextFrame
		/// ```
		///
		/// Never used.
		VarWait(u16 as Var, u16),

		// {asm} 6-byte nop
		skip!(1),

		ShopOpen(u8 as ShopId),

		/// Saves the order of the party, to be loaded by [`PartyLoad`](Self::PartyLoad).
		///
		/// It saves eight variables, I don't know what's up with that.
		///
		/// Never used.
		#[game(Fc, FcEvo, Sc, ScEvo)] PartySave(),
		#[game(Tc, TcEvo)] TcMonument(match {
			0 => Open(u8, u8, u8),
			1 => Disable(u8, u8, u8),
			2 => Enable(u8, u8, u8),
		}),

		/// Loads the order of the party, as saved by [`PartySave`](Self::PartySave).
		///
		/// Never used.
		PartyLoad(),

		/// Learns a cooking recipe.
		///
		/// Returns whether the recipe was already known, i.e. if it was *not* successfully learned.
		RecipeLearn(u16),

		ImageShow(file_ref(lookup) -> String alias VisFileRef, u16, u16, u32 alias Time), // [portrait_open]

		ImageHide(u32 alias Time), // [portrait_close]

		/// Attempts to submit a quest.
		///
		/// Returns a boolean value, probably whether the quest was successfully reported.
		/// What exactly this entails is unknown; the return value is never used.
		QuestSubmit(u8 as ShopId, u16 as QuestId),

		_ObjB0(u16 alias ObjectId, u8), // Used along with 6F, 70, and 73 during T0700#11

		OpLoad(String alias OpFileRef),

		_B2(match {
			0 => Set(u8, u16),
			1 => Unset(u8, u16),
		}),

		Video(match {
			0 => Play(String alias AviFileRef, u16_not_in_fc(iset) -> u16, u16_not_in_fc(iset) -> u16), // [movie(MOVIE_START)]
			1 => End(u8, u16_not_in_fc(iset) -> u16, u16_not_in_fc(iset) -> u16), // [movie(MOVIE_END)], probably the 0 is the null terminator of an empty string
		}),

		ReturnToTitle(u8),

		/// Unlocks a character's orbment slot.
		///
		/// In SC onward, this is merged into PartySetSlot.
		#[game(Fc, FcEvo)] PartyUnlockSlot(u16 as u8 as Member, u8),

		/// The argument is always zero in the scripts. According to the scripts something else happens if it is nonzero, but it is unknown what.
		_B6(u8),

		/// This is related to [`PartyAdd`](Self::PartyAdd), but what it actually does is unknown.
		///
		/// As with PartyUnlockSlot, this is really a u16.
		_B7(u8 as Member, u8, u8),

		/// This is related to [`PartyRemove`](Self::PartyRemove) and [`_B7`](Self::_B7), but as with that one, the details are unknown.
		_B8(u8 as Member),

		/// Opens the book reading interface, as if using it from the inventory.
		///
		/// It is unknown what the second argument means; all known uses have zero.
		/// The assembly hints that it might be a character, in which case the instruction might be a more general-purpose use-item instruction.
		ReadBook(u16 as ItemId, u16),

		/// Returns whether the given member has a particular orbal art.
		///
		/// Does not work on crafts.
		PartyHasSpell(u8 as Member, u16 as MagicId),

		/// Checks whether the given member has this orbment slot unlocked.
		PartyHasSlot(u8 as Member, u8),

		#[game(Fc, FcEvo)] skip!(10),

		#[game(Sc, ScEvo, Tc, TcEvo)] ScSetPortrait(u8 as Member, u8, u8, u8, u8, u8),
		// This instruction is only used a single time throughout FC..=3rd, but this is its signature according to the asm
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_BC(u8, match {
			0 => _0(u16),
			1 => _1(u16),
		}),
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_BD(),
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_BE(u8,u8,u8,u8, u16, u16, u8, i32,i32,i32,i32,i32,i32),
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_BF(u8,u8,u8,u8, u16),
		/// ```text
		///  1 ⇒ something about using items on the field
		/// 11 ⇒ roulette
		/// 12 ⇒ slots
		/// 13 ⇒ blackjack
		/// 14 ... ⇒ fishing
		/// 15 ⇒ poker
		/// 16 ... ⇒ used in Axis Pillar
		/// 17 ⇒ broken shooting minigame
		/// 18 n ⇒ check if have fish n
		/// 19 ⇒ menu with st/eq/orb
		/// 20 5000 ⇒ after beating Luciola
		/// 21 ⇒ used after a few battles
		/// 22 ⇒ after Weissman sets up a barrier
		/// 23 ⇒ used after sequences of ScLoadChcp
		/// ```
		#[game(Sc, ScEvo, Tc, TcEvo)] ScMinigame(u8, i32,i32,i32,i32,i32,i32,i32,i32),
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_C1(u16 as ItemId, u32),
		#[game(Sc, ScEvo)] Sc_C2(),
		#[game(Tc, TcEvo)] Tc_C2(u8, u8),

		/// Unused.
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_C3(u16),

		/// Something for setting some kind of bit flags I guess.
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_C4(match {
			0 => Set(u32),
			1 => Unset(u32),
		}),

		#[game(Fc)] skip!(3),
		#[game(FcEvo, Sc, ScEvo, Tc, TcEvo)] VisLoad(u8 alias VisId, i16,i16,u16,u16, i16,i16,u16,u16, i16,i16,u16,u16, u32 as Color, u8, String),
		#[game(FcEvo, Sc, ScEvo, Tc, TcEvo)] VisColor(u8 alias VisId, u8, u32 as Color, u32 alias Time, u32, u32_only_fc_evo(iset) -> u32),
		#[game(FcEvo, Sc, ScEvo, Tc, TcEvo)] VisDispose(u8, u8 alias VisId, u8),

		#[game(Fc,FcEvo)] skip!(19),

		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_C8(u16, u16, String, u8, u16), // Something with C_PLATnn._CH
		#[game(Sc, ScEvo, Tc, TcEvo)] ScPartySelect(u16, sc_party_select_mandatory() -> [Option<Member>; 4] alias MandatoryMembers, sc_party_select_optional() -> Vec<Member> alias OptionalMembers),
		#[game(Sc, ScEvo)] Sc_CA(u8 as u16 alias ObjectId, u8, u32),
		#[game(Tc, TcEvo)] Tc_CA(u8 as u16 alias ObjectId, u8, i32, u32),
		#[game(Sc, ScEvo)] Sc_CharInSlot(u8), // clearly related to CharId, but not the same
		#[game(Tc, TcEvo)] Tc_CharInSlot(u8, u8), // added team id I guess?
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_Select(match {
			0 => New(u8 alias SelectId, u16, u16, u8),
			1 => Add(u8 alias SelectId, String alias MenuItem),
			2 => Show(u8 alias SelectId),
			3 => SetDisabled(u8 alias SelectId, u8),
		}),
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_CD(u16 as CharId), // related to showing photographs
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_ExprUnk(u8, expr(iset, lookup) -> Expr), // I think this is integer variables that are not local
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_CF(u16 as CharId, u8, String), // something with skeleton animation
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_D0(i32 alias Angle32, u32 alias Time),
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_D1(u16 as CharId, i32, i32, i32, u32 alias Time), // something with camera?
		#[game(Sc, ScEvo, Tc, TcEvo)] ScLoadChcp(file_ref(lookup) -> String, file_ref(lookup) -> String, u8),
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_D3(u8),

		/// Unused.
		///
		/// First arg is an index into some array; second is a field selector, which can be `[0, 1, 2, 5, 6]`.
		/// Returns whatever that value is.
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_D4(u8, u8),

		#[game(Sc, ScEvo, Tc, TcEvo)] ScPartyIsEquipped(u8 as Member, u16, u16 as ItemId, u8, u8, u8),
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_D6(u8), // bool
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_D7(u8, u32, u16 as CharId),
		/// Always occurs before ObjSetFrame and ObjPlay. Probably animation speed?
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_D8(u8 as u16 alias ObjectId, u16),
		#[game(Sc, ScEvo, Tc, TcEvo)] ScCutIn(match {
			0 => Show(String), // CTInnnnn
			1 => Hide(),
		}),
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_DA(), // Something to do with menus

		#[game(Tc, TcEvo)] Tc_DB(u8, u8 as Member),
		#[game(Tc, TcEvo)] TcTeam(match {
			0 => Use(u8),
			1 => AddMember(u8, u8 as Member),
			2 => Clear(u8),
		}),
		#[game(Tc, TcEvo)] TcOrganizeTeams(u8, u8, u8, u32 alias TcMembers, u32 alias TcMembers, u32 alias TcMembers, u32 alias TcMembers),
		#[game(Tc, TcEvo)] Tc_DE(u8, u32),
		#[game(Tc, TcEvo)] Tc_DF(u8, u16),
		#[game(Tc, TcEvo)] Tc_E0(u16 as CharId, u8, u8),
		#[game(Tc, TcEvo)] TcIndexInTeam(u8 as Member, u8),
		#[game(Tc, TcEvo)] Tc_E2(match {
			0 => _0(u8),
			1 => _1(),
			3 => _3(u8),
			4 => _4(u8),
			5 => _5(u16, u16, u16),
			7 => _7(),
			8 => _8(),
			9 => _9(u8),
			10 => _10(), // A getter
			11 => _11(u8),
		}),
		#[game(Tc, TcEvo)] TcEpisode(match {
			0 => Start(u16, u32),
			1 => End(u8),
			4 => _4(u8),
		}),
		#[game(Tc, TcEvo)] skip!(1),
		#[game(Tc, TcEvo)] Tc_E5(match {
			0 => _0(u8 as u16 alias ObjectId, u8 as Member, u16, u16),
			1 => _1(u8, u8, u16, u16),
			2 => _2(u8 as u16 alias ObjectId, u8 as Member, u32),
		}),
		#[game(Tc, TcEvo)] Tc_E6(match {
			0 => _0(u8),
			1 => _1(u8),
			2 => _2(),
		}),
		#[game(Tc, TcEvo)] Tc_E7(u8 alias VisId, u8, u32 as Color, u32 alias Time),

		#[game(Fc)] skip!(2),
		/// A no-op. Always paired with [`Sc_DC`](Self::Sc_DC).
		#[game(FcEvo, Sc, ScEvo, TcEvo)] Sc_DB(),
		/// A no-op. Always paired with [`Sc_DB`](Self::Sc_DB).
		#[game(FcEvo, Sc, ScEvo, TcEvo)] Sc_DC(),
		#[game(Tc)] skip!(2),

		/// Opens the save menu in order to save clear data.
		SaveClearData(),

		#[game(FcEvo, Sc, ScEvo, TcEvo)] Sc_DE(String), // a place name. Not a t_town, strangely
		#[game(FcEvo, Sc, ScEvo, TcEvo)] skip!(1),
		#[game(FcEvo, Sc, ScEvo, TcEvo)] Sc_E0(u8 as u16 alias ObjectId, Pos3),
		#[game(FcEvo, Sc, ScEvo, TcEvo)] skip!(2),

		#[game(FcEvo)] EvoCtp(String), // Refers to /data/map2/{}.ctp

		#[game(Sc, ScEvo, TcEvo)] Sc_E3(u8, u16 as CharId, u8),
		/// A no-op.
		#[game(Sc, ScEvo, TcEvo)] Sc_E4(u8, u16),
		#[game(Sc, ScEvo)] Sc_E5(u16 as CharId, u8),
		#[game(TcEvo)] TcEvo_F2(u16 as CharId, u8, u16, u16),
		#[game(Sc, ScEvo)] Sc_E6(u8), // related to RAM saving, according to debug script
		#[game(TcEvo)] custom! {
			// What's Evo_E7 doing up here? Maybe they wanted FF to stay clear.
			read => |f| {
				Ok(Self::Evo_E7(f.u8()?, f.u8()?))
			},
			write Evo_E7(a, b) => |f| {
				f.u8(*a);
				f.u8(*b);
				Ok(())
			},
		},
		#[game(Sc, ScEvo)] Sc_E7(u8 as u16 alias ObjectId, String, u8,u8,u8,u8,u8),
		#[game(TcEvo)] skip!(1),
		#[game(Sc, ScEvo, TcEvo)] Sc_E8(u32 alias Time),
		#[game(Sc, ScEvo)] Sc_E9(u8), // related to RAM saving
		#[game(TcEvo)] skip!(1),

		#[game(Tc)] skip!(12),

		/// Probably nonexistent on ScEvo.
		#[game(Sc, ScEvo, Tc)] ScAchievement(u8, u16, u8),
		#[game(TcEvo)] TcEvo_F7(u8, u16, u8), // Used exactly once, after breaking out of the planes. ScImage is not used there.
		/// A no-op.
		#[game(Sc, ScEvo)] Sc_EB(u8, u8),
		#[game(TcEvo)] TcEvo_F8(u8, u8),
		/// Seems to be a way to apply [`Tc_E5_0`](Self::Tc_E5_0) to a large number of members.
		#[game(TcEvo)] TcEvo_F9(u16 alias ObjectId, u8),
		/// Seems to be a way to apply [`Tc_E5_2`](Self::Tc_E5_2) to a large number of members.
		///
		/// Always preceded by a [`TcEvo_F9`](Self::TcEvo_F9), with the object matching. But sometimes the object is 0.
		#[game(TcEvo)] TcEvo_FA(u16 alias ObjectId, u32),
		#[game(TcEvo)] TcEvo_FB(u8, u16 as CharId, u8),
		#[game(TcEvo)] TcEvo_FC(u8, u8),

		#[game(FcEvo, ScEvo, TcEvo)] EvoVoiceLine(u16), // [pop_msg]
		#[game(FcEvo, ScEvo, TcEvo)] Evo_E6(text() -> Text),
		#[game(FcEvo, ScEvo)] Evo_E7(u8 alias VisId, u8),
		#[game(TcEvo)] skip!(1),

		#[game(Fc)] skip!(33),
		#[game(FcEvo)] skip!(24),
		#[game(Sc)] skip!(20),
		#[game(ScEvo)] skip!(17),
		#[game(Tc)] skip!(8),
	]
}

mod color24 {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>) -> Result<Color, ReadError> {
		let a = f.u8()?;
		let b = f.u8()?;
		let c = f.u8()?;
		Ok(Color(u32::from_le_bytes([a, b, c, 0])))
	}

	pub(super) fn write(f: &mut impl Out, v: &Color) -> Result<(), WriteError> {
		let [a, b, c, _] = u32::to_le_bytes(v.0);
		f.u8(a);
		f.u8(b);
		f.u8(c);
		Ok(())
	}
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

mod party_set_slot {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, iset: InstructionSet, arg1: &u8) -> Result<i8, ReadError> {
		if !(0x7F..0xFF).contains(arg1) && !matches!(iset, InstructionSet::Fc|InstructionSet::FcEvo) {
			Ok(-1)
		} else {
			Ok(f.i8()?)
		}
	}

	pub(super) fn write(f: &mut impl Out, iset: InstructionSet, arg1: &u8, v: &i8) -> Result<(), WriteError> {
		if !(0x7F..0xFF).contains(arg1) && !matches!(iset, InstructionSet::Fc|InstructionSet::FcEvo) {
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

mod u16_not_in_fc {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, iset: InstructionSet) -> Result<u16, ReadError> {
		match iset {
			InstructionSet::Fc | InstructionSet::FcEvo => Ok(0),
			_ => Ok(f.u16()?)
		}
	}

	pub(super) fn write(f: &mut impl Out, iset: InstructionSet, v: &u16) -> Result<(), WriteError> {
		match iset {
			InstructionSet::Fc | InstructionSet::FcEvo => ensure!(*v == 0, "{v} must be 0"),
			_ => f.u16(*v)
		}
		Ok(())
	}
}

mod u32_only_fc_evo {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl In<'a>, iset: InstructionSet) -> Result<u32, ReadError> {
		match iset {
			InstructionSet::FcEvo => Ok(f.u32()?),
			_ => Ok(0)
		}
	}

	pub(super) fn write(f: &mut impl Out, iset: InstructionSet, v: &u32) -> Result<(), WriteError> {
		match iset {
			InstructionSet::FcEvo => f.u32(*v),
			_ => ensure!(*v == 0, "{v} must be 0"),
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
