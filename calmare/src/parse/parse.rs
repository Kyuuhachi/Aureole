use total_float::F64;

use super::lex::*;
use super::diag::Diag;
use crate::ast::*;
use crate::span::{Span, Spanned as S};

#[derive(Clone, Debug)]
pub struct Error;
type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug)]
struct Parse<'a> {
	tokens: &'a [S<Token<'a>>],
	eof: Span,
	pos: usize,
}

impl<'a> Parse<'a> {
	fn run<V>(
		tokens: &'a [S<Token<'a>>],
		eof: Span,
		f: impl FnOnce(&mut Parse<'a>) -> Result<V>,
	) -> Result<V> {
		let mut p = Parse { tokens, eof, pos: 0 };
		let v = f(&mut p);
		if v.is_ok() && !p.is_empty() {
			Diag::error(p.next_pos(), "expected end of data").emit();
		}
		v
	}

	fn next_span(&self) -> Span {
		self.peek().map_or(self.eof, |a| a.0)
	}

	fn prev_span(&self) -> Span {
		self.tokens[self.pos-1].0
	}

	fn next_pos(&self) -> Span {
		self.next_span().at_start()
	}

	fn prev_pos(&self) -> Span {
		self.prev_span().at_end()
	}

	fn next(&mut self) -> Result<S<&'a Token<'a>>> {
		if let Some(t) = self.peek() {
			self.pos += 1;
			Ok(t)
		} else {
			Err(Error)
		}
	}

	fn peek(&self) -> Option<S<&'a Token<'a>>> {
		self.tokens.get(self.pos).map(|S(s, a)| S(*s, a))
	}

	fn test(&mut self, t: &Token<'a>) -> bool {
		if self.peek().map_or(false, |a| a.1 == t) {
			self.pos += 1;
			true
		} else {
			false
		}
	}

	fn require(&mut self, t: &Token<'a>, name: &str) {
		if !self.test(t) {
			Diag::error(self.next_pos(), format_args!("expected {name}")).emit();
		}
	}

	fn is_tight(&self) -> bool {
		self.prev_pos().connects(self.next_pos())
	}

	fn rewind(&mut self) {
		self.pos -= 1;
	}

	fn is_empty(&self) -> bool {
		self.remaining().is_empty()
	}

	fn remaining(&self) -> &'a [S<Token<'a>>] {
		&self.tokens[self.pos..]
	}
}

fn parse_ident(p: &mut Parse) -> Result<S<String>> {
	match p.next()? {
		S(s, Token::Ident(v)) => Ok(S(s, (*v).to_owned())),
		S(s, _) => {
			Diag::error(s, "expected keyword").emit();
			Err(Error)
		}
	}
}

fn parse_insn_name(p: &mut Parse) -> Result<S<String>> {
	match p.next()? {
		S(s, Token::Insn(v)) => Ok(S(s, (*v).to_owned())),
		S(s, _) => {
			Diag::error(s, "expected insn").emit();
			Err(Error)
		}
	}
}

fn parse_unit(p: &mut Parse) -> Result<S<Unit>> {
	let s0 = p.prev_pos();
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

fn parse_term(p: &mut Parse) -> Result<S<Term>> {
	if let Some(a) = try_parse_term(p)? {
		Ok(a)
	} else {
		Diag::error(p.next_span(), "expected term").emit();
		Err(Error)
	}
}

fn try_parse_term(p: &mut Parse) -> Result<Option<S<Term>>> {
	let i0 = p.next_pos();
	let t = p.next()?;
	let t = match t.1 {
		Token::String(s) => {
			Term::String(s.clone())
		}

		Token::Int(_) => {
			let s = p.prev_span();
			let Token::Int(n) = t.1 else { unreachable!() };
			let unit = parse_unit(p)?;
			Term::Int(S(s, *n as i64), unit)
		}
		Token::Minus if p.is_tight() && matches!(p.peek(), Some(S(_, Token::Int(_)))) => {
			let s = p.prev_span();
			let Token::Int(n) = p.next()?.1 else { unreachable!() };
			let s = s | p.prev_span();
			let unit = parse_unit(p)?;
			Term::Int(S(s, -(*n as i64)), unit)
		}

		Token::Float(_) => {
			let s = p.prev_span();
			let Token::Float(n) = t.1 else { unreachable!() };
			let unit = parse_unit(p)?;
			Term::Float(S(s, *n), unit)
		}
		Token::Minus if p.is_tight() && matches!(p.peek(), Some(S(_, Token::Float(_)))) => {
			let s = p.prev_span();
			let Token::Float(n) = p.next()?.1 else { unreachable!() };
			let s = s | p.prev_span();
			let unit = parse_unit(p)?;
			Term::Float(S(s, F64(-n.0)), unit)
		}

		Token::Paren(d) => Term::Term(parse_delim(S(t.0, String::new()), d, "parenthesis")?),

		Token::Brace(d) => {
			let segs = d.tokens.iter().map(|t| Ok(S(t.0, match &t.1 {
				TextToken::Text(t) => TextSegment::Text(t.clone()),
				TextToken::Newline(n) => TextSegment::Newline(*n),
				TextToken::Hex(v) => TextSegment::Hex(*v),
				TextToken::Brace(a) => TextSegment::Directive(Parse::run(&a.tokens, a.close, |p| key_val(true, p, parse_ident))?),
			}))).collect::<Result<Vec<_>>>()?;
			Term::Text(segs)
		}

		Token::Ident(s) => {
			let key = S(t.0, (*s).to_owned());
			let key_val = if p.is_tight() && let Some(S(_, Token::Bracket(d))) = p.peek() {
				p.next()?;
				let a = parse_delim(key, d, "bracket")?;
				if a.terms.is_empty() {
					Diag::error(d.open|d.close, "this cannot be empty").emit();
				}
				a
			} else {
				KeyVal { key, terms: Vec::new(), end: p.prev_span().at_start() }
			};
			Term::Term(key_val)
		},

		_ => {
			p.rewind();
			return Ok(None)
		}
	};

	Ok(Some(S(i0 | p.prev_pos(), t)))
}

fn parse_delim(key: S<String>, d: &Delimited<Token>, name: &str) -> Result<KeyVal> {
	Parse::run(&d.tokens, d.close, |p| {
		let mut terms = Vec::new();
		while !p.is_empty() {
			terms.push(parse_term(p)?);
			if p.is_empty() {
				break
			}
			if !p.test(&Token::Comma) {
				Diag::error(p.next_span(), format!("expected comma or closing {name}"))
					.note(d.open, "opened here")
					.emit();
				return Err(Error)
			}
		}
		Ok(KeyVal { key, terms, end: d.close })
	})
}

fn key_val(abbrev: bool, p: &mut Parse, f: impl FnOnce(&mut Parse) -> Result<S<String>>) -> Result<KeyVal> {
	let key = f(p)?;
	// allow {item[413]} instead of {item item[413]}
	if abbrev && matches!(p.peek(), Some(S(_, Token::Bracket(_)))) {
		p.rewind()
	}
	let mut terms = Vec::new();
	while !p.is_empty() {
		terms.push(parse_term(p)?);
	}
	Ok(KeyVal { key, terms, end: p.next_pos() })
}

fn parse_type(line: &Line) -> Result<(Game, FileType)> {
	Parse::run(&line.head, line.eol, |p| {
		p.require(&Token::Ident("type"), "'type'");
		no_body(line);
		use Token::Ident as I;
		let game = match p.next()?.1 {
			I("fc") => Game::Fc, I("fc_e") => Game::FcEvo, I("fc_k") => Game::FcKai,
			I("sc") => Game::Sc, I("sc_e") => Game::ScEvo, I("sc_k") => Game::ScKai,
			I("tc") => Game::Tc, I("tc_e") => Game::TcEvo, I("tc_k") => Game::TcKai,
			I("zero") => Game::Zero, I("zero_e") => Game::ZeroEvo, I("zero_k") => Game::ZeroKai,
			I("ao") => Game::Ao, I("ao_e") => Game::AoEvo, I("ao_k") => Game::AoKai,
			_ => {
				Diag::error(p.prev_span(), "unknown game").emit();
				return Err(Error);
			}
		};
		let ty = match p.next()?.1 {
			I("scena") => FileType::Scena,
			_ => {
				Diag::error(p.prev_span(), "unknown file type").emit();
				return Err(Error);
			}
		};
		Ok((game, ty))
	})
}

fn parse_data(top: bool, line: &Line) -> Result<Data> {
	let mut head = Parse::run(&line.head, line.eol, |p| key_val(top, p, parse_ident))?;
	head.end = line.eol;
	let body = line.body.as_deref().map(|b| parse_lines(b, |p| parse_data(false, p)));
	Ok(Data { head, body })
}

fn parse_fn(line: &Line) -> Result<Function> {
	Parse::run(&line.head, line.eol, |p| {
		p.require(&Token::Ident("fn"), "'fn'");
		if matches!(p.peek(), Some(S(_, Token::Bracket(_)))) {
			p.rewind()
		}
		parse_term(p)?;
		let asm = p.test(&Token::Ident("asm"));
		let body = if asm {
			todo!()
		} else {
			FnBody::Code(parse_body(line, parse_code)?)
		};

		let mut head = Parse::run(&line.head, line.eol, |p| key_val(true, p, parse_ident))?;
		head.end = line.eol;
		Ok(Function { head, body })
	})
}

fn parse_code(line: &Line) -> Result<S<Code>> {
	let a = Parse::run(&line.head, line.eol, |p| match p.next()?.1 {
		Token::Ident("if") => {
			let e = parse_expr(p);
			let b = parse_body(line, parse_code);
			Ok(Code::If(e?, b?))
		}
		Token::Ident("elif") => {
			let e = parse_expr(p);
			let b = parse_body(line, parse_code);
			Ok(Code::Elif(e?, b?))
		}
		Token::Ident("else") => {
			let b = parse_body(line, parse_code);
			Ok(Code::Else(b?))
		}
		Token::Ident("while") => {
			let e = parse_expr(p);
			let b = parse_body(line, parse_code);
			Ok(Code::While(e?, b?))
		}
		Token::Ident("switch") => {
			let e = parse_expr(p);
			let b = parse_body(line, |line| {
				let mut head = Parse::run(&line.head, line.eol, |p| key_val(false, p, parse_ident))?;
				head.end = line.eol;
				let body = parse_body(line, parse_code);
				Ok((head, body?))
			});
			Ok(Code::Switch(e?, b?))
		}
		Token::Ident("break") => {
			no_body(line);
			Ok(Code::Break)
		}
		Token::Ident("continue") => {
			no_body(line);
			Ok(Code::Continue)
		}
		_ => {
			p.rewind();
			if let Some(t) = try_parse_term(p)? {
				let o = parse_assop(p).ok_or_else(|| {
					Diag::error(p.next_pos(), "expected assignment operator").emit();
					Error
				})?;
				let e = parse_expr(p)?;
				Ok(Code::Assign(t, o, e))
			} else {
				Ok(Code::Insn(key_val(false, p, parse_insn_name)?))
			}
		}
	})?;
	Ok(S(line.head_span(), a))
}

fn parse_expr(p: &mut Parse) -> Result<S<Expr>> {
	parse_expr0(p, 10)
}

fn parse_expr0(p: &mut Parse, prec: usize) -> Result<S<Expr>> {
	let mut e = parse_atom(p)?;
	while let Some(op) = parse_binop(p, prec) {
		let e2 = parse_expr0(p, prec-1)?;
		e = S(e.0 | e2.0, Expr::Binop(Box::new(e), op, Box::new(e2)));
	}
	Ok(e)
}

fn parse_atom(p: &mut Parse) -> Result<S<Expr>> {
	let i0 = p.next_pos();
	let e = match p.next()?.1 {
		Token::Paren(d) => {
			Parse::run(&d.tokens, d.close, parse_expr)?.1
		}
		Token::Minus => {
			let e = parse_atom(p)?;
			Expr::Unop(S(p.prev_span(), Unop::Neg), Box::new(e))
		}
		Token::Excl => {
			let e = parse_atom(p)?;
			Expr::Unop(S(p.prev_span(), Unop::Not), Box::new(e))
		}
		Token::Tilde => {
			let e = parse_atom(p)?;
			Expr::Unop(S(p.prev_span(), Unop::Inv), Box::new(e))
		}
		_ => {
			p.rewind();
			if let Some(t) = try_parse_term(p)? {
				Expr::Term(t.1)
			} else {
				let key = parse_insn_name(p)?;
				let mut terms = Vec::new();
				while let Some(t) = try_parse_term(p)? {
					terms.push(t);
				}
				Expr::Insn(KeyVal { key, terms, end: p.next_pos() })
			}
		}
	};
	Ok(S(i0 | p.prev_pos(), e))
}

macro op($p:ident; $t1:ident $($t:ident)* => $op:expr) {
	let i0 = $p.next_pos();
	let pos = $p.pos;
	if $p.test(&Token::$t1) $( && $p.is_tight() && $p.test(&Token::$t))* {
		return Some(S(i0 | $p.prev_pos(), $op))
	}
	$p.pos = pos;
}

fn parse_assop(p: &mut Parse) -> Option<S<Assop>> {
	op!(p;         Eq => Assop::Assign);
	op!(p; Plus    Eq => Assop::Add);
	op!(p; Minus   Eq => Assop::Sub);
	op!(p; Star    Eq => Assop::Mul);
	op!(p; Slash   Eq => Assop::Div);
	op!(p; Percent Eq => Assop::Mod);
	op!(p; Pipe    Eq => Assop::Or);
	op!(p; Amp     Eq => Assop::And);
	op!(p; Caret   Eq => Assop::Xor);

	None
}

fn parse_binop(p: &mut Parse, prec: usize) -> Option<S<Binop>> {
	macro prio($prio:literal, $p:stmt) {
		if prec >= $prio {
			$p
		}
	}
	prio!(4, op!(p; Eq Eq   => Binop::Eq));
	prio!(4, op!(p; Excl Eq => Binop::Ne));
	prio!(4, op!(p; Lt      => Binop::Lt));
	prio!(4, op!(p; Lt Eq   => Binop::Le));
	prio!(4, op!(p; Gt      => Binop::Gt));
	prio!(4, op!(p; Gt Eq   => Binop::Ge));

	prio!(1, op!(p; Pipe Pipe => Binop::BoolOr));
	prio!(3, op!(p; Amp  Amp  => Binop::BoolAnd));

	prio!(5, op!(p; Plus    => Binop::Add));
	prio!(5, op!(p; Minus   => Binop::Sub));
	prio!(6, op!(p; Star    => Binop::Mul));
	prio!(6, op!(p; Slash   => Binop::Div));
	prio!(6, op!(p; Percent => Binop::Mod));
	prio!(1, op!(p; Pipe    => Binop::Or));
	prio!(3, op!(p; Amp     => Binop::And));
	prio!(2, op!(p; Caret   => Binop::Xor));

	None
}

fn parse_lines<'a, T>(
	lines: &'a [Line],
	mut f: impl FnMut(&'a Line) -> Result<T>
) -> Vec<T> {
	lines.iter().filter_map(|a| f(a).ok()).collect::<Vec<_>>()
}

fn no_body(line: &Line) {
	if line.body.is_some() {
		Diag::error(line.eol, "a body is not allowed here").emit();
	}
}

fn parse_body<'a, T>(
	line: &'a Line,
	f: impl FnMut(&'a Line) -> Result<T>
) -> Result<Vec<T>> {
	if let Some(body) = &line.body {
		Ok(parse_lines(body, f))
	} else {
		Diag::error(line.eol, "a body is required here").emit();
		Err(Error)
	}
}

pub fn parse(lines: &[Line]) -> Result<File> {
	if lines.is_empty() {
		Diag::error(Span::new_at(0), "no type declaration").emit();
		return Err(Error);
	}
	let (game, ty) = parse_type(&lines[0])?;
	let decls = parse_lines(&lines[1..], |l| parse_decl(l, game, ty));
	Ok(File { game, ty, decls })
}

fn parse_decl(line: &Line, _game: Game, ty: FileType) -> Result<Decl> {
	match line.head.first() {
		Some(S(_, Token::Ident("fn"))) if ty == FileType::Scena =>
			Ok(Decl::Function(parse_fn(line)?)),
		_ =>
			Ok(Decl::Data(parse_data(true, line)?))
	}
}

#[test]
fn main() {
	let src = include_str!("/tmp/kiseki/ao_gf_en/a0000");
	let (v, diag) = super::diag::diagnose(|| {
		let tok = lex(src);
		parse(&tok)
	});
	println!("{:#?}", v);
	super::diag::print_diags("<input>", src, &diag);
}
