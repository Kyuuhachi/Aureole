use std::collections::BTreeMap;

use themelios::scena::*;
use themelios::types::*;
use themelios::util::array;
use themelios_archive::Lookup;

use super::diag::*;
use super::lex::{Line, Token, TextToken};
// use crate::ast::*;
use crate::span::{Spanned as S, Span};

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

pub macro f {
	($p:pat $(if $e:expr)? => $v:expr) => { |_a| {
		match _a {
			$p $(if $e)? => Some($v),
			_ => None
		}
	} },
	($p:pat $(if $e:expr)? ) => { |_a| {
		match _a {
			$p $(if $e)? => true,
			_ => false
		}
	} },
}

#[derive(Clone, Debug)]
pub struct Parse<'a> {
	tokens: &'a [S<Token<'a>>],
	pos: usize,
	body: Option<&'a [Line<'a>]>,
	context: &'a Context<'a>,
	eol: Span,
	commas: bool,
}

impl<'a> Parse<'a> {
	fn new(line: &'a Line, context: &'a Context) -> Self {
		Parse {
			tokens: &line.head,
			pos: 0,
			body: line.body.as_deref(),
			context,
			eol: line.eol,
			commas: false,
		}
	}

	fn parse<T: Val>(mut self) -> Result<T> {
		let v = T::parse(&mut self)?;
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

	fn next(&mut self) -> Option<&'a Token<'a>> {
		self.next_if(Some)
	}

	fn next_if<T>(&mut self, f: impl FnOnce(&'a Token<'a>) -> Option<T>) -> Option<T> {
		if let Some(S(_, t)) = self.tokens.get(self.pos) && let Some(x) = f(t) {
			self.pos += 1;
			Some(x)
		} else {
			None
		}
	}

	fn tuple<T: Val>(&mut self) -> Result<Option<T>> {
		if let Some(d) = self.next_if(f!(Token::Paren(d) => d)) {
			Parse {
				tokens: &d.tokens,
				pos: 0,
				body: None,
				context: self.context,
				eol: d.close,
				commas: true,
			}.parse().map(Some)
		} else {
			Ok(None)
		}
	}

	fn word(&mut self, name: &str) -> bool {
		self.next_if(f!(Token::Ident(a) if *a == name => ())).is_some()
	}

	fn term<T: Val>(&mut self, name: &str) -> Result<Option<T>> {
		let span = self.next_span().at_end();
		if self.word(name) {
			let space = self.space();
			if let Some(d) = self.next_if(f!(Token::Bracket(d) => d)) {
				if let Some(s) = space {
					Diag::error(s, "no space allowed here").emit()
				}
				if d.tokens.is_empty() {
					Diag::error(d.open | d.close, "this cannot be empty")
						.note(d.open | d.close, "maybe remove the brackets?")
						.emit()
				}
				Parse {
					tokens: &d.tokens,
					pos: 0,
					body: None,
					context: self.context,
					eol: d.close,
					commas: true,
				}.parse().map(Some)
			} else {
				Parse {
					tokens: &[],
					pos: 0,
					body: None,
					context: self.context,
					eol: span,
					commas: true,
				}.parse().map(Some)
			}
		} else {
			Ok(None)
		}
	}

	fn sep(&mut self) {
		if self.pos < self.tokens.len() && self.commas {
			let span = self.prev_span().at_end();
			if self.next_if(f!(Token::Comma => ())).is_none() {
				Diag::error(span, "expected comma").emit();
			}
		}
	}

	fn remaining(&self) -> &'a [S<Token<'a>>] {
		&self.tokens[self.pos..]
	}
}

trait Val: Sized {
	fn parse(p: &mut Parse) -> Result<Self>;
}

impl<T: Val> Val for S<T> {
	fn parse(p: &mut Parse) -> Result<Self> {
		let s1 = p.next_span().at_start();
		let p1 = p.pos;
		let v = T::parse(p)?;
		let p2 = p.pos;
		if p1 == p2 {
			Ok(S(s1, v))
		} else {
			Ok(S(p.tokens[p1].0 | p.tokens[p2-1].0, v))
		}
	}
}

fn parse_and_comma<T: Val>(p: &mut Parse) -> Result<T> {
	let a = T::parse(p)?;
	p.sep();
	Ok(a)
}

macro tuple($($T:ident)*) {
	impl<$($T: Val),*> Val for ($($T,)*) {
		fn parse(_p: &mut Parse) -> Result<Self> {
			Ok(($(parse_and_comma::<$T>(_p)?,)*))
		}
	}
}

tuple!();
tuple!(A);
tuple!(A B);
tuple!(A B C);
tuple!(A B C D);
tuple!(A B C D E);
tuple!(A B C D E F);
tuple!(A B C D E F G);
tuple!(A B C D E F G H);
tuple!(A B C D E F G H I);
tuple!(A B C D E F G H I J);
tuple!(A B C D E F G H I J K);

impl<T: Val> Val for Vec<T> {
	fn parse(p: &mut Parse) -> Result<Self> {
		let mut v = Vec::new();
		while p.pos < p.tokens.len() {
			v.push(parse_and_comma::<T>(p)?);
		}
		Ok(v)
	}
}

impl<const N: usize, T: Val> Val for [T; N] {
	fn parse(p: &mut Parse) -> Result<Self> {
		array(|| parse_and_comma::<T>(p))
	}
}

impl<T: Val> Val for Option<T> {
	fn parse(p: &mut Parse) -> Result<Self> {
		if let Some(()) = p.term("null")? {
			Ok(None)
		} else {
			T::parse(p).map(Some)
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
	impl Val for $T {
		fn parse(p: &mut Parse) -> Result<Self> {
			if let Some((s, u)) = parse_int(p)? {
				if u.1 != Unit::None {
					Diag::warn(u.0, "this should be unitless").emit();
				}
				s.1.try_into().map_err(|e| {
					Diag::error(s.0, e).emit();
					Error
				}).map(unless!($({$($CONV)? $T})?, {|a| a}))
			} else {
				Diag::error(p.next_span(), "expected int").emit();
				Err(Error)
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
int!(EntryFlags =>);
int!(CharFlags =>);

impl Val for String {
	fn parse(p: &mut Parse) -> Result<Self> {
		if let Some(s) = p.next_if(f!(Token::String(s) => s)) {
			Ok(s.to_owned())
		} else {
			Diag::error(p.next_span(), "expected string").emit();
			Err(Error)
		}
	}
}

impl Val for TString {
	fn parse(p: &mut Parse) -> Result<Self> {
		if let Some(d) = p.next_if(f!(Token::Brace(s) => s)) {
			match d.tokens.as_slice() {
				[] => Ok(TString(String::new())),
				[S(_, TextToken::Text(s))] => Ok(TString(s.to_owned())),
				_ => {
					Diag::error(p.next_span(), "expected short text").emit();
					Err(Error)
				}
			}
		} else {
			Diag::error(p.next_span(), "expected short text").emit();
			Err(Error)
		}
	}
}

macro unit($T:ident, $unit:ident, $unit_str:literal) {
	impl Val for $T {
		fn parse(p: &mut Parse) -> Result<Self> {
			if let Some((s, u)) = parse_int(p)? {
				if u.1 != Unit::$unit {
					Diag::warn(u.0, format_args!("unit should be '{}'", $unit_str)).emit();
				}
				s.1.try_into().map_err(|e| {
					Diag::error(s.0, e).emit();
					Error
				}).map(Self)
			} else {
				Diag::error(p.next_span(), format_args!("expected '{}' number", $unit_str)).emit();
				Err(Error)
			}
		}
	}
}

unit!(Angle, Deg, "deg");
unit!(Time, Ms, "ms");

impl Val for f32 {
	fn parse(p: &mut Parse) -> Result<Self> {
		if let Some((s, u)) = parse_float(p)? {
			if u.1 != Unit::None {
				Diag::warn(u.0, "this should be unitless").emit();
			}
			Ok(s.1 as f32)
		} else if let Some((s, u)) = parse_int(p)? {
			if u.1 != Unit::None {
				Diag::warn(u.0, "this should be unitless").emit();
			}
			Ok(s.1 as f32)
		} else {
			Diag::error(p.next_span(), "expected float").emit();
			Err(Error)
		}
	}
}

impl Val for Pos3 {
	fn parse(p: &mut Parse) -> Result<Self> {
		if let Some((x, y, z)) = p.tuple()? {
			Ok(Pos3(x, y, z))
		} else {
			Diag::error(p.next_span(), "expected pos3").emit();
			Err(Error)
		}
	}
}

impl Val for FuncRef {
	fn parse(p: &mut Parse) -> Result<Self> {
		if let Some((a, b)) = p.term("fn")? {
			Ok(FuncRef(a, b))
		} else {
			Diag::error(p.next_span(), "expected 'fn'").emit();
			Err(Error)
		}
	}
}

impl Val for CharAttr {
	fn parse(p: &mut Parse) -> Result<Self> {
		if let Some((a, b)) = p.term("char_attr")? {
			Ok(CharAttr(a, b))
		} else {
			Diag::error(p.next_span(), "expected 'char_attr'").emit();
			Err(Error)
		}
	}
}

impl Val for FileId {
	fn parse(p: &mut Parse) -> Result<Self> {
		if let Some(s) = p.next_if(f!(Token::String(s) => s)) {
			Ok(FileId(p.context.lookup.index(s).unwrap_or_else(|| {
				Diag::error(p.prev_span(), "could not resolve file id").emit();
				0x00000000
			})))
		} else if let Some(()) = p.term("null")? {
			Ok(FileId(0))
		} else if let Some(s) = p.term("file")? {
			Ok(FileId(s))
		} else {
			Diag::error(p.next_span(), "expected string, 'null', or 'file'").emit();
			Err(Error)
		}
	}
}

impl Val for CharId {
	fn parse(p: &mut Parse) -> Result<Self> {
		let span = p.next_span();
		if let Some(()) = p.term("self")? {
			Ok(CharId(254))
		} else if let Some(()) = p.term("null")? {
			Ok(CharId(255))
		} else if let Some(s) = p.term::<u16>("name")? {
			Ok(CharId(s + 257))
		} else if let Some(s) = p.term::<u16>("char")? {
			Ok(CharId(s + if p.context.game.base() == BaseGame::Tc { 16 } else { 8 }))
		} else if let Some(s) = p.term::<u16>("field_party")? {
			Ok(CharId(s))
		} else if let Some(s) = p.term::<u16>("party")? {
			Ok(CharId(s + if p.context.game.base() == BaseGame::Sc { 246 } else { 238 }))
		} else if let Some(s) = p.term::<u16>("custom")? {
			if p.context.game.base() == BaseGame::Ao {
				Ok(CharId(s + 244))
			} else {
				Diag::error(span, "'custom' is only supported on Azure").emit();
				Err(Error)
			}
		} else {
			Diag::error(span, "expected 'self', 'null', 'name', 'char', 'field_party', 'party', or 'custom'").emit();
			Err(Error)
		}
	}
}

macro newtype($T:ident, $s:literal) {
	impl Val for $T {
		fn parse(p: &mut Parse) -> Result<Self> {
			if let Some(v) = p.term($s)? {
				Ok(Self(v))
			} else {
				Diag::error(p.next_span(), format_args!("expected '{}'", $s)).emit();
				Err(Error)
			}
		}
	}
}

newtype!(TownId, "town");
newtype!(BgmId, "bgm");
newtype!(ChcpId, "chcp");
newtype!(BattleId, "battle");
newtype!(Flag, "flag");
newtype!(Var, "var");
newtype!(Attr, "system");
newtype!(Global, "global");
newtype!(AnimId, "anim");
newtype!(LookPointId, "look_point");
newtype!(LabelId, "label");
newtype!(TriggerId, "trigger");

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
		let mut $k = One::Empty;
	});)*

	let body = $d.body()?;

	for line in body {
		Parse::new(line, $d.context).parse_with(|p| {
			let Some(key) = p.next_if(f!(Token::Ident(a) => a)) else {
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
						let v = Val::parse(p);
						if v.is_err() {
							p.pos = p.tokens.len();
						}
						$k.set(p.prev_span(), v.ok());
					});
				})*
				_ => {
					let fields: &[&str] = &[
						$(concat!("'", stringify!($k), "'"),)*
					];
					Diag::error(p.prev_span(), "unknown field")
						.note(p.prev_span(), format_args!("allowed fields are {}", &fields.join(", ")))
						.emit();
				}
			}
		})
	}

	#[allow(unused_mut)]
	let mut failures: Vec<&str> = Vec::new();
	$(unless!($({when!($e);})?, {
		let $k = $k.get();
		if $k.is_none() {
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
		let Some($k) = $k.unwrap() else { Err(Error)?; unreachable!() };
	});)*
}

fn parse_type(line: &Line) -> Result<(Game, FileType)> {
	let dummy_ctx = &Context {
		game: Game::Fc,
		ty: FileType::Scena,
		lookup: &themelios_archive::NullLookup,
	};
	Parse::new(line, dummy_ctx).parse_with(|p| {
		// TODO just remove this word, it doesn't do any good
		if !p.word("type") {
			Diag::error(p.prev_span(), "expected 'type'").emit();
			return Err(Error);
		}
		let Some(a) = p.next_if(f!(Token::Ident(a) => a)) else {
			Diag::error(p.prev_span(), "expected a game").emit();
			return Err(Error);
		};
		let Some(b) = p.next_if(f!(Token::Ident(a) => a)) else {
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

pub fn parse(lines: &[Line], lookup: Option<&dyn Lookup>) -> Result<()> {
	if lines.is_empty() {
		Diag::error(Span::new_at(0), "no type declaration").emit();
		return Err(Error);
	}

	let (game, ty) = parse_type(&lines[0])?;
	let ctx = &Context {
		game,
		ty,
		lookup: lookup.unwrap_or_else(|| crate::util::default_lookup(game)),
	};

	match ty {
		FileType::Scena => {
			if game.is_ed7() {
				let a = scena::ed7::parse(&lines[1..], ctx)?;
				Ok(())
			} else {
				todo!();
				// lower_ed6_scena(&file);
			}
		}
	}
}

#[derive(Debug, Clone, Default)]
enum One<T> {
	#[default]
	Empty,
	Set(Span, T)
}

impl<T> One<T> {
	fn set(&mut self, s: Span, v: T) {
		if let One::Set(prev, _) = self {
			Diag::error(s, "duplicate item")
				.note(*prev, "previous here")
				.emit();
		}
		*self = One::Set(s, v);
	}

	fn get(self) -> Option<T> {
		match self {
			One::Empty => None,
			One::Set(_, v) => Some(v),
		}
	}
}

#[derive(Debug, Clone)]
struct Many<K, V>(BTreeMap<K, (Span, V)>);

impl<K, V> Default for Many<K, V> {
	fn default() -> Self {
		Self(Default::default())
	}
}

impl<K: Ord, V> Many<K, V> {
	fn insert(&mut self, s: Span, n: K, v: V) {
		if let Some((prev, _)) = self.0.get(&n) {
			Diag::error(s, "duplicate item")
				.note(*prev, "previous here")
				.emit();
		}
		self.0.insert(n, (s, v));
	}

	fn get(self, f: impl Fn(K) -> usize) -> Vec<V> {
		let mut vs = Vec::with_capacity(self.0.len());
		let mut expect = 0;
		for (k, (s, v)) in self.0 {
			let k = f(k);
			if k != expect {
				Diag::error(s, "gap in list")
					.note(s, format_args!("missing index {expect}"))
					.emit();
			}
			expect = k + 1;
			vs.push(v)
		}
		vs
	}
}

pub mod scena;

#[test]
fn main() {
	let src = include_str!("/tmp/kiseki/ao_gf_en/a0000");
	let (v, diag) = super::diag::diagnose(|| {
		let tok = crate::parse::lex::lex(src);
		parse(&tok, None)
	});
	println!("{:#?}", v);
	super::diag::print_diags("<input>", src, &diag);
}
