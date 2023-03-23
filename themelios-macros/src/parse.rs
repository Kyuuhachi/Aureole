use proc_macro2::TokenStream;
use syn::{
	Token,
	Ident,
	token::{Paren, Brace, Bracket},
	parse::{ParseStream, Parse},
	spanned::Spanned,
	punctuated::Punctuated,
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

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct Top {
	#[syn(parenthesized)]
	pub paren_token: Paren,
	#[syn(in = paren_token)]
	#[parse(|input| Punctuated::parse_terminated_with(input, |input| {
		Ok(syn::PatType {
			attrs: syn::Attribute::parse_outer(input)?,
			pat: Box::new(syn::Pat::parse_single(input)?),
			colon_token: input.parse()?,
			ty: input.parse()?,
		})
	}))]
	pub args: Punctuated<syn::PatType, Token![,]>,
	pub attrs: TopAttributes,
	#[syn(bracketed)]
	pub bracket_token: Bracket,
	#[syn(in = bracket_token)]
	#[parse(Punctuated::parse_terminated)]
	pub defs: Punctuated<Def, Token![,]>,

	#[syn(in = bracket_token)]
	#[parse(|input| Ok(input.span()))]
	#[to_tokens(|_, _| {})]
	pub end_span: proc_macro2::Span,
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
#[parse(prefix = syn::Attribute::parse_outer)]
pub enum Def {
	#[parse(peek_func = |i| i.peek(kw::skip) && i.peek2(Token![!]))]
	Skip(DefSkip),
	#[parse(peek_func = |i| i.peek(kw::custom) && i.peek2(Token![!]))]
	Custom(DefCustom),
	#[parse(peek_func = |i| i.peek(kw::def) && i.peek2(Token![!]))]
	Def(DefDef),
	Standard(DefStandard),
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct DefStandard {
	pub attrs: DefAttributes,
	pub ident: Ident,
	#[syn(parenthesized)]
	pub paren_token: Paren,
	#[syn(in = paren_token)]
	#[parse(Punctuated::parse_terminated)]
	pub args: Punctuated<Arg, Token![,]>,
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub enum Arg {
	#[parse(peek = Token![match])]
	Match(Match),
	Source(Source),
}

#[derive(Clone, syn_derive::ToTokens)]
pub enum Source {
	Simple(SourceSimple),
	Const(SourceConst),
	Cast(SourceCast),
	Block(SourceBlock),
	If(SourceIf),
}

impl Parse for Source {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let mut source = if input.peek(Token![if]) {
			Source::If(input.parse()?)
		} else if input.peek(Token![const]) {
			Source::Const(input.parse()?)
		} else if input.peek(Brace) {
			Source::Block(input.parse()?)
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

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct SourceBlock {
	#[syn(braced)]
	pub brace_token: Brace,
	#[syn(in = brace_token)]
	pub source: Box<Source>,
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct SourceSimple {
	pub ty: Box<syn::Type>,
	#[parse(|input| parse_if(input, |input| input.peek(kw::via)))]
	pub via: Option<Via>,
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct Via {
	pub via_token: kw::via,
	pub path: syn::Path,
}

#[derive(Clone, syn_derive::ToTokens)]
pub struct SourceCast {
	pub source: Box<Source>,
	pub as_token: Token![as],
	pub ty: Box<syn::Type>,
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct SourceConst {
	pub const_token: Token![const],
	pub lit: syn::LitInt,
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct Match {
	pub match_token: Token![match],
	#[syn(braced)]
	pub brace_token: Brace,
	#[syn(in = brace_token)]
	#[parse(Punctuated::parse_terminated)]
	pub arms: Punctuated<TailArm, Token![,]>,
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct TailArm {
	pub attrs: Attributes,
	pub key: syn::LitInt,
	pub arrow_token: Token![=>],
	// This does accidentally allow attrs here, unfortunately.
	pub def: DefStandard,
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct SourceIf {
	pub if_token: Token![if],
	#[parse(syn::Expr::parse_without_eager_brace, boxed)]
	pub cond: Box<syn::Expr>,
	#[parse(|input| Ok(Source::Block(input.parse()?)), boxed)]
	pub then_branch: Box<Source>,
	pub else_token: Token![else],
	#[parse(else_branch)]
	pub else_branch: Box<Source>,
}

fn else_branch(input: ParseStream) -> syn::Result<Box<Source>> {
	let lookahead = input.lookahead1();
	let else_branch = if input.peek(Token![if]) {
		Source::If(input.parse()?)
	} else if input.peek(Brace) {
		Source::Block(input.parse()?)
	} else {
		return Err(lookahead.error());
	};
	Ok(Box::new(else_branch))
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct SplitArm {
	#[parse(syn::Pat::parse_multi_with_leading_vert, boxed)]
	pub pat: Box<syn::Pat>,
	#[parse(|input| parse_if(input, |input| input.peek(Token![if])))]
	pub guard: Option<Guard>,
	pub fat_arrow_token: Token![=>],
	pub source: Source,
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct Guard {
	pub if_token: Token![if],
	pub expr: Box<syn::Expr>,
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct DefSkip {
	pub attrs: DefAttributes,
	pub skip_token: kw::skip,
	pub bang_token: Token![!],
	#[syn(parenthesized)]
	pub paren_token: Paren,
	#[syn(in = paren_token)]
	pub count: syn::LitInt,
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct DefCustom {
	pub attrs: DefAttributes,
	pub custom_token: kw::custom,
	pub bang_token: Token![!],
	#[syn(braced)]
	pub brace_token: Brace,
	#[syn(in = brace_token)]
	#[parse(Punctuated::parse_terminated)]
	pub clauses: Punctuated<Clause, Token![,]>,
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub enum Clause {
	#[parse(peek = kw::read)]
	Read(ClauseRead),
	#[parse(peek = kw::write)]
	Write(ClauseWrite),
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct ClauseRead {
	pub read_token: kw::read,
	pub arrow_token: Token![=>],
	pub expr: Box<syn::Expr>,
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct ClauseWrite {
	pub write_token: kw::write,
	#[parse(|p| {
		// Parse a full path in order to forbid multi-segment paths
		let path = p.fork().parse::<syn::Path>()?;
		path.get_ident().cloned().ok_or_else(|| syn::Error::new(path.span(), "must be ident"))
	})]
	pub ident: Ident,
	#[parse(syn::Pat::parse_single, boxed)]
	pub pat: Box<syn::Pat>,
	pub arrow_token: Token![=>],
	pub expr: Box<syn::Expr>,
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct DefDef {
	pub attrs: Attributes,
	pub def_token: kw::def,
	pub bang_token: Token![!],
	pub ident: Ident,
	#[syn(parenthesized)]
	pub paren_token: Paren,
	#[syn(in = paren_token)]
	#[parse(Punctuated::parse_terminated)]
	pub args: Punctuated<Box<syn::Type>, Token![,]>,
}

#[derive(Clone, syn_derive::ToTokens)]
pub struct TopAttributes {
	pub games: GamesAttr,
	pub other: Attributes,
}

impl Parse for TopAttributes {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let mut attrs = input.parse::<Attributes>()?;
		Ok(TopAttributes {
			games: pop_attr(&mut attrs, "games")
				.ok_or_else(|| syn::Error::new(input.span(), "no #[games]"))?
				.parse_args()?,
			other: attrs,
		})
	}
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct GamesAttr {
	pub expr: Box<syn::Expr>,
	pub arrow_token: Token![=>],
	#[parse(|input| {
		let mut ty = TokenStream::new();
		#[allow(clippy::nonminimal_bool)] // it's more readable this way
		while !input.is_empty() && !(input.peek(Token![::]) && input.peek3(Brace)) {
			ty.extend(input.parse::<proc_macro2::TokenTree>())
		}
		Ok(syn::parse_quote! { #ty })
	})]
	pub ty: Box<syn::Type>,
	pub colon2_token: Token![::],
	#[syn(braced)]
	pub brace_token: Brace,
	#[syn(in = brace_token)]
	#[parse(Punctuated::parse_terminated)]
	pub idents: Punctuated<Ident, Token![,]>,
}

#[derive(Clone, syn_derive::ToTokens)]
pub struct DefAttributes {
	pub game: Option<GameAttr>,
	pub other: Attributes,
}

impl Parse for DefAttributes {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let mut attrs = input.parse::<Attributes>()?;
		Ok(DefAttributes {
			game: pop_attr(&mut attrs, "game")
				.map(|a| a.parse_args())
				.transpose()?,
			other: attrs,
		})
	}
}

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct GameAttr {
	#[parse(Punctuated::parse_terminated)]
	pub idents: Punctuated<Ident, Token![,]>,
}

// Utils

#[derive(Clone, syn_derive::Parse, syn_derive::ToTokens)]
pub struct Attributes(
	#[parse(syn::Attribute::parse_outer)]
	#[to_tokens(|tokens, val| tokens.append_all(val))]
	Vec<syn::Attribute>
);

impl std::ops::Deref for Attributes {
	type Target = Vec<syn::Attribute>;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl std::ops::DerefMut for Attributes {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

fn parse_if<T: Parse>(input: ParseStream, cond: fn(ParseStream) -> bool) -> syn::Result<Option<T>> {
	if cond(input) {
		Ok(Some(input.parse()?))
	} else {
		Ok(None)
	}
}

fn pop_attr(attrs: &mut Vec<syn::Attribute>, name: &str) -> Option<syn::Attribute> {
	attrs.iter().position(|a| a.path().is_ident(name)).map(|i| attrs.remove(i))
}
