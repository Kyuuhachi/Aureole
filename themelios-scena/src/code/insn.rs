use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// InstructionSet
enum ISet {
	Fc, FcEvo,
	Sc, ScEvo,
	Tc, TcEvo,
	Zero, ZeroEvo,
	Ao, AoEvo,
}

fn iset(g: Game) -> ISet {
	match g {
		Game::Fc => ISet::Fc,
		Game::FcEvo => ISet::FcEvo,
		Game::FcKai => ISet::Fc,
		Game::Sc    => ISet::Sc,
		Game::ScEvo => ISet::ScEvo,
		Game::ScKai => ISet::Sc,
		Game::Tc    => ISet::Tc,
		Game::TcEvo => ISet::TcEvo,
		Game::TcKai => ISet::Tc,
		Game::Zero    => ISet::Zero,
		Game::ZeroEvo => ISet::ZeroEvo,
		Game::ZeroKai => ISet::Zero,
		Game::Ao    => ISet::Ao,
		Game::AoEvo => ISet::AoEvo,
		Game::AoKai => ISet::Ao,
	}
}

themelios_macros::bytecode! {
	(game: Game)
	#[games(iset(game) => ISet::{Fc, FcEvo, Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo})]
	[
		skip!(1), // null
		Return(), // [return]
		skip!(3), // control flow
		Call(FuncId via func_id),

		/// Loads another scena.
		///
		/// The second argument is which entrance (in the `._en` file) to start at, the others are
		/// unknown.
		///
		/// Official name is `new_scene`, which also implicitly adds a [`Hcf`](Self::Hcf).
		NewScene(FileId, u8, u8, u8),

		/// Simply halts the script forever.
		///
		/// Doesn't exist naturally in vanilla scripts, but instead inserted implicitly after
		/// `new_scene`.
		Hcf(),

		Sleep(if game.is_ed7() { u16 as u32 } else { u32 } as Time), // [delay]
		SystemFlagsSet(u32 as SystemFlags), // [set_system_flag]
		SystemFlagsUnset(u32 as SystemFlags), // [reset_system_flag]

		FadeOut(u32 as Time, u32 as Color, u8), // [fade_out]
		FadeIn(u32 as Time, u32 as Color), // [fade_in]
		FadeWait(), // [fade_wait]
		CrossFade(u32 as Time), // [cross_fade]

		#[game(Fc,FcEvo,Sc,ScEvo,Tc,TcEvo)]
		ED6Battle(u32 as BattleId, FileId, u8, u16, CharId via char_id8),

		def! ED7Battle(BattleId, FileId, u8, u16, u16, CharId),
		def! ED7NpcBattle(FileId, u8, [FileId; 8], u16, u16),

		#[game(Zero, ZeroEvo, Ao, AoEvo)]
		custom! {
			read => |f| {
				let ptr = f.u32()?;
				if ptr != 0xFFFFFFFF {
					// Pointer is filled in properly later
					Ok(Self::ED7Battle(
						BattleId(ptr),
						FileId(f.u32()?), f.u8()?,
						f.u16()?,
						f.u16()?,
						CharId::read(f, game)?,
					))
				} else {
					Ok(Self::ED7NpcBattle(
						FileId(f.u32()?), f.u8()?,
						array::<8, _>(|| Ok(FileId(f.u32()?))).strict()?,
						{
							f.check(&[0;16])?;
							f.u16()?
						},
						f.u16()?,
					))
				}
			},
			write ED7Battle(ptr, s1,s2, a1, a2, ch) => |f| {
				f.delay32(gospel::write::Label::known(ptr.0));
				f.u32(s1.0); f.u8(*s2);
				f.u16(*a1); f.u16(*a2);
				CharId::write(f, game, ch)?;
				Ok(())
			},
			write ED7NpcBattle(s1,s2, c, a1, a2) => |f| {
				f.u32(0xFFFFFFFF);
				f.u32(s1.0); f.u8(*s2);
				for c in c {
					f.u32(c.0);
				}
				f.array([0;16]);
				f.u16(*a1); f.u16(*a2);
				Ok(())
			},
		},

		/// Sets whether an entrance (or rather exit), defined in the accompanying `._en` file, is enabled.
		/// Specifically, it sets the 0x0001 flag.
		/// I think `1` sets the exit as enabled, `0` as disabled. But I could be misreading it.
		EntranceSetDisabled(u8 as EntranceId, u8),

		/// I have not been able to verify this one, the asm is complex.
		///
		/// Arguments are `D3DRS_FOGCOLOR` (24 bit, so color ignored), `D3DRS_FOGSTART`, `D3DRS_FOGEND`, `D3DRS_RANGEFOGENABLE`, `D3DRS_FOGDENSITY`.
		/// But since `D3DRS_FOGVERTEXMODE` is hardcoded to `D3DFOG_LINEAR` (at least in FC), the third parameter is never used.
		Fog(Color via color24, i32, i32, i32),

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
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)]
		BlurOn(
			if game.base() == BaseGame::Ao { u16 as u32 } else { u32 } as Time,
			u32 as Color,
			if game.base() == BaseGame::Ao { u16 as u32 } else { u32 },
			u8,
			if game.base() == BaseGame::Ao { u16 as u32 } else { u32 }
		),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)]
		BlurOff(u32 as Time),

		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)]
		_(match u8 {
			0 => MapHide(),
			1 => MapShow(),
			2 => MapSet(i32, Pos2, FileId),
		}),
		#[game(Zero, ZeroEvo)]
		_(match u8 {
			2 => ZeroMap_2(i32, Pos2, u16, u16, u16, u16),
			3 => ZeroMap_3(u8, u16),
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

		EntranceSetFunc(u8 as EntranceId, FuncId via func_id_u8_u16),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)]
		ED6_1C(u8 as u16 as ObjectId, FuncId via func_id_u8_u16),
		#[game(Zero, Ao, AoEvo)]
		ED7_1C(u8, u8 as u16 as ObjectId, u8, u8, FuncId via func_id, u16 as Flag, u16),

		#[game(Zero, ZeroEvo, Ao, AoEvo)]
		_(match u8 {
			0 => ED7_1D_0(u8, u8, u8, Pos3, i32, i32, i32),
			2 => ED7_1D_2(u8, u8),
			3 => ED7_1D_3(u8, u8),
		}),

		BgmPlay(
			if game.is_ed7() { u16 } else { u8 as u16 } as BgmId,
			if game.is_ed7() { u8 } else { const 0u8 },
		), // [play_bgm]
		BgmResume(), // [resume_bgm]
		BgmVolume(u8, u32 as Time), // [volume_bgm]
		BgmStop(u32 as Time), // [stop_bgm]
		BgmWait(), // [wait_bgm]

		SoundPlay(
			if matches!(game, Game::ZeroKai|Game::AoKai) { u32 } else { u16 as u32 } as SoundId,
			u8,
			if game.is_ed7() { u8 } else { const 0u8 },
			u8,
		), // [sound]
		SoundStop(
			if game == Game::AoKai { u32 } else { u16 as u32 } as SoundId,
		),
		SoundSetVolume(u16 as u32 as SoundId, u8),
		SoundPlayContinuously(u16 as u32 as SoundId, Pos3, u32, u32, u8, u32),
		SoundLoad(
			if game == Game::AoKai { u32 } else { u16 as u32 } as SoundId,
		), // [sound_load]

		#[game(Fc,FcEvo,Sc,ScEvo,Tc,TcEvo)] skip!(1),
		#[game(Zero,ZeroEvo,Ao,AoEvo)] NextFrame2(),

		_(u16 as QuestId, match u8 {
			0x01 => QuestTaskSet(u16 as QuestTask),
			0x02 => QuestTaskUnset(u16 as QuestTask),
			0x03 => QuestFlagsUnset(u8 as QuestFlags),
			0x04 => QuestFlagsSet(u8 as QuestFlags),
		}),
		_(u16 as QuestId, match u8 {
			0x00 => QuestFlagsGet(u8 as QuestFlags),
			0x01 => QuestTaskGet(u16 as QuestTask),
		}),
		QuestList(Vec<QuestId> via quest_list),
		QuestBonusBp(u16 as QuestId, u16),
		QuestBonusMira(u16 as QuestId, u16),

		PartyAdd(u8 as u16 as NameId, CharId via char_id8, if game.base() == BaseGame::Fc { const 0u8 } else { u8 }), // [join_party]
		PartyRemove(u8 as u16 as NameId, u8), // [separate_party]
		PartyClear(),
		#[game(Fc,FcEvo,Sc,ScEvo,Tc,TcEvo)] _30(u8),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_31(u8),
		PartySetAttr(u8 as u16 as NameId, u8, u16), // [set_status]
		#[game(Fc,FcEvo,Sc,ScEvo,Tc,TcEvo,Zero,Ao)] skip!(2),
		PartyAddArt(u8 as u16 as NameId, u16 as MagicId),
		PartyAddCraft(u8 as u16 as NameId, u16 as MagicId),
		#[game(Fc,FcEvo,Sc,ScEvo,Tc,TcEvo)] PartyAddSCraft(u8 as u16 as NameId, u16 as MagicId),
		#[game(Zero, ZeroEvo,Ao, AoEvo)] ED7_37(),

		def! PartySetSlot(NameId, u8, u8),
		custom! {
			read => |f| {
				let a = NameId(f.u8()? as u16);
				let b = f.u8()?;
				let c = if game.base() == BaseGame::Fc || (0x7F..=0xFE).contains(&b) {
					f.u8()?
				} else {
					0
				};
				Ok(Self::PartySetSlot(a, b, c))
			},
			write PartySetSlot(a, b, c) => |f| {
				f.u8(cast(a.0)?);
				f.u8(*b);
				if game.base() == BaseGame::Fc || (0x7F..=0xFE).contains(b) {
					f.u8(*c);
				} else {
					ensure!(*c == 0, "{:?} must be {:?}", *c, 0);
				};
				Ok(())
			},
		},

		SepithAdd(u8, u16),
		SepithRemove(u8, u16),
		MiraAdd(u16), // [get_gold]
		MiraSub(u16),
		BpAdd(u16),
		BpSub(u16),
		ItemAdd(u16 as ItemId, u16),
		ItemRemove(u16 as ItemId, u16), // [release_item]
		ItemHas(u16 as ItemId, if game.base() == BaseGame::Fc { const 0u8 } else { u8 }), // or is it ItemGetCount?

		PartyEquip(u8 as u16 as NameId, u16 as ItemId, if game.base() == BaseGame::Fc && !(600..=799).contains(&_1.0) { const 0u8 } else { u8 }),
		PartyPosition(u8 as u16 as NameId),

		ForkFunc(CharId, u8 as u16 as ForkId, FuncId via func_id), // [execute]
		ForkQuit(CharId, u8 as u16 as ForkId), // [terminate]
		Fork(    CharId, if game.is_ed7() { u8 as u16 } else { u16 } as ForkId, Code via fork), // [preset]? In t0311, only used with a single instruction inside
		ForkLoop(CharId, if game.is_ed7() { u8 as u16 } else { u16 } as ForkId, Code via fork_loop),
		ForkWait(CharId, if game.is_ed7() { u8 as u16 } else { u16 } as ForkId), // [wait_terminate]
		NextFrame(), // [next_frame]

		Event(FuncId via func_id), // [event] Not sure how this differs from Call

		_Char4A(CharId, u8), // Argument is almost always 255, but sometimes 0, and in a single case 1
		_Char4B(CharId, u8),

		skip!(1), // {asm} one-byte nop
		Var(u16 as Var, Expr),
		skip!(1), // {asm} one-byte nop
		Attr(u8 as Attr, Expr), // [system[n]]
		skip!(1), // {asm} one-byte nop
		CharAttr(CharAttr via char_attr, Expr),

		TextStart(CharId), // [talk_start]
		TextEnd(CharId), // [talk_end]
		/// Shows a text box without a speak bubble arrow.
		///
		/// I believe the CharId, which is only present in ED7, is used to select the textbox title.
		/// However, it is 999 on chests.
		TextMessage(CharId via char_id_text_message, Text), // [mes]
		skip!(1), // {asm} same as NextFrame
		TextClose(u8), // [mes_close]
		ScMenuSetTitle(u16, u16, u16, TString),
		TextWait(), // [wait_prompt]
		_59(), // Always directly after a TextReset 1, and exists in all but one such case. I suspect that one is a bug.
		TextSetPos(i16, i16, i16, i16), // [mes_pos]
		TextTalk(CharId, Text), // [popup]
		TextTalkNamed(CharId, TString, Text), // [popup2]
		Menu(u16 as MenuId, i16, i16, u8, Vec<TString> via menu), // [menu] (the u8 is a bool)
		MenuWait(u16 as Var), // [wait_menu]
		MenuClose(u16 as MenuId), // [menu_close]
		TextSetName(TString), // [name]
		CharName2(CharId), // [name2]

		Emote(CharId, i32 as Length, i32 as Length, u8, u8, u32 as Time, u8), // [emotion] mostly used through macros such as EMO_BIKKURI3().
		EmoteStop(CharId), // [emotion_close]

		LookPointFlagsSet(u8 as u16 as LookPointId, u16 as LookPointFlags),
		LookPointFlagsUnset(u8 as u16 as LookPointId, u16 as LookPointFlags),

		CamChangeAxis(u16), // [camera_change_axis] 0 CAMERA_ABSOLUTE_MODE, 1 CAMERA_RELATIVE_MODE
		CamSetDistance(i32, i32, i32, u32 as Time), // [camera_move]
		#[game(Fc,FcEvo,Sc,ScEvo,Tc,TcEvo)] _Cam68(u8), // TODO this isn't in any scripts? Is it from the asm?
		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7_Cam69(u8, u16),
		CamLookChar(CharId, u32 as Time), // [camera_look_chr]
		_Char6A(CharId),
		CamZoom(i32, u32 as Time), // [camera_zoom]
		#[game(Fc,FcEvo,Sc,ScEvo,Tc,TcEvo)] CamRotate(i32 as Angle32, u32 as Time), // [camera_rotate]
		#[game(Fc,FcEvo,Sc,ScEvo,Tc,TcEvo)] CamLookPos(Pos3, u32 as Time), // [camera_look_at]
		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7CamRotate(i16 as Angle, i16 as Angle, i16 as Angle, u32 as Time),
		CamPers(i32, u32 as Time), // [camera_pers]

		/// Waits for camera animations to finish.
		///
		/// The argument might be a bitfield, telling which animations to wait for. The values are
		/// unknown. however. A common value seems to be 121.
		#[game(Zero,ZeroEvo,Ao,AoEvo)]
		CamWait(u8),

		/// Instantly sets a map object's frame counter.
		///
		/// Official name is `mapobj_frame`.
		ObjFrame(
			if game.is_ed7() { u8 as u16 } else { u16 } as ObjectId,
			if game.is_ed7() { u16 as u32 } else { u32 },
		),

		/// Animates a map object to the given frame.
		///
		/// Official name is `mapobj_play`.
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)]
		ED6ObjPlay(u16 as ObjectId, u32),

		/// Animates a map object between the given frames.
		///
		/// The latter two arguments are unknown.
		#[game(Zero,ZeroEvo,Ao,AoEvo)]
		ED7ObjPlay(u8 as u16 as ObjectId, u16, u16, u16, u32),

		/// Sets a flag on a map object.
		///
		/// See [`ObjectFlags`] for details.
		///
		/// Official name is `mapobj_set_flag`.
		ObjFlagsSet(
			if matches!(game.base(), BaseGame::Fc|BaseGame::Sc) { u16 } else { u8 as u16 } as ObjectId,
			if matches!(game.base(), BaseGame::Fc|BaseGame::Sc) { u16 as u32 } else { u32 } as ObjectFlags,
		),

		/// Unsets a flag on a map object.
		///
		/// See [`ObjectFlags`] for details.
		///
		/// Official name is `mapobj_reset_flag`.
		ObjFlagsUnset(
			if matches!(game.base(), BaseGame::Fc|BaseGame::Sc) { u16 } else { u8 as u16 } as ObjectId,
			if matches!(game.base(), BaseGame::Fc|BaseGame::Sc) { u16 as u32 } else { u32 } as ObjectFlags,
		),

		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7_74(u8 as u16 as ObjectId, u8, u8),
		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7_75(u8 as u16 as ObjectId, u8, u32 as Time),
		#[game(Zero,ZeroEvo,Ao,AoEvo)] _(u8 as u16 as ObjectId, String, match u8 {
			0 => ED7_76_0(u32),
			1 => ED7_76_1(u32),
			2 => ED7_76_2(String),
			3 => ED7_76_3(i32),
			4 => ED7_76_4(i32),
		}),
		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7_77(u8 as u16 as ObjectId, u16),
		#[game(Zero,ZeroEvo,Ao,AoEvo)] ED7_78(u8 as u16 as ObjectId, CharId),

		/// Waits for a map object to finish its animation.
		ObjWait(u16 as ObjectId),

		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _74(u16 as ObjectId, u32, u16),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _75(u8 as u16 as ObjectId, u32, u8),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _76(u16 as ObjectId, u32, u16, i32, i32, i32, u8, u8),
		#[game(Zero)] skip!(2),
		#[game(Ao,AoEvo)] EventSkip(u8, u32), // TODO this one will need label handling
		#[game(Ao,AoEvo)] ED7_7B(u8 as u16 as NameId),
		#[game(Zero,Ao)] skip!(1),
		MapColor(u32 as Color, u32 as Time), // [map_color]
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _78(Color via color24),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _79(u8 as u16 as ObjectId, u16),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _7A(u8 as u16 as ObjectId, u16),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _7B(),
		#[game(Zero,Ao)] skip!(4),

		Shake(u32, u32, u32, u32 as Time), // [quake]

		#[game(Fc, FcEvo)] skip!(1), // {asm} two-byte nop
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] _(match u8 {
			0 => Sc_7D_0(CharId, u16, u16),
			1 => Sc_7D_1(CharId, u16, u16), // args always zero; always paired with a _0 except when the char is 254
		}),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _7E(i16, i16, i16, u8, u32),

		#[game(Zero, Ao, AoEvo)] ED7_84(u8, u8), // Wild guess: master quartz.
		EffLoad(u8 as EffId, String),
		EffPlay(
			u8 as EffId, u8 as EffInstanceId,
			CharId, if game.is_ed7() { u16 } else { const 0u16 }, Pos3, // source
			i16, i16, i16,
			u32, u32, u32, // scale?
			CharId, Pos3, // target
			u32 as Time, // period (0 if one-shot)
		),
		EffPlay2(
			u8 as EffId, u8 as EffInstanceId,
			u8 as u16 as ObjectId, String, if game.is_ed7() { u16 } else { const 0u16 }, Pos3, // source
			i16, i16, i16,
			u32, u32, u32, // scale
			u32 as Time, // period (0 if one-shot)
		),
		EffStop(u8 as EffInstanceId, u8),
		#[game(Fc, FcEvo)] FcAchievement(u8, u8),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] _83(u8 as EffInstanceId, u8),
		EffUnload(u8 as EffId),
		_85(CharId),

		CharSetChipBase    (CharId, if game.is_ed7() { u8 as u16 } else { u16 } as ChipId), // [set_chr_base]
		CharSetChipPattern (CharId, if game.is_ed7() { u8 as u16 } else { u16 }), // [set_chr_ptn]

		#[game(Zero, ZeroEvo, Ao, AoEvo)]
		CharSetName(CharId, TString),

		CharSetPos     (CharId, Pos3, i16 as Angle), // [set_pos]
		CharSetPos2    (CharId, Pos3, i16 as Angle),

		/// Turns to look at another character.
		///
		/// Official name is `look_to`.
		CharTurnToChar (CharId, CharId, u16 as AngularSpeed),

		/// Turns to look at a specific position.
		CharTurnToPos  (CharId, Pos2,          u16 as AngularSpeed),

		/// Turns to look in a specific direction.
		///
		/// Official name is `turn_to`.
		CharTurnTo     (CharId, i16 as Angle,  u16 as AngularSpeed),

		/// Walks to a random position inside the given rectangle.
		CharIdle       (CharId, Pos2, Pos2, u32 as Speed),

		/// Walks to a position.
		///
		/// Official name is `walk_to`.
		CharWalkToPos  (CharId, Pos3, u32 as Speed, u8),

		/// Walks to a position, without looking forward.
		CharWalkToPos2 (CharId, Pos3, u32 as Speed, u8),

		_Char90        (CharId, i32, i32, i32, u32, u8),
		_Char91        (CharId, i32, i32, i32, i32, u8),

		/// Walks to near where another character is standing.
		///
		/// Official name is `walk_to_chr`.
		CharWalkToChar (CharId, CharId, i32 as Length, u32 as Speed, u8),

		/// Walks to near where another character is standing, without looking forward.
		CharWalkToChar2(CharId, CharId, i32 as Length, u32 as Speed, u8),

		_94        (u8, CharId, i16 as Angle, i32, u32 as Speed, u8),
		CharJump       (CharId, i32, i32, i32, u32, u32 as Speed), // [jump]
		_Char96        (CharId, Pos3, i32, i32),
		_Char97        (CharId, Pos2, i32 as Angle32, u32, u16),
		_(match u8 {
			0 => CharPathNew(CharId),
			1 => CharPathAdd(Pos3),
			2 => CharPathRun(CharId, u32 as Speed, u8),
		}),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_A0(CharId, u16 as u32 as Time, u8, u8),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] CharAnimation(CharId, u8, u8, u32 as Time), // [chr_anime]
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7CharAnimation(CharId, u16 as u32 as Time, Vec<u8> via char_animation),
		CharFlagsSet   (CharId, u16 as CharFlags), // [set_state]
		CharFlagsUnset (CharId, u16 as CharFlags), // [reset_state]
		CharFlags2Set  (CharId, u16 as CharFlags),
		CharFlags2Unset(CharId, u16 as CharFlags),
		CharShake      (CharId, u32, u32, u32, u32 as Time),
		CharColor      (CharId, u32 as Color, u32 as Time),
		Sc_A0          (CharId, u32 as Color, u8,u8,u8), // TODO Double-check
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] CharAttachObj(CharId, u16 as ObjectId),
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

		#[game(Tc, TcEvo)] _(match u8 {
			0 => TcMonumentOpen(u8, u8, u8),
			1 => TcMonumentDisable(u8, u8, u8),
			2 => TcMonumentEnable(u8, u8, u8),
		}),
		#[game(Tc, TcEvo)] skip!(1),

		/// Learns a cooking recipe.
		///
		/// Returns whether the recipe was already known, i.e. if it was *not* successfully learned.
		RecipeLearn(u16),

		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] ImageShow(FileId, u16, u16, u32 as Time), // [portrait_open]
		// This is sometimes called with a vis[]. I think that's a bug in the scripts, 
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] ImageHide(u32 as Time), // [portrait_close]

		/// Attempts to submit a quest.
		///
		/// Returns a boolean value, probably whether the quest was successfully reported.
		/// What exactly this entails is unknown; the return value is never used.
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] QuestSubmit(u8 as ShopId, u16 as QuestId),
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] _ObjB0(u16 as ObjectId, u8), // Used along with 6F, 70, and 73 during T0700#11
		#[game(Fc, FcEvo, Sc, ScEvo, Tc, TcEvo)] OpLoad(String),

		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_B1(u8),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] skip!(3),

		_(match u8 {
			0 => TriggerFlagsSet(u8 as u16 as TriggerId, u16 as TriggerFlags),
			1 => TriggerFlagsUnset(u8 as u16 as TriggerId, u16 as TriggerFlags),
		}),

		_(match u8 {
			0 => VideoPlay(String, if game.base() == BaseGame::Fc { const 0u16 } else { u16 }, if game.base() == BaseGame::Fc { const 0u16 } else { u16 }), // [movie(MOVIE_START)]
			1 => VideoEnd(String,  if game.base() == BaseGame::Fc { const 0u16 } else { u16 }, if game.base() == BaseGame::Fc { const 0u16 } else { u16 }), // [movie(MOVIE_END)]
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

		ItemUse(u16 as ItemId, CharId),

		/// Returns whether the given member has a particular orbal art.
		///
		/// Does not work on crafts.
		PartyHasSpell(u8 as u16 as NameId, u16 as MagicId),

		/// Checks whether the given member has this orbment slot unlocked.
		PartyHasSlot(u8 as u16 as NameId, u8),

		#[game(Fc, FcEvo)] skip!(10),

		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] PartySetPortrait(u8 as u16 as NameId, u8, u8, u8, u8, u8),
		// This instruction is only used a single time throughout FC..=3rd, but this is its signature according to the asm
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, Ao)] _(u8, match u8 {
			0 => Sc_BC_0(u16),
			1 => Sc_BC_1(u16),
		}),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] PartySetPortraitFinish(),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_BE(u8,u8,u8,u8, u16, u16, u8, i32,i32,i32,i32,i32,i32),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_BF(u8,u8,u8,u8, u16 as Flag),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Minigame(u8, i32,i32,i32,i32,i32,i32,i32,i32),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao)] Sc_C1(u16 as ItemId, u32),
		#[game(Sc, ScEvo)] Sc_C2(),
		#[game(Tc, TcEvo)] Tc_C2(u8, u8),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_C5(u8, u8), // Achievement?

		/// Unused.
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, Ao)] Sc_C3(u16),

		/// Something for setting some kind of bit flags I guess.
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, Ao, AoEvo, ZeroEvo)] _(match u8 {
			0 => Sc_C4Set(u32),
			1 => Sc_C4Unset(u32),
		}),

		#[game(Fc)] skip!(3),
		#[game(FcEvo, Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)]
		VisShow(u8 as VisId, i16,i16,u16,u16, i16,i16,u16,u16, i16,i16,u16,u16, u32 as Color, u8, String),
		#[game(FcEvo, Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)]
		/// Attribute 3 is color. The others are unknown, but probably include at least position, scale, and rotation.
		VisSet(u8 as VisId, u8, i32, i32, i32, if matches!(game, Game::FcEvo|Game::Ao|Game::AoEvo|Game::AoKai) { i32 } else { const 0i32 }),
		// VisSet vis[_] 0 i32 i32 Time 0
		// VisSet vis[_] 1 i32 i32 Time 0
		// VisSet vis[_] 2 i32 i32 Time 0
		// VisSet vis[_] 3 Color Time 0 0
		// 4..9 are unknown
		#[game(FcEvo, Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] _(match u8 {
			0 => VisWait(u8 as VisId, u8),
			1 => VisHide(u8 as VisId, u8),
		}),

		#[game(Fc,FcEvo)] skip!(19),

		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_C8(u16, u16, String, u8, u16), // Something with C_PLATnn._CH
		#[game(Zero, Ao, AoEvo)] ED7_CC(u8),
		#[game(Zero, Ao, AoEvo)] _(match u8 {
			1 => CharIdSet(CharId via char_id8, u8 as u16 as NameId),
		}),
		#[game(ZeroEvo)] skip!(1),

		#[game(Sc, ScEvo, Tc, TcEvo)] PartySelect(u16, [Option<NameId>; 4] via sc_party_select_mandatory, Vec<NameId> via sc_party_select_optional),
		#[game(Sc, ScEvo)] Sc_CA(u8 as u16 as ObjectId, u8, u32),
		#[game(Tc, TcEvo)] Tc_CA(u8 as u16 as ObjectId, u8, i32, u32),
		#[game(Sc, ScEvo, Tc, TcEvo)] CharInSlot(if game.base() == BaseGame::Tc { u8 } else { const 0u8 }, CharId via char_id8),
		#[game(Sc, ScEvo, Tc, TcEvo)] _(match u8 {
			0 => ED6MenuNew(u8 as u16 as MenuId, u16, u16, u8),
			1 => ED6MenuAdd(u8 as u16 as MenuId, TString),
			2 => ED6MenuShow(u8 as u16 as MenuId),
			3 => ED6MenuSetDisabled(u8 as u16 as MenuId, u8),
		}),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] _(match u8 {
			0 => ED7MenuNew(u8 as u16 as MenuId),
			1 => ED7MenuAdd(u8 as u16 as MenuId, TString),
			2 => ED7MenuShow(u8 as u16 as MenuId, u16, u16, u8),
			3 => ED7MenuSetDisabled(u8 as u16 as MenuId, u8),
			4 => ED7MenuSetStyle(u8 as u16 as MenuId, u8),
			5 => ED7MenuSelect(u8 as u16 as MenuId, u8),
			6 => ED7Menu_6(u8 as u16 as MenuId),
		}),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_CD(CharId), // related to showing photographs
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Global(u8 as Global, Expr),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_CF(CharId, u8, String), // something with skeleton animation
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_D0(i32 as Angle32, u32 as Time),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_D1(CharId, i32, i32, i32, u32 as Time), // something with camera?
		#[game(Sc, ScEvo, Tc, TcEvo)] ED6LoadChip(FileId, FileId, u8 as u16 as ChipId),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7LoadChip(FileId, u8 as u16 as ChipId),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] UnloadChip(u8 as u16 as ChipId),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] PartyGetAttr(u8 as u16 as NameId, u8),
		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] PartyGetEquipped(u8 as u16 as NameId, u8),

		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_D6(u8), // bool
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_D7(u8, u32, CharId),
		/// Always occurs before ObjSetFrame and ObjPlay. Probably animation speed?
		#[game(Sc, ScEvo, Tc, TcEvo)] Sc_D8(u8 as u16 as ObjectId, u16),
		#[game(Sc, ScEvo, Tc, TcEvo)] _(match u8 {
			0 => ScCutInShow(String), // CTInnnnn
			1 => ScCutInHide(),
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
		#[game(Zero, ZeroEvo, Ao, AoEvo)] _(match u8 {
			0 => _(u8, match u8 {
				0 => ED7NoteFishCount(),
				1 => ED7NoteFishMaxSize(),
			}),
			1 => _(match u8 {
				0 => ED7NoteBattleMonsterCount(),
			}),
			2 => ED7Note_2(u8),
			3 => ED7Note_3(),
		}),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] ED7_E3(u8),
		#[game(Zero, ZeroEvo)] skip!(1),
		#[game(Ao, AoEvo)] Ao_E6(u8, u8, u32 as Color, u32),

		#[game(Sc, ScEvo, Tc, TcEvo, Zero, ZeroEvo, Ao, AoEvo)] Sc_DA(), // Something to do with menus

		#[game(Tc, TcEvo)] _(match u8 {
			0 => TcTeamMemberEnable(u8 as u16 as NameId),
			1 => TcTeamMemberDisable(u8 as u16 as NameId),
			2 => TcTeamMember_2(u8),
		}),
		#[game(Tc, TcEvo)] _(match u8 {
			0 => TcTeamUse(u8),
			1 => TcTeamAddMember(u8, u8 as u16 as NameId),
			2 => TcTeamClear(u8),
		}),
		#[game(Tc, TcEvo)] TcOrganizeTeams(u8, u8, u8, u32 as TcMembers, u32 as TcMembers, u32 as TcMembers, u32 as TcMembers),
		#[game(Tc, TcEvo)] Tc_DE(u8, u32),
		#[game(Tc, TcEvo)] Tc_DF(u8, u16),
		#[game(Tc, TcEvo)] Tc_E0(CharId, u8, u8),
		#[game(Tc, TcEvo)] TcIndexInTeam(u8 as u16 as NameId, u8),
		/// Only used in a0028. Possibly related to minigames?
		#[game(Tc, TcEvo)] _(match u8 {
			0 => Tc_E2_0(u8),
			1 => Tc_E2_1(),
			3 => Tc_E2_3(u8),
			4 => Tc_E2_4(u8),
			5 => Tc_E2_5(u16, u16, u16),
			7 => Tc_E2_7(),
			8 => Tc_E2_8(),
			9 => Tc_E2_9(u8),
			10 => Tc_E2_10(), // A getter
			11 => Tc_E2_11(u8),
		}),
		#[game(Tc, TcEvo)] _(match u8 {
			0 => TcEpisodeStart(u8, u32, u8),
			1 => TcEpisodeEnd(u8),
			4 => TcEpisode_4(u8),
		}),
		#[game(Tc, TcEvo)] skip!(1),
		#[game(Tc, TcEvo)] _(match u8 {
			0 => Tc_E5_0(u8, u8, u16, u16),
			1 => Tc_E5_1(u8, u8, u16, u16),
			2 => Tc_E5_2(u8, u8, u32),
		}),
		#[game(Tc, TcEvo)] _(match u8 {
			0 => Tc_E6_0(u8),
			1 => Tc_E6_1(u8),
			2 => Tc_E6_2(),
		}),
		#[game(Tc, TcEvo)] Tc_E7(u8 as VisId, u8, u32 as Color, u32 as Time),

		#[game(Fc)] skip!(2),
		/// A no-op. Always paired with [`Sc_DC`](Self::Sc_DC).
		#[game(FcEvo, Sc, ScEvo, TcEvo)] Sc_DB(),
		/// A no-op. Always paired with [`Sc_DB`](Self::Sc_DB).
		#[game(FcEvo, Sc, ScEvo, TcEvo)] Sc_DC(),
		#[game(Zero, ZeroEvo, Ao, AoEvo)] AoEvo_D8(),
		#[game(Tc)] skip!(2),

		/// Opens the save menu in order to save clear data.
		SaveClearData(),

		#[game(FcEvo, Sc, ScEvo, TcEvo)] PlaceNameString(TString),
		#[game(FcEvo, Sc, ScEvo, TcEvo)] skip!(1),
		#[game(FcEvo, Sc, ScEvo, TcEvo)] Sc_E0(u8 as u16 as ObjectId, Pos3),
		#[game(FcEvo, Sc, ScEvo, TcEvo)] skip!(2),

		#[game(FcEvo)] EvoCtp(String), // Refers to /data/map2/{}.ctp

		#[game(Sc, ScEvo, TcEvo)] Sc_E3(u8, CharId, u8),
		/// A no-op.
		#[game(Sc, ScEvo, TcEvo)] Sc_E4(u8, u16),
		#[game(Sc, ScEvo)] Sc_E5(CharId, u8),
		#[game(TcEvo)] TcEvo_F2(CharId, u8, u16, u16),
		#[game(Sc, ScEvo)] Sc_E6(u8), // related to RAM saving, according to debug script
		#[game(TcEvo)] custom! {
			// What's EvoVisLipSync doing up here? Maybe they wanted FF to stay clear.
			read => |f| {
				Ok(Self::EvoVisLipSync(VisId(f.u8()?), f.u8()?))
			},
			write EvoVisLipSync(a, b) => |f| {
				f.u8(a.0);
				f.u8(*b);
				Ok(())
			},
		},
		#[game(Sc, ScEvo)] Sc_E7(u8 as u16 as ObjectId, String, u8,u8,u8,u8,u8),
		#[game(TcEvo)] skip!(1),
		#[game(Sc, ScEvo, TcEvo)] Sc_E8(u32 as Time),
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
		#[game(TcEvo)] TcEvo_F9(u16 as ObjectId, u8),
		/// Seems to be a way to apply [`Tc_E5_2`](Self::Tc_E5_2) to a large number of members.
		///
		/// Always preceded by a [`TcEvo_F9`](Self::TcEvo_F9), with the object matching. But sometimes the object is 0.
		#[game(TcEvo)] TcEvo_FA(u16 as ObjectId, u32),
		#[game(TcEvo)] TcEvo_FB(u8, CharId, u8),
		#[game(TcEvo)] TcEvo_FC(u8, u8),

		#[game(FcEvo, ScEvo, TcEvo)] EvoVoiceLine(u16), // [pop_msg]
		#[game(FcEvo, ScEvo, TcEvo)] Evo_E6(Text),
		#[game(FcEvo, ScEvo)] EvoVisLipSync(u8 as VisId, u8),
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
		#[game(Ao, AoEvo)] Ao_FF(u8 as u16 as ObjectId, Pos3),

		#[game(ZeroEvo, AoEvo)] ZeroEvo_E0(),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E1(u32,u32,u32,u32, u32,u32,u32,u32, u32,u32,u32,u32),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E2(u32,u32,u32, u8, u32, u32),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E3(u8, String, u32, u32, u8),
		#[game(AoEvo)] AoEvo_E7(u32, i32, i32, i32),
		#[game(AoEvo)] AoEvo_E8(u8),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E4(u8, u16),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E5(u8, u8, String, String, Pos3, Pos3, Pos3, u8),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E6(u8, u8, Pos3, Pos3, Pos3),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E7(u8, u8),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E8(u8),
		#[game(ZeroEvo, AoEvo)] ZeroEvo_E9(u16, u32,u32,u32,u32, u32,u32,u32,u32),
		#[game(AoEvo)] AoEvo_EF(),
		#[game(AoEvo)] AoEvoStopwatchGet(),
		#[game(AoEvo)] AoEvoStopwatchStart(),
		#[game(AoEvo)] AoEvo_F2(u8, u8),
		#[game(AoEvo)] AoEvo_F3(u8, u8),
		#[game(ZeroEvo)] skip!(22),
		#[game(AoEvo)] skip!(12),
	]
}

impl Insn {
	pub fn name(&self) -> &'static str {
		macro run(
			[$(($ident:ident $(($_n:ident $ty:ty))*))*]
		) {
			return match self {
				$(Self::$ident(..) => stringify!($ident),)*
			}
		}
		introspect!(run);
	}

	pub fn validate(game: Game, i: &Insn) -> Result<(), WriteError> {
		let mut w = Writer::new();
		Self::write(&mut w, game, i)
	}
}

trait Arg: Sized {
	fn read(f: &mut Reader, _: Game) -> Result<Self, ReadError>;
	fn write(f: &mut Writer, _: Game, v: &Self) -> Result<(), WriteError>;
}

macro arg($t:ty,
	|$fr:pat_param, $gr:pat_param| $r:expr,
	|$fw:pat_param, $gw:pat_param, $v:pat_param| $w:expr $(,)?
) {
	impl Arg for $t {
		fn read<'a>($fr: &mut Reader, $gr: Game) -> Result<$t, ReadError> {
			Ok($r)
		}

		fn write($fw: &mut Writer, $gw: Game, $v: &$t) -> Result<(), WriteError> {
			Ok($w)
		}
	}
}

macro prim_arg($t:ty, $i:ident) {
	arg!($t,
		|f, _| f.$i()?,
		|f, _, v| f.$i(*v),
	);
}

prim_arg!(u8, u8);
prim_arg!(u16, u16);
prim_arg!(u32, u32);
prim_arg!(i8, i8);
prim_arg!(i16, i16);
prim_arg!(i32, i32);
prim_arg!(Pos3, pos3);
prim_arg!(Pos2, pos2);

arg!(String,
	|f, _| f.string()?,
	|f, _, v| f.string(v.as_str())?,
);

arg!(TString,
	|f, _| TString(f.string()?),
	|f, _, v| f.string(v.as_str())?,
);

arg!(FileId,
	|f, _| FileId(f.u32()?),
	|f, _, v| f.u32(v.0),
);

arg!(Text,
	|f, _| Text::read(f)?,
	|f, _, v| Text::write(f, v)?,
);

arg!(Expr,
	|f, g| Expr::read(f, g)?,
	|f, g, v| Expr::write(f, g, v)?,
);

arg!(CharId,
	|f, g| to_char_id(g, f.u16()?)?,
	|f, g, v| f.u16(from_char_id(g, *v)),
);

mod char_id8 {
	use super::*;
	pub(super) fn read(f: &mut Reader, g: Game) -> Result<CharId, ReadError> {
		to_char_id(g, f.u8()? as u16)
	}
	pub(super) fn write(f: &mut Writer, g: Game, v: &CharId) -> Result<(), WriteError> {
		f.u8(cast(from_char_id(g, *v))?);
		Ok(())
	}
}

mod char_id_text_message {
	use super::*;
	pub(super) fn read(f: &mut Reader, g: Game) -> Result<CharId, ReadError> {
		if g.is_ed7() {
			CharId::read(f, g)
		} else {
			Ok(CharId::Null)
		}
	}

	pub(super) fn write(f: &mut Writer, g: Game, v: &CharId) -> Result<(), WriteError> {
		if g.is_ed7() {
			CharId::write(f, g, v)
		} else {
			ensure!(*v == CharId::Null, "can only be null here");
			Ok(())
		}
	}
}

fn to_char_id(g: Game, v: u16) -> Result<CharId, ReadError> {
	use BaseGame::*;
	let g = g.base();
	Ok(match v {
		257.. => CharId::Name(NameId(v - 257)),
		256   => return Err("char id 256".into()),
		255   => CharId::Null,
		254   => CharId::Self_,
		244.. if g == Ao => CharId::Custom(v-244),
		246.. if g == Sc => CharId::Party(v-246),
		238.. if g != Sc => CharId::Party(v-238),
		16..  if g == Tc => CharId::Local(LocalCharId(v - 16)),
		8..   if g != Tc => CharId::Local(LocalCharId(v - 8)),
		0..   => CharId::FieldParty(v),
	})
}

fn from_char_id(g: Game, v: CharId) -> u16 {
	use BaseGame::*;
	let g = g.base();
	match v {
		CharId::FieldParty(v) => v,
		CharId::Local(v) if g == Tc => 16 + v.0,
		CharId::Local(v)            => 8  + v.0,
		CharId::Party(v) if g == Sc => 246 + v,
		CharId::Party(v)            => 238 + v,
		CharId::Custom(v) => 244 + v,
		CharId::Null      => 255,
		CharId::Self_     => 254,
		CharId::Name(v)   => 257 + v.0,
	}
}

mod color24 {
	use super::*;
	pub(super) fn read(f: &mut Reader, _: Game) -> Result<Color, ReadError> {
		let r = f.u8()?;
		let g = f.u8()?;
		let b = f.u8()?;
		Ok(Color(u32::from_le_bytes([r, g, b, 0xFF])))
	}

	pub(super) fn write(f: &mut Writer, _: Game, v: &Color) -> Result<(), WriteError> {
		let [r, g, b, a] = u32::to_le_bytes(v.0);
		ensure!(a == 0xFF, "alpha must be opaque");
		f.u8(r);
		f.u8(g);
		f.u8(b);
		Ok(())
	}
}

mod quest_list {
	use super::*;
	pub(super) fn read(f: &mut Reader, _: Game) -> Result<Vec<QuestId>, ReadError> {
		let mut quests = Vec::new();
		loop {
			match f.u16()? {
				0xFFFF => break,
				q => quests.push(QuestId(q))
			}
		}
		Ok(quests)
	}

	pub(super) fn write(f: &mut Writer, _: Game, v: &Vec<QuestId>) -> Result<(), WriteError> {
		for &i in v {
			f.u16(i.0);
		}
		f.u16(0xFFFF);
		Ok(())
	}
}

mod fork {
	use super::*;
	pub(super) fn read(f: &mut Reader, game: Game) -> Result<Code, ReadError> {
		let len = f.u8()? as usize;
		let pos = f.pos();
		let code = Code::read(f, game, Some(pos+len))?;
		if len > 0 {
			f.check_u8(0)?;
		}
		Ok(code)
	}

	pub(super) fn write(f: &mut Writer, game: Game, v: &Code) -> Result<(), WriteError> {
		let l1 = GLabel::new();
		let l2 = GLabel::new();
		f.delay(move |l| Ok(u8::to_le_bytes(cast(l.label(l2)? - l.label(l1)?)?)));
		f.label(l1);
		Code::write(f, game, v)?;
		f.label(l2);
		if !v.is_empty() {
			f.u8(0);
		}
		Ok(())
	}
}

mod fork_loop {
	use super::*;
	pub(super) fn read(f: &mut Reader, game: Game) -> Result<Code, ReadError> {
		let len = f.u8()? as usize;
		let pos = f.pos();
		let code = Code::read(f, game, Some(pos+len))?;
		let next = if game.is_ed7() {
			Insn::NextFrame2()
		} else {
			Insn::NextFrame()
		};
		ensure!(read_raw_insn(f, game)? == RawIInsn::Insn(next), "invalid loop");
		ensure!(read_raw_insn(f, game)? == RawIInsn::Goto(pos), "invalid loop");
		Ok(code)
	}

	pub(super) fn write(f: &mut Writer, game: Game, v: &Code) -> Result<(), WriteError> {
		let l1 = GLabel::new();
		let l2 = GLabel::new();
		f.delay(move |l| Ok(u8::to_le_bytes(cast(l.label(l2)? - l.label(l1)?)?)));
		f.label(l1);
		Code::write(f, game, v)?;
		f.label(l2);
		let next = if game.is_ed7() {
			Insn::NextFrame2()
		} else {
			Insn::NextFrame()
		};
		write_raw_insn(f, game, RawOInsn::Insn(&next))?;
		write_raw_insn(f, game, RawOInsn::Goto(l1))?;
		Ok(())
	}
}

mod menu {
	use super::*;
	pub(super) fn read(f: &mut Reader, _: Game) -> Result<Vec<TString>, ReadError> {
		Ok(f.string()?.split_terminator('\x01').map(|a| TString(a.to_owned())).collect())
	}

	pub(super) fn write(f: &mut Writer, _: Game, v: &[TString]) -> Result<(), WriteError> {
		let mut s = String::new();
		for line in v {
			s.push_str(line.as_str());
			s.push('\x01');
		}
		f.string(&s)?;
		Ok(())
	}
}

pub(super) mod char_attr {
	use super::*;
	pub fn read(f: &mut Reader, g: Game) -> Result<CharAttr, ReadError> {
		let a = CharId::read(f, g)?;
		let b = f.u8()?;
		Ok(CharAttr(a, b))
	}

	pub fn write(f: &mut Writer, g: Game, CharAttr(a, b): &CharAttr) -> Result<(), WriteError> {
		CharId::write(f, g, a)?;
		f.u8(*b);
		Ok(())
	}
}

mod func_id {
	use super::*;
	pub(super) fn read(f: &mut Reader, game: Game) -> Result<FuncId, ReadError> {
		let a = f.u8()? as u16;
		let b = if game.is_ed7() {
			f.u8()? as u16
		} else {
			f.u16()?
		};
		Ok(FuncId(a, b))
	}

	pub(super) fn write(f: &mut Writer, game: Game, &FuncId(a, b): &FuncId) -> Result<(), WriteError> {
		f.u8(cast(a)?);
		if game.is_ed7() {
			f.u8(cast(b)?)
		} else {
			f.u16(b)
		};
		Ok(())
	}
}

mod func_id_u8_u16 {
	use super::*;
	pub(super) fn read(f: &mut Reader, _: Game) -> Result<FuncId, ReadError> {
		let a = f.u8()? as u16;
		let b = f.u16()?;
		Ok(FuncId(a, b))
	}

	pub(super) fn write(f: &mut Writer, _: Game, &FuncId(a, b): &FuncId) -> Result<(), WriteError> {
		f.u8(cast(a)?);
		f.u16(b);
		Ok(())
	}
}

mod sc_party_select_mandatory {
	use super::*;
	pub(super) fn read(f: &mut Reader, _: Game) -> Result<[Option<NameId>; 4], ReadError> {
		array(|| match f.u16()? {
			0xFF => Ok(None),
			n => Ok(Some(NameId(n))),
		})
	}

	pub(super) fn write(f: &mut Writer, _: Game, v: &[Option<NameId>; 4]) -> Result<(), WriteError> {
		for a in v {
			f.u16(match a {
				Some(v) => v.0,
				None => 0xFF,
			})
		}
		Ok(())
	}
}

mod sc_party_select_optional {
	use super::*;
	pub(super) fn read(f: &mut Reader, _: Game) -> Result<Vec<NameId>, ReadError> {
		let mut quests = Vec::new();
		loop {
			match f.u16()? {
				0xFFFF => break,
				q => quests.push(NameId(cast(q)?))
			}
		}
		Ok(quests)
	}

	pub(super) fn write(f: &mut Writer, _: Game, v: &Vec<NameId>) -> Result<(), WriteError> {
		for &i in v {
			f.u16(i.0);
		}
		f.u16(0xFFFF);
		Ok(())
	}
}

mod char_animation {
	use super::*;
	pub(super) fn read(f: &mut Reader, _: Game) -> Result<Vec<u8>, ReadError> {
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

	pub(super) fn write(f: &mut Writer, _: Game, v: &Vec<u8>) -> Result<(), WriteError> {
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
