use proc_macro2::TokenStream;
use syn::{
	*,
	parse::{ParseStream, Parse},
	punctuated::*,
};
use quote::TokenStreamExt;

pub mod kw {
	syn::custom_keyword!(alias);
	syn::custom_keyword!(skip);
	syn::custom_keyword!(custom);
	syn::custom_keyword!(read);
	syn::custom_keyword!(write);
}

// {{{1 AST and parse

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct Top {
	#[syn(parenthesized)]
	pub paren_token: syn::token::Paren,
	#[syn(in = paren_token)]
	#[parse(|input| Punctuated::parse_terminated_with(input, |input| {
		Ok(PatType {
			attrs: Attribute::parse_outer(input)?,
			pat: input.parse()?,
			colon_token: input.parse()?,
			ty: input.parse()?,
		})
	}))]
	pub args: Punctuated<PatType, Token![,]>,
	pub attrs: Attributes,
	#[syn(bracketed)]
	pub bracket_token: syn::token::Bracket,
	#[syn(in = bracket_token)]
	#[parse(Punctuated::parse_terminated)]
	pub defs: Punctuated<Def, Token![,]>,

	#[syn(in = bracket_token)]
	#[parse(|input| Ok(input.span()))]
	#[to_tokens(|_, _| {})]
	pub end_span: proc_macro2::Span,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct GamesAttr {
	pub expr: Box<Expr>,
	pub arrow_token: Token![=>],
	#[parse(|input| {
		let mut ty = TokenStream::new();
		while !input.is_empty() && !(input.peek(Token![::]) && input.peek3(token::Brace)) {
			ty.extend(input.parse::<proc_macro2::TokenTree>())
		}
		Ok(parse_quote! { #ty })
	})]
	pub ty: Box<Type>,
	pub colon2_token: Token![::],
	#[syn(braced)]
	pub brace_token: syn::token::Brace,
	#[syn(in = brace_token)]
	#[parse(Punctuated::parse_terminated)]
	pub idents: Punctuated<Ident, Token![,]>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct GameAttr {
	#[parse(Punctuated::parse_terminated)]
	pub idents: Punctuated<Ident, Token![,]>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
#[parse(prefix = Attribute::parse_outer)]
pub enum Def {
	#[parse(peek_func = |i| i.peek(kw::skip) && i.peek2(Token![!]))]
	Skip(DefSkip),
	#[parse(peek_func = |i| i.peek(kw::custom) && i.peek2(Token![!]))]
	Custom(DefCustom),
	Standard(DefStandard),
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct DefStandard {
	pub attrs: Attributes,
	pub ident: Ident,
	#[syn(parenthesized)]
	pub paren_token: token::Paren,
	#[syn(in = paren_token)]
	#[parse(Punctuated::parse_terminated)]
	pub args: Punctuated<Arg, Token![,]>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub enum Arg {
	#[parse(peek = Token![match])]
	Tail(ArgTail),
	#[parse(peek = token::Brace)]
	Split(ArgSplit),
	Standard(ArgStandard),
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct ArgStandard {
	pub source: Source,
	#[parse(|input| parse_while(input, |input| input.peek(Token![as])))]
	#[to_tokens(|tokens, val| tokens.append_all(val))]
	pub ty: Vec<ArgTy>,
	#[parse(|input| parse_if(input, |input| input.peek(kw::alias)))]
	pub alias: Option<ArgAlias>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct ArgTy {
	pub as_token: Token![as],
	pub ty: Box<Type>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct ArgAlias {
	pub alias_token: kw::alias,
	pub ident: Ident,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub enum Source {
	#[parse(peek_func = |i| i.peek2(token::Paren))]
	Call(SourceCall),
	Simple(Ident),
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct SourceCall {
	pub name: Ident,
	#[syn(parenthesized)]
	pub paren_token: token::Paren,
	#[syn(in = paren_token)]
	#[parse(Punctuated::parse_terminated)]
	pub args: Punctuated<Box<Expr>, Token![,]>,
	pub arrow_token: Token![->],
	pub ty: Box<Type>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct ArgTail {
	pub match_token: Token![match],
	#[syn(braced)]
	pub brace_token: token::Brace,
	#[syn(in = brace_token)]
	#[parse(Punctuated::parse_terminated)]
	pub arms: Punctuated<TailArm, Token![,]>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct TailArm {
	pub attrs: Attributes,
	pub key: LitInt,
	pub arrow_token: Token![=>],
	// This does accidentally allow attrs here, unfortunately.
	pub def: DefStandard,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct ArgSplit {
	#[syn(braced)]
	pub brace_token: token::Brace,
	#[syn(in = brace_token)]
	#[parse(Punctuated::parse_terminated)]
	pub arms: Punctuated<SplitArm, Token![,]>,
	pub alias: ArgAlias,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct SplitArm {
	pub pat: Pat,
	pub arrow_token: Token![=>],
	pub source: Source,
	#[parse(|input| parse_while(input, |input| input.peek(Token![as])))]
	#[to_tokens(|tokens, val| tokens.append_all(val))]
	pub ty: Vec<ArgTy>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct DefSkip {
	pub attrs: Attributes,
	pub skip_token: kw::skip,
	pub bang_token: Token![!],
	#[syn(parenthesized)]
	pub paren_token: token::Paren,
	#[syn(in = paren_token)]
	pub count: LitInt,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct DefCustom {
	pub attrs: Attributes,
	pub custom_token: kw::custom,
	pub bang_token: Token![!],
	#[syn(braced)]
	pub brace_token: token::Brace,
	#[syn(in = brace_token)]
	#[parse(Punctuated::parse_terminated)]
	pub clauses: Punctuated<Clause, Token![,]>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub enum Clause {
	#[parse(peek = kw::read)]
	Read(ClauseRead),
	#[parse(peek = kw::write)]
	Write(ClauseWrite),
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct ClauseRead {
	pub read_token: kw::read,
	pub arrow_token: Token![=>],
	pub expr: Box<Expr>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct ClauseWrite {
	pub write_token: kw::write,
	pub ident: Ident,
	#[syn(parenthesized)]
	pub paren_token: token::Paren,
	#[syn(in = paren_token)]
	#[parse(Punctuated::parse_terminated)]
	pub args: Punctuated<Ident, Token![,]>,
	pub arrow_token: Token![=>],
	pub expr: Box<Expr>,
}

// Utils

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct Attributes(
	#[parse(Attribute::parse_outer)]
	#[to_tokens(|tokens, val| tokens.append_all(val))]
	Vec<Attribute>
);

impl std::ops::Deref for Attributes {
	type Target = Vec<Attribute>;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl std::ops::DerefMut for Attributes {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

fn parse_while<T: Parse>(input: ParseStream, cond: fn(ParseStream) -> bool) -> Result<Vec<T>> {
	let mut xs = Vec::new();
	while cond(input) {
		xs.push(input.parse()?)
	}
	Ok(xs)
}

fn parse_if<T: Parse>(input: ParseStream, cond: fn(ParseStream) -> bool) -> Result<Option<T>> {
	if cond(input) {
		Ok(Some(input.parse()?))
	} else {
		Ok(None)
	}
}
