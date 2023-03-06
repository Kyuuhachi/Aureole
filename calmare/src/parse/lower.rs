use std::collections::BTreeMap;

use themelios::scena::*;
use themelios::types::*;
use themelios_archive::Lookup;

use super::diag::*;
use crate::ast::*;
use crate::span::{Spanned as S, Span};

#[derive(Clone, Copy)]
struct Context<'a> {
	game: Game,
	ty: FileType,
	lookup: &'a dyn Lookup,
}
impl<'a> Context<'a> {
    fn new(file: &File, lookup: Option<&'a dyn Lookup>) -> Self {
		Context {
			game: file.game,
			ty: file.ty,
			lookup: lookup.unwrap_or_else(|| crate::util::default_lookup(file.game)),
		}
    }
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

#[derive(Clone, Debug)]
pub struct Parse<'a> {
	key_val: &'a KeyVal,
	pos: usize,
	context: &'a Context<'a>,
}

impl<'a> Parse<'a> {
	fn new(key_val: &'a KeyVal, context: &'a Context<'a>) -> Self {
		Parse { key_val, pos: 0, context }
	}

	fn pos(&self) -> Span {
		self.key_val.terms.get(self.pos).map_or(self.key_val.end, |a| a.0)
	}

	fn next(&mut self) -> Result<&'a Term> {
		if let Some(t) = self.peek() {
			self.pos += 1;
			Ok(t)
		} else {
			Err(Error)
		}
	}

	fn peek(&self) -> Option<&'a Term> {
		self.key_val.terms.get(self.pos).map(|a| &a.1)
	}

	fn term<T: Val>(&mut self, name: &str) -> Result<Option<T>> {
		if let Some(Term::Term(s)) = self.peek() && s.key.1 == name {
			self.pos += 1;
			Ok(Some(s.parse(self.context)?))
		} else {
			Ok(None)
		}
	}
}

impl KeyVal {
	fn parse<V: Val>(&self, context: &Context) -> Result<V> {
		let mut a = Parse::new(self, context);
		let v = V::parse(&mut a)?;
		if a.peek().is_some() {
			Diag::error(a.pos(), "expected end of data").emit();
		}
		Ok(v)
	}
}

trait Val: Sized {
	fn parse(p: &mut Parse) -> Result<Self>;
}

impl<T: Val> Val for S<T> {
	fn parse(p: &mut Parse) -> Result<Self> {
		let s1 = p.pos().at_start();
		let p1 = p.pos;
		let v = T::parse(p)?;
		let p2 = p.pos;
		if p1 == p2 {
			Ok(S(s1, v))
		} else {
			Ok(S(p.key_val.terms[p1].0 | p.key_val.terms[p2-1].0, v))
		}
	}
}


macro tuple($($T:ident)*) {
	impl<$($T: Val),*> Val for ($($T,)*) {
		fn parse(_p: &mut Parse) -> Result<Self> {
			Ok(($($T::parse(_p)?,)*))
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

macro int($T:ident $(=> $(#$CONV:ident)?)?) {
	impl Val for $T {
		fn parse(p: &mut Parse) -> Result<Self> {
			if let Some(Term::Int(s, u)) = p.peek() {
				p.next()?;
				if u.1 != Unit::None {
					Diag::warn(u.0, "this should be unitless").emit();
				}
				s.1.try_into().map_err(|e| {
					Diag::error(s.0, e).emit();
					Error
				}).map(unless!($($($CONV)? $T)?, {|a| a}))
			} else {
				Diag::error(p.pos(), "expected int").emit();
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
		if let Some(Term::String(s)) = p.peek() {
			p.next()?;
			Ok(s.to_owned())
		} else {
			Diag::error(p.pos(), "expected string").emit();
			Err(Error)
		}
	}
}

impl Val for TString {
	fn parse(p: &mut Parse) -> Result<Self> {
		if let Some(Term::Text(s)) = p.peek() {
			p.next()?;
			if let [S(_, TextSegment::Text(s))] = s.as_slice() {
				Ok(TString(s.to_owned()))
			} else {
				Diag::error(p.pos(), "expected short text").emit();
				Err(Error)
			}
		} else {
			Diag::error(p.pos(), "expected short text").emit();
			Err(Error)
		}
	}
}

macro unit($T:ident, $unit:ident, $unit_str:literal) {
	impl Val for $T {
		fn parse(p: &mut Parse) -> Result<Self> {
			if let Some(Term::Int(s, u)) = p.peek() {
				p.next()?;
				if u.1 != Unit::$unit {
					Diag::warn(u.0, format_args!("unit should be '{}'", $unit_str)).emit();
				}
				s.1.try_into().map_err(|e| {
					Diag::error(s.0, e).emit();
					Error
				}).map(Self)
			} else {
				Diag::error(p.pos(), format_args!("expected '{}' number", $unit_str)).emit();
				Err(Error)
			}
		}
	}
}

unit!(Angle, Deg, "deg");

impl Val for Pos3 {
	fn parse(p: &mut Parse) -> Result<Self> {
		if let Some((x, y, z)) = p.term("")? {
			Ok(Pos3(x, y, z))
		} else {
			Diag::error(p.pos(), "expected pos3").emit();
			Err(Error)
		}
	}
}

impl Val for FuncRef {
	fn parse(p: &mut Parse) -> Result<Self> {
		if let Some((a, b)) = p.term("fn")? {
			Ok(FuncRef(a, b))
		} else {
			Diag::error(p.pos(), "expected 'fn'").emit();
			Err(Error)
		}
	}
}

impl Val for FileId {
	fn parse(p: &mut Parse) -> Result<Self> {
		if let Some(Term::String(s)) = p.peek() {
			let pos = p.pos();
			p.next()?;
			Ok(FileId(p.context.lookup.index(s).unwrap_or_else(|| {
				Diag::error(pos, "could not resolve file id").emit();
				0x00000000
			})))
		} else if let Some(s) = p.term("file")? {
			Ok(FileId(s))
		} else {
			Diag::error(p.pos(), "expected string or 'file'").emit();
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
				Diag::error(p.pos(), format_args!("expected '{}'", $s)).emit();
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
newtype!(AnimId, "anim");

macro when {
	($t1:tt) => {},
	($t1:tt, $($t:tt)*) => { $($t)* }
}
macro unless {
	(, $v:tt) => { $v },
	($t:tt,$v:tt) => { $t },
}

macro parse_data {
	($d:expr, $c:expr => $head:pat) => {
		let d = $d;
		let c = $c;
		if d.body.is_some() {
			Diag::error(d.head.end, "a body is not allowed here").emit();
		}
		let $head = d.head.parse(c)?;
	},
	($d:expr, $c:expr => $head:pat, {
		$($k:ident $(: $t:ty)? $(=> $e:expr)?),* $(,)?
	}) => {
		let d = $d;
		let c = $c;
		let head = d.head.parse(c);

		$($(let mut $k: One<Option<$t>> = One::Empty;)?)*
		let Some(body) = &d.body else {
			Diag::error(d.head.end, "a body is required here").emit();
			Err(Error)?;
			unreachable!()
		};
		for line in body {
			let head = &line.head.key;
			match head.1.as_str() {
				$(stringify!($k) => {
					let _val = unless!($({
						let a: Result<_> = $e(line);
						a
					})?, {
						if line.body.is_some() {
							Diag::error(d.head.end, "body is not allowed here").emit();
						}
						line.head.parse(c)
					});

					unless!($({
						when!($t);
						$k.set(head.0, _val.ok());
					})?, {
						let _: Result<()> = _val;
					});
				})*
				_ => {
					Diag::error(head.0, "unknown field")
						.note(head.0, format_args!("allowed fields are {}", [
							$(concat!("'", stringify!($k), "'"),)*
						].join(", ")))
						.emit();
				}
			}
		}
		let mut failures = Vec::new();
		$($(when!($t);
			let $k = $k.optional();
			if $k.is_none() {
				failures.push(concat!("'", stringify!($k), "'"));
			}
		)?)*;
		if !failures.is_empty() {
			Diag::error(d.head.span(), "missing fields")
				.note(d.head.span(), failures.join(", "))
				.emit();
			Err(Error)?;
			unreachable!()
		}

		#[allow(clippy::let_unit_value)]
		let $head = head?;
		$($(let Some($k): $t = $k.unwrap() else { Err(Error)?; unreachable!() };)?)*
	}
}

pub fn lower(file: &File, lookup: Option<&dyn Lookup>) {
	match file.ty {
		FileType::Scena => {
			if file.game.is_ed7() {
				scena::ed7::lower(file, lookup);
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

	fn optional(self) -> Option<T> {
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

	fn finish(self, f: impl Fn(K) -> usize) -> Vec<V> {
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
		let ast = crate::parse::parse::parse(&tok).unwrap();
		lower(&ast, None)
	});
	println!("{:#?}", v);
	super::diag::print_diags("<input>", src, &diag);
}
