mod lex;

use std::{cell::RefCell, rc::Rc};
use lex::{Lex, Token, TokenKind, Span, Error};

#[extend::ext]
pub impl<A, B> Result<A, B> {
	fn consume_err(self, f: impl FnOnce(B)) -> Option<A> {
		match self {
			Ok(a) => Some(a),
			Err(e) => {
				f(e);
				None
			}
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Spanned<'a, T>(Span<'a>, T);

impl<'a, T: std::fmt::Debug> std::fmt::Debug for Spanned<'a, T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("Spanned").field(&self.1).finish()
	}
}

impl<'a, A> Spanned<'a, A> {
	pub fn map<B>(self, f: impl FnOnce(A) -> B) -> Spanned<'a, B> {
		Spanned(self.0, f(self.1))
	}
}

#[inline]
fn is_indented(incl: bool, a: &str, b: &str) -> bool {
	(b.len() > a.len() || incl && b.len() == a.len()) && b.starts_with(a)
}

pub struct Parse<'a> {
	src: &'a str,
	indent: &'a str,
	tokens: &'a [Token<'a>],
	eof: &'a Token<'a>,
	errors: Rc<RefCell<Vec<Error<'a>>>>,
}

impl<'a> Parse<'a> {
	pub fn new(src: &'a str, tokens: &'a [Token<'a>], eof: &'a Token<'a>) -> Self {
		let indent = tokens.first().map_or_else(|| eof.span.as_str(), |a| a.indent.expect("parser must start on a line"));
		assert_eq!(eof.token, TokenKind::Eof);
		Parse {
			src,
			indent,
			tokens,
			eof,
			errors: Default::default(),
		}
	}

	fn peek(&self) -> &'a Token<'a> {
		self.tokens.first().unwrap_or(self.eof)
	}

	fn next(&mut self) -> &'a Token<'a> {
		if let Some((a, b)) = self.tokens.split_first() {
			self.tokens = b;
			a
		} else {
			// TODO the eof has wrong span
			self.eof
		}
	}

	fn next_if(&mut self, f: impl Fn(&Token<'a>) -> bool) -> Option<&Token<'a>> {
		f(self.peek()).then(|| self.next())
	}

	fn test(&mut self, token: &TokenKind<'a>) -> Option<&Token<'a>> {
		self.next_if(|a| &a.token == token)
	}

	fn require(&mut self, token: &TokenKind<'a>) -> Result<&Token<'a>, Error<'a>> {
		let span = self.prev_span();
		match self.test(token) {
			Some(token) => Ok(token),
			None => {
				Err(Error::Missing {
					span,
					token: token.clone(),
				})
			}
		}
	}

	fn skip_line(&mut self) -> Result<(), Error<'a>> {
		self.take_while(|t| t.indent.is_none())
			.end()
	}

	fn end(&self) -> Result<(), Error<'a>> {
		if let Some((a, b)) = self.tokens.first().zip(self.tokens.last()) {
			Err(Error::Misc {
				span: Span::join(self.src, a.span, b.span),
				desc: "expected end of block".to_owned(),
			})
		} else {
			Ok(())
		}
	}

	fn take_while(&mut self, f: impl Fn(&Token<'a>) -> bool) -> Parse<'a> {
		let mut i = 0;
		while i < self.tokens.len() {
			if !f(&self.tokens[i]) {
				break
			}
			i += 1;
		}
		let (t1, t2) = self.tokens.split_at(i);
		self.tokens = t2;
		Parse {
			src: self.src,
			indent: self.indent,
			tokens: t1,
			eof: self.eof,
			errors: Rc::clone(&self.errors),
		}
	}

	pub fn tight(&mut self) -> Result<(), Error<'a>> {
		let trivia = self.peek().trivia;
		if trivia.is_empty() {
			Ok(())
		} else {
			Err(Error::Misc {
				span: Span::from_str(trivia),
				desc: "no space allowed here".to_owned(),
			})
		}
	}

	pub fn indented(&mut self, inclusive: bool) -> Parse<'a> {
		self.take_while(|t| t.indent.is_none())
			.end()
			.consume_err(|e| self.emit(e));
		let indent = self.peek().indent.unwrap();
		if is_indented(true, self.indent, indent) {
			let mut v = self.take_while(|t| t.indent.map_or(true, |i| std::ptr::eq(i, indent) || is_indented(inclusive, indent, i)));
			v.indent = indent;
			v
		} else {
			self.take_while(|_| false)
		}
	}

	pub fn next_span(&self) -> Span<'a> {
		Span::from_str(self.peek().trivia).end()
	}

	pub fn prev_span(&self) -> Span<'a> {
		Span::from_str(self.peek().trivia).start()
	}

	pub fn span(&self, start: Span<'a>) -> Span<'a> {
		Span::join(self.src, start, self.prev_span())
	}

	fn emit(&mut self, e: Error<'a>) {
		self.errors.borrow_mut().push(e);
	}

	pub fn is_empty(&self) -> bool {
		self.tokens.is_empty()
	}
}

fn parse_ident<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, &'a str>, Error<'a>> {
	let t = p.next();
	match &t.token {
		TokenKind::Ident(i) => {
			Ok(Spanned(t.span, i))
		}
		_ => {
			Err(Error::Misc {
				span: t.span,
				desc: "expected an ident".to_owned(),
			})
		}
	}
}

fn parse_string<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, &'a str>, Error<'a>> {
	match p.next() {
		Token { token: TokenKind::String(s), span, .. } => {
			let s: &'a str = s;
			Ok(Spanned(*span, s))
		}
		Token { span, .. } => {
			Err(Error::Misc {
				span: *span,
				desc: "expected a string".to_owned(),
			})
		}
	}
}

fn parse_int<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, u64>, Error<'a>> {
	match p.next() {
		Token { token: TokenKind::Number(v), span, .. } => {
			if v.dec.is_some() {
				Err(Error::Misc {
					span: *span,
					desc: "no decimals allowed".to_owned(),
				})
			} else {
				Ok(Spanned(*span, v.val))
			}
		},
		Token { span, .. } => Err(Error::Misc {
			span: *span,
			desc: "expected an integer".to_owned(),
		})
	}
}

fn parse_signed_int<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, i64>, Error<'a>> {
	let pos = p.tokens;
	let i0 = p.next_span();
	let neg = p.test(&TokenKind::Minus).is_some();
	if neg {
		if let Err(e) = p.tight() {
			p.tokens = pos;
			return Err(e)
		}
	}
	let n = parse_int(p)?.1;
	let v = n as i64;
	let v = if neg { -v } else { v };
	Ok(Spanned(p.span(i0), v))
}

fn ident_line<'a>(p: &mut Parse<'a>) -> Result<(Spanned<'a, &'a str>, Parse<'a>), Error<'a>> {
	let mut p = p.indented(false);
	let i = parse_ident(&mut p)?;
	Ok((i, p))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Insn<'a> {
	name: Spanned<'a, &'a str>,
	args: Vec<Spanned<'a, Term<'a>>>,
}


#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Unit {
	None,
	Mm,
	MmPerS,
	Ms,
	Deg,
	MDeg,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Term<'a> {
	// Basics
	Number(Spanned<'a, i64>, Spanned<'a, Unit>),
	String(&'a str),
	Tuple(Vec<Spanned<'a, Term<'a>>>),

	Text(),
	Attr(Box<Term<'a>>, Spanned<'a, u64>),

	// Expr
	Random,
	Flag(Spanned<'a, u64>),
	System(Spanned<'a, u64>),
	Var(Spanned<'a, u64>),
	Global(Spanned<'a, u64>),

	// Mainly chars
	Null, // Though null can be useful for other cases too
	Self_,
	Custom(Spanned<'a, u64>),
	Party(Spanned<'a, u64>),
	FieldParty(Spanned<'a, u64>),

	// Scena specific
	Fn(Spanned<'a, u64>, Spanned<'a, u64>),
	Char(Spanned<'a, u64>),
	Entrance(Spanned<'a, u64>), // defined externally
	Object(Spanned<'a, u64>),   // defined externally
	LookPoint(Spanned<'a, u64>),
	Chcp(Spanned<'a, u64>),

	// Script resource ids
	Fork(Spanned<'a, u64>),
	Menu(Spanned<'a, u64>),
	Select(Spanned<'a, u64>),
	Vis(Spanned<'a, u64>),
	Eff(Spanned<'a, u64>),
	EffInstance(Spanned<'a, u64>),

	// Global tables
	Name(Spanned<'a, u64>),
	Battle(Spanned<'a, u64>), // This one is scena-specific in ed7, but whatever
	Bgm(Spanned<'a, u64>),
	Sound(Spanned<'a, u64>),
	Item(Spanned<'a, u64>),
	Magic(Spanned<'a, u64>),
	Quest(Spanned<'a, u64>),
	Shop(Spanned<'a, u64>),
	Town(Spanned<'a, u64>),
}

fn parse_nested_insn<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, Insn<'a>>, Error<'a>> {
	let i0 = p.next_span();
	let name = parse_ident(p)?;
	let mut args = Vec::new();
	while !p.is_empty() {
		let start = p.tokens as *const _;
		match parse_term(p) {
			Ok(t) => args.push(t),
			Err(e) => {
				if p.tokens as *const _ != start {
					p.emit(e);
				}
				break
			}
		}
	}
	Ok(Spanned(p.span(i0), Insn { name, args }))
}

fn parse_insn<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, Insn<'a>>, Error<'a>> {
	let i0 = p.next_span();
	let name = parse_ident(p)?;
	let mut args = Vec::new();
	while !p.is_empty() {
		args.push(parse_term(p)?);
	}
	Ok(Spanned(p.span(i0), Insn { name, args }))
}

fn parse_unit<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, Unit>, Error<'a>> {
	let i0 = p.next_span();
	if p.tight().is_ok() {
		if let TokenKind::Ident(i) = p.peek().token {
			let span = p.next().span;
			let u = match i {
				"mm" if p.tight().is_ok() && p.test(&TokenKind::Slash).is_some() => {
					p.require(&TokenKind::Ident("s"))?;
					Unit::MmPerS
				}
				"mm" => Unit::Mm,
				"ms" => Unit::Ms,
				"deg" => Unit::Deg,
				"mdeg" => Unit::MDeg,
				_ => return Err(Error::Misc {
					span: p.peek().span,
					desc: "invalid unit".to_owned(),
				})
			};
			return Ok(Spanned(p.span(i0), u))
		}
	}
	Ok(Spanned(p.prev_span(), Unit::None))
}

fn parse_term<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, Term<'a>>, Error<'a>> {
	fn brack<'a>(p: &mut Parse<'a>, f: impl FnOnce(&mut Parse<'a>) -> Result<Term<'a>, Error<'a>>) -> Result<Term<'a>, Error<'a>> {
		p.next(); // will be an ident, but that's checked outside
		p.tight()?;
		p.require(&TokenKind::LBrack)?;
		let e = f(p)?;
		p.require(&TokenKind::RBrack)?;
		Ok(e)
	}

	fn brack_int<'a>(p: &mut Parse<'a>, f: impl FnOnce(Spanned<'a, u64>) -> Term<'a>) -> Result<Term<'a>, Error<'a>> {
		brack(p, |p| parse_int(p).map(f))
	}

	let i0 = p.next_span();
	let t = match p.peek().token {
		TokenKind::String(_) => {
			Term::String(parse_string(p)?.1)
		}
		TokenKind::Minus | TokenKind::Number(_) => {
			let n = parse_signed_int(p)?;
			let unit = parse_unit(p)?;
			Term::Number(n, unit)
		}
		TokenKind::LParen => {
			p.require(&TokenKind::LParen)?;
			let mut terms = Vec::new();
			while p.test(&TokenKind::RParen).is_none() {
				terms.push(parse_term(p)?);
				if p.test(&TokenKind::Comma).is_none() {
					p.require(&TokenKind::RParen)?;
					break
				}
			}
			Term::Tuple(terms)
		}

		TokenKind::Ident("random") => { p.next(); Term::Random },
		TokenKind::Ident("flag") => brack_int(p, Term::Flag)?,
		TokenKind::Ident("system") => brack_int(p, Term::System)?,
		TokenKind::Ident("var") => brack_int(p, Term::Var)?,
		TokenKind::Ident("global") => brack_int(p, Term::Global)?,

		TokenKind::Ident("null") => { p.next(); Term::Null },
		TokenKind::Ident("self") => { p.next(); Term::Self_ },
		TokenKind::Ident("custom") => brack_int(p, Term::Custom)?,
		TokenKind::Ident("party") => brack_int(p, Term::Party)?,
		TokenKind::Ident("field_party") => brack_int(p, Term::FieldParty)?,

		TokenKind::Ident("fn") => brack(p, |p| {
			let a = parse_int(p)?;
			p.require(&TokenKind::Comma)?;
			let b = parse_int(p)?;
			Ok(Term::Fn(a, b))
		})?,
		TokenKind::Ident("char") => brack_int(p, Term::Char)?,
		TokenKind::Ident("entrance") => brack_int(p, Term::Entrance)?,
		TokenKind::Ident("object") => brack_int(p, Term::Object)?,
		TokenKind::Ident("look_point") => brack_int(p, Term::LookPoint)?,
		TokenKind::Ident("chcp") => brack_int(p, Term::Chcp)?,

		TokenKind::Ident("fork") => brack_int(p, Term::Fork)?,
		TokenKind::Ident("menu") => brack_int(p, Term::Menu)?,
		TokenKind::Ident("select") => brack_int(p, Term::Select)?,
		TokenKind::Ident("vis") => brack_int(p, Term::Vis)?,
		TokenKind::Ident("eff") => brack_int(p, Term::Eff)?,
		TokenKind::Ident("eff_instance") => brack_int(p, Term::EffInstance)?,

		TokenKind::Ident("name") => brack_int(p, Term::Name)?,
		TokenKind::Ident("battle") => brack_int(p, Term::Battle)?,
		TokenKind::Ident("bgm") => brack_int(p, Term::Bgm)?,
		TokenKind::Ident("sound") => brack_int(p, Term::Sound)?,
		TokenKind::Ident("item") => brack_int(p, Term::Item)?,
		TokenKind::Ident("magic") => brack_int(p, Term::Magic)?,
		TokenKind::Ident("quest") => brack_int(p, Term::Quest)?,
		TokenKind::Ident("shop") => brack_int(p, Term::Shop)?,
		TokenKind::Ident("town") => brack_int(p, Term::Town)?,

		_ => return Err(Error::Misc {
			span: p.peek().span,
			desc: "unexpected token".to_owned(),
		})
	};

	let mut t = t;
	while p.tight().is_ok() && p.test(&TokenKind::Dot).is_some() {
		p.tight()?;
		let b = parse_int(p)?;
		t = Term::Attr(Box::new(t), b)
	}

	println!("{:?}", t);
	Ok(Spanned(p.span(i0), t))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Expr<'a> {
	first: Spanned<'a, Atom<'a>>,
	rest: Vec<(Spanned<'a, Binop>, Spanned<'a, Atom<'a>>)>,
}

fn parse_expr<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, Expr<'a>>, Error<'a>> {
	let i0 = p.next_span();
	let first = parse_atom(p)?;
	let mut rest = Vec::new();
	while let Some(op) = parse_binop(p) {
		let next = parse_atom(p)?;
		rest.push((op, next))
	}
	Ok(Spanned(p.span(i0), Expr { first, rest }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Binop {
	Eq, Ne,
	Lt, Le,
	Gt, Ge,
	BoolAnd, BoolOr,
	Add, Sub, Mul, Div, Mod,
	Or, And, Xor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unop {
	Not,
	Neg,
	Plus,
	Inv,
}

fn parse_binop<'a>(p: &mut Parse<'a>) -> Option<Spanned<'a, Binop>> {
	macro op($p:ident, $t1:ident $($t:ident)* => $op:ident) {
		let i0 = p.next_span();
		let pos = $p.tokens;
		if let Ok::<(), Error>(()) = try {
			p.require(&TokenKind::$t1)?;
			$(p.tight()?; p.require(&TokenKind::$t)?;)*
		} {
			return Some(Spanned(p.span(i0), Binop::$op))
		}
		$p.tokens = pos;
	}

	op!(p, Eq Eq   => Eq);
	op!(p, Excl Eq => Ne);
	op!(p, Lt      => Lt);
	op!(p, Lt Eq   => Le);
	op!(p, Gt      => Gt);
	op!(p, Gt Eq   => Ge);

	op!(p, Pipe Pipe => BoolOr);
	op!(p, Amp  Amp  => BoolAnd);

	op!(p, Plus    => Add);
	op!(p, Minus   => Sub);
	op!(p, Star    => Mul);
	op!(p, Slash   => Div);
	op!(p, Percent => Mod);
	op!(p, Pipe    => Or);
	op!(p, Amp     => And);
	op!(p, Caret   => Xor);

	None
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Atom<'a> {
	Term(Term<'a>),
	Insn(Insn<'a>),
	Unop(Unop, Box<Spanned<'a, Atom<'a>>>),
	Paren(Box<Spanned<'a, Expr<'a>>>),
}

fn parse_atom<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, Atom<'a>>, Error<'a>> {
	let i0 = p.next_span();
	let v = match &p.peek().token {
		TokenKind::LParen => {
			p.require(&TokenKind::LParen)?;
			let e = parse_expr(p)?;
			p.require(&TokenKind::RParen)?;
			Atom::Paren(Box::new(e))
		}
		TokenKind::Plus => {
			p.require(&TokenKind::Plus)?;
			let e = parse_atom(p)?;
			Atom::Unop(Unop::Plus, Box::new(e))
		}
		TokenKind::Minus => {
			p.require(&TokenKind::Minus)?;
			let e = parse_atom(p)?;
			Atom::Unop(Unop::Neg, Box::new(e))
		}
		TokenKind::Excl => {
			p.require(&TokenKind::Excl)?;
			let e = parse_atom(p)?;
			Atom::Unop(Unop::Not, Box::new(e))
		}
		TokenKind::Tilde => {
			p.require(&TokenKind::Tilde)?;
			let e = parse_atom(p)?;
			Atom::Unop(Unop::Inv, Box::new(e))
		}
		_ => {
			let start = p.tokens as *const _;
			match parse_term(p) {
				Ok(t) => Atom::Term(t.1),
				Err(_) if p.tokens as *const _ == start => {
					Atom::Insn(parse_nested_insn(p)?.1)
				}
				Err(e) => return Err(e)
			}
		}
	};
	Ok(Spanned(p.span(i0), v))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Data<'a> {
	id: Spanned<'a, &'a str>,
	args: Vec<Spanned<'a, Term<'a>>>,
	body: Option<Vec<Spanned<'a, Data<'a>>>>,
}

fn parse_data<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, Data<'a>>, Error<'a>> {
	let mut p = p.indented(false);
	let p = &mut p;
	let i0 = p.next_span();
	let id = parse_ident(p)?;
	let mut args = Vec::new();
	let mut body = None;
	while !p.is_empty() {
		if p.test(&TokenKind::Colon).is_some() {
			body = Some(parse_block(p, parse_data));
			break
		}
		match parse_term(p) {
			Ok(a) => args.push(a),
			Err(e) => {
				p.emit(e);
				break
			}
		}
	}
	Ok(Spanned(p.span(i0), Data { id, args, body }))
}

fn parse_block<'a, T>(p: &mut Parse<'a>, mut f: impl FnMut(&mut Parse<'a>) -> Result<T, Error<'a>>) -> Vec<T> {
	let mut p = p.indented(true);
	let p = &mut p;
	let mut items = Vec::new();
	while !p.is_empty() {
		match f(p) {
			Ok(d) => items.push(d),
			Err(e) => {
				p.emit(e);
				continue
			}
		}
	}
	items
}

fn parse_function<'a>(p: &mut Parse<'a>) -> Result<(), Error<'a>> {
	p.require(&TokenKind::Ident("fn"))?;
	let id = parse_term(p)?;
	p.require(&TokenKind::Colon)?;
	let b = parse_code_block(p)?;
	Ok(())
}

fn parse_code_block<'a>(p: &mut Parse<'a>) -> Result<(), Error<'a>> {
	let mut p = p.indented(true);
	let i0 = p.next_span();
	while !p.is_empty() {
		let e: Result<(), Error> = try {
			let mut p = p.indented(false);
			let p = &mut p;
			match p.peek().token {
				// TODO can I do better handling of malformed these?
				TokenKind::Ident("if"|"elif"|"while"|"switch") => {
					p.next();
					let e = parse_expr(p)?;
					p.require(&TokenKind::Colon)?;
					let b = parse_code_block(p)?;
				}
				TokenKind::Ident("else") => {
					p.next();
					p.require(&TokenKind::Colon)?;
					let b = parse_code_block(p)?;
				}
				TokenKind::Ident("break"|"continue") => {
					p.next();
				}
				_ => {
					parse_insn(p)?;
				}
			}
			p.end()?;
		};
		e.consume_err(|e| p.emit(e));
	}
	Ok(())
}


pub fn parse_top<'a>(p: &mut Parse<'a>) -> Result<(), Error<'a>> {
	while !p.is_empty() {
		let e: Result<(), Error> = try {
			let mut p = p.indented(false);
			match p.peek().token {
				TokenKind::Ident("fn") => {
					parse_function(&mut p)?;
				}
				_ => {
					println!("parsing data");
					let d = parse_data(&mut p)?;
					println!("{:?}", d);
				}
			}
			p.end()?;
		};
		e.consume_err(|e| p.emit(e));
	}

	Ok(())
}

pub fn parse(src: &str) {
	let mut l = Lex::new(src);
	let tokens = lex::tokens(&mut l);
	let (eof, tokens) = tokens.split_last().unwrap();
	let mut errors = l.errors;

	let mut p = Parse::new(src, tokens, eof);
	parse_top(&mut p).consume_err(|e| p.emit(e));
	errors.extend(p.errors.take());

	use codespan_reporting::diagnostic::{Diagnostic, Label};
	use codespan_reporting::files::SimpleFiles;
	use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

	errors.sort_by_key(|a| a.span().as_str() as *const str);

	let writer = StandardStream::stderr(ColorChoice::Always);
	let config = codespan_reporting::term::Config::default();
	let mut files = SimpleFiles::new();
	let file_id = files.add("<input>", src);

	for e in &errors {
		let d = match e {
			Error::Missing { span, token } =>
				Diagnostic::error()
				.with_message(format!("missing {:?}", token))
				.with_labels(vec![
					Label::primary(file_id, span.position_in(src).unwrap())
						.with_message(format!("missing {:?}", token)),
				]),
			Error::Misc { span, desc } =>
				Diagnostic::error()
				.with_message(desc)
				.with_labels(vec![
					Label::primary(file_id, span.position_in(src).unwrap())
						.with_message(desc),
				]),
			Error::Duplicate { span, prev_span } =>
				Diagnostic::error()
				.with_message("duplicate field")
				.with_labels(vec![
					Label::primary(file_id, span.position_in(src).unwrap())
						.with_message("this field..."),
					Label::secondary(file_id, prev_span.position_in(src).unwrap())
						.with_message("...was already declared here"),
				]),
		};
		codespan_reporting::term::emit(&mut writer.lock(), &config, &files, &d).unwrap();
	}
}

#[test]
fn main() {
	parse(include_str!("/tmp/kiseki/tc/t1121"));
}
