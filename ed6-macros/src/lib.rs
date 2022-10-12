#![feature(proc_macro_diagnostic)]

use std::collections::BTreeMap;

use convert_case::{Case, Casing, Boundary};
use proc_macro::{TokenStream as TokenStream0, Diagnostic, Level};
use proc_macro2::{TokenStream, Span};
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
pub fn bytecode(tokens: TokenStream0) -> TokenStream0 {
	let body = parse_macro_input!(tokens as Body);
	let mut ctx = Ctx::default();
	// Used in the dump
	ctx.args.insert(Ident::new("String", Span::call_site()), parse_quote!{ String });

	let read_body = gather_top(&mut ctx, &body);

	let func_args = &body.args;

	let write_body = ctx.items.iter().map(|(span, Item { name, vars, write, .. })| {
		quote_spanned! { *span =>
			Self::#name(#vars) => { #write }
		}
	}).collect::<TokenStream>();

	let Insn_body = ctx.items.iter().map(|(span, Item { hex, attrs, name, types, .. })| {
		let doc = format!("{hex:02X}");
		quote_spanned! { *span =>
			#(#attrs)*
			#[doc = #doc]
			#name(#(#types),*),
		}
	}).collect::<TokenStream>();

	let main = quote! {
		#[allow(non_camel_case_types)]
		#[derive(Debug, Clone, PartialEq, Eq)]
		pub enum Insn {
			#Insn_body
		}

		impl Insn {
			#[allow(clippy::needless_borrow)]
			pub fn read<'a>(__f: &mut impl In<'a>, #func_args) -> Result<Self, ReadError> {
				#read_body
			}

			#[allow(clippy::needless_borrow)]
			pub fn write(__f: &mut impl OutDelay, #func_args, __insn: &Insn) -> Result<(), WriteError> {
				match __insn {
					#write_body
				}
				Ok(())
			}
		}
	};

	let InsnArg_names = ctx.args.keys().collect::<Vec<_>>();
	let InsnArg_types = ctx.args.values().collect::<Vec<_>>();

	let name_body = ctx.items.iter().map(|(span, Item { name, .. })| {
		quote_spanned! { *span =>
			Self::#name(..) => stringify!(#name),
		}
	}).collect::<TokenStream>();

	let args_body = ctx.items.iter().map(|(span, Item { name, vars, aliases, .. })| {
		let varnames = vars.into_iter();
		quote_spanned! { *span =>
			Self::#name(#vars) => Box::new([#(Arg::#aliases(#varnames)),*]),
		}
	}).collect::<TokenStream>();

	let arg_types_body = ctx.items.iter().map(|(span, Item { name, aliases, .. })| {
		quote_spanned! { *span =>
			stringify!(#name) => Box::new([#(Arg::#aliases),*]),
		}
	}).collect::<TokenStream>();

	let from_args_body = ctx.items.iter().map(|(span, Item { name, vars, aliases, .. })| {
		let varnames = vars.into_iter();
		quote_spanned! { *span =>
			stringify!(#name) => {
				#(let #varnames = if let Some(Arg::#aliases(v)) = it.next() { v } else { return None; };)*
				Self::#name(#vars)
			},
		}
	}).collect::<TokenStream>();

	let introspection = quote! {
		#[allow(non_camel_case_types)]
		#[derive(Debug, Clone)]
		pub enum InsnArgOwned {
			#(#InsnArg_names(#InsnArg_types),)*
		}

		#[allow(non_camel_case_types)]
		#[derive(Debug, Clone, Copy)]
		pub enum InsnArg<'a> {
			#(#InsnArg_names(&'a #InsnArg_types),)*
		}

		#[allow(non_camel_case_types)]
		#[derive(Debug)]
		pub enum InsnArgMut<'a> {
			#(#InsnArg_names(&'a mut #InsnArg_types),)*
		}

		#[allow(non_camel_case_types)]
		#[derive(Debug, Clone, Copy)]
		pub enum InsnArgType {
			#(#InsnArg_names,)*
		}

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
}

// {{{1 AST and parse
#[derive(Clone, Debug)]
struct Body {
	or1_token: Token![|],
	args: Punctuated<PatType, Token![,]>,
	or2_token: Token![|],
	attrs: Vec<Attribute>,
	bracket_token: token::Bracket,
	insns: Punctuated<WithAttrs<Arm>, Token![,]>,
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
						attrs: Attribute::parse_outer(input)?,
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
			attrs: Attribute::parse_outer(input)?,
			bracket_token: bracketed!(content in input),
			insns: Punctuated::parse_terminated(&content)?,
		})
	}
}

impl ToTokens for Body {
	fn to_tokens(&self, ts: &mut TokenStream) {
		self.or1_token.to_tokens(ts);
		self.args.to_tokens(ts);
		self.or2_token.to_tokens(ts);
		self.bracket_token.surround(ts, |ts| {
			self.insns.to_tokens(ts);
		});
	}
}

#[derive(Clone, Debug)]
struct WithAttrs<T> {
	attrs: Vec<Attribute>,
	value: T,
}

impl<T: Parse> Parse for WithAttrs<T> {
	fn parse(input: ParseStream) -> Result<Self> {
		Ok(Self {
			attrs: Attribute::parse_outer(input)?,
			value: input.parse()?,
		})
	}
}

impl<T: ToTokens> ToTokens for WithAttrs<T> {
	fn to_tokens(&self, ts: &mut TokenStream) {
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
	fn to_tokens(&self, ts: &mut TokenStream) {
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
	fn to_tokens(&self, ts: &mut TokenStream) {
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
	fn to_tokens(&self, ts: &mut TokenStream) {
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
	fn to_tokens(&self, ts: &mut TokenStream) {
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
	fn to_tokens(&self, ts: &mut TokenStream) {
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
	fn to_tokens(&self, ts: &mut TokenStream) {
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
	arms: Punctuated<WithAttrs<TableArm>, Token![,]>,
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
	fn to_tokens(&self, ts: &mut TokenStream) {
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
	fn to_tokens(&self, ts: &mut TokenStream) {
		self.key.to_tokens(ts);
		self.arrow_token.to_tokens(ts);
		self.insn.to_tokens(ts);
	}
}
// }}}1

#[derive(Default)]
#[allow(non_snake_case)]
struct Ctx {
	args: BTreeMap<Ident, Box<Type>>,
	attrs: Vec<Attribute>,
	items: Vec<(Span, Item)>,
}

#[derive(Debug, Clone)]
struct Item {
	hex: u8,
	attrs: Vec<Attribute>,
	name: Ident,
	vars: Punctuated<Ident, Token![,]>,
	#[allow(clippy::vec_box)]
	types: Vec<Box<Type>>,
	aliases: Vec<Ident>,
	write: TokenStream,
}

fn gather_top(ctx: &mut Ctx, t: &Body) -> TokenStream {
	let mut arms = Vec::new();

	let mut attrs = t.attrs.clone();
	let games_attr = attrs.iter().position(|a| a.path.is_ident("games"))
		.map(|i| attrs.remove(i));
	ctx.attrs = attrs;

	let mut n = 0;
	for arm in &t.insns {
		let mut attrs = arm.attrs.clone();
		let game_attr = attrs.iter().position(|a| a.path.is_ident("game"))
			.map(|i| attrs.remove(i));

		match &arm.value {
			Arm::SkipArm(arm) => {
				n += arm.number.1 as usize;
				for attr in &attrs {
					// Doesn't work so great for non-ident paths, but whatever
					Diagnostic::spanned(attr.path.span().unwrap(), Level::Error, format!("cannot find attribute `{}` in this scope", attr.path.to_token_stream())).emit();
				}
			},
			Arm::InsnArm(arm) => {
				let hex = n as u8;
				let mut item = Item {
					hex,
					attrs,
					name: arm.name.clone(),
					vars: Punctuated::new(),
					types: Vec::new(),
					aliases: Vec::new(),
					write: TokenStream::new(),
				};
				item.write.extend(quote_spanned! { arm.span() => __f.u8(#hex); });
				let read = gather_arm(ctx, arm, item);
				arms.push(quote_spanned! { arm.span() => #hex => { #read } });
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

fn gather_arm(ctx: &mut Ctx, arm: &InsnArm, mut item: Item) -> TokenStream {
	let mut read = TokenStream::new();

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
			read.extend(quote_spanned! { arg.span() => let #varname = #val; });
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
			item.write.extend(quote_spanned! { arg.span() => #val; });
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

	read.extend(if let Some(tail) = &arm.tail {
		let mut arms = Vec::new();
		for arm in &tail.arms {
			let mut item = item.clone();
			item.name = format_ident!("{}{}", &item.name, &arm.value.insn.name, span=arm.value.insn.name.span());
			item.attrs.extend(arm.attrs.clone());
			let key = &arm.value.key;
			item.write.extend(quote_spanned! { arm.span() => __f.u8(#key); });
			let body = gather_arm(ctx, &arm.value.insn, item);
			arms.push(quote_spanned! { arm.span() => #key => { #body } });
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
		quote_spanned! { arm.span() => Ok(Self::#name(#vars)) }
	});

	read
}

fn to_snake(ident: &Ident) -> Ident {
	Ident::new(
		&ident.to_string().with_boundaries(&[Boundary::LowerUpper]).to_case(Case::Snake),
		ident.span(),
	)
}