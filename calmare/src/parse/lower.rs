use std::collections::BTreeMap;

use themelios::scena::*;
use themelios::text::{Text, TextSegment};
use themelios::types::*;
use themelios::lookup::Lookup;

use super::diag::*;
use super::lex::{Line, Token, TextToken};
// use crate::ast::*;
use crate::span::{Spanned as S, Span};

pub mod scena;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
	Scena,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Unit {
	None,
	Mm,
	MmPerS,
	Ms,
	Deg,
	DegPerS,
	MDeg,
}

#[derive(Clone, Copy)]
pub struct Context<'a> {
	pub game: Game,
	pub ty: FileType,
	pub lookup: &'a dyn Lookup,
}

impl<'a> std::fmt::Debug for Context<'a> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Context")
			.field("game", &self.game)
			.field("ty", &self.ty)
			.field("lookup", &format_args!("_"))
			.finish()
	}
}

#[derive(Debug, Clone)]
pub struct Error;
type Result<T, E=Error> = std::result::Result<T, E>;

macro test {
	($q:expr, $p:pat $(if $e:expr)? => $v:expr) => {
		$q.next_if(|a| {
			match a {
				$p $(if $e)? => Some($v),
				_ => None
			}
		})
	},
	($q:expr, $p:pat $(if $e:expr)? ) => {
		test!($q, $p $(if $e)? => ()).is_some()
	},
}

#[derive(Clone, Debug)]
pub struct Parse<'a> {
	tokens: &'a [S<Token<'a>>],
	pos: usize,
	body: Option<&'a [Line<'a>]>,
	context: &'a Context<'a>,
	eol: Span,
}

impl<'a> Parse<'a> {
	fn new(line: &'a Line, context: &'a Context) -> Self {
		Parse {
			tokens: &line.head,
			pos: 0,
			body: line.body.as_deref(),
			context,
			eol: line.eol,
		}
	}

	fn new_inner(tokens: &'a [S<Token<'a>>], eol: Span, context: &'a Context) -> Self {
		Parse {
			tokens,
			pos: 0,
			body: None,
			context,
			eol,
		}
	}

	fn parse_comma<T: ValComma>(mut self) -> Result<T> {
		let v = T::parse_comma(&mut self)?;
		self.finish();
		Ok(v)
	}

	fn parse_with<T>(mut self, f: impl FnOnce(&mut Parse) -> T) -> T {
		let v = f(&mut self);
		self.finish();
		v
	}

	fn finish(&self) {
		if self.pos < self.tokens.len() {
			Diag::error(self.tokens[self.pos].0, "expected end of data").emit();
		}

		if self.body.is_some() {
			Diag::error(self.eol, "body not expected here").emit();
		}
	}

	fn body(&mut self) -> Result<&'a [Line<'a>]> {
		match self.body.take() {
			Some(a) => Ok(a),
			None => {
				Diag::error(self.eol, "expected a body").emit();
				Err(Error)
			},
		}
	}

	fn space(&self) -> Option<Span> {
		if self.pos == 0 { return None }
		if self.pos == self.tokens.len() { return None }
		let s0 = self.tokens[self.pos-1].0;
		let s1 = self.tokens[self.pos].0;
		if s0.connects(s1) {
			None
		} else {
			Some(s0.at_end() | s1.at_start())
		}
	}

	fn head_span(&self) -> Span {
		self.tokens.get(0).map_or(self.eol, |a| a.0 | self.eol)
	}

	fn prev_span(&self) -> Span {
		if self.pos == 0 {
			self.next_span().at_start()
		} else {
			self.tokens[self.pos-1].0
		}
	}

	fn next_span(&self) -> Span {
		self.tokens.get(self.pos).map_or(self.eol, |a| a.0)
	}

	fn next_if<T>(&mut self, f: impl FnOnce(&'a Token<'a>) -> Option<T>) -> Option<T> {
		if let Some(S(_, t)) = self.tokens.get(self.pos) && let Some(x) = f(t) {
			self.pos += 1;
			Some(x)
		} else {
			None
		}
	}

	fn word(&mut self, name: &str) -> bool {
		test!(self, Token::Ident(a) if *a == name)
	}

	fn tuple<T: ValComma>(&mut self) -> Result<Option<T>> {
		if let Some(d) = test!(self, Token::Paren(d) => d) {
			Parse::new_inner(&d.tokens, d.close, self.context).parse_comma().map(Some)
		} else {
			Ok(None)
		}
	}

	fn term<T: ValComma>(&mut self, name: &str) -> Result<Option<T>> {
		if self.word(name) {
			let space = self.space();
			if let Some(d) = test!(self, Token::Bracket(d) => d) {
				if let Some(s) = space {
					Diag::error(s, "no space allowed here").emit()
				}
				if d.tokens.is_empty() {
					Diag::error(d.open | d.close, "this cannot be empty")
						.note(d.open | d.close, "maybe remove the brackets?")
						.emit()
				}
				Parse::new_inner(&d.tokens, d.close, self.context).parse_comma().map(Some)
			} else {
				Parse::new_inner(&[], self.prev_span().at_end(), self.context).parse_comma().map(Some)
			}
		} else {
			Ok(None)
		}
	}

	fn remaining(&self) -> &'a [S<Token<'a>>] {
		&self.tokens[self.pos..]
	}
}

trait Val: Sized {
	fn parse(p: &mut Parse) -> Result<Self>;
}

trait TryVal: Sized {
	fn desc() -> String;

	fn try_parse(p: &mut Parse) -> Result<Option<Self>>;
}

impl<T: TryVal> Val for T {
	fn parse(p: &mut Parse) -> Result<Self> {
		if let Some(v) = Self::try_parse(p)? {
			Ok(v)
		} else {
			Diag::error(p.next_span(), format_args!("expected {}", Self::desc())).emit();
			Err(Error)
		}
	}
}

trait ValComma: Sized {
	fn parse_comma(p: &mut Parse) -> Result<Self>;
}

impl<T: TryVal> TryVal for S<T> {
	fn desc() -> String { T::desc() }

	fn try_parse(p: &mut Parse) -> Result<Option<Self>> {
		let p1 = p.pos;
		let v = T::try_parse(p)?;
		let p2 = p.pos;
		let s = if p1 == p2 {
			p.next_span().at_start()
		} else {
			p.tokens[p1].0 | p.tokens[p2-1].0
		};
		Ok(v.map(|a| S(s, a)))
	}
}

macro tuple($($T:ident)*) {
	#[allow(non_snake_case)]
	impl<$($T: Val),*> Val for ($($T,)*) {
		fn parse(_p: &mut Parse) -> Result<Self> {
			Ok(($($T::parse(_p)?,)*))
		}
	}

	#[allow(non_snake_case)]
	impl<$($T: Val),*> ValComma for ($($T,)*) {
		fn parse_comma(_p: &mut Parse) -> Result<Self> {
			$(
				let $T = $T::parse(_p)?;
				if _p.pos < _p.tokens.len() && !test!(_p, Token::Comma) {
					Diag::error(_p.next_span().at_start(), "expected comma").emit();
				}
			)*
			Ok(($($T,)*))
		}
	}
}

tuple!();
tuple!(A);
tuple!(A B);
tuple!(A B C);
tuple!(A B C D);

impl<T: TryVal> Val for Vec<T> {
	fn parse(p: &mut Parse) -> Result<Self> {
		let mut v = Vec::new();
		while let Some(a) = T::try_parse(p)? {
			v.push(a);
		}
		Ok(v)
	}
}

impl<const N: usize, T: Val> Val for [T; N] {
	fn parse(p: &mut Parse) -> Result<Self> {
		[(); N].try_map(|()| T::parse(p))
	}
}

impl<T: TryVal> TryVal for Option<T> {
	fn desc() -> String { format!("{}, 'null'", T::desc()) }

	fn try_parse(p: &mut Parse) -> Result<Option<Self>> {
		if let Some(()) = p.term("null")? {
			Ok(None)
		} else {
			T::try_parse(p).map(Some)
		}
	}
}

fn parse_unit(p: &mut Parse) -> Result<S<Unit>> {
	let s0 = p.prev_span().at_end();
	let (s, u1, u2) = match p.remaining() {
		[S(s1, Token::Ident(u1)), S(s2, Token::Slash), S(s3, Token::Ident(u2)), ..]
		if s0.connects(*s1) && s1.connects(*s2) && s2.connects(*s3) => {
			let v = (*s1 | *s3, *u1, Some(*u2)); 
			p.pos += 3;
			v
		}
		[S(s1, Token::Ident(u1)), ..]
		if s0.connects(*s1) => {
			let v = (*s1, *u1, None);
			p.pos += 1;
			v
		}
		_ => return Ok(S(s0, Unit::None))
	};
	let u = match (u1, u2) {
		("mm", None) => Unit::Mm,
		("mm", Some("s")) => Unit::MmPerS,
		("ms", None) => Unit::Ms,
		("deg", None) => Unit::Deg,
		("deg", Some("s")) => Unit::DegPerS,
		("mdeg", None) => Unit::MDeg,
		_ => {
			Diag::error(s, "invalid unit").emit();
			return Err(Error)
		}
	};
	Ok(S(s, u))
}

fn parse_int(p: &mut Parse) -> Result<Option<(S<i64>, S<Unit>)>> {
	match p.remaining() {
		[S(s, Token::Int(n)), ..] => {
			p.pos += 1;
			Ok(Some((S(*s, *n as i64), parse_unit(p)?)))
		}
		[S(s1, Token::Minus), S(s, Token::Int(n)), ..] if s1.connects(*s) => {
			p.pos += 2;
			Ok(Some((S(*s1 | *s, -(*n as i64)), parse_unit(p)?)))
		}
		_ => {
			Ok(None)
		}
	}
}

fn parse_float(p: &mut Parse) -> Result<Option<(S<f64>, S<Unit>)>> {
	match p.remaining() {
		[S(s, Token::Float(n)), ..] => {
			p.pos += 1;
			Ok(Some((S(*s, n.0), parse_unit(p)?)))
		}
		[S(s1, Token::Minus), S(s, Token::Float(n)), ..] if s1.connects(*s) => {
			p.pos += 2;
			Ok(Some((S(*s1 | *s, -n.0), parse_unit(p)?)))
		}
		_ => {
			Ok(None)
		}
	}
}

macro int($T:ident $(=> $(#$CONV:ident)?)?) {
	impl TryVal for $T {
		fn desc() -> String { "int".to_owned() }

		fn try_parse(p: &mut Parse) -> Result<Option<Self>> {
			if let Some((s, u)) = parse_int(p)? {
				if u.1 != Unit::None {
					Diag::warn(u.0, "this should be unitless").emit();
				}
				let v = s.1.try_into().map_err(|e| {
					Diag::error(s.0, e).emit();
					Error
				})?;
				let v = unless!($({$($CONV)? $T(v)})?, {v});
				Ok(Some(v))
			} else {
				Ok(None)
			}
		}
	}
}

int!(u8);
int!(u16);
int!(u32);
int!(u64);
int!(i8);
int!(i16);
int!(i32);
int!(i64);

int!(SystemFlags =>);
int!(CharFlags =>);
int!(QuestFlags =>);
int!(ObjectFlags =>);
int!(LookPointFlags =>);
int!(TriggerFlags =>);
int!(EntryFlags =>);

int!(Color =>);
int!(TcMembers =>);

int!(QuestTask =>);

impl TryVal for String {
	fn desc() -> String { "string".to_owned() }

	fn try_parse(p: &mut Parse) -> Result<Option<Self>> {
		if let Some(s) = test!(p, Token::String(s) => s) {
			Ok(Some(s.to_owned()))
		} else {
			Ok(None)
		}
	}
}

impl TryVal for TString {
	fn desc() -> String { "string".to_owned() }

	fn try_parse(p: &mut Parse) -> Result<Option<Self>> {
		if let Some(s) = test!(p, Token::String(s) => s) {
			Ok(Some(TString(s.to_owned())))
		} else {
			Ok(None)
		}
	}
}

impl TryVal for Text {
	fn desc() -> String { "dialogue text".to_owned() }

	fn try_parse(p: &mut Parse) -> Result<Option<Self>> {
		if let Some(d) = test!(p, Token::Brace(s) => s) {
			let mut out = Vec::new();
			parse_text_chunk(&mut out, p.context, &d.tokens);
			while let Some(d) = test!(p, Token::Brace(s) => s) {
				out.push(TextSegment::Page);
				parse_text_chunk(&mut out, p.context, &d.tokens);
			}

			Ok(Some(Text(out)))
		} else {
			Ok(None)
		}
	}
}

fn parse_text_chunk(out: &mut Vec<TextSegment>, ctx: &Context, d: &[S<TextToken>]) {
	for S(_, t) in d {
		match t {
			TextToken::Text(s) => {
				out.push(TextSegment::String(s.to_owned()))
			}
			TextToken::Newline(b) => {
				out.push(if *b { TextSegment::Line2 } else { TextSegment::Line } )
			}
			TextToken::Brace(d) => {
				Parse::new_inner(&d.tokens, d.close, ctx).parse_with(|p| {
					if p.pos == p.tokens.len() {
						return
					}

					if let Some(n) = test!(p, Token::Int(a) => *a) {
						match n.try_into() {
							Ok(n) => out.push(TextSegment::Byte(n)),
							Err(e) => Diag::error(p.prev_span(), e).emit()
						}
						return
					}

					let Some(key) = test!(p, Token::Ident(a) => a) else {
						Diag::error(p.next_span(), "expected word").emit();
						p.pos = p.tokens.len();
						return;
					};
					if test!(p, Token::Bracket(_)) {
						p.pos -= 2;
					}

					match *key {
						"wait" => {
							out.push(TextSegment::Wait)
						}
						"color" => {
							if let Some(c) = test!(p, Token::Int(i) if *i <= 255 => *i) {
								out.push(TextSegment::Color(c as u8))
							} else {
								Diag::error(p.next_span(), "expected u8").emit();
								p.pos = p.tokens.len();
							}
						}
						"item" => {
							if let Ok(c) = ItemId::parse(p) {
								out.push(TextSegment::Item(c))
							}
						}
						_ => {
							Diag::error(p.prev_span(), "expected u8, 'wait', 'color', 'item'").emit();
						}
					}
				})
			}
		}
	}
}

macro unit($T:ident, $unit:ident, $unit_str:literal) {
	impl TryVal for $T {
		fn desc() -> String { format!("'{}' number", $unit_str) }

		fn try_parse(p: &mut Parse) -> Result<Option<Self>> {
			if let Some((s, u)) = parse_int(p)? {
				if u.1 != Unit::$unit {
					Diag::warn(u.0, format_args!("unit should be '{}'", $unit_str)).emit();
				}
				let v = s.1.try_into().map_err(|e| {
					Diag::error(s.0, e).emit();
					Error
				})?;
				Ok(Some(Self(v)))
			} else {
				Ok(None)
			}
		}
	}
}

unit!(Angle, Deg, "deg");
unit!(AngularSpeed, DegPerS, "deg/s");
unit!(Angle32, MDeg, "mdeg");
unit!(Time, Ms, "ms");
unit!(Length, Mm, "mm");
unit!(Speed, MmPerS, "mm/s");

impl TryVal for f32 {
	fn desc() -> String { "float".to_owned() }

	fn try_parse(p: &mut Parse) -> Result<Option<Self>> {
		if let Some((s, u)) = parse_float(p)? {
			if u.1 != Unit::None {
				Diag::warn(u.0, "this should be unitless").emit();
			}
			Ok(Some(s.1 as f32))
		} else if let Some((s, u)) = parse_int(p)? {
			if u.1 != Unit::None {
				Diag::warn(u.0, "this should be unitless").emit();
			}
			Ok(Some(s.1 as f32))
		} else {
			Ok(None)
		}
	}
}

impl TryVal for Pos3 {
	fn desc() -> String { "pos3".to_owned() }

	fn try_parse(p: &mut Parse) -> Result<Option<Self>> {
		if let Some((x, y, z)) = p.tuple()? {
			Ok(Some(Pos3(x, y, z)))
		} else {
			Ok(None)
		}
	}
}

struct Null;
impl TryVal for Null {
	fn desc() -> String { "'null'".to_owned() }

	fn try_parse(p: &mut Parse) -> Result<Option<Self>> {
		if let Some(()) = p.term("null")? {
			Ok(Some(Null))
		} else {
			Ok(None)
		}
	}
}

impl TryVal for Pos2 {
	fn desc() -> String { "pos2".to_owned() }

	fn try_parse(p: &mut Parse) -> Result<Option<Self>> {
		if let Some((x, Null, z)) = p.tuple()? {
			Ok(Some(Pos2(x, z)))
		} else {
			Ok(None)
		}
	}
}

impl TryVal for FuncId {
	fn desc() -> String { "'fn'".to_owned() }

	fn try_parse(p: &mut Parse) -> Result<Option<Self>> {
		if let Some((a, b)) = p.term("fn")? {
			Ok(Some(FuncId(a, b)))
		} else {
			Ok(None)
		}
	}
}

impl TryVal for CharAttr {
	fn desc() -> String { "'char_attr'".to_owned() }

	fn try_parse(p: &mut Parse) -> Result<Option<Self>> {
		if let Some((a, b)) = p.term("char_attr")? {
			Ok(Some(CharAttr(a, b)))
		} else {
			Ok(None)
		}
	}
}

impl TryVal for FileId {
	fn desc() -> String { "string, 'null', 'file'".to_owned() }

	fn try_parse(p: &mut Parse) -> Result<Option<Self>> {
		if let Some(s) = test!(p, Token::String(s) => s) {
			Ok(Some(FileId(p.context.lookup.index(s).unwrap_or_else(|| {
				Diag::error(p.prev_span(), "could not resolve file id").emit();
				0x00000000
			}))))
		} else if let Some(()) = p.term("null")? {
			Ok(Some(FileId(0)))
		} else if let Some((s,)) = p.term("file")? {
			Ok(Some(FileId(s)))
		} else {
			Ok(None)
		}
	}
}

impl TryVal for Emote {
	fn desc() -> String { "'emote'".to_owned() }

	fn try_parse(p: &mut Parse) -> Result<Option<Self>> {
		if let Some((a, b, c, d)) = p.term("emote")? {
			Ok(Some(Emote(a, b, c, d)))
		} else {
			Ok(None)
		}
	}
}

impl TryVal for CharId {
	fn desc() -> String { "'self', 'null', 'name', 'char', 'field_party', 'party', 'custom'".to_owned() }

	fn try_parse(p: &mut Parse) -> Result<Option<Self>> {
		let span = p.next_span();
		if let Some(()) = p.term("self")? {
			Ok(Some(CharId(254)))
		} else if let Some(()) = p.term("null")? {
			Ok(Some(CharId(255)))
		} else if let Some((s,)) = p.term::<(u16,)>("name")? {
			Ok(Some(CharId(s + 257)))
		} else if let Some((s,)) = p.term::<(u16,)>("char")? {
			Ok(Some(CharId(s + if p.context.game.base() == BaseGame::Tc { 16 } else { 8 })))
		} else if let Some((s,)) = p.term::<(u16,)>("field_party")? {
			Ok(Some(CharId(s)))
		} else if let Some((s,)) = p.term::<(u16,)>("party")? {
			Ok(Some(CharId(s + if p.context.game.base() == BaseGame::Sc { 246 } else { 238 })))
		} else if let Some((s,)) = p.term::<(u16,)>("custom")? {
			if p.context.game.base() != BaseGame::Ao {
				Diag::error(span, "'custom' is only supported on Azure").emit();
			}
			Ok(Some(CharId(s + 244)))
		} else {
			Ok(None)
		}
	}
}

macro newtype($T:ident, $s:literal) {
	impl TryVal for $T {
		fn desc() -> String { format!("'{}'", $s) }

		fn try_parse(p: &mut Parse) -> Result<Option<Self>> {
			if let Some((v,)) = p.term($s)? {
				Ok(Some(Self(v)))
			} else {
				Ok(None)
			}
		}
	}
}

newtype!(Flag, "flag");
newtype!(Attr, "system");
newtype!(Var, "var");
newtype!(Global, "global");

newtype!(NameId,   "name");
newtype!(BgmId,    "bgm");
newtype!(MagicId,  "magic");
newtype!(QuestId,  "quest");
newtype!(ShopId,   "shop");
newtype!(SoundId,  "sound");
newtype!(TownId,   "town");
newtype!(BattleId, "battle");
newtype!(ItemId,   "item");

newtype!(LookPointId, "look_point");
newtype!(EntranceId,  "entrance");
newtype!(ObjectId,    "object");
newtype!(TriggerId,   "trigger");
newtype!(LabelId,     "label");
newtype!(AnimId,      "anim");

newtype!(ChipId,  "chip");
newtype!(VisId,   "vis");
newtype!(ForkId,  "fork");
newtype!(EffId,   "eff");
newtype!(EffInstanceId, "eff_instance");
newtype!(MenuId,  "menu");

macro when {
	($t1:tt) => {},
	($t1:tt, $($t:tt)*) => { $($t)* }
}
macro unless {
	(, {$($v:tt)*}) => { $($v)* },
	({$($t:tt)*},{$($v:tt)*}) => { $($t)* },
}

macro parse_data($d:ident => { $($k:ident $(=> $e:expr)?),* $(,)? }) {
	$(unless!($({when!($e);})?, {
		let mut $k = One::default();
	});)*

	let body = $d.body()?;

	for line in body {
		Parse::new(line, $d.context).parse_with(|p| {
			let Some(key) = test!(p, Token::Ident(a) => a) else {
				Diag::error(p.next_span(), "expected word").emit();
				return
			};
			match *key {
				$(stringify!($k) => {
					unless!($({
						let v: Result<()> = $e(p);
						if v.is_err() {
							p.pos = p.tokens.len();
						}
					})?, {
						$k.mark(p.prev_span());
						match Val::parse(p) {
							Ok(v) => $k.set(v),
							Err(_) => p.pos = p.tokens.len()
						}
					});
				})*
				_ => {
					let fields: &[&str] = &[
						$(concat!("'", stringify!($k), "'"),)*
					];
					Diag::error(p.prev_span(), "unknown field")
						.note(p.prev_span(), format_args!("allowed fields are {}", &fields.join(", ")))
						.emit();
					p.pos = p.tokens.len();
				}
			}
		})
	}

	#[allow(unused_mut)]
	let mut failures: Vec<&str> = Vec::new();
	$(unless!($({when!($e);})?, {
		if !$k.is_present() {
			failures.push(concat!("'", stringify!($k), "'"));
		}
	});)*

	if !failures.is_empty() {
		Diag::error($d.head_span(), "missing fields")
			.note($d.head_span(), failures.join(", "))
			.emit();
		Err(Error)?;
		unreachable!()
	}

	$(unless!($({when!($e);})?, {
		let Some($k) = $k.get() else { Err(Error)?; unreachable!() };
	});)*
}

fn parse_type(line: &Line) -> Result<(Game, FileType)> {
	let dummy_ctx = &Context {
		game: Game::Fc,
		ty: FileType::Scena,
		lookup: &themelios::lookup::NullLookup,
	};
	Parse::new(line, dummy_ctx).parse_with(|p| {
		if !p.word("calmare") {
			Diag::error(p.prev_span(), "expected 'calmare'").emit();
			return Err(Error);
		}
		let Some(a) = test!(p, Token::Ident(a) => a) else {
			Diag::error(p.prev_span(), "expected a game").emit();
			return Err(Error);
		};
		let Some(b) = test!(p, Token::Ident(a) => a) else {
			Diag::error(p.prev_span(), "expected a type").emit();
			return Err(Error);
		};
		let game = match *a {
			"fc" => Game::Fc, "fc_e" => Game::FcEvo, "fc_k" => Game::FcKai,
			"sc" => Game::Sc, "sc_e" => Game::ScEvo, "sc_k" => Game::ScKai,
			"tc" => Game::Tc, "tc_e" => Game::TcEvo, "tc_k" => Game::TcKai,
			"zero" => Game::Zero, "zero_e" => Game::ZeroEvo, "zero_k" => Game::ZeroKai,
			"ao" => Game::Ao, "ao_e" => Game::AoEvo, "ao_k" => Game::AoKai,
			_ => {
				Diag::error(p.prev_span(), "unknown game").emit();
				return Err(Error);
			}
		};
		let ty = match *b {
			"scena" => FileType::Scena,
			_ => {
				Diag::error(p.prev_span(), "unknown file type").emit();
				return Err(Error);
			}
		};
		Ok((game, ty))
	})
}

#[derive(Debug, Clone)]
struct One<T>(Option<S<Option<T>>>);

impl<T> Default for One<T> {
	fn default() -> Self {
		Self(None)
	}
}

impl<T> One<T> {
	fn mark(&mut self, s: Span) {
		if let Some(S(prev, _)) = &self.0 {
			Diag::error(s, "duplicate item")
				.note(*prev, "previous here")
				.emit();
		}
		self.0 = Some(S(s, None));
	}

	fn set(&mut self, v: T) {
		if let Some(S(_, q)) = &mut self.0 {
			*q = Some(v)
		} else {
			panic!("not marked")
		}
	}

	fn is_present(&self) -> bool {
		self.0.is_some()
	}

	fn get(self) -> Option<T> {
		match self.0 {
			Some(S(_, Some(v))) => Some(v),
			_ => None,
		}
	}
}

#[derive(Debug, Clone)]
struct Many<K, V>(BTreeMap<K, S<Option<V>>>);

impl<K, V> Default for Many<K, V> {
	fn default() -> Self {
		Self(Default::default())
	}
}

impl<K: Ord, V> Many<K, V> {
	fn mark(&mut self, s: Span, n: K) {
		if let Some(S(prev, _)) = self.0.get(&n) {
			Diag::error(s, "duplicate item")
				.note(*prev, "previous here")
				.emit();
		}
		self.0.insert(n, S(s, None));
	}

	fn insert(&mut self, n: K, v: V) {
		if let Some(S(_, q)) = self.0.get_mut(&n) {
			*q = Some(v)
		} else {
			panic!("not marked")
		}
	}

	fn get(self, f: impl Fn(K) -> usize) -> Vec<V> {
		let mut vs = Vec::with_capacity(self.0.len());
		let mut expect = 0;
		for (k, S(s, v)) in self.0 {
			let k = f(k);
			if k != expect {
				Diag::error(s, "gap in list")
					.note(s, format_args!("missing index {expect}"))
					.emit();
			}
			expect = k + 1;
			vs.extend(v)
		}
		vs
	}
}

pub fn parse(lines: &[Line], lookup: Option<&dyn Lookup>) -> Result<(Game, crate::Content)> {
	if lines.is_empty() {
		Diag::error(Span::new_at(0), "no type declaration").emit();
		return Err(Error);
	}

	let (game, ty) = parse_type(&lines[0])?;
	let ctx = &Context {
		game,
		ty,
		lookup: lookup.unwrap_or_else(|| themelios::lookup::default_for(game)),
	};

	match ty {
		FileType::Scena => {
			if game.is_ed7() {
				Ok((game, crate::Content::ED7Scena(scena::ed7::parse(&lines[1..], ctx)?)))
			} else {
				Ok((game, crate::Content::ED6Scena(scena::ed6::parse(&lines[1..], ctx)?)))
			}
		}
	}
}


#[test]
fn main() {
	let src = include_str!("/tmp/kiseki/ao_gf_en/c1200");
	let (v, diag) = super::diag::diagnose(|| {
		let tok = crate::parse::lex::lex(src);
		parse(&tok, None)
	});
	println!("{:#?}", v);
	super::diag::print_diags("<input>", src, &diag);
}
