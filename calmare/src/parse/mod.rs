mod lex;

use std::{cell::RefCell, rc::Rc};

use lex::Lex;
use themelios::scena::{FuncRef, Pos3};

use self::lex::{Token, TokenKind, Span, Error};

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
        f.debug_tuple("Spanned").field(&(..)).field(&self.1).finish()
    }
}

#[inline]
fn is_indented(incl: bool, a: &str, b: &str) -> bool {
	(b.len() > a.len() || incl && b.len() == a.len()) && b.starts_with(a)
}

// pub struct Parse<'a> {
// 	lex: Lex<'a>,
// 	peek: Option<Token<'a>>,
// 	indent: &'a str,
// 	eof_token: Token<'a>,
// }

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

fn parse_int<'a>(p: &mut Parse<'a>) -> Result<u64, Error<'a>> {
	match p.next() {
		Token { token: TokenKind::Number(v), span, .. } => {
			if v.dec.is_some() {
				Err(Error::Misc {
					span: *span,
					desc: "no decimals allowed".to_owned(),
				})
			} else {
				Ok(v.val)
			}
		},
		Token { span, .. } => Err(Error::Misc {
			span: *span,
			desc: "expected an integer".to_owned(),
		})
	}
}

fn parse_signed_int<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, i64>, Error<'a>> {
	let i0 = p.next_span();
	let neg = p.test(&TokenKind::Minus).is_some();
	let n = parse_int(p)?;
	let v = n as i64;
	let v = if neg { -v } else { v };
	Ok(Spanned(p.span(i0), v))
}

type Milli = i32;

fn parse_length<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, Milli>, Error<'a>> {
	parse_signed_int(p).map(|Spanned(a, v)| Spanned(a, v as i32))
}

fn parse_angle<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, i64>, Error<'a>> {
	let i0 = p.next_span();
	let v = parse_signed_int(p)?;
	p.tight()?;
	p.require(&TokenKind::Ident("deg"))?;
	Ok(Spanned(p.span(i0), v.1))
}

fn parse_wrapped_int<'a>(p: &mut Parse<'a>, s: &'a str) -> Result<Spanned<'a, u64>, Error<'a>> {
	let i0 = p.next_span();
	p.require(&TokenKind::Ident(s))?;
	p.tight()?;
	p.require(&TokenKind::LParen)?;
	let v = parse_int(p)?;
	p.require(&TokenKind::RParen)?;
	Ok(Spanned(p.span(i0), v))
}

fn parse_func_ref<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, FuncRef>, Error<'a>> {
	let i0 = p.next_span();
	let a = parse_int(p)? as u16;
	p.tight()?;
	p.require(&TokenKind::Colon)?;
	p.tight()?;
	let b = parse_int(p)? as u16;
	Ok(Spanned(p.span(i0), FuncRef(a, b)))
}

fn parse_pos<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, Pos3>, Error<'a>> {
	let i0 = p.next_span();
	p.require(&TokenKind::LParen)?;
	let x = parse_length(p)?.1;
	p.require(&TokenKind::Comma)?;
	let y = parse_length(p)?.1;
	p.require(&TokenKind::Comma)?;
	let z = parse_length(p)?.1;
	p.require(&TokenKind::RParen)?;
	Ok(Spanned(p.span(i0), Pos3(x, y, z)))
}

fn ident_line<'a>(p: &mut Parse<'a>) -> Result<(Spanned<'a, &'a str>, Parse<'a>), Error<'a>> {
	let mut p = p.indented(false);
	let t = p.next();
	match &t.token {
		TokenKind::Ident(i) => {
			Ok((Spanned(t.span, i), p))
		}
		_ => {
			Err(Error::Misc {
				span: t.span,
				desc: "expected an ident".to_owned(),
			})
		}
	}
}

macro parse_fields($p:ident $(, $name:ident => $f:expr)* $(,)?) {
	$(let mut $name = None;)*
	let mut p = $p.indented(true);
	let i0 = p.next_span();
	while !p.is_empty() {
		let e: Result<(), Error> = try {
			let (t, mut p) = ident_line(&mut p)?;
			match t.1 {
				$(stringify!($name) => {
					let o = $name.replace(Spanned(t.0, None));
					$name = {
						let $p = &mut p;
						Some(Spanned(t.0, Some($f)))
					};
					if let Some(Spanned(prev_span, _)) = o {
						Err(Error::Duplicate { span: t.0, prev_span })?;
					}
				})*
				_ => {
					const ALTS: &[&str] = &[$(concat!("'", stringify!($name), "'")),*];
					Err(Error::Misc {
						span: t.0,
						desc: format!("unknown field, expected {}", ALTS.join(", ")),
					})?;
				}
			}
			p.end()?;
		};
		e.consume_err(|e| p.emit(e));
	}
	let mut missing = Vec::new();
	$(let $name = match $name {
		Some(Spanned(_, v)) => v,
		None => {
			missing.push(concat!("'", stringify!($name), "'"));
			None
		}
	};)*
	if !missing.is_empty() {
		$p.emit(Error::Misc {
			span: p.span(i0),
			desc: format!("missing fields: {}", missing.join(", ")),
		});
	}
}

fn parse_header<'a>(p: &mut Parse<'a>) -> Result<(), Error<'a>> {
	p.require(&TokenKind::Ident("ed6"))?;
	p.require(&TokenKind::Colon)?;

	parse_fields! { p,
		name => (parse_string(p)?, parse_string(p)?),
		town => parse_wrapped_int(p, "TownId")?,
		bgm  => parse_wrapped_int(p, "BgmId")?,
		item => parse_func_ref(p)?,
	};

	println!("{:?}", (name,town,bgm,item));

	Ok(())
}

fn parse_scp<'a>(p: &mut Parse<'a>) -> Result<(), Error<'a>> {
	let n = parse_int(p)?;
	let file = parse_string(p)?;
	println!("{:?}", (n, file));
	Ok(())
}

fn parse_entry<'a>(p: &mut Parse<'a>) -> Result<(), Error<'a>> {
	p.require(&TokenKind::Colon)?;
	parse_fields! { p,
		pos => parse_pos(p)?,
		chr => parse_int(p)?,
		angle  => parse_angle(p)?,
		cam_from => parse_pos(p)?,
		cam_at => parse_pos(p)?,
		cam_zoom => parse_int(p)?,
		cam_pers => parse_int(p)?,
		cam_deg => parse_angle(p)?,
		cam_limit => (parse_angle(p)?, parse_angle(p)?),
		north => parse_angle(p)?,
		flags => parse_int(p)?,
		town => parse_wrapped_int(p, "TownId")?,
		init => parse_func_ref(p)?,
		reinit => parse_func_ref(p)?,
	};
	println!("{:?}", (pos,chr,angle));
	println!("{:?}", (cam_from,cam_at,cam_zoom,cam_pers,cam_deg,cam_limit,north));
	println!("{:?}", (north,flags,town,init,reinit));
	Ok(())
}

pub fn parse_top<'a>(p: &mut Parse<'a>) -> Result<(), Error<'a>> {
	p.require(&TokenKind::Ident("scena"))?;
	parse_header(p)?;

	while !p.is_empty() {
		let e: Result<(), Error> = try {
			let (t, mut p) = ident_line(p)?;
			match t.1 {
				"scp" => parse_scp(&mut p)?,
				"entry" => parse_entry(&mut p)?,
				_ => {
					Err(Error::Misc {
						span: t.0,
						desc: "invalid declaration".to_owned(),
					})?;
				}
			}
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
