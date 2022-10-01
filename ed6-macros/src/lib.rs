#![feature(proc_macro_diagnostic)]

use std::collections::BTreeMap;

use convert_case::{Case, Casing, Boundary};
use proc_macro::{TokenStream, Diagnostic, Level};
use proc_macro2::{TokenStream as TokenStream2, Span};
use quote::{quote, quote_spanned, format_ident, ToTokens};
use syn::{
	*,
	spanned::Spanned,
	parse::{ParseStream, Parse},
	punctuated::*,
};

// {{{1 Main
#[proc_macro]
#[allow(non_snake_case)]
pub fn bytecode(tokens: TokenStream) -> TokenStream {
	let body = parse_macro_input!(tokens as Body);
	let f = Ident::new("f", Span::call_site());
	let mut ctx = Ctx::new(f.clone());
	let read_body = gather(&mut ctx, &body.table, &Item::default());

	let items = ctx.items.iter().map(|(span, name, Item { types, .. })| quote_spanned! { *span =>
		#name(#(#types),*)
	});
	let Insn = quote! {
		#[allow(non_camel_case_types)]
		#[derive(Debug, Clone, PartialEq, Eq)]
		pub enum Insn {
			#(#items),*
		}
	};

	let items = ctx.args.iter().map(|(k, v)| quote!(#k(#v)));
	let InsnArg = quote! {
		#[allow(non_camel_case_types)]
		#[derive(Debug, Clone)]
		pub enum InsnArg {
			#(#items),*
		}
	};

	let items = ctx.args.iter().map(|(k, v)| quote!(#k(&'a #v)));
	let InsnArgRef = quote! {
		#[allow(non_camel_case_types)]
		#[derive(Debug, Clone)]
		pub enum InsnArgRef<'a> {
			#(#items),*
		}
	};

	let items = ctx.args.iter().map(|(k, v)| quote!(#k(&'a mut #v)));
	let InsnArgMut = quote! {
		#[allow(non_camel_case_types)]
		#[derive(Debug)]
		pub enum InsnArgMut<'a> {
			#(#items),*
		}
	};

	let args = &body.args;
	let read = quote! {
		#[allow(clippy::needless_borrow)]
		pub fn read<'a>(#f: &mut impl In<'a>, #args) -> Result<Self, ReadError> {
			#read_body
		}
	};

	let items = ctx.items.iter().map(|(span, name, Item { vars, write, .. })| quote_spanned! { *span =>
		Self::#name(#(#vars),*) => { #(#write)* }
	});
	let write = quote! {
		#[allow(clippy::needless_borrow)]
		pub fn write(&self, #f: &mut impl Out, #args) -> Result<(), WriteError> {
			match self {
				#(#items)*
			}
			Ok(())
		}
	};

	let items = ctx.items.iter().map(|(span, name, _)| quote_spanned! { *span =>
		Self::#name(..) => stringify!(#name),
	});
	let name = quote! {
		pub fn name(&self) -> &'static str {
			match self {
				#(#items)*
			}
		}
	};

	let items = ctx.items.iter().map(|(span, name, Item { vars, aliases, .. })| quote_spanned! { *span =>
		Self::#name(#(#vars),*) => Box::new([#(Arg::#aliases(#vars)),*]),
	}).collect::<Vec<_>>();
	let args = quote! {
		pub fn args(&self) -> Box<[InsnArgRef]> {
			use InsnArgRef as Arg;
			match self {
				#(#items)*
			}
		}
	};
	let args_mut = quote! {
		pub fn args_mut(&mut self) -> Box<[InsnArgMut]> {
			use InsnArgMut as Arg;
			match self {
				#(#items)*
			}
		}
	};
	let into_args = quote! {
		pub fn into_args(self) -> Box<[InsnArg]> {
			use InsnArg as Arg;
			match self {
				#(#items)*
			}
		}
	};

	quote! {
		#Insn
		#InsnArg
		#InsnArgRef
		#InsnArgMut

		impl Insn {
			#read
			#write
			#name
			#args
			#args_mut
			#into_args
		}
	}.into()
}

mod kw {
	syn::custom_keyword!(alias);
}

// {{{1 AST and parse
#[derive(Clone, Debug)]
struct Body {
	or1_token: Token![|],
	args: Punctuated<PatType, Token![,]>,
	or2_token: Token![|],
	table: Table,
}

impl Parse for Body {
	fn parse(input: ParseStream) -> Result<Self> {
		Ok(Self {
			or1_token: input.parse()?,
			args: {
				let mut inputs = Punctuated::new();
				loop {
					if input.peek(Token![|]) {
						break;
					}
					let value = PatType {
						attrs: Vec::new(),
						pat: input.parse()?,
						colon_token: input.parse()?,
						ty: input.parse()?,
					};
					inputs.push_value(value);
					if input.peek(Token![|]) {
						break;
					}
					let punct: Token![,] = input.parse()?;
					inputs.push_punct(punct);
				}
				inputs
			},
			or2_token: input.parse()?,
			table: input.parse()?,
		})
	}
}

impl ToTokens for Body {
	fn to_tokens(&self, ts: &mut TokenStream2) {
		self.or1_token.to_tokens(ts);
		self.args.to_tokens(ts);
		self.or2_token.to_tokens(ts);
		self.table.to_tokens(ts);
	}
}

#[derive(Clone, Debug)]
struct Table {
	match_token: Token![match],
	brace_token: token::Brace,
	arms: Punctuated<InsnArm, Token![,]>,
}

impl Parse for Table {
	fn parse(input: ParseStream) -> Result<Self> {
		let content;
		Ok(Self {
			match_token: input.parse()?,
			brace_token: braced!(content in input),
			arms: Punctuated::parse_terminated(&content)?,
		})
	}
}

impl ToTokens for Table {
	fn to_tokens(&self, ts: &mut TokenStream2) {
		self.match_token.to_tokens(ts);
		self.brace_token.surround(ts, |ts| {
			self.arms.to_tokens(ts);
		});
	}
}

#[derive(Clone, Debug)]
struct InsnArm {
	key: LitInt,
	arrow_token: Token![=>],
	name: Ident,
	paren_token: token::Paren,
	args: Punctuated<Arg, Token![,]>,
	tail: Option<Table>,
}

impl Parse for InsnArm {
	fn parse(input: ParseStream) -> Result<Self> {
		let content;
		Ok(Self {
			key: input.parse()?,
			arrow_token: input.parse()?,
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
			tail: if content.peek(Token![match]) {
				Some(content.parse()?)
			} else {
				None
			},
		})
	}
}

impl ToTokens for InsnArm {
	fn to_tokens(&self, ts: &mut TokenStream2) {
		self.key.to_tokens(ts);
		self.arrow_token.to_tokens(ts);
		self.name.to_tokens(ts);
		self.paren_token.surround(ts, |ts| {
			self.args.to_tokens(ts);
			if let Some(t) = &self.tail {
				t.to_tokens(ts);
			}
		});
	}
}


#[derive(Clone, Debug)]
struct Arg {
	source: Source,
	ty: Option<(Token![as], Box<Type>)>,
	alias: Option<(kw::alias, Ident)>,
}

impl Parse for Arg {
	fn parse(input: ParseStream) -> Result<Self> {
		Ok(Self {
			source: input.parse()?,
			ty: if input.peek(Token![as]) {
				Some((input.parse()?, input.parse()?))
			} else {
				None
			},
			alias: if input.peek(kw::alias) {
				Some((input.parse()?, input.parse()?))
			} else {
				None
			},
		})
	}
}

impl ToTokens for Arg {
	fn to_tokens(&self, ts: &mut TokenStream2) {
		self.source.to_tokens(ts);
		if let Some((a, b)) = &self.ty {
			a.to_tokens(ts);
			b.to_tokens(ts);
		}
		if let Some((a, b)) = &self.alias {
			a.to_tokens(ts);
			b.to_tokens(ts);
		}
	}
}

impl Arg {
	fn alias(&self) -> Ident {
		let ty = if let Some((_, alias)) = &self.alias {
			return alias.clone();
		} else if let Some((_, ty)) = &self.ty {
			ty
		} else {
			match &self.source {
				Source::Simple(ident) => return ident.clone(),
				Source::Call(s) => &s.ty,
			}
		};

		if let Type::Path(ty) = Box::as_ref(ty) {
			if let Some(ident) = ty.path.get_ident() {
				return ident.clone()
			}
		}

		Diagnostic::spanned(ty.span().unwrap(), Level::Error, "invalid identifier").emit();
		Ident::new("__error", Span::call_site())
	}

	fn ty(&self) -> Box<Type> {
		if let Some((_, ty)) = &self.ty {
			ty.clone()
		} else {
			match &self.source {
				Source::Simple(ident) => parse_quote_spanned! { ident.span() => #ident },
				Source::Call(s) => s.ty.clone(),
			}
		}
	}
}

#[derive(Clone, Debug)]
enum Source {
	Simple(Ident),
	Call(SourceCall),
}

impl Parse for Source {
	fn parse(input: ParseStream) -> Result<Self> {
		if input.peek2(token::Paren) {
			Ok(Source::Call(input.parse()?))
		} else {
			Ok(Source::Simple(input.parse()?))
		}
	}
}

impl ToTokens for Source {
	fn to_tokens(&self, ts: &mut TokenStream2) {
		match self {
			Source::Simple(a) => a.to_tokens(ts),
			Source::Call(a) => a.to_tokens(ts),
		}
	}
}

#[derive(Clone, Debug)]
struct SourceCall {
	name: Ident,
	paren_token: token::Paren,
	args: Punctuated<Member, Token![,]>,
	arrow_token: Token![->],
	ty: Box<Type>,
}

impl Parse for SourceCall {
	fn parse(input: ParseStream) -> Result<Self> {
		let content;
		Ok(SourceCall {
			name: input.parse()?,
			paren_token: parenthesized!(content in input),
			args: Punctuated::parse_terminated(&content)?,
			arrow_token: input.parse()?,
			ty: input.parse()?,
		})
	}
}

impl ToTokens for SourceCall {
	fn to_tokens(&self, ts: &mut TokenStream2) {
		self.name.to_tokens(ts);
		self.paren_token.surround(ts, |ts| {
			self.args.to_tokens(ts);
		});
		self.arrow_token.to_tokens(ts);
		self.ty.to_tokens(ts);
	}
}
// }}}1

struct Ctx {
	f: Ident,
	args: BTreeMap<Ident, Box<Type>>,
	items: Vec<(Span, Ident, Item)>,
}

impl Ctx {
	fn new(f: Ident) -> Self {
		Self {
			f,
			args: Default::default(),
			items: Default::default(),
		}
	}
}

#[derive(Debug, Clone, Default)]
struct Item {
	name: String,
	vars: Vec<Ident>,
	#[allow(clippy::vec_box)]
	types: Vec<Box<Type>>,
	aliases: Vec<Ident>,
	write: Vec<TokenStream2>,
}

fn gather(ctx: &mut Ctx, t: &Table, item: &Item) -> TokenStream2 {
	let mut arms = Vec::new();
	for arm in &t.arms {
		let mut item = item.clone();
		let f = &ctx.f;

		item.name.push_str(&arm.name.to_string());
		let key = &arm.key;
		item.write.push(quote_spanned! { key.span() => #f.u8(#key); });

		let mut read = Vec::new();

		for arg in &arm.args {
			let varname = format_ident!("_{}", item.vars.len(), span=arg.span());

			{
				let mut val = match &arg.source {
					Source::Simple(name) => {
						let name = to_snake(name);
						quote_spanned! { name.span() => #f.#name()? }
					},
					Source::Call(a) => {
						let name = &a.name;
						let mut args = vec![quote_spanned! { f.span() => #f }];
						parse_index(&mut args, a.args.iter());
						quote_spanned! { a.span() => #name::read(#(#args),*)? }
					},
				};
				if let Some((a, ty)) = &arg.ty {
					let span = a.span().join(ty.span()).unwrap();
					val = quote_spanned! { span => cast(#val)? };
				}
				read.push(quote_spanned! { arg.span() => let #varname = #val; });
			}

			{
				let mut val = quote_spanned! { varname.span() => #varname };
				if let Source::Simple(a) = &arg.source {
					if a != "String" {
						val = quote_spanned! { arg.span() => *#val };
					}
				}
				if let Some((a, ty)) = &arg.ty {
					let span = a.span().join(ty.span()).unwrap();
					val = quote_spanned! { span => cast(#val)? };
				}
				val = match &arg.source {
					Source::Simple(name) => {
						let name = to_snake(name);
						quote_spanned! { name.span() => #f.#name(#val) }
					},
					Source::Call(a) => {
						let name = &a.name;
						let mut args = vec![val, quote_spanned! { f.span() => #f }];
						parse_index(&mut args, a.args.iter());
						quote_spanned! { a.span() => #name::write(#(#args),*)? }
					},
				};
				if let Source::Simple(a) = &arg.source {
					if a == "String" {
						val = quote_spanned! { arg.span() => #val? };
					}
				}
				item.write.push(quote_spanned! { arg.span() => #val; });
			}

			let ty = arg.ty();
			item.vars.push(varname.clone());
			item.types.push(ty.clone());

			let alias = arg.alias();
			// collisions will be errored about at type checking
			if !ctx.args.contains_key(&alias) {
				ctx.args.insert(alias.clone(), ty);
			}
			item.aliases.push(alias.clone());
		}

		let resolution = if let Some(tail) = &arm.tail {
			gather(ctx, tail, &item)
		} else {
			let name = Ident::new(&item.name, arm.name.span());
			ctx.items.push((arm.name.span().join(arm.paren_token.span).unwrap(), name.clone(), item.clone()));

			let vars = &item.vars;
			quote_spanned! { arm.span() => Ok(Self::#name(#(#vars),*)) }
		};

		let key = &arm.key;
		arms.push(quote_spanned! { arm.span() =>
			#key => {
				#(#read)*
				#resolution
			}
		});
	}
	
	let f = &ctx.f;
	let n = Ident::new("n", t.match_token.span());
	let name = &item.name;
	quote_spanned! { t.span() =>
		match #f.u8()? {
			#(#arms)*
			#n => Err(format!("invalid {}: 0x{:02X}", #name, #n).into())
		}
	}
}

fn parse_index<'a>(args: &mut Vec<TokenStream2>, iter: impl Iterator<Item=&'a Member>) {
	for n in iter {
		let ident = match n {
			Member::Named(n) => n.clone(),
			Member::Unnamed(n) => format_ident!("_{}", n.index, span=n.span),
		};
		args.push(quote_spanned! { ident.span() => &#ident })
	}
}

fn to_snake(ident: &Ident) -> Ident {
	Ident::new(
		&ident.to_string().with_boundaries(&[Boundary::LowerUpper]).to_case(Case::Snake),
		ident.span(),
	)
}
