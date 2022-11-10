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
#[derive(Clone, Debug, syn_derive::ToTokens)]
pub struct InsnArm {
	pub name: Ident,
	#[syn(parenthesized)]
	pub paren_token: token::Paren,
	#[syn(in = paren_token)]
	pub args: Punctuated<Arg, Token![,]>,
	#[syn(in = paren_token)]
	pub tail: Option<Table>,
}

impl Parse for InsnArm {
	fn parse(input: ParseStream) -> Result<Self> {
		let content;
		Ok(Self {
			name: input.parse()?,
			paren_token: parenthesized!(content in input),
			args: {
				let mut punctuated = Punctuated::new();
				loop {
					if content.is_empty() {
						break;
					}
					if content.peek(Token![match]) {
						break;
					}
					let value = content.parse()?;
					punctuated.push_value(value);
					if content.is_empty() {
						break;
					}
					let punct = content.parse()?;
					punctuated.push_punct(punct);
				}

				punctuated
			},
			tail: parse_if(&content, |content| content.peek(Token![match]))?,
		})
	}
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct Arg {
	pub source: Source,
	#[parse(|input| parse_while(input, |input| input.peek(Token![as])))]
	#[to_tokens(|tokens, val| tokens.append_all(val))]
	pub ty: Vec<ArgTy>,
	#[parse(|input| parse_if(input, |input| input.peek(kw::alias)))]
	#[to_tokens(|tokens, val| tokens.append_all(val))]
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
pub struct Table {
	pub match_token: Token![match],
	#[syn(braced)]
	pub brace_token: token::Brace,
	#[syn(in = brace_token)]
	#[parse(Punctuated::parse_terminated)]
	pub arms: Punctuated<TableArm, Token![,]>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct TableArm {
	#[parse(Attribute::parse_outer)]
	#[to_tokens(|tokens, val| tokens.append_all(val))]
	pub attrs: Vec<Attribute>,
	pub key: LitInt,
	pub arrow_token: Token![=>],
	pub insn: InsnArm,
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
