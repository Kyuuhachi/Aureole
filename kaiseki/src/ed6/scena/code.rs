use super::*;

#[kaiseki_macros::bytecode(
	#[derive(Debug, Clone, PartialEq, Eq)]
	pub enum Insn {}

	#[allow(non_camel_case_types)]
	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
	pub enum InsnArg<'a> {}

	pub fn parts(&self) -> (&'static str, Box<[InsnArg]>) {}
)]
pub(super) fn read(i: &mut CodeParser) -> Result<Self> {
	match u8 {
		0x01 => Return(),
		0x05 => Call(FuncRef),
		0x06 => NewScene(scena_file/FileRef, u8, u8, u8, u8),
		0x08 => Sleep(time/u32),
		0x09 => FlagsSet(flags/u32),
		0x0A => FlagsUnset(flags/u32),
		0x0B => FadeOn(time/u32, color/u32, u8),
		0x0C => FadeOff(time/u32, color/u32),
		0x0D => _0D(),
		0x0E => Blur(time/u32),
		0x0F => Battle(battle/u16, u16, u16, u16, u8, u16, i8),
		0x12 => _12(i32, i32, u32),
		0x13 => PlaceSetName(town/u16),
		0x16 => Map(match u8 {
			0x00 => Hide(),
			0x01 => Show(),
			0x02 => Set(i32, Pos2, map_file/FileRef),
		}),
		0x17 => Save(),
		0x19 => EventBegin(u8),
		0x1A => EventEnd(u8),
		0x1B => _1B(u16, u16),
		0x1C => _1C(u16, u16),
		0x1D => BgmSet(bgmtbl/u8),
		0x1E => _1E(),
		0x1F => BgmSetVolume(u8, time/u32),
		0x20 => _20(time/u32),
		0x21 => _21(), // Always paired with _20
		0x22 => SoundPlay(sound/u16, u8, u8),
		0x23 => SoundStop(sound/u16),
		0x24 => SoundLoop(sound/u16, u8),
		0x26 => _Sound26(sound/u16),
		0x28 => Quest(quest/u16, match u8 {
			0x01 => TaskSet(quest_task/u16),
			0x02 => TaskUnset(quest_task/u16),
			0x03 => FlagsSet(quest_flag/u8),
			0x04 => FlagsUnset(quest_flag/u8),
		}),
		0x29 => Quest(quest/u16, match u8 {
			0x00 => FlagsGet(quest_flag/u8),
			0x01 => TaskGet(quest_task/u16),
		}),
		0x2A => QuestList(quests/{
			let mut quests = Vec::new();
			loop {
				match i.u16()? {
					0xFFFF => break,
					q => quests.push(q)
				}
			}
			quests
		} as Vec<u16>),
		0x2B => QuestBonusBp(quest/u16, u16),
		0x2C => QuestBonusMira(quest/u16, u16),
		0x2D => PartyAdd(member/u8, char/{i.u8()? as u16} as u16),
		0x2E => PartyRemove(member/u8, char/{i.u8()? as u16} as u16),
		0x30 => _Party30(u8),
		0x31 => PartySetAttr(member/u8, member_attr/u8, u16),
		0x34 => PartyAddArt(member/u8, magic/u16),
		0x35 => PartyAddCraft(member/u8, magic/u16),
		0x36 => PartyAddSCraft(member/u8, magic/u16),
		0x37 => PartySet(member/u8, u8, u8),
		0x38 => SepithAdd(sepith_element/u8, u16),
		0x39 => SepithRemove(sepith_element/u8, u16),
		0x3A => MiraAdd(u16),
		0x3B => MiraSub(u16),
		0x3C => BpAdd(u16),
		// I have a guess what 3D is, but it doesn't exist in any scripts
		0x3E => ItemAdd(item/u16, u16),
		0x3F => ItemRemove(item/u16, u16),
		0x40 => ItemHas(item/u16), // or is it ItemGetCount?
		0x41 => PartyEquip(member/u8, item/u16, {
			if (600..800).contains(&_1) {
				i.i8()?
			} else {
				-1
			}
		} as i8),
		0x43 => CharForkFunc(char/u16, fork_id/u8, FuncRef),
		0x44 => CharForkQuit(char/u16, fork_id/u8),
		0x45 => CharFork(char/u16, fork_id/u8, u8, fork/{
			let len = i.u8()? as usize;
			let pos = i.pos();
			let mut insns = Vec::new();
			while i.pos() < pos+len {
				i.marks.insert(i.pos(), "\x1B[0;7;2m•".to_owned());
				insns.push(i.insn()?);
			}
			eyre::ensure!(i.pos() == pos+len, "Overshot: {:X} > {:X}", i.pos(), pos+len);
			i.check_u8(0)?;
			insns
		} as Vec<Insn>),
		0x46 => CharForkLoop(char/u16, fork_id/u8, u8, fork/{
			let len = i.u8()? as usize;
			let pos = i.pos();
			let mut insns = Vec::new();
			while i.pos() < pos+len {
				i.marks.insert(i.pos(), "\x1B[0;7;2m•".to_owned());
				insns.push((i.insn())?);
			}
			eyre::ensure!(i.pos() == pos+len, "Overshot: {:X} > {:X}", i.pos(), pos+len);
			eyre::ensure!(i.flow_insn()? == FlowInsn::Insn(Insn::Yield()), "Invalid loop");
			eyre::ensure!(i.flow_insn()? == FlowInsn::Goto(pos), "Invalid loop");
			insns
		} as Vec<Insn>),
		0x47 => CharForkAwait(char/u16, fork_id/u8, u8),
		0x48 => Yield(), // Used in tight loops, probably wait until next frame
		0x49 => Event(FuncRef), // Not sure how this differs from Call
		0x4A => _Char4A(char/u16, u8),
		0x4B => _Char4B(char/u16, u8),
		0x4D => Var(var/u16, Expr),
		0x4F => Attr(attr/u8, Expr),
		0x51 => CharAttr(char/u16, char_attr/u8, Expr),
		0x52 => TextStart(char/u16),
		0x53 => TextEnd(char/u16),
		0x54 => TextMessage(Text),
		0x56 => TextReset(u8),
		0x58 => TextWait(),
		0x59 => _59(),
		0x5A => TextSetPos(i16, i16, i16, i16),
		0x5B => TextTalk(char/u16, Text),
		0x5C => TextTalkNamed(char/u16, text_title/String, Text),
		0x5D => Menu(menu_id/u16, i16, i16, u8, menu/{i.string()?.split_terminator('\x01').map(|a| a.to_owned()).collect()} as Vec<String>),
		0x5E => MenuWait(var/u16),
		0x5F => MenuClose(menu_id/u16),
		0x60 => TextSetName(text_title/String),
		0x61 => _61(char/u16),
		0x62 => Emote(char/u16, i32, time/u32, emote/{(i.u8()?, i.u8()?, i.u32()?, i.u8()?)} as (u8, u8, u32, u8)),
		0x63 => EmoteStop(char/u16),
		0x64 => _64(u8, u16),
		0x65 => _65(u8, u16),
		0x66 => _Cam66(u16),
		0x6E => _Cam6E(data/{i.array()?} as [u8; 4], time/u32),
		0x67 => CamOffset(i32, i32, i32, time/u32),
		0x69 => CamLookAt(char/u16, time/u32),
		0x6A => _Char6A(char/u16),
		0x6B => CamDistance(i32, time/u32),
		0x6C => CamAngle(angle32/i32, time/u32),
		0x6D => CamPos(Pos3, time/u32),
		0x6F => _Obj6F(object/u16, u32),
		0x70 => _Obj70(object/u16, u32),
		0x71 => _Obj71(object/u16, u16),
		0x72 => _Obj72(object/u16, u16),
		0x73 => _Obj73(object/u16),
		0x77 => _77(color/u32, time/u32),
		0x7C => Shake(u32, u32, u32, time/u32),
		0x7F => EffLoad(u8, eff_file/String),
		0x80 => EffPlay(u8, u8, i16, Pos3, u16, u16, u16, u32, u32, u32, u16, u32, u32, u32, u32),
		0x81 => EffPlay2(u16, u8, eff_file/String, Pos3, u16, u16, u16, u32, u32, u32, u32),
		0x82 => _82(u16),
		0x83 => Achievement(u8, u8),
		0x86 => CharSetChcp(char/u16, chcp/u16),
		0x87 => CharSetFrame(char/u16, u16),
		0x88 => CharSetPos(char/u16, Pos3, angle/u16),
		0x89 => _Char89(char/u16, Pos3, u16),
		0x8A => CharLookAt(char/u16, char/u16, time16/u16),
		0x8C => CharSetAngle(char/u16, angle/u16, time16/u16),
		0x8D => CharIdle(char/u16, Pos2, Pos2, speed/u32),
		0x8E => CharWalkTo(char/u16, Pos3, speed/u32, u8),
		0x8F => CharWalkTo2(char/u16, Pos3, speed/u32, u8), // how are these two different?
		0x90 => DontGoThere(u16, relative/Pos3, u32, u8),
		0x91 => _Char91(char/u16, relative/Pos3, i32, u8),
		0x92 => _Char92(char/u16, char/u16, u32, time/u32, u8),
		0x94 => _94(u8, char/u16, u16, u32, u32, u8), // used with chickens
		0x95 => CharJump(char/u16, relative/Pos3, time/u32, u32),
		0x97 => _Char97(char/u16, Pos2, angle32/i32, time/u32, u16), // used with pigeons
		0x99 => CharAnimation(char/u16, u8, u8, time/u32),
		0x9A => CharFlagsSet(char/u16, char_flags/u16),
		0x9B => CharFlagsUnset(char/u16, char_flags/u16),
		0x9C => _Char9C(char/u16, u16),
		0x9D => _Char9D(char/u16, u16),
		0x9E => CharShake(char/u16, u32, u32, u32, time/u32),
		0x9F => CharColor(char/u16, color/u32, time/u32),
		0xA1 => _CharA1(char/u16, u16),
		0xA2 => FlagSet(flag/u16),
		0xA3 => FlagUnset(flag/u16),
		0xA5 => FlagAwaitUnset(flag/u16),
		0xA6 => FlagAwaitSet(flag/u16),
		0xA9 => ShopOpen(shop/u8),
		0xAC => RecipeLearn(u16), // TODO check type
		0xAD => ImageShow(vis_file/FileRef, u16, u16, time/u32),
		0xAE => ImageHide(time/u32),
		0xAF => QuestSubmit(shop/u8, quest/u16),
		0xB1 => OpLoad(op_file/String),
		0xB2 => _B2(u8, u8, u16),
		0xB3 => Video(match u8 {
			0x00 => Show0(avi_file/String),
			0x01 => Show1(u8),
		}),
		0xB4 => ReturnToTitle(u8),
		0xB5 => PartySlot(member/u8, u8, u8), // FC only
		0xB9 => ReadBook(item/u16, u16), // FC only
		0xBA => PartyHasSpell(member/u8, magic/u16),
		0xBB => PartyHasSlot(member/u8, u8), // FC only
		0xDE => SaveClearData(), // FC only
	}
}
