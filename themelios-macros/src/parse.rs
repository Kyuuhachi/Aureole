use proc_macro2::TokenStream;
use syn::{
	*,
	parse::{ParseStream, Parse},
	spanned::Spanned,
	punctuated::*,
};
use quote::TokenStreamExt;

pub mod kw {
	syn::custom_keyword!(via);
	syn::custom_keyword!(skip);
	syn::custom_keyword!(custom);
	syn::custom_keyword!(def);
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
	pub attrs: TopAttributes,
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
#[parse(prefix = Attribute::parse_outer)]
pub enum Def {
	#[parse(peek_func = |i| i.peek(kw::skip) && i.peek2(Token![!]))]
	Skip(DefSkip),
	#[parse(peek_func = |i| i.peek(kw::custom) && i.peek2(Token![!]))]
	Custom(DefCustom),
	#[parse(peek_func = |i| i.peek(kw::def) && i.peek2(Token![!]))]
	Def(DefDef),
	Standard(DefStandard),
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct DefStandard {
	pub attrs: DefAttributes,
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
	Standard(Source),
}

#[derive(Clone, Debug, syn_derive::ToTokens)]
pub enum Source {
	Simple(SourceSimple),
	Const(SourceConst),
	Cast(SourceCast),
	Split(SourceSplit),
}

impl Parse for Source {
	fn parse(input: ParseStream) -> Result<Self> {
		let mut source = if input.peek(token::Brace) {
			Source::Split(input.parse()?)
		} else if input.peek(Token![const]) {
			Source::Const(input.parse()?)
		} else {
			Source::Simple(input.parse()?)
		};
		while input.peek(Token![as]) {
			source = Source::Cast(SourceCast {
				source: Box::new(source),
				as_token: input.parse()?,
				ty: input.parse()?,
			})
		}
		Ok(source)
	}
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct SourceSimple {
	pub ty: Box<Type>,
	#[parse(|input| parse_if(input, |input| input.peek(kw::via)))]
	pub via: Option<Via>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct Via {
	pub via_token: kw::via,
	pub path: Path,
}

#[derive(Clone, Debug, syn_derive::ToTokens)]
pub struct SourceCast {
	pub source: Box<Source>,
	pub as_token: Token![as],
	pub ty: Box<Type>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct SourceConst {
	pub const_token: Token![const],
	pub lit: LitInt,
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
pub struct SourceSplit {
	#[syn(braced)]
	pub brace_token: token::Brace,
	#[syn(in = brace_token)]
	#[parse(Punctuated::parse_terminated)]
	pub arms: Punctuated<SplitArm, Token![,]>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct SplitArm {
	#[parse(multi_pat_with_leading_vert)]
	pub pat: Pat,
	#[parse(|input| parse_if(input, |input| input.peek(Token![if])))]
	pub guard: Option<Guard>,
	pub fat_arrow_token: Token![=>],
	pub source: Source,
}

// Copied from syn::pat::parsing
pub fn multi_pat_with_leading_vert(input: ParseStream) -> Result<Pat> {
	let leading_vert: Option<Token![|]> = input.parse()?;
	multi_pat_impl(input, leading_vert)
}

fn multi_pat_impl(input: ParseStream, leading_vert: Option<Token![|]>) -> Result<Pat> {
	let mut pat: Pat = input.parse()?;
	if leading_vert.is_some()
		|| input.peek(Token![|]) && !input.peek(Token![||]) && !input.peek(Token![|=])
	{
		let mut cases = Punctuated::new();
		cases.push_value(pat);
		while input.peek(Token![|]) && !input.peek(Token![||]) && !input.peek(Token![|=]) {
			let punct = input.parse()?;
			cases.push_punct(punct);
			let pat: Pat = input.parse()?;
			cases.push_value(pat);
		}
		pat = Pat::Or(PatOr {
			attrs: Vec::new(),
			leading_vert,
			cases,
		});
	}
	Ok(pat)
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct Guard {
	pub if_token: Token![if],
	pub expr: Box<Expr>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct DefSkip {
	pub attrs: DefAttributes,
	pub skip_token: kw::skip,
	pub bang_token: Token![!],
	#[syn(parenthesized)]
	pub paren_token: token::Paren,
	#[syn(in = paren_token)]
	pub count: LitInt,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct DefCustom {
	pub attrs: DefAttributes,
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
	#[parse(|p| {
		let path = p.fork().parse::<Path>()?;
		path.get_ident().cloned().ok_or_else(|| Error::new(path.span(), "must be ident"))
	})]
	pub ident: Ident,
	pub pat: Box<Pat>,
	pub arrow_token: Token![=>],
	pub expr: Box<Expr>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct DefDef {
	pub attrs: Attributes,
	pub def_token: kw::def,
	pub bang_token: Token![!],
	pub ident: Ident,
	#[syn(parenthesized)]
	pub paren_token: token::Paren,
	#[syn(in = paren_token)]
	#[parse(Punctuated::parse_terminated)]
	pub args: Punctuated<Box<Type>, Token![,]>,
}

#[derive(Clone, Debug, syn_derive::ToTokens)]
pub struct TopAttributes {
	pub games: GamesAttr,
	pub other: Attributes,
}

impl Parse for TopAttributes {
	fn parse(input: ParseStream) -> Result<Self> {
		let mut attrs = input.parse::<Attributes>()?;
		Ok(TopAttributes {
			games: pop_attr(&mut attrs, "games")
				.ok_or_else(|| Error::new(input.span(), "no #[games]"))?
				.parse_args()?,
			other: attrs,
		})
	}
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct GamesAttr {
	pub expr: Box<Expr>,
	pub arrow_token: Token![=>],
	#[parse(|input| {
		let mut ty = TokenStream::new();
		#[allow(clippy::nonminimal_bool)] // it's more readable this way
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

#[derive(Clone, Debug, syn_derive::ToTokens)]
pub struct DefAttributes {
	pub game: Option<GameAttr>,
	pub other: Attributes,
}

impl Parse for DefAttributes {
	fn parse(input: ParseStream) -> Result<Self> {
		let mut attrs = input.parse::<Attributes>()?;
		Ok(DefAttributes {
			game: pop_attr(&mut attrs, "game")
				.map(|a| a.parse_args())
				.transpose()?,
			other: attrs,
		})
	}
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
pub struct GameAttr {
	#[parse(Punctuated::parse_terminated)]
	pub idents: Punctuated<Ident, Token![,]>,
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

fn parse_if<T: Parse>(input: ParseStream, cond: fn(ParseStream) -> bool) -> Result<Option<T>> {
	if cond(input) {
		Ok(Some(input.parse()?))
	} else {
		Ok(None)
	}
}

fn pop_attr(attrs: &mut Vec<Attribute>, name: &str) -> Option<Attribute> {
	attrs.iter().position(|a| a.path.is_ident(name)).map(|i| attrs.remove(i))
}
