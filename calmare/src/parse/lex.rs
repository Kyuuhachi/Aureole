use std::str::pattern::Pattern;
use std::ops::Range;

use unicode_xid::UnicodeXID;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span<'a>(&'a str);

// from https://github.com/rust-lang/rfcs/pull/2796
pub fn range_of<T>(slice: &[T], subslice: &[T]) -> Option<Range<usize>> {
	let range = slice.as_ptr_range();
	let subrange = subslice.as_ptr_range();
	if subrange.start >= range.start && subrange.end <= range.end {
		unsafe {
			Some(Range {
				start: subrange.start.offset_from(range.start) as usize,
				end: subrange.end.offset_from(range.start) as usize,
			})
		}
	} else {
		None
	}
}

impl<'a> Span<'a> {
	pub fn from_str(s: &'a str) -> Self {
		Span(s)
	}

	pub fn as_str(&self) -> &'a str {
		self.0
	}

	pub fn start(&self) -> Self {
		Span(&self.0[..0])
	}

	pub fn end(&self) -> Self {
		Span(&self.0[self.0.len()..])
	}

	pub fn from_prefix(start: &'a str, end: &'a str) -> Self {
		let range = range_of(start.as_bytes(), end.as_bytes()).expect("should be a subslice");
		assert_eq!(range.end, start.len());
		Span(&start[..range.start])
	}

	pub fn join(src: &'a str, a: Span<'a>, b: Span<'a>) -> Self {
		let range1 = range_of(src.as_bytes(), a.0.as_bytes()).expect("`a` should be a subslice");
		let range2 = range_of(src.as_bytes(), b.0.as_bytes()).expect("`b` should be a subslice");
		assert!(range1.start <= range2.start);
		assert!(range1.end <= range2.end);
		Span(&src[range1.start..range2.end])
	}

	pub fn position_in(&self, o: &'a str) -> Option<Range<usize>> {
		range_of(o.as_bytes(), self.0.as_bytes())
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error<'a> {
	Missing {
		span: Span<'a>,
		token: TokenKind<'a>,
	},
	Misc {
		span: Span<'a>,
		desc: String,
	},
	Duplicate {
		span: Span<'a>,
		prev_span: Span<'a>,
	},
}

impl<'a> Error<'a> {
	pub fn span(&self) -> Span<'a> {
		match self {
			Error::Missing { span, .. } => *span,
			Error::Misc { span, .. } => *span,
			Error::Duplicate { span, .. } => *span,
		}
	}
}

pub struct Lex<'a> {
	pub src: &'a str,
	pub pos: &'a str,
	pub errors: Vec<Error<'a>>,
}

impl<'a> Lex<'a> {
	pub fn new(src: &'a str) -> Self {
		Lex {
			src,
			pos: src,
			errors: Vec::new(),
		}
	}

	fn emit<B: ToString>(&mut self, start: &'a str, e: B) {
		self.errors.push(Error::Misc {
			span: self.span(start),
			desc: e.to_string(),
		});
	}

	fn emit_result<A, B: ToString>(&mut self, start: &'a str, result: Result<A, B>) -> Option<A> {
		match result {
			Ok(v) => Some(v),
			Err(e) => {
				self.emit(start, e);
				None
			}
		}
	}

	fn span(&self, start: &'a str) -> Span<'a> {
		Span::from_prefix(start, self.pos)
	}
}

fn pat<'a, P: Pattern<'a> + 'a>(i: &mut Lex<'a>, p: P) -> Option<&'a str> {
	let i0 = i.pos;
	i.pos = i.pos.strip_prefix(p)?;
	Some(i.span(i0).as_str())
}

fn pat_mul<'a, P: Pattern<'a> + Clone>(i: &mut Lex<'a>, p: P) -> &'a str {
	let i0 = i.pos;
	while let Some(j1) = i.pos.strip_prefix(p.clone()) {
		i.pos = j1;
	}
	i.span(i0).as_str()
}

fn hex_number(i: &mut Lex) -> Option<Number> {
	let i0 = i.pos;
	pat(i, "0x").or_else(|| pat(i, "0X"))?;
	let s = pat_mul(i, UnicodeXID::is_xid_continue);
	i.emit_result(i0, u64::from_str_radix(s, 16))
		.map(|val| Number { val, dec: None })
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Number {
	pub val: u64,
	pub dec: Option<u32>,
}

// 1 → Number(1, None)
// 1. → Number(1, Some(0))
// 11 → Number(11, None)
// 1.1 → Number(11, Some(1))
fn number(i: &mut Lex) -> Option<Number> {
	let start = i.pos;
	let int = pat_mul(i, |c| char::is_ascii_digit(&c));
	if int.is_empty() {
		return None
	}

	let (result, dec) = if pat(i, '.').is_some() {
		let frac = pat_mul(i, |c| char::is_ascii_digit(&c));
		if frac.is_empty() {
			(int.parse(), Some(0))
		} else {
			(format!("{int}{frac}").parse(), Some(frac.len() as u32))
		}
	} else {
		(int.parse(), None)
	};

	let val = i.emit_result(start, result).unwrap_or_default();
	Some(Number { val, dec })
}

fn ident<'a>(i: &mut Lex<'a>) -> Option<&'a str> {
	let j = i.pos;
	pat(i, |c| UnicodeXID::is_xid_start(c) || c == '_')?;
	i.pos = j;
	Some(pat_mul(i, UnicodeXID::is_xid_continue))
}

fn string(i: &mut Lex) -> Option<String> {
	pat(i, '"')?;
	let mut s = String::new();
	let i0 = i.pos;
	loop {
		let i1 = i.pos;
		match symbol(i) {
			None | Some('\n') => {
				i.emit(i0, "unterminated string");
				break;
			}
			Some('"') => break,
			Some('\\') => match symbol(i) {
				None | Some('\n') => {
					i.emit(i0, "unterminated string");
					break;
				}
				Some('"') => s.push('"'),
				Some('\\') => s.push('\\'),
				_ => i.emit(i1, "invalid escape sequence"),
			},
			Some(c) => s.push(c),
		}
	}
	Some(s)
}

fn text<'a>(i: &mut Lex<'a>) -> Option<&'a str> {
	pat(i, '{')?;
	let i0 = i.pos;
	let mut i1;
	loop {
		i1 = i.pos;
		match symbol(i) {
			None => {
				i.emit(i0, "unterminated text");
				break;
			}
			Some('}') => break,
			Some('{') => loop {
				match symbol(i) {
					None | Some('\n') => {
						i.emit(i1, "unterminated text directive");
						break;
					}
					Some('}') => break,
					_ => {}
				}
			},
			_ => {}
		}
	}
	Some(Span::from_prefix(i0, i1).as_str())
}

fn symbol(i: &mut Lex) -> Option<char> {
	pat(i, |_| true).map(|a| a.chars().next().unwrap())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenKind<'a> {
	Ident(&'a str),
	Number(Number),
	String(String),
	Text(&'a str),

	Colon,  // :
	Comma,  // ,
	LParen, // (
	RParen, // )
	LBrack, // [
	RBrack, // ]

	Plus,    // +
	Minus,   // -
	Star,    // *
	Slash,   // /
	Percent, // %
	Excl,    // !

	Eq,    // =
	Lt,    // <
	Gt,    // >
	Amp,   // &
	Pipe,  // |
	Caret, // ^
	Tilde, // ~
	Dot,   // .
	At,    // @

	Eof,
	Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'a> {
	pub trivia: &'a str,
	pub indent: Option<&'a str>,
	pub span: Span<'a>,
	pub token: TokenKind<'a>,
}

fn space<'a>(i: &mut Lex<'a>) -> &'a str {
	let i0 = i.pos;
	loop {
		if pat(i, "//").is_some() {
			pat_mul(i, |c| c != '\n');
		} else if pat_mul(i, [' ', '\t']).is_empty() {
			break
		}
	}
	i.span(i0).as_str()
}

fn trivia<'a>(i: &mut Lex<'a>) -> (&'a str, Option<&'a str>) {
	let i0 = i.pos;
	space(i);
	let mut indent = None;
	while pat(i, '\n').is_some() {
		indent = Some(space(i))
	}
	(i.span(i0).as_str(), indent)
}

#[allow(dead_code)]
pub fn token<'a>(i: &mut Lex<'a>) -> Token<'a> {
	let (trivia, indent) = trivia(i);
	let i0 = i.pos;
	if let Some(token) = None
		.or_else(|| ident(i).map(TokenKind::Ident))
		.or_else(|| hex_number(i).map(TokenKind::Number))
		.or_else(|| number(i).map(TokenKind::Number))
		.or_else(|| string(i).map(TokenKind::String))
		.or_else(|| text(i).map(TokenKind::Text))

		.or_else(|| pat(i, ':').map(|_| TokenKind::Colon))
		.or_else(|| pat(i, ',').map(|_| TokenKind::Comma))
		.or_else(|| pat(i, '(').map(|_| TokenKind::LParen))
		.or_else(|| pat(i, ')').map(|_| TokenKind::RParen))
		.or_else(|| pat(i, '[').map(|_| TokenKind::LBrack))
		.or_else(|| pat(i, ']').map(|_| TokenKind::RBrack))

		.or_else(|| pat(i, '+').map(|_| TokenKind::Plus))
		.or_else(|| pat(i, '-').map(|_| TokenKind::Minus))
		.or_else(|| pat(i, '*').map(|_| TokenKind::Star))
		.or_else(|| pat(i, '/').map(|_| TokenKind::Slash))
		.or_else(|| pat(i, '%').map(|_| TokenKind::Percent))
		.or_else(|| pat(i, '!').map(|_| TokenKind::Excl))

		.or_else(|| pat(i, '=').map(|_| TokenKind::Eq))
		.or_else(|| pat(i, '<').map(|_| TokenKind::Lt))
		.or_else(|| pat(i, '>').map(|_| TokenKind::Gt))
		.or_else(|| pat(i, '&').map(|_| TokenKind::Amp))
		.or_else(|| pat(i, '|').map(|_| TokenKind::Pipe))
		.or_else(|| pat(i, '^').map(|_| TokenKind::Caret))
		.or_else(|| pat(i, '~').map(|_| TokenKind::Tilde))
		.or_else(|| pat(i, '.').map(|_| TokenKind::Dot))
		.or_else(|| pat(i, '@').map(|_| TokenKind::At))
	{
		Token {
			trivia,
			indent,
			span: i.span(i0),
			token
		}
	} else if i.pos.is_empty() {
		// Eof tokens are always at a new line, but not indented.
		// This will make sure all indent blocks end as they should.
		let span = i.span(i0);
		Token {
			trivia,
			indent: Some(span.end().as_str()),
			span,
			token: TokenKind::Eof
		}
	} else {
		pat(i, |_| true);
		i.emit(i0, "unexpected token");
		Token {
			trivia,
			indent,
			span: i.span(i0),
			token: TokenKind::Error
		}
	}
}

pub fn tokens<'a>(i: &mut Lex<'a>) -> Vec<Token<'a>> {
	let mut tokens = Vec::new();
	loop {
		tokens.push(token(i));
		if tokens.last().unwrap().token == TokenKind::Eof {
			break
		}
	}
	tokens[0].indent = Some(&i.src[..0]);
	tokens
}

#[test]
fn test_hexnum() {
	println!("{:?}", hex_number(&mut Lex::new("abc")));
	println!("{:?}", hex_number(&mut Lex::new("0xa")));
	println!("{:?}", hex_number(&mut Lex::new("0xabD")));
	println!("{:?}", hex_number(&mut Lex::new("0x123456789abcdef12345789")));
	println!("{:?}", hex_number(&mut Lex::new("0Xabcq")));
	println!("{:?}", hex_number(&mut Lex::new("0Xabc q")));
}

#[test]
fn test_num() {
	println!("{:?}", number(&mut Lex::new("abc")));
	println!("{:?}", number(&mut Lex::new("1")));
	println!("{:?}", number(&mut Lex::new("11")));
	println!("{:?}", number(&mut Lex::new("1.0")));
	println!("{:?}", number(&mut Lex::new("1.")));
	println!("{:?}", number(&mut Lex::new(".1")));
	println!("{:?}", number(&mut Lex::new("123e")));
	println!("{:?}", number(&mut Lex::new("1.0")));
	println!("{:?}", number(&mut Lex::new("1.00")));
	println!("{:?}", number(&mut Lex::new("10.00")));
}

#[test]
fn main() {
	let src = include_str!("/tmp/kiseki/tc/t1121");
	let mut parse = Lex::new(src);
	loop {
		let t = token(&mut parse);
		println!("{:?} {:?} {:?} {:?}", t.span.position_in(src).unwrap(), t.trivia, t.indent, t.token);
		if let TokenKind::Eof = &t.token { break }
	}
	for e in &parse.errors {
		println!("{:?}", e);
	}
}
