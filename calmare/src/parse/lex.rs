use std::str::pattern::Pattern;
use crate::span::{Spanned, Span};
use super::diag::Diag;

use total_float::F64;
use unicode_xid::UnicodeXID;

// from https://github.com/rust-lang/rfcs/pull/2796
pub fn range_of<T>(slice: &[T], subslice: &[T]) -> Option<std::ops::Range<usize>> {
	let range = slice.as_ptr_range();
	let subrange = subslice.as_ptr_range();
	if subrange.start >= range.start && subrange.end <= range.end {
		unsafe {
			Some(std::ops::Range {
				start: subrange.start.offset_from(range.start) as usize,
				end: subrange.end.offset_from(range.start) as usize,
			})
		}
	} else {
		None
	}
}

#[derive(Debug, Clone, Copy, Eq)]
struct Indent<'a>(&'a str);

impl<'a, 'b> PartialEq<Indent<'b>> for Indent<'a> {
	fn eq(&self, other: &Indent<'b>) -> bool {
		self.0 == other.0
	}
}

impl<'a, 'b> PartialOrd<Indent<'b>> for Indent<'a> {
	fn partial_cmp(&self, other: &Indent<'b>) -> Option<std::cmp::Ordering> {
		use std::cmp::Ordering::*;
		let a = self.0;
		let b = other.0;
		if a == b {
			Some(Equal)
		} else if a.starts_with(b) {
			Some(Greater)
		} else if b.starts_with(a) {
			Some(Less)
		} else {
			None
		}
	}
}

impl<'a, 'b> PartialEq<Indent<'b>> for Option<Indent<'a>> {
	fn eq(&self, other: &Indent<'b>) -> bool {
		self.as_ref().map_or(false, |a| a == other)
	}
}

impl<'a, 'b> PartialOrd<Indent<'b>> for Option<Indent<'a>> {
	fn partial_cmp(&self, other: &Indent<'b>) -> Option<std::cmp::Ordering> {
		use std::cmp::Ordering::*;
		self.as_ref().map_or(Some(Greater), |a| a.partial_cmp(other))
	}
}

#[derive(Clone)]
struct Lex<'a> {
	src: &'a str,
	pos_: &'a str,
	last_indent: Option<Indent<'a>>,
}

impl<'a> Lex<'a> {
	fn new(src: &'a str) -> Self {
		let mut new = Lex {
			src,
			pos_: src,
			last_indent: None,
		};
		new.space();
		if new.last_indent.is_none() {
			new.last_indent = Some(Indent(""));
		}
		new
	}

	fn is_empty(&self) -> bool {
		self.pos_.is_empty()
	}

	// TODO track some whitespace-aware positions too:
	// - position before any space
	// - position before indentation (i.e. start of line)
	fn pos(&self) -> Span {
		let r = range_of(self.src.as_bytes(), self.pos_.as_bytes()).unwrap();
		Span::new_at(r.start)
	}

	fn pat_<P: Pattern<'a> + 'a>(&mut self, p: P) -> Option<Span> {
		let i0 = self.pos();
		self.pos_ = self.pos_.strip_prefix(p)?;
		self.last_indent = None;
		Some(i0 | self.pos())
	}

	fn pat<P: Pattern<'a> + 'a>(&mut self, p: P) -> Option<&'a str> {
		self.pat_(p).map(|s| self.span_text(s))
	}

	fn pat_mul<P: Pattern<'a> + Clone + 'a>(&mut self, p: P) -> &'a str {
		let mut s = self.pos();
		while let Some(s1) = self.pat_(p.clone()) {
			s |= s1;
		}
		self.span_text(s)
	}

	fn any(&mut self) -> Option<char> {
		self.pat(|_| true).map(|a| a.chars().next().unwrap())
	}

	fn span_text(&self, s: Span) -> &'a str {
		&self.src[s.as_range()]
	}

	fn space(&mut self) -> Option<Indent<'a>> {
		if self.last_indent.is_none() {
			self.space_inner();
			while self.pat('\n').is_some() {
				self.last_indent = Some(self.space_inner())
			}
			if self.is_empty() {
				self.last_indent = Some(Indent(""))
			}
		}
		self.last_indent
	}

	fn space_inner(&mut self) -> Indent<'a> {
		let i0 = self.pos();
		self.pat_mul([' ', '\t']);
		if self.pat("//").is_some() {
			self.pat_mul(|c| c != '\n');
		}
		Indent(self.span_text(i0 | self.pos()))
	}

	fn clear_space(&mut self) {
		self.last_indent = None;
	}
}

enum Number {
	Int(u64),
	Float(F64),
}

fn number(i: &mut Lex) -> Option<Number> {
	i.clone().pat(|c| char::is_ascii_digit(&c))?;
	let i0 = i.pos();

	if i.pat("0x").is_some() {
		let s = i.pat_mul(UnicodeXID::is_xid_continue);
		match u64::from_str_radix(s, 16) {
			Ok(v) => return Some(Number::Int(v)),
			Err(e) => {
				Diag::error(i0 | i.pos(), e).emit();
				return Some(Number::Int(0))
			}
		}
	} else {
		i.pat_mul(|a| char::is_ascii_digit(&a));
		if i.pat('.').is_some() {
			i.pat_mul(|a| char::is_ascii_digit(&a));
			let s = i.span_text(i0|i.pos());
			match s.parse::<f64>() {
				Ok(v) => return Some(Number::Float(F64(v))),
				Err(e) => {
					Diag::error(i0 | i.pos(), e).emit();
					return Some(Number::Float(F64(0.)))
				}
			}
		} else {
			let s = i.span_text(i0|i.pos());
			match s.parse::<u64>() {
				Ok(v) => return Some(Number::Int(v)),
				Err(e) => {
					Diag::error(i0 | i.pos(), e).emit();
					return Some(Number::Int(0))
				}
			}
		}
	}
}

fn ident<'a>(i: &mut Lex<'a>) -> Option<&'a str> {
	i.clone().pat(|c| UnicodeXID::is_xid_start(c) || c == '_')?;
	Some(i.pat_mul(UnicodeXID::is_xid_continue))
}

fn string(i: &mut Lex) -> Option<String> {
	let i0 = i.pos();
	i.pat('"')?;
	let mut s = String::new();
	loop {
		let i1 = i.pos();
		match i.any() {
			None | Some('\n') => {
				Diag::error(i0 | i.pos(), "unterminated string").emit();
				break;
			}
			Some('"') => break,
			Some('\\') => match i.any() {
				None | Some('\n') => {
					Diag::error(i0 | i.pos(), "unterminated string").emit();
					break;
				}
				Some('"') => s.push('"'),
				Some('\\') => s.push('\\'),
				_ => Diag::error(i1 | i.pos(), "invalid escape sequence").emit(),
			},
			Some(c) => s.push(c),
		}
	}
	Some(s)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line<'a> {
	pub span: Span,
	pub head: Vec<Spanned<Token<'a>>>,
	pub eol: Span, // points either to the colon or an empty span at the end of the line
	pub body: Option<Vec<Line<'a>>>,
}

impl Line<'_> {
	pub fn head_span(&self) -> Span {
		if let Some(a) = self.head.first() {
			a.0 | self.eol
		} else {
			self.eol
		}
	}
}

#[derive(Clone, PartialEq, Eq)]
pub struct Delimited<T> {
	pub open: Span,
	pub tokens: Vec<Spanned<T>>,
	pub close: Span,
}

impl<T: std::fmt::Debug> std::fmt::Debug for Delimited<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if f.alternate() {
			self.open.fmt(f)?;
		}
		f.write_str("@")?;
		self.tokens.fmt(f)?;
		f.write_str("@")?;
		if f.alternate() {
			self.close.fmt(f)?;
		}
		Ok(())
	}
}

#[derive(Clone, PartialEq, Eq)]
pub enum Token<'a> {
	Ident(&'a str),
	Insn(&'a str),
	Int(u64),
	Float(F64),
	String(String),
	Var(&'a str),

	Paren(Delimited<Token<'a>>),
	Bracket(Delimited<Token<'a>>),
	Brace(Delimited<TextToken<'a>>),

	Comma,   // ,
	Dot,     // .
	At,      // @

	Plus,    // +
	Minus,   // -
	Star,    // *
	Slash,   // /
	Percent, // %
	Excl,    // !
	Eq,      // =
	Lt,      // <
	Gt,      // >
	Amp,     // &
	Pipe,    // |
	Caret,   // ^
	Tilde,   // ~
}

impl std::fmt::Debug for Token<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Ident(v)   => { write!(f, "Ident(")?;   v.fmt(f)?; write!(f, ")") }
			Self::Insn(v)    => { write!(f, "Insn(")?;    v.fmt(f)?; write!(f, ")") }
			Self::Int(v)     => { write!(f, "Int(")?;     v.fmt(f)?; write!(f, ")") }
			Self::Float(v)   => { write!(f, "Float(")?;   v.fmt(f)?; write!(f, ")") }
			Self::String(v)  => { write!(f, "String(")?;  v.fmt(f)?; write!(f, ")") }
			Self::Var(v)     => { write!(f, "Var(")?;     v.fmt(f)?; write!(f, ")") }
			Self::Paren(v)   => { write!(f, "Paren(")?;   v.fmt(f)?; write!(f, ")") }
			Self::Bracket(v) => { write!(f, "Bracket(")?; v.fmt(f)?; write!(f, ")") }
			Self::Brace(v)   => { write!(f, "Brace(")?;   v.fmt(f)?; write!(f, ")") }
			Self::Comma   => write!(f, "Comma"),
			Self::Dot     => write!(f, "Dot"),
			Self::At      => write!(f, "At"),
			Self::Plus    => write!(f, "Plus"),
			Self::Minus   => write!(f, "Minus"),
			Self::Star    => write!(f, "Star"),
			Self::Slash   => write!(f, "Slash"),
			Self::Percent => write!(f, "Percent"),
			Self::Excl    => write!(f, "Excl"),
			Self::Eq      => write!(f, "Eq"),
			Self::Lt      => write!(f, "Lt"),
			Self::Gt      => write!(f, "Gt"),
			Self::Amp     => write!(f, "Amp"),
			Self::Pipe    => write!(f, "Pipe"),
			Self::Caret   => write!(f, "Caret"),
			Self::Tilde   => write!(f, "Tilde"),
		}
	}
}

#[derive(Clone, PartialEq, Eq)]
pub enum TextToken<'a> {
	Text(String),
	// NISA's ed7 have two newlines (01 and 0D), which here are differentiated by a backslash.
	Newline(bool),
	Hex(u8),
	Brace(Delimited<Token<'a>>),
}

fn tokens<'a>(indent: Indent, i: &mut Lex<'a>) -> Option<Vec<Spanned<Token<'a>>>> {
	let mut out = Vec::new();

	macro test($e:expr => $f:expr) {
		let i0 = i.pos();
		let x = $e;
		if let Some(x) = x {
			let f = $f;
			out.push(Spanned(i0 | i.pos(), f(x)));
			continue
		} else if i.pos() != i0 {
			return None
		}
	}

	i.clear_space();
	while continue_line(indent, i) {
		test!(ident(i) => |s: &'a str| {
			if s.chars().next().unwrap().is_lowercase() {
				Token::Ident(s)
			} else {
				Token::Insn(s)
			}
		});
		test!(number(i) => |s| match s {
			Number::Int(x) => Token::Int(x),
			Number::Float(x) => Token::Float(x),
		});
		test!(string(i) => Token::String);

		test!(i.pat('$').and_then(|_| ident(i)) => Token::Var);

		test!(delim(indent, i, '(', tokens, ')', "parenthesis") => Token::Paren);
		test!(delim(indent, i, '[', tokens, ']', "bracket") => Token::Bracket);
		test!(delim(indent, i, '{', text_tokens, '}', "brace") => Token::Brace);

		test!(i.pat(',') => |_| Token::Comma);
		test!(i.pat('.') => |_| Token::Dot);
		test!(i.pat('@') => |_| Token::At);

		test!(i.pat('+') => |_| Token::Plus);
		test!(i.pat('-') => |_| Token::Minus);
		test!(i.pat('*') => |_| Token::Star);
		test!(i.pat('/') => |_| Token::Slash);
		test!(i.pat('%') => |_| Token::Percent);
		test!(i.pat('!') => |_| Token::Excl);

		test!(i.pat('=') => |_| Token::Eq);
		test!(i.pat('<') => |_| Token::Lt);
		test!(i.pat('>') => |_| Token::Gt);
		test!(i.pat('&') => |_| Token::Amp);
		test!(i.pat('|') => |_| Token::Pipe);
		test!(i.pat('^') => |_| Token::Caret);
		test!(i.pat('~') => |_| Token::Tilde);

		break
	}
	Some(out)
}

impl std::fmt::Debug for TextToken<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Text(v)    => { write!(f, "Text(")?;    v.fmt(f)?; write!(f, ")") }
			Self::Newline(v) => { write!(f, "Newline(")?; v.fmt(f)?; write!(f, ")") }
			Self::Hex(v)     => { write!(f, "Hex(")?;     v.fmt(f)?; write!(f, ")") }
			Self::Brace(v)   => { write!(f, "Brace(")?;   v.fmt(f)?; write!(f, ")") }
		}
	}
}

fn text_tokens<'a>(indent: Indent, i: &mut Lex<'a>) -> Option<Vec<Spanned<TextToken<'a>>>> {
	let mut out = Vec::new();

	i.pat_mul([' ', '\t']);
	i.pat('\n')?;
	i.last_indent = Some(Indent(i.pat_mul([' ', '\t'])));

	let mut i0 = i.pos();
	let mut s = String::new();

	macro push {
		() => {
			if !s.is_empty() {
				out.push(Spanned(i0 | i.pos(), TextToken::Text(s)));
				#[allow(unused_assignments)]
				{
					s = String::new();
					i0 = i.pos();
				}
			}
		},
		($($tt:tt)*) => {
			push!();
			let i1 = i.pos();
			let v = { $($tt)* };
			out.push(Spanned(i1 | i.pos(), v));
			i0 = i.pos();
		}
	}

	while i.last_indent > indent {
		if i.clone().pat('}').is_some() {
			break
		}

		if i.clone().pat('{').is_some() {
			push! {
				let body = delim(Indent("-"), i, '{', tokens, '}', "brace")?;
				TextToken::Brace(body)
			};
		}

		if i.clone().pat_('\n').is_some() {
			push! {
				i.pat('\n');
				TextToken::Newline(false)
			};
			i.last_indent = Some(Indent(i.pat_mul([' ', '\t'])));
		}

		if i.clone().pat_('\\').is_some() {
			let i1 = i.pos();
			i.pat('\\');

			if let Some(s2) = i.pat(['\\', '{', '}']) {
				s.push_str(s2)
			} else if i.pat('\n').is_some() {
				push!();
				out.push(Spanned(i1 | i.pos(), TextToken::Newline(true)));
				i0 = i.pos();
			} else {
				Diag::error(i1 | i.pos(), "invalid escape").emit();
			}
		}

		s.push_str(i.pat_mul(|a| !"{}\\\n".contains(a)));
	}
	push!();
	Some(out)
}

fn delim<'a, T: 'a>(
	indent: Indent,
	i: &mut Lex<'a>,
	c1: char,
	tokens: fn(Indent, &mut Lex<'a>) -> Option<Vec<Spanned<T>>>,
	c2: char,
	name: &str,
) -> Option<Delimited<T>> {
	let i0 = i.pos();
	i.pat(c1)?;
	let open = i0 | i.pos();

	let tokens = tokens(indent, i)?;

	let i0 = i.pos();
	#[allow(clippy::neg_cmp_op_on_partial_ord)]
	if !(i.space() >= indent) && !i.is_empty() {
		Diag::error(i0, "invalid indentation")
			.note(open, format_args!("inside {name} opened here"))
			.emit();
		return None
	} else if i.pat(c2).is_none() {
		Diag::error(i0, format_args!("expected closing {name}"))
			.note(open, "opened here")
			.emit();
		return None
	}
	let close = i0 | i.pos();
	Some(Delimited {
		open,
		tokens,
		close,
	})
}

fn continue_line(indent: Indent, i: &mut Lex) -> bool {
	let ind = i.space();
	ind > indent || ind == indent && i.clone().pat(['{', ')', ']', '}']).is_some()
}

fn line<'a>(indent: Indent, i: &mut Lex<'a>) -> Line<'a> {
	let i0 = i.pos();
	let head = tokens(indent, i).unwrap_or_default();

	let (eol, body) = if i.space() <= indent || i.is_empty() {
		let eol = head.last().unwrap().0.at_end();
		(eol, None)
	} else {
		let i1 = i.pos();
		while continue_line(indent, i) && i.clone().pat(':').is_none() {
			i.pat_mul(|a| a != '\n' && a != ':');
		}
		if i1 != i.pos() {
			Diag::error(i1 | i.pos(), "unexpected character").emit();
		}

		if let Some(eol) = i.pat_(':') {
			let ind = i.space();
			#[allow(clippy::unnecessary_unwrap)]
			let body = if ind.is_none() {
				vec![line(indent, i)]
			} else if ind > indent {
				lines(ind.unwrap(), i)
			} else {
				Vec::new()
			};
			(eol, Some(body))
		} else {
			(i1, None)
		}
	};
	Line { span: i0 | i.pos(), head, eol, body }
}

fn lines<'a>(indent: Indent, i: &mut Lex<'a>) -> Vec<Line<'a>> {
	let mut indent_pos = i.pos();
	let mut lines = Vec::new();
	while {i.space(); !i.is_empty()} {
		let ind = i.space();
		ind.expect("lines() can only be called at start of line");
		if ind > indent {
			// This error can only happen at the start of the file, or if a line terminates one block but is not consistent with the next.
			// In the latter case it would probably be better to point to the last line of the preceding block, not here.
			Diag::error(i.pos(), "unexpected indent")
				.note(indent_pos, "should perhaps be equal to here")
				.emit();
		} else if ind == indent {
			indent_pos = i.pos(); // print reference indent closer to where the error is shown
		}

		if ind >= indent {
			lines.push(line(ind.unwrap(), i))
		} else {
			break
		}
	}
	lines
}

pub fn lex(src: &str) -> Vec<Line> {
	let mut i = Lex::new(src);
	let v = lines(Indent(""), &mut i);
	assert!(i.is_empty());
	v
}

#[test]
fn main() {
	let src = include_str!("/tmp/kiseki/tc/t1121");
	let (ast, diag) = super::diag::diagnose(|| lex(src));
	println!("{:#?}", ast);
	super::diag::print_diags("<input>", src, &diag);
}
