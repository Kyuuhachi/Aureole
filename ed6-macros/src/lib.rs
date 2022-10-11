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
	let mut ctx = Ctx::new();

	// Used in the dump
	ctx.args.insert(Ident::new("String", Span::call_site()), parse_quote!{ String });

	let read_body = gather_top(&mut ctx, &body);

	let items = ctx.items.iter().map(|(span, Item { hex, name, types, .. })| {
		let doc = format!("{hex:02X}");
		quote_spanned! { *span =>
			#[doc = #doc]
			#name(#(#types),*)
		}
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
		#[derive(Debug, Clone, Copy)]
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
		pub fn read<'a>(__f: &mut impl In<'a>, #args) -> Result<Self, ReadError> {
			#read_body
		}
	};

	let items = ctx.items.iter().map(|(span, Item { name, vars, write, .. })| quote_spanned! { *span =>
		Self::#name(#(#vars),*) => { #(#write)* }
	});
	let write = quote! {
		#[allow(clippy::needless_borrow)]
		pub fn write(__f: &mut impl OutDelay, #args, __insn: &Insn) -> Result<(), WriteError> {
			match __insn {
				#(#items)*
			}
			Ok(())
		}
	};

	let items = ctx.items.iter().map(|(span, Item { name, .. })| quote_spanned! { *span =>
		Self::#name(..) => stringify!(#name),
	});
	let name = quote! {
		pub fn name(&self) -> &'static str {
			match self {
				#(#items)*
			}
		}
	};

	let items = ctx.items.iter().map(|(span, Item { name, vars, aliases, .. })| quote_spanned! { *span =>
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
	syn::custom_keyword!(skip);
}

// {{{1 AST and parse
#[derive(Clone, Debug)]
struct Body {
	or1_token: Token![|],
	args: Punctuated<PatType, Token![,]>,
	or2_token: Token![|],
	bracket_token: token::Bracket,
	insns: Punctuated<AttrArm, Token![,]>,
}

impl Parse for Body {
	fn parse(input: ParseStream) -> Result<Self> {
		let content;
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
			bracket_token: bracketed!(content in input),
			insns: Punctuated::parse_terminated(&content)?,
		})
	}
}

impl ToTokens for Body {
	fn to_tokens(&self, ts: &mut TokenStream2) {
		self.or1_token.to_tokens(ts);
		self.args.to_tokens(ts);
		self.or2_token.to_tokens(ts);
		self.bracket_token.surround(ts, |ts| {
			self.insns.to_tokens(ts);
		});
	}
}

#[derive(Clone, Debug)]
struct AttrArm {
	attrs: Vec<Attribute>,
	value: Arm,
}

impl Parse for AttrArm {
	fn parse(input: ParseStream) -> Result<Self> {
		Ok(Self {
			attrs: Attribute::parse_outer(input)?,
			value: input.parse()?,
		})
	}
}

impl ToTokens for AttrArm {
	fn to_tokens(&self, ts: &mut TokenStream2) {
		for a in &self.attrs {
			a.to_tokens(ts);
		}
		self.value.to_tokens(ts);
	}
}

#[derive(Clone, Debug)]
enum Arm {
	SkipArm(SkipArm),
	InsnArm(InsnArm),
}

impl Parse for Arm {
	fn parse(input: ParseStream) -> Result<Self> {
		if input.peek(kw::skip) {
			Ok(Arm::SkipArm(input.parse()?))
		} else {
			Ok(Arm::InsnArm(input.parse()?))
		}
	}
}

impl ToTokens for Arm {
	fn to_tokens(&self, ts: &mut TokenStream2) {
		match self {
			Arm::SkipArm(a) => a.to_tokens(ts),
			Arm::InsnArm(a) => a.to_tokens(ts),
		}
	}
}

#[derive(Clone, Debug)]
struct SkipArm {
	skip_token: kw::skip,
	bang_token: Token![!],
	paren_token: token::Paren,
	number: (LitInt, u8),
}

impl Parse for SkipArm {
	fn parse(input: ParseStream) -> Result<Self> {
		let content;
		Ok(Self {
			skip_token: input.parse()?,
			bang_token: input.parse()?,
			paren_token: parenthesized!(content in input),
			number: {
				let lit: LitInt = content.parse()?;
				let val = lit.base10_parse()?;
				(lit, val)
			},
		})
	}
}

impl ToTokens for SkipArm {
	fn to_tokens(&self, ts: &mut TokenStream2) {
		self.skip_token.to_tokens(ts);
		self.bang_token.to_tokens(ts);
		self.paren_token.surround(ts, |ts| {
			self.number.0.to_tokens(ts);
		});
	}
}

#[derive(Clone, Debug)]
struct InsnArm {
	name: Ident,
	paren_token: token::Paren,
	args: Punctuated<Arg, Token![,]>,
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
	args: Punctuated<Box<Expr>, Token![,]>,
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

#[derive(Clone, Debug)]
struct Table {
	match_token: Token![match],
	brace_token: token::Brace,
	arms: Punctuated<TableArm, Token![,]>,
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
struct TableArm {
	key: LitInt,
	arrow_token: Token![=>],
	insn: InsnArm,
}

impl Parse for TableArm {
	fn parse(input: ParseStream) -> Result<Self> {
		Ok(Self {
			key: input.parse()?,
			arrow_token: input.parse()?,
			insn: input.parse()?,
		})
	}
}

impl ToTokens for TableArm {
	fn to_tokens(&self, ts: &mut TokenStream2) {
		self.key.to_tokens(ts);
		self.arrow_token.to_tokens(ts);
		self.insn.to_tokens(ts);
	}
}
// }}}1

struct Ctx {
	args: BTreeMap<Ident, Box<Type>>,
	items: Vec<(Span, Item)>,
}

impl Ctx {
	fn new() -> Self {
		Self {
			args: Default::default(),
			items: Default::default(),
		}
	}
}

#[derive(Debug, Clone)]
struct Item {
	hex: u8,
	name: Ident,
	vars: Vec<Ident>,
	#[allow(clippy::vec_box)]
	types: Vec<Box<Type>>,
	aliases: Vec<Ident>,
	write: Vec<TokenStream2>,
}

fn gather_top(ctx: &mut Ctx, t: &Body) -> TokenStream2 {
	let mut arms = Vec::new();

	let mut n = 0;
	for arm in &t.insns {
		match &arm.value {
			Arm::SkipArm(arm) => {
				n += arm.number.1 as usize;
			},
			Arm::InsnArm(arm) => {
				let hex = n as u8;
				let mut item = Item {
					hex,
					name: arm.name.clone(),
					vars: Vec::new(),
					types: Vec::new(),
					aliases: Vec::new(),
					write: Vec::new(),
				};
				item.write.push(quote_spanned! { arm.span() => __f.u8(#hex); });
				let read = gather_arm(ctx, arm, item);
				arms.push(quote_spanned! { arm.span() => #hex => #read, });
				n += 1;
			},
		}
	}
	if n != 256 {
		let span = if let Some(last) = t.insns.last() {
			last.span()
		} else {
			Span::call_site()
		};
		Diagnostic::spanned(span.unwrap(), Level::Warning, format!("Instructions sum up to {n}, not 256")).emit();
	}

	quote_spanned! { t.bracket_token.span =>
		match __f.u8()? {
			#(#arms)*
			_v => Err(format!("invalid Insn: 0x{:02X}", _v).into())
		}
	}
}

fn gather_arm(ctx: &mut Ctx, arm: &InsnArm, mut item: Item) -> TokenStream2 {
	let mut read = Vec::new();

	for arg in &arm.args {
		let varname = format_ident!("_{}", item.vars.len(), span=arg.span());

		{
			let mut val = match &arg.source {
				Source::Simple(name) => {
					let name = to_snake(name);
					quote_spanned! { name.span() => __f.#name()? }
				},
				Source::Call(a) => {
					let name = &a.name;
					let mut args = vec![quote_spanned! { a.span() => __f }];
					for e in &a.args {
						args.push(quote_spanned! { e.span() => #e })
					}
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
					quote_spanned! { name.span() => __f.#name(#val) }
				},
				Source::Call(a) => {
					let name = &a.name;
					let mut args = vec![quote_spanned! { a.span() => __f }];
					for e in &a.args {
						args.push(quote_spanned! { e.span() => #e })
					}
					args.push(val);
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

	read.push(if let Some(tail) = &arm.tail {
		let mut arms = Vec::new();
		for arm in &tail.arms {
			let mut item = item.clone();
			item.name = format_ident!("{}{}", &item.name, &arm.insn.name, span=arm.insn.name.span());
			let key = &arm.key;
			item.write.push(quote_spanned! { arm.span() => __f.u8(#key); });
			let body = gather_arm(ctx, &arm.insn, item);
			let key = &arm.key;
			arms.push(quote_spanned! { arm.span() => #key => #body, });
		}
		let name = &item.name;
		quote_spanned! { tail.span() =>
			match __f.u8()? {
				#(#arms)*
				_v => Err(format!("invalid Insn::{}*: 0x{:02X}", stringify!(#name), _v).into())
			}
		}
	} else {
		ctx.items.push((arm.span(), item.clone()));
		let name = &item.name;
		let vars = &item.vars;
		quote_spanned! { arm.span() => Ok(Self::#name(#(#vars),*)) }
	});

	quote_spanned! { arm.span() => { #(#read)* } }
}

fn to_snake(ident: &Ident) -> Ident {
	Ident::new(
		&ident.to_string().with_boundaries(&[Boundary::LowerUpper]).to_case(Case::Snake),
		ident.span(),
	)
}
