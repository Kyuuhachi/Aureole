mod lex;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spanned<'a, T>(Span<'a>, T);

#[inline]
fn is_indented(incl: bool, a: &str, b: &str) -> bool {
	(b.len() > a.len() || incl && b.len() == a.len()) && b.starts_with(a)
}

struct Parse<'a> {
	lex: Lex<'a>,
	peek: Option<Token<'a>>,
	indent: &'a str,
	eof_token: Token<'a>,
}

impl<'a> Parse<'a> {
	pub fn new(src: &'a str) -> Self {
		Parse {
			lex: Lex::new(src),
			peek: None,
			indent: &src[..0],
			eof_token: Token {
				trivia: &src[..0],
				indent: Some(&src[..0]),
				span: Span::from_prefix(src, src),
				token: TokenKind::Eof,
			}
		}
	}

	// This will return an eof token if the peeked one is insufficiently indented.
	fn peek_(&mut self) -> &Token<'a> {
		self.peek.get_or_insert_with(|| lex::token(&mut self.lex))
	}

	fn peek(&mut self) -> &Token<'a> {
		let tok = self.peek.get_or_insert_with(|| lex::token(&mut self.lex));

		if !tok.indent.map_or(true, |ind| is_indented(false, self.indent, ind)) {
			let span = tok.span.start();
			self.eof_token = Token {
				trivia: tok.trivia,
				indent: Some(span.as_str()),
				span,
				token: TokenKind::Eof,
			};
			return &self.eof_token
		}

		tok
	}

	// This will return the next token regardless.
	fn next(&mut self) -> Token<'a> {
		self.peek_();
		self.peek.take().unwrap()
	}

	fn next_if(&mut self, f: impl Fn(&Token<'a>) -> bool) -> Option<Token<'a>> {
		f(self.peek()).then(|| self.next())
	}

	fn test(&mut self, token: &TokenKind<'a>) -> Option<Token<'a>> {
		self.next_if(|a| &a.token == token)
	}

	fn require(&mut self, token: &TokenKind<'a>) -> Result<Token<'a>, Error<'a>> {
		if let Some(token) = self.test(token) {
			Ok(token)
		} else {
			let span = self.peek().span.start();
			Err(Error::Missing {
				span,
				token: token.clone(),
			})
		}
	}

	fn skip_to_eol(&mut self) -> Result<(), Error<'a>> {
		let ind = self.indent;
		self.skip_while(|t| t.indent.map_or(true, |i| is_indented(false, ind, i)))
	}

	fn skip_while(&mut self, f: impl Fn(&Token<'a>) -> bool) -> Result<(), Error<'a>> {
		if f(self.peek()) {
			let i0 = self.peek().span;
			while f(self.peek()) {
				self.next();
			}
			Err(Error::Misc {
				span: self.span(i0),
				desc: "unexpected data".to_owned(),
			})
		} else {
			Ok(())
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

	pub fn block(&mut self, body: impl FnOnce(&mut Self)) -> Span<'a> {
		self.skip_while(|t| t.indent.is_none())
			.consume_err(|e| self.emit(e));

		let ind = self.peek().indent.unwrap();
		let i0 = Span::from_str(ind).start();

		if is_indented(false, self.indent, ind) {
			let prev_ind = std::mem::replace(&mut self.indent, ind);
			body(self);
			self.skip_while(|t| t.token != TokenKind::Eof)
				.consume_err(|e| self.emit(e));
			self.indent = prev_ind;
		}

		let ind = self.peek().indent.unwrap();
		let i1 = Span::from_str(ind).start();

		Span::join(self.lex.src, i0, i1)
	}

	pub fn is_eof(&mut self) -> bool {
		self.peek().token == TokenKind::Eof
	}

	pub fn next_span(&mut self) -> Span<'a> {
		Span::from_str(self.peek().trivia).end()
	}

	pub fn prev_span(&mut self) -> Span<'a> {
		Span::from_str(self.peek().trivia).start()
	}

	pub fn span(&mut self, start: Span<'a>) -> Span<'a> {
		Span::join(self.lex.src, start, self.prev_span())
	}

	fn emit(&mut self, e: Error<'a>) {
		self.lex.errors.push(e);
	}
}

fn ident_or_skip_line<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, &'a str>, Error<'a>> {
	match p.next() {
		Token { span, token: TokenKind::Ident(token), .. } => Ok(Spanned(span, token)),
		Token { span, .. } => Err(Error::Misc {
			span,
			desc: "expected word".to_owned(),
		})
	}
}

fn ident_lines<'a>(
	p: &mut Parse<'a>,
	mut f: impl FnMut(&mut Parse<'a>, Spanned<'a, &'a str>) -> Result<(), Error<'a>>
) {
	while p.peek_().token != TokenKind::Eof && is_indented(true, p.indent, p.peek_().indent.unwrap()) {
		match ident_or_skip_line(p).and_then(|t| f(p, t)) {
			Ok(()) => {
				let _ = p.skip_to_eol();
			}
			Err(e) => {
				p.lex.errors.push(e);
				p.skip_to_eol()
					.consume_err(|e| p.emit(e));
			}
		}
	}
}

fn parse_string<'a>(p: &mut Parse<'a>) -> Result<Spanned<'a, String>, Error<'a>> {
	match p.next() {
		Token { token: TokenKind::String(s), span, .. } => Ok(Spanned(span, s)),
		Token { span, .. } => {
			Err(Error::Misc {
				span,
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
					span,
					desc: "no decimals allowed".to_owned(),
				})
			} else {
				Ok(v.val)
			}
		},
		Token { span, .. } => Err(Error::Misc {
			span,
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
	let b = parse_int(p)? as u16;
	Ok(Spanned(p.span(i0), Pos3(x, y, z)))
}

macro parse_fields($p:ident $(, $name:ident => $f:expr)* $(,)?) {
	$(let mut $name = None;)*
	let span = $p.block(|$p| ident_lines($p, |$p, Spanned(span, token)| {
		match token {
			$(stringify!($name) => {
				println!(stringify!($name));
				let o = $name.replace(Spanned(span, None));
				$name = Some(Spanned(span, Some($f)));
				if let Some(Spanned(prev_span, _)) = o {
					return Err(Error::Duplicate { span, prev_span })
				}
			})*
			_ => {
				const ALTS: &[&str] = &[$(concat!("'", stringify!($name), "'")),*];
				return Err(Error::Misc {
					span,
					desc: format!("unknown field, expected {}", ALTS.join(", ")),
				});
			}
		}
		Ok(())
	}));
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
			span,
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

	Ok(())
}

fn parse_scp<'a>(p: &mut Parse<'a>) -> Result<(), Error<'a>> {
	let n = parse_int(p)?;
	let file = parse_string(p)?;
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
		cam_zoom => parse_pos(p)?,
		cam_pers => parse_int(p)?,
		cam_deg => parse_angle(p)?,
		cam_limit => (parse_angle(p)?, parse_angle(p)?),
		north => parse_angle(p)?,
		flags => parse_int(p)?,
		town => parse_wrapped_int(p, "TownId")?,
		init => parse_func_ref(p)?,
		reinit => parse_func_ref(p)?,
	};
	Ok(())
}

pub fn parse_top<'a>(p: &mut Parse<'a>) -> Result<(), Error<'a>> {
	p.require(&TokenKind::Ident("scena"))?;
	parse_header(p)?;

	ident_lines(p, |p, t| {
		println!("{:?}", t);
		match t.1 {
			"scp" => parse_scp(p)?,
			"entry" => parse_entry(p)?,
			_ => {
				return Err(Error::Misc {
					span: t.0,
					desc: "invalid declaration".to_owned(),
				});
			}
		}
		Ok(())
	});

	Ok(())
}

pub fn parse(src: &str) {
	let mut p = Parse::new(src);
	parse_top(&mut p);

	for e in &p.lex.errors {
		println!("{:?} {:?}", e.span().position_in(src).unwrap(), e);
	}
}

#[test]
fn main() {
	parse(include_str!("/tmp/kiseki/tc/t1121"));
}
