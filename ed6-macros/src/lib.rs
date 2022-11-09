#![feature(proc_macro_diagnostic)]
#![feature(let_chains)]

use std::collections::BTreeMap;

use convert_case::{Case, Casing, Boundary};
use proc_macro::{TokenStream as TokenStream0, Diagnostic, Level};
use proc_macro2::{TokenStream, Span};
use quote::{quote, format_ident, ToTokens, TokenStreamExt};
use syn::{
	*,
	spanned::Spanned,
	parse::{ParseStream, Parse},
	punctuated::*,
};

macro_rules! q {
	($a:expr=> $($b:tt)*) => {
		::quote::quote_spanned! { ($a).span() => $($b)* }
	}
}

// {{{1 Main
#[proc_macro]
#[allow(non_snake_case)]
pub fn bytecode(tokens: TokenStream0) -> TokenStream0 {
	let ctx = parse_macro_input!(tokens with gather_top);

	let func_args = &ctx.func_args;
	let attrs = &ctx.attrs;
	let game_expr = &ctx.game_expr;
	let game_ty = &ctx.game_ty;

	let read = ctx.reads.iter().map(|ReadArm { span, games, body }| {
		let games_name = games.iter().map(|a| &a.0).collect::<Vec<_>>();
		let games_hex  = games.iter().map(|a| &a.1).collect::<Vec<_>>();
		q!{span=>
			#((IS::#games_name, #games_hex))|* => {
				run(__f, #body)
			}
		}
	}).collect::<TokenStream>();
	let read = quote! {
		pub fn read<'a>(__f: &mut impl In<'a>, #func_args) -> Result<Self, ReadError> {
			fn run<'a, I: In<'a>, A>(__f: &mut I, fun: impl FnOnce(&mut I) -> Result<A, ReadError>) -> Result<A, ReadError> {
				fun(__f)
			}
			type IS = #game_ty;
			match (#game_expr, __f.u8()?) {
				#read
				(_g, _v) => Err(format!("invalid Insn on {:?}: 0x{:02X}", _g, _v).into())
			}
		}
	};

	let write = ctx.writes.iter().map(|WriteArm { span, games, ident, args, body }| {
		let games_name = games.iter().map(|a| &a.0).collect::<Vec<_>>();
		let games_hex  = games.iter().map(|a| &a.1).collect::<Vec<_>>();
		q!{span=>
			(__iset@(#(IS::#games_name)|*), Self::#ident(#args)) => {
				__f.u8(match __iset {
					#(IS::#games_name => #games_hex,)*
					_g => unsafe { std::hint::unreachable_unchecked() }
				});
				run(__f, #body)?;
			}
		}
	}).collect::<TokenStream>();
	let write = quote! {
		pub fn write(__f: &mut impl OutDelay, #func_args, __insn: &Insn) -> Result<(), WriteError> {
			fn run<O: OutDelay>(__f: &mut O, fun: impl FnOnce(&mut O) -> Result<(), WriteError>) -> Result<(), WriteError> {
				fun(__f)
			}
			type IS = #game_ty;
			#[allow(unused_parens)]
			match (#game_expr, __insn) {
				#write
				(_is, _i) => return Err(format!("`{}` is not supported on `{:?}`", _i.name(), _is).into())
			}
			Ok(())
		}
	};

	let mut hex: BTreeMap<Ident, BTreeMap<u8, Vec<String>>> = BTreeMap::new();
	for WriteArm { games, ident, .. } in &ctx.writes {
		let entry = hex.entry(ident.clone()).or_default();
		for (game, hex) in games {
			entry.entry(*hex).or_default().push(game.to_string())
		}
	}

	let doc_insn_table = choubun::node("table", |n| {
		let mut hex: BTreeMap<Ident, BTreeMap<Ident, u8>> = BTreeMap::new();
		for WriteArm { games, ident, .. } in &ctx.writes {
			let entry = hex.entry(ident.clone()).or_default();
			for (game, hex) in games {
				entry.insert(game.clone(), *hex);
			}
		}

		n.attr("style", "text-align: center; width: min-content; overflow-x: unset");
		n.node("thead", |n| {
			n.attr("style", "position: sticky; top: 0");
			n.node("tr", |n| {
				n.node("th", |_| {});
				for game in &ctx.games {
					n.node("th", |n| {
						n.attr("style", "writing-mode: vertical-lr");
						n.text(format_args!("[`{game}`]({ty}::{game})", ty=&ctx.game_ty))
					});
				}
			});
		});

		n.node("tbody", |n| {
			let mut insns = ctx.defs.iter().peekable();
			while let Some(def) = insns.next() {
				let games = hex.get(&def.ident).unwrap();
				let mut defs = vec![def];
				while let Some(next) = insns.peek() && hex.get(&next.ident).unwrap() == games {
					defs.push(insns.next().unwrap());
				}

				n.node("tr", |n| {
					n.node("td", |n| {
						n.attr("style", "text-align: left");
						for Def { ident, aliases, ..} in defs {
							let mut title = String::new();
							title.push_str(&ident.to_string());
							for alias in aliases {
								title.push_str(&format!(" {alias}"));
							}
							n.node("span", |n| {
								n.attr("title", title);
								n.text(format_args!("[`{ident}`](Self::{ident})"));
							});
							n.text(" ");
						}
					});

					let mut columns = Vec::<choubun::Node>::new();
					let mut prev = None;
					for game in &ctx.games {
						let node = choubun::node("td", |n| {
							n.attr("style", "vertical-align: middle");
							let hex = games.get(game);
							if let Some(hex) = hex {
								n.text(format_args!("{hex:02X}"));
							}
							if prev == Some(hex) {
								let prev = columns.last_mut().unwrap();
								n.attrs_mut().get_mut("style").unwrap().push_str("; border-left: none");
								prev.attrs_mut().get_mut("style").unwrap().push_str("; border-right: none");
							}
							prev = Some(hex);
						});
						columns.push(node);
					}
					for item in columns {
						n.add(item);
					}
				});
			}
		});
	});
	let doc_insn_table = choubun::node("div", |n| {
		n.class("example-wrap");
		n.add(doc_insn_table)
	});
	let doc_insn_table = format!("\n\n<span></span>{}\n\n", doc_insn_table.render_to_string());

	let Insn_body = ctx.defs.iter().map(|Def { span, attrs, ident, types, aliases, .. }| {
		let mut predoc = String::new();
		predoc.push_str("**`");
		predoc.push_str(&ident.to_string());
		predoc.push_str("`**");
		for alias in aliases {
			predoc.push_str(&format!(" [`{alias}`](InsnArg::{alias})"));
		}
		predoc.push_str("\n\n");

		q!{span=>
			#[doc = #predoc]
			#(#attrs)*
			#ident(#(#types),*),
		}
	}).collect::<TokenStream>();

	let main = quote! {
		#[allow(non_camel_case_types)]
		#[derive(Debug, Clone, PartialEq, Eq)]
		#(#attrs)*
		/// # Encoding
		/// Below is a table listing the hex codes for each instruction.
		/// This can for example be used to see which instructions are available in each game.
		/// Though do keep in mind that this is only based on research; it may not fully reflect what the games actually accept.
		// /// <details><summary>Click to expand</summary>
		#[doc = #doc_insn_table]
		// /// </details>
		pub enum Insn {
			#Insn_body
		}

		impl Insn {
			#read
			#write
		}
	};

	let InsnArg_names = ctx.arg_types.keys().collect::<Vec<_>>();
	let InsnArg_types = ctx.arg_types.values().collect::<Vec<_>>();

	let name_body = ctx.defs.iter().map(|Def { span, ident, .. }| {
		q!{span=>
			Self::#ident(..) => stringify!(#ident),
		}
	}).collect::<TokenStream>();

	let args_body = ctx.defs.iter().map(|Def { span, ident, args, aliases, .. }| {
		q!{span=>
			Self::#ident(#(#args),*) => Box::new([#(Arg::#aliases(#args)),*]),
		}
	}).collect::<TokenStream>();

	let arg_types_body = ctx.defs.iter().map(|Def { span, ident, aliases, .. }| {
		q!{span=>
			stringify!(#ident) => Box::new([#(Arg::#aliases),*]),
		}
	}).collect::<TokenStream>();

	let from_args_body = ctx.defs.iter().map(|Def { span, ident, args, aliases, .. }| {
		q!{span=>
			stringify!(#ident) => {
				#(let #args = if let Some(Arg::#aliases(v)) = it.next() { v } else { return None; };)*
				Self::#ident(#(#args),*)
			},
		}
	}).collect::<TokenStream>();

	let introspection = quote! {
		#[cfg(not(doc))]
		#[allow(non_camel_case_types)]
		#[derive(Debug, Clone)]
		pub enum InsnArgOwned {
			#(#InsnArg_names(#InsnArg_types),)*
		}

		#[cfg(not(doc))]
		#[allow(non_camel_case_types)]
		#[derive(Debug, Clone, Copy)]
		pub enum InsnArg<'a> {
			#(#InsnArg_names(&'a #InsnArg_types),)*
		}

		#[cfg(not(doc))]
		#[allow(non_camel_case_types)]
		#[derive(Debug)]
		pub enum InsnArgMut<'a> {
			#(#InsnArg_names(&'a mut #InsnArg_types),)*
		}

		#[cfg(not(doc))]
		#[allow(non_camel_case_types)]
		#[derive(Debug, Clone, Copy)]
		pub enum InsnArgType {
			#(#InsnArg_names,)*
		}

		// doc shims
		#[cfg(doc)]
		#[doc(alias="InsnArgOwned")]
		#[doc(alias="InsnArgMut")]
		#[doc(alias="InsnArgType")]
		#[allow(non_camel_case_types)]
		#[derive(Debug)]
		pub enum InsnArg {
			#(#InsnArg_names(#InsnArg_types),)*
		}

		#[cfg(doc)]
		#[doc(inline, hidden)]
		pub use InsnArg as InsnArgOwned;

		#[cfg(doc)]
		#[doc(inline, hidden)]
		pub use InsnArg as InsnArgMut;

		#[cfg(doc)]
		#[doc(inline, hidden)]
		pub use InsnArg as InsnArgType;

		impl Insn {
			pub fn name(&self) -> &'static str {
				match self {
					#name_body
				}
			}

			pub fn args(&self) -> Box<[InsnArg]> {
				use InsnArg as Arg;
				match self {
					#args_body
				}
			}

			pub fn args_mut(&mut self) -> Box<[InsnArgMut]> {
				use InsnArgMut as Arg;
				match self {
					#args_body
				}
			}

			pub fn into_parts(self) -> (&'static str, Box<[InsnArgOwned]>) {
				use InsnArgOwned as Arg;
				let name = self.name();
				let args: Box<[Arg]> = match self {
					#args_body
				};
				(name, args)
			}

			pub fn arg_types(name: &str) -> Option<Box<[InsnArgType]>> {
				use InsnArgType as Arg;
				let types: Box<[Arg]> = match name {
					#arg_types_body
					_ => return None,
				};
				Some(types)
			}

			pub fn from_parts(name: &str, args: impl IntoIterator<Item=InsnArgOwned>) -> Option<Insn> {
				use InsnArgOwned as Arg;
				let mut it = args.into_iter();
				let v = match name {
					#from_args_body
					_ => return None,
				};
				if let Some(_) = it.next() { return None; }
				Some(v)
			}
		}
	};

	quote! {
		#main
		#introspection
	}.into()
}

mod kw {
	syn::custom_keyword!(alias);
	syn::custom_keyword!(skip);
	syn::custom_keyword!(custom);
	syn::custom_keyword!(read);
	syn::custom_keyword!(write);
}

// {{{1 AST and parse
#[derive(Clone, Debug, syn_derive::ToTokens)]
struct InsnArm {
	name: Ident,
	#[syn(parenthesized)]
	paren_token: token::Paren,
	#[syn(in = paren_token)]
	args: Punctuated<Arg, Token![,]>,
	#[syn(in = paren_token)]
	tail: Option<Table>,
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
struct Arg {
	source: Source,
	#[parse(|input| parse_while(input, |input| input.peek(Token![as])))]
	#[to_tokens(|tokens, val| tokens.append_all(val))]
	ty: Vec<ArgTy>,
	#[parse(|input| parse_if(input, |input| input.peek(kw::alias)))]
	#[to_tokens(|tokens, val| tokens.append_all(val))]
	alias: Option<ArgAlias>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
struct ArgTy {
	as_token: Token![as],
	ty: Box<Type>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
struct ArgAlias {
	alias_token: kw::alias,
	ident: Ident,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
enum Source {
	#[parse(peek_func = |i| i.peek2(token::Paren))]
	Call(SourceCall),
	Simple(Ident),
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
struct SourceCall {
	name: Ident,
	#[syn(parenthesized)]
	paren_token: token::Paren,
	#[syn(in = paren_token)]
	#[parse(Punctuated::parse_terminated)]
	args: Punctuated<Box<Expr>, Token![,]>,
	arrow_token: Token![->],
	ty: Box<Type>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
struct Table {
	match_token: Token![match],
	#[syn(braced)]
	brace_token: token::Brace,
	#[syn(in = brace_token)]
	#[parse(Punctuated::parse_terminated)]
	arms: Punctuated<TableArm, Token![,]>,
}

#[derive(Clone, Debug, syn_derive::Parse, syn_derive::ToTokens)]
struct TableArm {
	#[parse(Attribute::parse_outer)]
	#[to_tokens(|tokens, val| tokens.append_all(val))]
	attrs: Vec<Attribute>,
	key: LitInt,
	arrow_token: Token![=>],
	insn: InsnArm,
}

// }}}1

struct Ctx {
	arg_types: BTreeMap<Ident, Box<Type>>,
	func_args: Punctuated<PatType, Token![,]>,
	games: Vec<Ident>,
	attrs: Vec<Attribute>,
	defs: Vec<Def>,
	reads: Vec<ReadArm>,
	writes: Vec<WriteArm>,
	game_expr: Expr,
	game_ty: TokenStream,
}

#[derive(Clone)]
struct InwardContext {
	ident: Ident,
	attrs: Vec<Attribute>,
	args: Punctuated<Ident, Token![,]>,
	aliases: Vec<Ident>,
	types: Vec<Box<Type>>,
	games: GameSpec,
	write: TokenStream,
}

type GameSpec = Vec<(Ident, u8)>;

struct Def {
	span: Span,
	ident: Ident,
	attrs: Vec<Attribute>,
	args: Vec<Ident>,
	aliases: Vec<Ident>,
	types: Vec<Box<Type>>,
}

struct ReadArm {
	span: Span,
	games: GameSpec,
	body: TokenStream,
}

struct WriteArm {
	span: Span,
	games: GameSpec,
	ident: Ident,
	args: Punctuated<Ident, Token![,]>,
	body: TokenStream,
}

fn gather_top(input: ParseStream) -> Result<Ctx> {
	let content;
	parenthesized!(content in input);
	let func_args = Punctuated::parse_terminated_with(&content, |input| {
		Ok(PatType {
			attrs: Attribute::parse_outer(input)?,
			pat: input.parse()?,
			colon_token: input.parse()?,
			ty: input.parse()?,
		})
	})?;

	let mut attrs = Attribute::parse_outer(input)?;
	let games_attr = attrs.iter().position(|a| a.path.is_ident("games"))
		.map(|i| attrs.remove(i))
		.expect("no #[games]");

	let (game_expr, game_ty, all_games) = games_attr.parse_args_with(|input: ParseStream| {
		let expr = input.parse::<Expr>()?;
		input.parse::<Token![=>]>()?;
		let mut ty = TokenStream::new();
		while !(input.is_empty() || input.peek(Token![::]) && input.peek3(token::Brace)) {
			ty.extend(input.parse::<proc_macro2::TokenTree>())
		}
		input.parse::<Token![::]>()?;
		let content;
		braced!(content in input);
		let all_games = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?
			.iter().cloned().collect::<Vec<_>>();
		Ok((expr, ty, all_games))
	})?;

	let mut ctx = Ctx {
		arg_types: BTreeMap::new(),
		func_args,
		attrs,
		games: all_games.clone(),
		defs: Vec::new(),
		reads: Vec::new(),
		writes: Vec::new(),
		game_expr,
		game_ty,
	};

	// Used in the dump
	ctx.arg_types.insert(Ident::new("String", Span::call_site()), parse_quote! { String });

	let content;
	bracketed!(content in input);
	let input = &content;

	let mut n = vec![0; all_games.len()];
	let mut last_span;
	loop {
		last_span = input.span();
		if input.is_empty() {
			break;
		}

		let mut attrs = Attribute::parse_outer(input)?;

		if input.peek2(Token![!]) {
			if input.peek(kw::skip) {
				input.parse::<kw::skip>()?;
				input.parse::<Token![!]>()?;
				let content;
				parenthesized!(content in input);
				let lit = content.parse::<LitInt>()?;
				let val = lit.base10_parse::<u8>()?;

				get_games(&mut attrs, &all_games, &mut n, val as usize)?;

				for attr in &attrs {
					Diagnostic::spanned(attr.path.span().unwrap(), Level::Error, format!("cannot find attribute `{}` in this scope", attr.path.to_token_stream())).emit();
				}
			} else if input.peek(kw::custom) {
				input.parse::<kw::custom>()?;
				input.parse::<Token![!]>()?;
				let content;
				braced!(content in input);
				let input = content;

				let games = get_games(&mut attrs, &all_games, &mut n, 1)?;

				for attr in &attrs {
					Diagnostic::spanned(attr.path.span().unwrap(), Level::Error, format!("cannot find attribute `{}` in this scope", attr.path.to_token_stream())).emit();
				}

				let mut has_read = false;
				while !input.is_empty() {
					if input.peek(kw::read) {
						let token = input.parse::<kw::read>()?;

						input.parse::<Token![=>]>()?;
						let body = input.parse::<ExprClosure>()?;

						if has_read {
							Diagnostic::spanned(token.span().unwrap(), Level::Error, "only one `read` allowed").emit();
						}
						has_read = true;

						ctx.reads.push(ReadArm {
							span: token.span().join(body.span()).unwrap(),
							games: games.clone(),
							body: quote! { #body },
						});
					} else if input.peek(kw::write) {
						let token = input.parse::<kw::write>()?;

						let ident = input.parse::<Ident>()?;
						let content;
						parenthesized!(content in input);
						let args = Punctuated::parse_terminated(&content)?;

						input.parse::<Token![=>]>()?;
						let body = input.parse::<ExprClosure>()?;
						ctx.writes.push(WriteArm {
							span: token.span().join(body.span()).unwrap(),
							games: games.clone(),
							ident,
							args,
							body: quote! { #body },
						});
					} else {
						Diagnostic::spanned(input.span().unwrap(), Level::Error, "invalid definition").emit();
						break
					}
					if !input.is_empty() {
						input.parse::<Token![,]>()?;
					}
				}
			} else {
				Diagnostic::spanned(input.span().unwrap(), Level::Error, "invalid definition").emit();
			}
		} else if let Some(def_attr) = pop_attr(&mut attrs, "def") {
			if !def_attr.tokens.is_empty() {
				Diagnostic::spanned(def_attr.span().unwrap(), Level::Error, "#[def] does not take arguments").emit();
			}
			let ident = input.parse::<Ident>()?;
			let content;
			let paren = parenthesized!(content in input);

			let input = content;
			let mut args = Vec::new();
			let mut aliases = Vec::new();
			let mut types = Vec::new();

			while !input.is_empty() {
				let ty = input.parse::<Box<Type>>()?;

				let alias = if input.peek(kw::alias) {
					input.parse::<kw::alias>()?;
					input.parse::<Ident>()?
				} else {
					reparse(&ty)?
				};

				let varname = format_ident!("_{}", args.len(), span=ty.span().join(alias.span()).unwrap());
				types.push(ty);
				aliases.push(alias);
				args.push(varname);

				if !input.is_empty() {
					input.parse::<Token![,]>()?;
				}
			}

			ctx.defs.push(Def {
				span: ident.span().join(paren.span).unwrap(),
				ident,
				attrs,
				args,
				aliases,
				types,
			});
		} else {
			let games = get_games(&mut attrs, &all_games, &mut n, 1)?;

			let arm = InsnArm::parse(input)?;
			let ictx = InwardContext {
				ident: arm.name.clone(),
				attrs,
				args: Punctuated::new(),
				aliases: Vec::new(),
				types: Vec::new(),
				games: games.clone(),
				write: TokenStream::new(),
			};
			let read = gather_arm(&mut ctx, ictx, &arm);
			ctx.reads.push(ReadArm {
				span: arm.span(),
				games: games.clone(),
				body: q!{arm=> |__f| { #read } },
			});
		}

		last_span = input.span();
		if input.is_empty() {
			break;
		}
		input.parse::<Token![,]>()?;
	}

	for (game, n) in all_games.iter().zip(n.iter()) {
		if *n != 256 {
			Diagnostic::spanned(last_span.unwrap(), Level::Warning, format!("instructions do not sum up to 256: {game} => {n}")).emit();
		}
	}

	Ok(ctx)
}

fn pop_attr(attrs: &mut Vec<Attribute>, name: &str) -> Option<Attribute> {
	attrs.iter().position(|a| a.path.is_ident(name)).map(|i| attrs.remove(i))
}

fn get_games(attrs: &mut Vec<Attribute>, all_games: &[Ident], n: &mut [usize], num: usize) -> Result<GameSpec> {
	let games = if let Some(game_attr) = pop_attr(attrs, "game") {
		game_attr.parse_args_with(Punctuated::<Ident, Token![,]>::parse_terminated)?
			.iter().cloned().collect()
	} else {
		all_games.to_owned()
	};

	let game_idx: Vec<usize> = games.iter().filter_map(|game| {
		if let Some(n) = all_games.iter().position(|a| a == game) {
			Some(n)
		} else {
			Diagnostic::spanned(game.span().unwrap(), Level::Error, format!("unknown game `{game}`")).emit();
			None
		}
	}).collect();

	let games = games.iter().cloned()
		.zip(game_idx.iter().map(|idx| n[*idx] as u8))
		.collect::<GameSpec>();

	for idx in &game_idx {
		n[*idx] += num;
	}

	Ok(games)
}

fn gather_arm(ctx: &mut Ctx, mut ictx: InwardContext, arm: &InsnArm) -> TokenStream {
	let mut read = TokenStream::new();

	for arg in &arm.args {
		let varname = format_ident!("_{}", ictx.args.len(), span=arg.span());

		let mut types = Vec::new();
		types.push(match &arg.source {
			Source::Simple(name) => {
				q!{name=> #name }
			}
			Source::Call(a) => {
				let ty = &a.ty;
				q!{ty=> #ty }
			}
		});
		for ty in &arg.ty {
			let ty_ = &ty.ty;
			types.push(q!{ty=> #ty_});
		}

		{
			let mut val = match &arg.source {
				Source::Simple(name) => {
					let name = to_snake(name);
					q!{name=> __f.#name()? }
				},
				Source::Call(a) => {
					let name = &a.name;
					let mut args = vec![q!{a=> __f }];
					for e in &a.args {
						args.push(
							if let Expr::Path(ExprPath { attrs, qself: None, path }) = &**e
								&& attrs.is_empty()
								&& let Some(ident) = path.get_ident()
							{
								if ictx.args.iter().any(|a| a == ident) {
									q!{ident=> &#ident }
								} else {
									q!{ident=> #ident }
								}
							} else {
								let v = ictx.args.iter();
								q!{e=> #[allow(clippy::let_and_return)] { #(let #v = &#v;)* #e } }
							}
						);
					}
					q!{a=> #name::read(#(#args),*)? }
				},
			};
			for ty in types.iter().skip(1) {
				val = q!{ty=> cast::<_, #ty>(#val)? };
			}
			read.extend(q!{arg=> let #varname = #val; });
		}

		{
			let mut val = q!{varname=> #varname };
			if let Source::Simple(a) = &arg.source {
				if a != "String" {
					val = q!{arg=> *#val };
				}
			}
			for ty in types.iter().rev().skip(1) {
				val = q!{ty=> cast::<_, #ty>(#val)? };
			}
			val = match &arg.source {
				Source::Simple(name) => {
					let name = to_snake(name);
					q!{name=> __f.#name(#val) }
				},
				Source::Call(a) => {
					let name = &a.name;
					let mut args = vec![q!{a=> __f }];
					for e in &a.args {
						args.push(q!{e=> #e })
					}
					args.push(val);
					q!{a=> #name::write(#(#args),*)? }
				},
			};
			if let Source::Simple(a) = &arg.source {
				if a == "String" {
					val = q!{arg=> #val? };
				}
			}
			ictx.write.extend(q!{arg=> #val; });
		}

		let ty = if let Some(ty) = &arg.ty.last() {
			ty.ty.clone()
		} else {
			match &arg.source {
				Source::Simple(ident) => reparse(ident).unwrap(),
				Source::Call(s) => s.ty.clone(),
			}
		};
		let alias = 'a: {
			let ty = if let Some(alias) = &arg.alias {
				break 'a alias.ident.clone();
			} else if let Some(ty) = &arg.ty.last() {
				&ty.ty
			} else {
				match &arg.source {
					Source::Simple(ident) => break 'a ident.clone(),
					Source::Call(s) => &s.ty,
				}
			};

			if let Type::Path(ty) = Box::as_ref(ty) {
				if let Some(ident) = ty.path.get_ident() {
					break 'a ident.clone()
				}
			}

			Diagnostic::spanned(ty.span().unwrap(), Level::Error, "invalid identifier").emit();
			Ident::new("__error", Span::call_site())
		};

		ictx.args.push(varname.clone());
		ictx.aliases.push(alias.clone());
		ictx.types.push(ty.clone());
		if !ctx.arg_types.contains_key(&alias) {
			// collisions will be errored about at type checking
			ctx.arg_types.insert(alias.clone(), ty);
		}
	}

	if let Some(tail) = &arm.tail {
		let mut arms = Vec::new();
		for arm in &tail.arms {
			let mut ictx = ictx.clone();
			ictx.ident = format_ident!("{}{}", &ictx.ident, &arm.insn.name, span=arm.insn.name.span());
			ictx.attrs.extend(arm.attrs.clone());
			let key = &arm.key;
			ictx.write.extend(q!{arm=> __f.u8(#key); });
			let body = gather_arm(ctx, ictx, &arm.insn);
			arms.push(q!{arm=> #key => { #body } });
		}
		let name = &ictx.ident;
		read.extend(q!{tail=>
			match __f.u8()? {
				#(#arms)*
				_v => Err(format!("invalid Insn::{}*: 0x{:02X}", stringify!(#name), _v).into())
			}
		})
	} else {
		let ident = &ictx.ident;
		let args = &ictx.args;
		read.extend(q!{arm=> Ok(Self::#ident(#args)) });

		ctx.defs.push(Def {
			span: arm.span(),
			ident: ictx.ident.clone(),
			attrs: ictx.attrs,
			args: ictx.args.iter().cloned().collect(),
			aliases: ictx.aliases,
			types: ictx.types,
		});

		let write = ictx.write;
		ctx.writes.push(WriteArm {
			span: arm.span(),
			games: ictx.games,
			ident: ictx.ident,
			args: ictx.args,
			body: q!{arm=> |__f| { #write Ok(()) } },
		});
	};

	read
}

fn to_snake(ident: &Ident) -> Ident {
	Ident::new(
		&ident.to_string().with_boundaries(&[Boundary::LowerUpper]).to_case(Case::Snake),
		ident.span(),
	)
}

fn reparse<A: ToTokens, B: Parse>(a: &A) -> Result<B> {
	Ok(parse_quote! { #a })
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
