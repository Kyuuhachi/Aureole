use super::*;

themelios_macros::bytecode! {
	(game: &GameData)
	#[games(game.iset => InstructionSet::{Fc, FcEvo, Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo})]
	[
		skip!(1), // null
		Return(), // [return]
		skip!(3), // control flow
		Call(func_ref(game) -> FuncRef),

		/// Loads another scena.
		///
		/// The second argument is which entrance (in the `._en` file) to start at, the others are
		/// unknown.
		///
		/// Official name is `new_scene`, which also implicitly adds a [`Hcf`](Self::Hcf).
		NewScene(file_ref(game) -> String alias ScenaFileRef, u8, u8, u8),

		/// Simply halts the script forever.
		///
		/// Doesn't exist naturally in vanilla scripts, but instead inserted implicitly after
		/// `new_scene`.
		Hcf(),

		Sleep({ i if i.is_ed7() => u16 as u32, _ => u32 } alias Time), // [delay]
		SystemFlagsSet(u32 as SystemFlags), // [set_system_flag]
		SystemFlagsUnset(u32 as SystemFlags), // [reset_system_flag]

		FadeOut(u32 alias Time, u32 as Color, u8), // [fade_out]
		FadeIn(u32 alias Time, u32 as Color), // [fade_in]
		FadeWait(), // [fade_wait]
		CrossFade(u32 alias Time), // [cross_fade]

		#[game(Fc,FcEvo,Sc,ScEvo,Tc,TcEvo)]
		Battle(u32 as BattleId, u16, u16, u8, u16, u8 as u16 as CharId),

		def! ED7Battle(BattleId, u16,u8,u8,u8, u16, u16, CharId),
		def! ED7NpcBattle(u16,u8,u8,u8, [Option<String>; 8] alias NpcBattleCombatants, u16, u16),

		#[game(Zero, ZeroEvo, Ao, AoEvo)]
		custom! {
			read => |f| {
				let ptr = f.u32()?;
				if ptr != 0xFFFFFFFF {
					// Pointer is filled in properly later
					Ok(Self::ED7Battle(
						BattleId(ptr),
						f.u16()?, f.u8()?, f.u8()?, f.u8()?,
						f.u16()?,
						f.u16()?,
						CharId(f.u16()?),
					))
				} else {
					Ok(Self::ED7NpcBattle(
						f.u16()?, f.u8()?, f.u8()?, f.u8()?,
						array::<8, _>(|| match f.u32()? {
							0 => Ok(None),
							n => Ok(Some(game.lookup.name(n)?))
						}).strict()?,
						{
							f.check(&[0;16])?;
							f.u16()?
						},
						f.u16()?,
					))
				}
			},
			write ED7Battle(ptr, s1,s2,s3,s4, a1, a2, ch) => |f| {
				f.delay_u32(hamu::write::Label::known(ptr.0).0);
				f.u16(*s1); f.u8(*s2); f.u8(*s3); f.u8(*s4);
				f.u16(*a1); f.u16(*a2);
				f.u16(ch.0);
				Ok(())
			},
			write ED7NpcBattle(s1,s2,s3,s4, c, a1, a2) => |f| {
				f.u32(0xFFFFFFFF);
				f.u16(*s1); f.u8(*s2); f.u8(*s3); f.u8(*s4);
				for c in c {
					f.u32(c.as_ref().map_or(Ok(0), |a| game.lookup.index(a))?);
				}
				f.array([0;16]);
				f.u16(*a1); f.u16(*a2);
				Ok(())
			},
		},

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
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)]
		ED6_12(i32, i32, u32),
		#[game(Zero, ZeroEvo, Ao, AoEvo)]
		ED7_12(u16, u16, u8),
		#[game(AoEvo)]
		AoEvo_13(u16),

		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo, Zero, Ao)] PlaceSetName(u16 as TownId), // I am not certain whether it is this one or the one before that is not in Evo

		#[game(Fc, FcEvo)] skip!(1), // Unused one-byte instruction that calls an unknown function
		#[game(Fc, FcEvo)] skip!(1), // One-byte nop
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_14({ IS::Ao|IS::AoEvo => u16 as u32, _ => u32 }, u32 as Color, { IS::Ao|IS::AoEvo => u16 as u32, _ => u32 }, u8, { IS::Ao|IS::AoEvo => u16 as u32, _ => u32 }),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_15(u32),

		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)]
		Map(match {
			0 => Hide(),
			1 => Show(),
			2 => Set(i32, Pos2, file_ref(game) -> String alias MapFileRef),
		}),
		#[game(Zero, ZeroEvo)]
		ZeroMap(match {
			2 => _2(i32, Pos2, u16, u16, u16, u16),
			3 => _3(u8, u16),
		}),
		#[game(Ao, AoEvo)]
		AoMap(u8),
		#[game(Fc, Sc, Tc, Zero, ZeroEvo, Ao, AoEvo)] Save(),
		#[game(FcEvo, ScEvo, TcEvo)] EvoSave(u8),
		#[game(Fc, FcEvo, Zero, Ao)] skip!(1), // two-byte nop
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

		_1B(u8, func_ref_u8_u16() -> FuncRef),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)]
		_1C(u8, u8, u16),
		#[game(Zero, Ao, AoEvo)]
		ED7_1C(u8, u8, u8, u8, u8, u8, u16 as Flag, u16),

		#[game(Zero, ZeroEvo, Ao, AoEvo)]
		ED7_1D(match {
			0 => _0(u8, u8, u8, Pos3, i32, i32, i32),
			2 => _2(u8, u8),
			3 => _3(u8, u8),
		}),

		BgmPlay({ i if i.is_ed7() => u16, _ => u8 as u16 } as BgmId, { i if i.is_ed7() => u8, _ => const 0u8 }), // [play_bgm]
		BgmResume(), // [resume_bgm]
		BgmVolume(u8, u32 alias Time), // [volume_bgm]
		BgmStop(u32 alias Time), // [stop_bgm]
		BgmWait(), // [wait_bgm]

		SoundPlay({ i if i.is_ed7() && game.kai => u32, _ => u16 as u32 } as SoundId, u8, { i if i.is_ed7() => u8, _ => const 0u8 }, u8), // [sound]
		SoundStop({ IS::Ao|IS::AoEvo if game.kai => u32, _ => u16 as u32 } as SoundId),
		SoundSetVolume(u16 as u32 as SoundId, u8),
		SoundPlayContinuously(u16 as u32 as SoundId, Pos3, u32, u32, u8, u32),
		SoundLoad({ IS::Ao|IS::AoEvo if game.kai => u32, _ => u16 as u32 } as SoundId), // [sound_load]

		#[game(Fc,FcEvo,Sc,ScEvo,Tc,TcEvo)] skip!(1),
		#[game(Zero,ZeroEvo,Ao,AoEvo)] NextFrame2(),

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

		PartyAdd(u8 as u16 as NameId, u8 as u16 as CharId, { IS::Fc|IS::FcEvo => const 0u8, _ => u8 }), // [join_party]
		PartyRemove(u8 as u16 as NameId, u8), // [separate_party]
		ScPartyClear(),
		#[game(Fc,FcEvo,Sc,ScEvo,Tc,TcEvo)] _30(u8),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_31(u8),
		PartySetAttr(u8 as u16 as NameId, u8 as MemberAttr, u16), // [set_status]
		#[game(Fc,FcEvo,Sc,ScEvo,Tc,TcEvo,Zero,Ao)] skip!(2),
		PartyAddArt(u8 as u16 as NameId, u16 as MagicId),
		PartyAddCraft(u8 as u16 as NameId, u16 as MagicId),
		#[game(Fc,FcEvo,Sc,ScEvo,Tc,TcEvo)] PartyAddSCraft(u8 as u16 as NameId, u16 as MagicId),
		#[game(Zero, ZeroEvo,Ao, AoEvo)] ED7_37(),
		PartySetSlot(u8 as u16 as NameId, u8, {
			IS::Fc|IS::FcEvo => u8,
			_ if (0x7F..=0xFE).contains(_1) => u8,
			_ => const 0u8
		}), // merged with FC's PartyUnlockSlot

		SepithAdd(u8, u16),
		SepithRemove(u8, u16),
		MiraAdd(u16), // [get_gold]
		MiraSub(u16),
		BpAdd(u16),
		BpSub(u16),
		ItemAdd(u16 as ItemId, u16),
		ItemRemove(u16 as ItemId, u16), // [release_item]
		ItemHas(u16 as ItemId, { IS::Fc|IS::FcEvo => const 0u8, _ => u8 }), // or is it ItemGetCount?

		PartyEquip(u8 as u16 as NameId, u16 as ItemId, {
			IS::Fc|IS::FcEvo if !(600..=799).contains(&_1.0) => const 0u8,
			_ => u8,
		}),
		PartyPosition(u8 as u16 as NameId),

		ForkFunc(u16 as CharId, u8 as u16 alias ForkId, func_ref(game) -> FuncRef), // [execute]
		ForkQuit(u16 as CharId, u8 as u16 alias ForkId), // [terminate]
		Fork(u16 as CharId, { i if i.is_ed7() => u8 as u16, _ => u16 } alias ForkId, fork(game) -> Vec<Insn> alias Fork), // [preset]? In t0311, only used with a single instruction inside
		ForkLoop(u16 as CharId, { i if i.is_ed7() => u8 as u16, _ => u16 } alias ForkId, fork_loop(game) -> Vec<Insn> alias Fork),
		ForkWait(u16 as CharId, { i if i.is_ed7() => u8 as u16, _ => u16 } alias ForkId), // [wait_terminate]
		NextFrame(), // [next_frame]

		Event(func_ref(game) -> FuncRef), // [event] Not sure how this differs from Call

		_Char4A(u16 as CharId, u8), // Argument is almost always 255, but sometimes 0, and in a single case 1
		_Char4B(u16 as CharId, u8),

		skip!(1), // {asm} one-byte nop
		Var(u16 as Var, expr(game) -> Expr),
		skip!(1), // {asm} one-byte nop
		Attr(u8 as Attr, expr(game) -> Expr), // [system[n]]
		skip!(1), // {asm} one-byte nop
		CharAttr(char_attr() -> CharAttr, expr(game) -> Expr),

		TextStart(u16 as CharId), // [talk_start]
		TextEnd(u16 as CharId), // [talk_end]
		/// Shows a text box without a speak bubble arrow.
		///
		/// I believe the CharId, which is only present in ED7, is used to select the textbox title.
		/// However, it is 999 on chests.
		TextMessage({ i if i.is_ed7() => u16, _ => const 255u16 } as CharId, text() -> Text), // [mes]
		skip!(1), // {asm} same as NextFrame
		TextClose(u8), // [mes_close]
		ScMenuSetTitle(u16, u16, u16, text() -> Text),
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

		LookPointFlagsSet(u8 as u16 alias LookPointId, u16 as u32 as LookPointFlags),
		LookPointFlagsUnset(u8 as u16 alias LookPointId, u16 as u32 as LookPointFlags),

		CamChangeAxis(u16), // [camera_change_axis] 0 CAMERA_ABSOLUTE_MODE, 1 CAMERA_RELATIVE_MODE
		CamMove(i32, i32, i32, u32 alias Time), // [camera_move]
		#[game(Fc,FcEvo,Sc,ScEvo,Tc,TcEvo)] _Cam68(u8), // TODO this isn't in any scripts? Is it from the asm?
		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7_Cam69(u8, u16),
		CamLookChar(u16 as CharId, u32 alias Time), // [camera_look_chr]
		_Char6A(u16 as CharId),
		CamZoom(i32, u32 alias Time), // [camera_zoom]
		#[game(Fc,FcEvo,Sc,ScEvo,Tc,TcEvo)] CamRotate(i32 alias Angle32, u32 alias Time), // [camera_rotate]
		#[game(Fc,FcEvo,Sc,ScEvo,Tc,TcEvo)] CamLookPos(Pos3, u32 alias Time), // [camera_look_at]
		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7CamRotate(i16 alias Angle, i16 alias Angle, i16 alias Angle, u32 alias Time),
		CamPers(u32, u32 alias Time), // [camera_pers]

		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] ObjFrame(u16 alias ObjectId, u32), // [mapobj_frame]
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] ObjPlay(u16 alias ObjectId, u32), // [mapobj_play]
		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7_6F(u8),
		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7ObjFrame(u8 as u16 alias ObjectId, u16),
		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7ObjPlay(u8 as u16 alias ObjectId, u16, u32, u32), // TODO EDDec thinks the first u32 is two u16
		ObjFlagsSet( // [mapobj_set_flag]
			{ IS::Fc|IS::FcEvo|IS::Sc|IS::ScEvo => u16, _ => u8 as u16 } alias ObjectId,
			{ IS::Fc|IS::FcEvo|IS::Sc|IS::ScEvo => u16 as u32, _ => u32 } as ObjectFlags,
		),
		ObjFlagsUnset( // [mapobj_reset_flag]
			{ IS::Fc|IS::FcEvo|IS::Sc|IS::ScEvo => u16, _ => u8 as u16 } alias ObjectId,
			{ IS::Fc|IS::FcEvo|IS::Sc|IS::ScEvo => u16 as u32, _ => u32 } as ObjectFlags,
		),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] ObjWait(u16 alias ObjectId),
		// I can confirm with 100% certainty that ObjFlags(Un)Set, ED7_76_0, ED7_74, and ED7ObjPlay have the same namespace, being "mapobj"

		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _74(u16, u32, u16),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _75(u8 as u16 alias ObjectId, u32, u8),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _76(u16, u32, u16, i32, i32, i32, u8, u8),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] MapColor(u32 as Color, u32 alias Time), // [map_color]
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _78(u8, u8, u8),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _79(u8 as u16 alias ObjectId, u16),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _7A(u8 as u16 alias ObjectId, u16),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _7B(),

		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7_74(u8 as u16 alias ObjectId, u16),
		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7_75(u8, u8, u32),
		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7_76(u8 as u16 alias ObjectId, String, match {
			0 => _0(u32),
			1 => _1(u32),
			2 => _2(String),
			3 => _3(i32),
			4 => _4(i32),
		}),
		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7_77(u8, u16),
		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7_78(u8 as u16 alias ObjectId, u16 as CharId),
		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7_79(u16 alias ObjectId),
		#[game(Zero)] skip!(2),
		#[game(Ao,AoEvo)] EventSkip(u8, u32), // TODO this one will need label handling
		#[game(Ao,AoEvo)] ED7_7B(u8),
		#[game(Zero,Ao)] skip!(1),
		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7_7D(u32 as Color, u32),
		#[game(Zero,Ao)] skip!(4),

		Shake(u32, u32, u32, u32 alias Time), // [quake]

		#[game(Fc, FcEvo)] skip!(1), // {asm} two-byte nop
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_7D(match {
			0 => _0(u16 as CharId, u16, u16),
			1 => _1(u16 as CharId, u16, u16), // args always zero; always paired with a _0 except when the char is 254
		}),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _7E(i16, i16, i16, u8, u32),

		#[game(Zero, Ao, AoEvo)] ED7_84(u8, u8),
		EffLoad(u8 alias EffId, String alias EffFileRef),
		EffPlay(
			u8 alias EffId, u8 /*alias EffInstanceId*/,
			u16 as CharId, { i if i.is_ed7() => u16, _ => const 0u16 }, Pos3, // source
			i16, i16, i16,
			u32, u32, u32, // scale?
			u16 as CharId, Pos3, // target
			u32 alias Time, // period (0 if one-shot)
		),
		EffPlay2(
			u8 alias EffId, u8 /*alias EffInstanceId*/,
			u8 as u16 alias ObjectId, String, { i if i.is_ed7() => u16, _ => const 0u16 }, Pos3, // source
			i16, i16, i16,
			u32, u32, u32, // scale
			u32 alias Time, // period (0 if one-shot)
		),
		EffStop(u8 alias EffId /*alias EffInstanceId*/, u8),
		#[game(Fc, FcEvo)] FcAchievement(u8, u8),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] _83(u8, u8), // might have to do with EffPlay
		_84(u8),
		_85(u8, u8),

		CharSetBase    (u16 as CharId, { i if i.is_ed7() => u8 as u16, _ => u16 } alias ChcpId), // [set_chr_base]
		CharSetPattern (u16 as CharId, { i if i.is_ed7() => u8 as u16, _ => u16 }), // [set_chr_ptn]
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7CharSetName (u16 as CharId, String alias TextTitle), // debug script only
		CharSetPos     (u16 as CharId, Pos3, i16 alias Angle), // [set_pos]
		CharSetPos2    (u16 as CharId, Pos3, i16 alias Angle),
		CharLookAtChar (u16 as CharId, u16 as CharId, u16 as u32 alias Time), // [look_to]
		CharLookAtPos  (u16 as CharId, Pos2, u16 as u32 alias Time),
		CharTurn       (u16 as CharId, i16 alias Angle, u16 as u32 alias Time), // [turn_to]
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
		_Char97        (u16 as CharId, Pos2, i32 alias Angle32, u32, u16),
		Sc_Char98(match {
			0 => _0(u16 as CharId),
			1 => _1(Pos3),
			2 => _2(u16 as CharId, u32, u8),
		}),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_A0(u16 as CharId, u16, u16),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] CharAnimation(u16 as CharId, u8, u8, u32 alias Time), // [chr_anime]
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7CharAnimation(u16 as CharId, u16, char_animation() -> Vec<u8> alias Animation),
		CharFlagsSet   (u16 as CharId, u16 as CharFlags), // [set_state]
		CharFlagsUnset (u16 as CharId, u16 as CharFlags), // [reset_state]
		CharFlag2Set   (u16 as CharId, u16 as CharFlags),
		CharFlags2Unset(u16 as CharId, u16 as CharFlags),
		CharShake      (u16 as CharId, u32, u32, u32, u32 alias Time),
		CharColor      (u16 as CharId, u32 as Color, u32 alias Time),
		Sc_A0          (u16 as CharId, u32 as Color, u8,u8,u8), // TODO Double-check
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] CharAttachObj(u16 as CharId, u16 alias ObjectId),
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
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] VarWait(u16 as Var, u16),

		// {asm} 6-byte nop
		skip!(1),

		ShopOpen(u8 as ShopId),

		#[game(Ao)] skip!(2),

		/// Saves the order of the party, to be loaded by [`PartyLoad`](Self::PartyLoad).
		///
		/// It saves eight variables, I don't know what's up with that.
		///
		/// Never used.
		#[game(Fc, FcEvo, Sc, ScEvo)] PartySave(),
		/// Loads the order of the party, as saved by [`PartySave`](Self::PartySave).
		///
		/// Never used.
		#[game(Fc, FcEvo, Sc, ScEvo)] PartyLoad(),

		#[game(Tc, TcEvo)] TcMonument(match {
			0 => Open(u8, u8, u8),
			1 => Disable(u8, u8, u8),
			2 => Enable(u8, u8, u8),
		}),
		#[game(Tc, TcEvo)] skip!(1),

		/// Learns a cooking recipe.
		///
		/// Returns whether the recipe was already known, i.e. if it was *not* successfully learned.
		RecipeLearn(u16),

		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] ImageShow(file_ref(game) -> String alias VisFileRef, u16, u16, u32 alias Time), // [portrait_open]
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] ImageHide(u32 alias Time), // [portrait_close]

		/// Attempts to submit a quest.
		///
		/// Returns a boolean value, probably whether the quest was successfully reported.
		/// What exactly this entails is unknown; the return value is never used.
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] QuestSubmit(u8 as ShopId, u16 as QuestId),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _ObjB0(u16 alias ObjectId, u8), // Used along with 6F, 70, and 73 during T0700#11
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] OpLoad(String alias OpFileRef),

		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_B1(u8),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] skip!(3),

		_B2(match {
			0 => Set(u8, u16),
			1 => Unset(u8, u16),
		}),

		Video(match {
			0 => Play(String alias AviFileRef, { IS::Fc|IS::FcEvo => const 0u16, _ => u16 }, { IS::Fc|IS::FcEvo => const 0u16, _ => u16 }), // [movie(MOVIE_START)]
			1 => End(u8, { IS::Fc|IS::FcEvo => const 0u16, _ => u16 }, { IS::Fc|IS::FcEvo => const 0u16, _ => u16 }), // [movie(MOVIE_END)], probably the 0 is the null terminator of an empty string
		}),

		ReturnToTitle(u8),

		/// Unlocks a character's orbment slot.
		///
		/// In SC onward, this is merged into PartySetSlot.
		#[game(Fc, FcEvo)] PartyUnlockSlot(u16 as NameId, u8),

		/// The argument is always zero in the scripts. According to the asm something else happens if it is nonzero, but it is unknown what.
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _B6(u8),

		/// This is related to [`PartyAdd`](Self::PartyAdd), but what it actually does is unknown.
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _B7(u16 as NameId, u8),

		/// This is related to [`PartyRemove`](Self::PartyRemove) and [`_B7`](Self::_B7), but as with that one, the details are unknown.
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _B8(u8 as u16 as NameId),

		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_B8(u16, u16),
		#[game(Zero, ZeroEvo, Ao)] skip!(1),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_BA(u8),

		/// Opens the book reading interface, as if using it from the inventory.
		///
		/// It is unknown what the second argument means; all known uses have zero.
		/// The assembly hints that it might be a character, in which case the instruction might be a more general-purpose use-item instruction.
		ReadBook(u16 as ItemId, u16),

		/// Returns whether the given member has a particular orbal art.
		///
		/// Does not work on crafts.
		PartyHasSpell(u8 as u16 as NameId, u16 as MagicId),

		/// Checks whether the given member has this orbment slot unlocked.
		PartyHasSlot(u8 as u16 as NameId, u8),

		#[game(Fc, FcEvo)] skip!(10),

		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] ScSetPortrait(u8 as u16 as NameId, u8, u8, u8, u8, u8),
		// This instruction is only used a single time throughout FC..=3rd, but this is its signature according to the asm
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, Ao)] Sc_BC(u8, match {
			0 => _0(u16),
			1 => _1(u16),
		}),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] ScSetPortraitFinish(),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_BE(u8,u8,u8,u8, u16, u16, u8, i32,i32,i32,i32,i32,i32),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_BF(u8,u8,u8,u8, u16 as Flag),
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
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] ScMinigame(u8, i32,i32,i32,i32,i32,i32,i32,i32),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao)] Sc_C1(u16 as ItemId, u32),
		#[game(Sc, ScEvo)] Sc_C2(),
		#[game(Tc, TcEvo)] Tc_C2(u8, u8),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_C5(u8, u8), // Achievement?

		/// Unused.
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, Ao)] Sc_C3(u16),

		/// Something for setting some kind of bit flags I guess.
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, Ao, AoEvo, ZeroEvo)] Sc_C4(match {
			0 => Set(u32),
			1 => Unset(u32),
		}),

		#[game(Fc)] skip!(3),
		#[game(FcEvo, Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] VisLoad(u8 alias VisId, i16,i16,u16,u16, i16,i16,u16,u16, i16,i16,u16,u16, u32 as Color, u8, String),
		#[game(FcEvo, Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] VisColor(u8 alias VisId, u8, u32 as Color, u32 alias Time, u32, { IS::FcEvo|IS::Ao|IS::AoEvo => u32, _ => const 0u32 }),
		#[game(FcEvo, Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] VisDispose(u8, u8 alias VisId, u8),

		#[game(Fc,FcEvo)] skip!(19),

		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_C8(u16, u16, String, u8, u16), // Something with C_PLATnn._CH
		#[game(Zero, Ao, AoEvo)] ED7_CC(u8),
		#[game(Zero, Ao, AoEvo)] CharId(match {
			1 => Set(u8 as u16 as CharId, u8 as u16 as NameId),
		}),
		#[game(ZeroEvo)] skip!(1),

		#[game(Sc, ScEvo, Tc, TcEvo)] ScPartySelect(u16, sc_party_select_mandatory() -> [Option<NameId>; 4] alias MandatoryMembers, sc_party_select_optional() -> Vec<NameId> alias OptionalMembers),
		#[game(Sc, ScEvo)] Sc_CA(u8 as u16 alias ObjectId, u8, u32),
		#[game(Tc, TcEvo)] Tc_CA(u8 as u16 alias ObjectId, u8, i32, u32),
		#[game(Sc, ScEvo)] ScCharInSlot(u8), // clearly related to CharId, but not the same
		#[game(Tc, TcEvo)] TcCharInSlot(u8, u8), // added team id I guess?
		#[game(Sc, ScEvo, Tc, TcEvo)] ScSelect(match {
			0 => New(u8 alias SelectId, u16, u16, u8),
			1 => Add(u8 alias SelectId, String alias MenuItem),
			2 => Show(u8 alias SelectId),
			3 => SetDisabled(u8 alias SelectId, u8),
		}),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7Select(match {
			0 => New(u8 alias SelectId),
			1 => Add(u8 alias SelectId, String alias MenuItem),
			2 => Show(u8 alias SelectId, u16, u16, u8),
			3 => SetDisabled(u8 alias SelectId, u8),
			4 => _4(u8 alias SelectId, u8),
			5 => _5(u8 alias SelectId, u8),
			6 => _6(u8 alias SelectId),
		}),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_CD(u16 as CharId), // related to showing photographs
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Global(u8 as Global, expr(game) -> Expr),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_CF(u16 as CharId, u8, String), // something with skeleton animation
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_D0(i32 alias Angle32, u32 alias Time),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_D1(u16 as CharId, i32, i32, i32, u32 alias Time), // something with camera?
		#[game(Sc, ScEvo, Tc, TcEvo)] ED6LoadChcp(file_ref(game) -> String, file_ref(game) -> String, u8 as u16 alias ChcpId),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7LoadChcp(file_ref(game) -> String, u8 as u16 alias ChcpId),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] UnloadChcp(u8 as u16 alias ChcpId),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] PartyGetAttr(u8 as u16 as NameId, u8),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] PartyGetEquipped(u8 as u16 as NameId, u8),

		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_D6(u8), // bool
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_D7(u8, u32, u16 as CharId),
		/// Always occurs before ObjSetFrame and ObjPlay. Probably animation speed?
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_D8(u8 as u16 alias ObjectId, u16),
		#[game(Sc, ScEvo, Tc, TcEvo)] ScCutIn(match {
			0 => Show(String), // CTInnnnn
			1 => Hide(),
		}),

		#[game(Zero)] skip!(2),
		#[game(Ao, AoEvo)] Ao_DA(u8),
		#[game(ZeroEvo, Ao)] skip!(1),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_DA(u8),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_DB(),
		#[game(Zero, ZeroEvo)] skip!(2),
		#[game(Ao, AoEvo)] Ao_DE(String),
		#[game(Ao, AoEvo)] skip!(1),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_DE(u16),
		#[game(Zero, Ao)] skip!(1),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_E0(u8),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_E1(Pos3),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7Note(match {
			0 => Fish(u8, match {
				0 => Count(),
				1 => MaxSize(),
			}),
			1 => Battle(match {
				0 => MonsterCount(),
			}),
			2 => _2(u8),
			3 => _3(),
		}),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_E3(u8),
		#[game(Zero, ZeroEvo)] skip!(1),
		#[game(Ao, AoEvo)] Ao_E6(u8, u8, u32 as Color, u32),

		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_DA(), // Something to do with menus

		#[game(Tc, TcEvo)] TcTeamMember(match {
			0 => Enable(u8 as u16 as NameId),
			1 => Disable(u8 as u16 as NameId),
			2 => _2(u8),
		}),
		#[game(Tc, TcEvo)] TcTeam(match {
			0 => Use(u8),
			1 => AddMember(u8, u8 as u16 as NameId),
			2 => Clear(u8),
		}),
		#[game(Tc, TcEvo)] TcOrganizeTeams(u8, u8, u8, u32 alias TcMembers, u32 alias TcMembers, u32 alias TcMembers, u32 alias TcMembers),
		#[game(Tc, TcEvo)] Tc_DE(u8, u32),
		#[game(Tc, TcEvo)] Tc_DF(u8, u16),
		#[game(Tc, TcEvo)] Tc_E0(u16 as CharId, u8, u8),
		#[game(Tc, TcEvo)] TcIndexInTeam(u8 as u16 as NameId, u8),
		/// Only used in a0028. Possibly related to minigames?
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
			0 => Start(u8, u32, u8),
			1 => End(u8),
			4 => _4(u8),
		}),
		#[game(Tc, TcEvo)] skip!(1),
		#[game(Tc, TcEvo)] Tc_E5(match {
			0 => _0(u8, u8 as u16 as NameId, u16, u16),
			1 => _1(u8, u8 as u16 as NameId, u16, u16),
			2 => _2(u8, u8 as u16 as NameId, u32),
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
		#[game(Zero, ZeroEvo, Ao, AoEvo)] AoEvo_D8(),
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
		#[game(TcEvo)] TcEvo_F7(u8, u16, u8), // Used exactly once, after breaking out of the planes. ScAchievement is not used there.
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
		#[game(TcEvo)] skip!(0),

		#[game(Zero, ZeroEvo, Ao)] skip!(1),
		#[game(Zero, Ao)] skip!(5),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_EE(u8, u16),
		#[game(Zero, ZeroEvo)] skip!(3),
		#[game(Ao)] skip!(2),
		#[game(Ao, AoEvo)] Ao_F3(i32),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_F2(u8),
		#[game(Zero)] skip!(5),
		#[game(ZeroEvo)] skip!(3),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_F8(u16),
		#[game(Zero)] skip!(7),
		#[game(Ao)] skip!(5),
		#[game(Ao, AoEvo)] Ao_FB(u8, u16),
		#[game(Ao, AoEvo)] Ao_FC(u16),
		#[game(Ao, AoEvo)] Ao_FD(u16, u16),
		#[game(Ao, AoEvo)] Ao_FE(u8),
		#[game(Ao, AoEvo)] Ao_FF(u8, Pos3),

		#[game(ZeroEvo, AoEvo)] ZeroEvo_E0(),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E1(u32,u32,u32,u32, u32,u32,u32,u32, u32,u32,u32,u32),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E2(u32,u32,u32, u8, u32, u32),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E3(u8, String, u32, u32, u8),
		#[game(AoEvo)] AoEvo_E7(u32, u32 as Color, u32, u32),
		#[game(AoEvo)] AoEvo_E8(u8),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E4(u8, u16),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E5(u8, u8, String, String, Pos3, Pos3, Pos3, u8),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E6(u8, u8, Pos3, Pos3, Pos3),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E7(u8, u8),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E8(u8),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E9(u16, u32,u32,u32,u32, u32,u32,u32,u32),
		#[game(AoEvo)] AoEvo_EF(),
		#[game(AoEvo)] AoEvo_F0(),
		#[game(AoEvo)] AoEvo_F1(),
		#[game(AoEvo)] AoEvo_F2(u8, u8),
		#[game(AoEvo)] AoEvo_F3(u8, u8),
		#[game(ZeroEvo)] skip!(22),
		#[game(AoEvo)] skip!(12),
	]
}

macro make_args(
	// Names need to be passed from outside for hygiene. Ugh.
	{ $name:ident $args:ident $args_mut:ident $into_parts:ident $arg_types:ident $from_parts:ident }
	[$(($ident:ident $(($_n:ident $alias:ident))*))*]
) {
	impl Insn {
		pub fn $name(&self) -> &'static str {
			match self {
				$(Self::$ident(..) => stringify!($ident),)*
			}
		}

		pub fn $args(&self) -> Box<[InsnArg]> {
			match self {
				$(Self::$ident($($_n),*) => Box::new([$(InsnArg::$alias($_n)),*]),)*
			}
		}

		pub fn $args_mut(&mut self) -> Box<[InsnArgMut]> {
			match self {
				$(Self::$ident($($_n),*) => Box::new([$(InsnArgMut::$alias($_n)),*]),)*
			}
		}

		pub fn $into_parts(self) -> (&'static str, Box<[InsnArgOwned]>) {
			let name = self.$name();
			let args: Box<[InsnArgOwned]> = match self {
				$(Self::$ident($($_n),*) => Box::new([$(InsnArgOwned::$alias($_n)),*]),)*
			};
			(name, args)
		}

		pub fn $arg_types(name: &str) -> Option<&'static [InsnArgType]> {
			match name {
				$(stringify!($ident) => Some(&[$(InsnArgType::$alias),*]),)*
				_ => None
			}
		}

		pub fn $from_parts(name: &str, args: impl IntoIterator<Item=InsnArgOwned>) -> Option<Self> {
			let mut it = args.into_iter();
			let v = match name {
				$(stringify!($ident) => {
					$(let Some(InsnArgOwned::$alias($_n)) = it.next() else { return None; };)*
					Self::$ident($($_n),*)
				})*
				_ => return None
			};
			if let Some(_) = it.next() { return None; }
			Some(v)
		}
	}
}
introspect!(make_args {name args args_mut into_parts arg_types from_parts});

mod color24 {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl Read<'a>) -> Result<Color, ReadError> {
		let a = f.u8()?;
		let b = f.u8()?;
		let c = f.u8()?;
		Ok(Color(u32::from_le_bytes([a, b, c, 0])))
	}

	pub(super) fn write(f: &mut impl Write, v: &Color) -> Result<(), WriteError> {
		let [a, b, c, _] = u32::to_le_bytes(v.0);
		f.u8(a);
		f.u8(b);
		f.u8(c);
		Ok(())
	}
}

mod quest_list {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl Read<'a>) -> Result<Vec<QuestId>, ReadError> {
		let mut quests = Vec::new();
		loop {
			match f.u16()? {
				0xFFFF => break,
				q => quests.push(QuestId(q))
			}
		}
		Ok(quests)
	}

	pub(super) fn write(f: &mut impl Write, v: &Vec<QuestId>) -> Result<(), WriteError> {
		for &i in v {
			f.u16(i.0);
		}
		f.u16(0xFFFF);
		Ok(())
	}
}

mod fork {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl Read<'a>, game: &GameData) -> Result<Vec<Insn>, ReadError> {
		let len = f.u8()? as usize;
		let pos = f.pos();
		let mut insns = Vec::new();
		while f.pos() < pos+len {
			insns.push(Insn::read(f, game)?);
		}
		ensure!(f.pos() == pos+len, "overshot while reading fork");
		if len > 0 {
			f.check_u8(0)?;
		}
		Ok(insns)
	}

	pub(super) fn write(f: &mut impl Write, game: &GameData, v: &[Insn]) -> Result<(), WriteError> {
		let (l1, l1_) = HLabel::new();
		let (l2, l2_) = HLabel::new();
		f.delay(move |l| Ok(u8::to_le_bytes(hamu::write::cast_usize(l(l2)? - l(l1)?)?)));
		f.label(l1_);
		for i in v {
			Insn::write(f, game, i)?;
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
	pub(super) fn read<'a>(f: &mut impl Read<'a>, game: &GameData) -> Result<Vec<Insn>, ReadError> {
		let len = f.u8()? as usize;
		let pos = f.pos();
		let mut insns = Vec::new();
		while f.pos() < pos+len {
			insns.push(Insn::read(f, game)?);
		}
		ensure!(f.pos() == pos+len, "overshot while reading fork loop");
		let next = if game.iset.is_ed7() {
			Insn::NextFrame2()
		} else {
			Insn::NextFrame()
		};
		ensure!(read_raw_insn(f, game)? == RawIInsn::Insn(next), "invalid loop");
		ensure!(read_raw_insn(f, game)? == RawIInsn::Goto(pos), "invalid loop");
		Ok(insns)
	}

	pub(super) fn write(f: &mut impl Write, game: &GameData, v: &[Insn]) -> Result<(), WriteError> {
		let (l1, l1_) = HLabel::new();
		let (l2, l2_) = HLabel::new();
		let l1c = l1.clone();
		f.delay(|l| Ok(u8::to_le_bytes(hamu::write::cast_usize(l(l2)? - l(l1)?)?)));
		f.label(l1_);
		for i in v {
			Insn::write(f, game, i)?;
		}
		f.label(l2_);
		let next = if game.iset.is_ed7() {
			Insn::NextFrame2()
		} else {
			Insn::NextFrame()
		};
		write_raw_insn(f, game, RawOInsn::Insn(&next))?;
		write_raw_insn(f, game, RawOInsn::Goto(l1c))?;
		Ok(())
	}
}

mod menu {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl Read<'a>) -> Result<Vec<String>, ReadError> {
		Ok(f.string()?.split_terminator('\x01').map(|a| a.to_owned()).collect())
	}

	pub(super) fn write(f: &mut impl Write, v: &[String]) -> Result<(), WriteError> {
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
	pub(super) fn read<'a>(f: &mut impl Read<'a>) -> Result<Emote, ReadError> {
		let a = f.u8()?;
		let b = f.u8()?;
		let c = f.u32()?;
		Ok(Emote(a, b, c))
	}

	pub(super) fn write(f: &mut impl Write, &Emote(a, b, c): &Emote) -> Result<(), WriteError> {
		f.u8(a);
		f.u8(b);
		f.u32(c);
		Ok(())
	}
}

pub(super) mod char_attr {
	use super::*;
	pub fn read<'a>(f: &mut impl Read<'a>) -> Result<CharAttr, ReadError> {
		let a = CharId(f.u16()?);
		let b = f.u8()?;
		Ok(CharAttr(a, b))
	}

	pub fn write(f: &mut impl Write, &CharAttr(a, b): &CharAttr) -> Result<(), WriteError> {
		f.u16(a.0);
		f.u8(b);
		Ok(())
	}
}

mod file_ref {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl Read<'a>, game: &GameData) -> Result<String, ReadError> {
		Ok(game.lookup.name(f.u32()?)?)
	}

	pub(super) fn write(f: &mut impl Write, game: &GameData, v: &str) -> Result<(), WriteError> {
		f.u32(game.lookup.index(v)?);
		Ok(())
	}
}

mod func_ref {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl Read<'a>, game: &GameData) -> Result<FuncRef, ReadError> {
		let a = f.u8()? as u16;
		let b = if game.iset.is_ed7() {
			f.u8()? as u16
		} else {
			f.u16()?
		};
		Ok(FuncRef(a, b))
	}

	pub(super) fn write(f: &mut impl Write, game: &GameData, &FuncRef(a, b): &FuncRef) -> Result<(), WriteError> {
		f.u8(cast(a)?);
		if game.iset.is_ed7() {
			f.u8(cast(b)?)
		} else {
			f.u16(b)
		};
		Ok(())
	}
}

mod func_ref_u8_u16 {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl Read<'a>) -> Result<FuncRef, ReadError> {
		let a = f.u8()? as u16;
		let b = f.u16()?;
		Ok(FuncRef(a, b))
	}

	pub(super) fn write(f: &mut impl Write, &FuncRef(a, b): &FuncRef) -> Result<(), WriteError> {
		f.u8(cast(a)?);
		f.u16(b);
		Ok(())
	}
}

mod text {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl Read<'a>) -> Result<Text, ReadError> {
		crate::text::Text::read(f)
	}

	pub(super) fn write(f: &mut impl Write, v: &Text) -> Result<(), WriteError> {
		crate::text::Text::write(f, v)
	}
}

mod sc_party_select_mandatory {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl Read<'a>) -> Result<[Option<NameId>; 4], ReadError> {
		f.multiple_loose::<4, _>(&[0xFF,0], |g| Ok(NameId(cast(g.u16()?)?)))
	}

	pub(super) fn write(f: &mut impl Write, v: &[Option<NameId>; 4]) -> Result<(), WriteError> {
		f.multiple_loose::<4, _>(&[0xFF,0], v, |g, a| { g.u16(a.0.into()); Ok(()) })
	}
}

mod sc_party_select_optional {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl Read<'a>) -> Result<Vec<NameId>, ReadError> {
		let mut quests = Vec::new();
		loop {
			match f.u16()? {
				0xFFFF => break,
				q => quests.push(NameId(cast(q)?))
			}
		}
		Ok(quests)
	}

	pub(super) fn write(f: &mut impl Write, v: &Vec<NameId>) -> Result<(), WriteError> {
		for &i in v {
			f.u16(i.0.into());
		}
		f.u16(0xFFFF);
		Ok(())
	}
}

mod char_animation {
	use super::*;
	pub(super) fn read<'a>(f: &mut impl Read<'a>) -> Result<Vec<u8>, ReadError> {
		let n = f.u8()?;
		let mut a = Vec::with_capacity(n as usize);
		if n == 0 {
			f.check_u8(0)?;
		}
		for _ in 0..n {
			a.push(f.u8()?);
		}
		Ok(a)
	}

	pub(super) fn write(f: &mut impl Write, v: &Vec<u8>) -> Result<(), WriteError> {
		f.u8(cast(v.len())?);
		if v.is_empty() {
			f.u8(0)
		}
		for &i in v {
			f.u8(i);
		}
		Ok(())
	}
}
