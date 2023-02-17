use super::lex::*;
use super::diag::*;
use super::ast::*;
use Spanned as S;

struct Error;
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

	fn require_tight(&self) {
		if !self.is_tight() {
			Diag::error(self.prev_pos() | self.next_pos(), "no space allowed here").emit();
		}
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

fn parse_int(p: &mut Parse) -> Result<S<u64>> {
	match p.next()? {
		S(s, Token::Int(v)) => Ok(Spanned(s, *v)),
		S(s, _) => {
			Diag::error(s, "expected integer").emit();
			Err(Error)
		}
	}
}

fn parse_ident(p: &mut Parse) -> Result<S<String>> {
	match p.next()? {
		S(s, Token::Ident(v)) => Ok(Spanned(s, (*v).to_owned())),
		S(s, _) => {
			Diag::error(s, "expected keyword").emit();
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
		_ => return Ok(Spanned(s0, Unit::None))
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
	Ok(Spanned(s, u))
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
	fn brack(p: &mut Parse, f: impl FnOnce(&mut Parse) -> Result<Term>) -> Result<Term> {
		p.require_tight();
		match p.next()? {
			S(_, Token::Bracket(b)) => {
				Parse::run(&b.tokens, b.close, f)
			}
			S(s, _) => {
				Diag::error(s, "expected bracket").emit();
				Err(Error)
			}
		}
	}

	fn brack_int(p: &mut Parse, f: impl FnOnce(S<u64>) -> Term) -> Result<Term> {
		brack(p, |p| parse_int(p).map(f))
	}

	let i0 = p.next_pos();

	let t = match p.next()?.1 {
		Token::String(s) => {
			Term::String(s.clone())
		}
		Token::Int(_) => {
			p.rewind();
			let n = parse_int(p)?.map(|a| a as i64);
			let unit = parse_unit(p)?;
			Term::Int(n, unit)
		}
		Token::Minus if p.is_tight() && matches!(p.peek(), Some(S(_, Token::Int(_)))) => {
			let n = parse_int(p)?.map(|a| -(a as i64));
			let unit = parse_unit(p)?;
			Term::Int(n, unit)
		}

		Token::Paren(d) => Parse::run(&d.tokens, d.close, |p| {
			let mut terms = Vec::new();
			while !p.is_empty() {
				terms.push(parse_term(p)?);
				if p.is_empty() {
					break
				}
				if !p.test(&Token::Comma) {
					Diag::error(p.next_span(), "expected comma or closing parenthesis")
						.note(d.open, "opened here")
						.emit();
					return Err(Error)
				}
			}
			Ok(Term::Tuple(terms))
		})?,

		Token::Brace(d) => {
			let segs = d.tokens.iter().map(|t| Ok(Spanned(t.0, match &t.1 {
				TextToken::Text(t) => TextSegment::Text(t.clone()),
				TextToken::Newline(n) => TextSegment::Newline(*n),
				TextToken::Hex(v) => TextSegment::Hex(*v),
				TextToken::Brace(a) => TextSegment::Directive(Parse::run(&a.tokens, a.close, |p| key_val(true, p))?),
			}))).collect::<Result<Vec<_>>>()?;
			Term::Text(segs)
		}

		Token::Ident("random") => Term::Random,
		Token::Ident("flag") => brack_int(p, Term::Flag)?,
		Token::Ident("system") => brack_int(p, Term::System)?,
		Token::Ident("var") => brack_int(p, Term::Var)?,
		Token::Ident("global") => brack_int(p, Term::Global)?,

		Token::Ident("emote") => brack(p, |p| {
			let a = parse_int(p)?;
			p.require(&Token::Comma, "comma");
			let b = parse_int(p)?;
			p.require(&Token::Comma, "comma");
			let c = parse_term(p)?;
			Ok(Term::Emote(a,b,Box::new(c)))
		})?,

		Token::Ident("null") => Term::Null,
		Token::Ident("self") => Term::Self_,
		Token::Ident("custom") => brack_int(p, Term::Custom)?,
		Token::Ident("party") => brack_int(p, Term::Party)?,
		Token::Ident("field_party") => brack_int(p, Term::FieldParty)?,

		Token::Ident("fn") => brack(p, |p| {
			let a = parse_int(p)?;
			p.require(&Token::Comma, "comma");
			let b = parse_int(p)?;
			Ok(Term::Fn(a, b))
		})?,
		Token::Ident("char") => brack_int(p, Term::Char)?,
		Token::Ident("entrance") => brack_int(p, Term::Entrance)?,
		Token::Ident("object") => brack_int(p, Term::Object)?,
		Token::Ident("look_point") => brack_int(p, Term::LookPoint)?,
		Token::Ident("chcp") => brack_int(p, Term::Chcp)?,

		Token::Ident("fork") => brack_int(p, Term::Fork)?,
		Token::Ident("menu") => brack_int(p, Term::Menu)?,
		Token::Ident("select") => brack_int(p, Term::Select)?,
		Token::Ident("vis") => brack_int(p, Term::Vis)?,
		Token::Ident("eff") => brack_int(p, Term::Eff)?,
		Token::Ident("eff_instance") => brack_int(p, Term::EffInstance)?,

		Token::Ident("name") => brack_int(p, Term::Name)?,
		Token::Ident("battle") => brack_int(p, Term::Battle)?,
		Token::Ident("bgm") => brack_int(p, Term::Bgm)?,
		Token::Ident("sound") => brack_int(p, Term::Sound)?,
		Token::Ident("item") => brack_int(p, Term::Item)?,
		Token::Ident("magic") => brack_int(p, Term::Magic)?,
		Token::Ident("quest") => brack_int(p, Term::Quest)?,
		Token::Ident("shop") => brack_int(p, Term::Shop)?,
		Token::Ident("town") => brack_int(p, Term::Town)?,

		_ => {
			p.rewind();
			return Ok(None)
		}
	};

	let mut t = t;
	while p.is_tight() && p.test(&Token::Dot) {
		p.require_tight();
		let b = parse_int(p)?;
		t = Term::Attr(Box::new(t), b)
	}

	Ok(Some(Spanned(i0 | p.prev_pos(), t)))
}

fn parse_top(line: &Line) -> Result<Decl> {
	match line.head.first() {
		Some(S(_, Token::Ident("fn"))) =>
			Ok(Decl::Function(parse_fn(line)?)),
		_ =>
			Ok(Decl::Data(parse_data(true, line)?))
	}
}

fn key_val(abbrev: bool, p: &mut Parse) -> Result<KeyVal> {
	let key = parse_ident(p)?;
	// allow {item[413]} instead of {item item[413]}
	if abbrev && matches!(p.peek(), Some(S(_, Token::Bracket(_)))) {
		p.rewind()
	}
	let mut terms = Vec::new();
	while !p.is_empty() {
		terms.push(parse_term(p)?);
	}
	Ok(KeyVal { key, terms })
}

fn parse_data(top: bool, line: &Line) -> Result<Data> {
	let head = Parse::run(&line.head, line.eol, |p| key_val(top, p))?;
	let body = line.body.as_deref().map(|b| parse_lines(b, |p| parse_data(false, p)));
	Ok(Data { head, body })
}

fn parse_fn(line: &Line) -> Result<Function> {
	Parse::run(&line.head, line.eol, |p| {
		assert_eq!(p.next()?.1, &Token::Ident("fn"));
		if matches!(p.peek(), Some(S(_, Token::Bracket(_)))) {
			p.rewind()
		}
		let id = parse_term(p)?;
		let asm = p.test(&Token::Ident("asm"));
		let body = if asm {
			todo!()
		} else {
			FnBody::Code(parse_body(line, parse_code)?)
		};
		Ok(Function { id, body })
	})
}

fn parse_code(line: &Line) -> Result<Spanned<Code>> {
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
				let head = Parse::run(&line.head, line.eol, |p| match p.next()?.1 {
					Token::Ident("case") => Ok(SwitchCase::Case(parse_term(p)?)),
					Token::Ident("default") => Ok(SwitchCase::Default),
					_ => {
						p.rewind();
						Diag::error(p.next_span(), "invalid switch case").emit();
						Err(Error)
					}
				});
				let body = parse_body(line, parse_code);
				Ok((S(line.head_span(), head?), body?))
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
				Ok(Code::Insn(key_val(false, p)?))
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
				Expr::Insn(key_val(false, p)?)
			}
		}
	};
	Ok(S(i0 | p.prev_pos(), e))
}

macro op($p:ident; $t1:ident $($t:ident)* => $op:expr) {
	let i0 = $p.next_pos();
	let pos = $p.pos;
	if $p.test(&Token::$t1) $( && $p.is_tight() && $p.test(&Token::$t))* {
		return Some(Spanned(i0 | $p.prev_pos(), $op))
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

pub fn parse(lines: &[Line]) -> Vec<Decl> {
	parse_lines(lines, parse_top)
}

#[test]
fn main() {
	let src = include_str!("/tmp/kiseki/tc/t1121");
	let (v, diag) = diagnose(|| {
		let tok = lex(src);
		parse(&tok)
	});
	println!("{:#?}", v);

	use codespan_reporting::diagnostic::{Diagnostic, Label};
	use codespan_reporting::files::SimpleFiles;
	use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

	let writer = StandardStream::stderr(ColorChoice::Always);
	let config = codespan_reporting::term::Config::default();
	let mut files = SimpleFiles::new();
	let file_id = files.add("<input>", src);

	for d in diag {
		let mut l = vec![
			Label::primary(file_id, d.span.as_range()).with_message(d.text),
		];
		for (s, t) in d.notes {
			l.push(Label::secondary(file_id, s.as_range()).with_message(t));
		}
		let d = Diagnostic::error().with_labels(l);
		codespan_reporting::term::emit(&mut writer.lock(), &config, &files, &d).unwrap();
	}
}

