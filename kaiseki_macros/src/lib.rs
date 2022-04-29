#![feature(proc_macro_diagnostic)]

use std::collections::BTreeMap;

use either::Either;
use convert_case::{Case, Casing, Boundary};
use proc_macro::{TokenStream as TS, Diagnostic, Level};
use proc_macro2::{Span, TokenStream};
use quote::{quote, format_ident};
use syn::{
	*,
	spanned::Spanned,
	parse::{Parser, ParseStream},
	punctuated::*
};

type Label = LitInt; // replace with more complex type later

#[derive(Clone, Debug)]
struct Field {
	span: Span,
	expr: Either<Expr, Ident>,
	ty: Box<Type>,
	alias: Ident,
}

#[derive(Clone, Debug)]
struct Instruction {
	span: Span,
	lhs: Label,
	name: Ident,
	fields: Vec<Field>,
	tail: Option<Table>,
}

#[derive(Clone, Debug)]
struct Table {
	span: Span,
	expr: Ident,
	arms: Vec<Instruction>
}

// {{{1 Parsing
macro_rules! cast {
	($p:path, $e:expr) => { {
		let e = $e;
		if let $p(v) = e {
			Ok(v)
		} else {
			Err(syn::Error::new(e.span(), concat!("expected ", stringify!($p))))
		} }
	}
}

fn to_ident(path: &Path) -> Result<Ident> {
	Ok(parse_quote! { #path })
}

fn to_snake(ident: &Ident) -> Ident {
	Ident::new(
		&ident.to_string().with_boundaries(&[Boundary::LowerUpper]).to_case(Case::Snake),
		ident.span(),
	)
}

fn emit<A>(e: impl FnOnce() -> Result<A>) -> Option<A> {
	match e() {
		Ok(a) => Some(a),
		Err(e) => {
			Diagnostic::spanned(e.span().unwrap(), Level::Error, e.to_string()).emit();
			None
		},
	}
}

fn parse_fn(item: &ItemFn) -> Result<(&Ident, Table)> {
	let first_arg = item.sig.inputs.first()
		.ok_or_else(|| Error::new(item.sig.span(), "need at least one argument"))?;
	let first_arg = cast!(FnArg::Typed, first_arg)?;
	let first_arg = cast!(Pat::Ident, &*first_arg.pat)?;
	let first_arg = &first_arg.ident;

	let expr = match &item.block.stmts[..] {
		[Stmt::Expr(a)] => a,
		_ => return Err(Error::new(item.block.span(), "expected a singular Stmt::Expr"))
	};
	let expr = cast!(Expr::Match, expr)?;
	let table = parse_table(expr)?;

	Ok((first_arg, table))
}

fn parse_table(e: &ExprMatch) -> Result<Table> {
	let span = e.span();

	let expr = cast!(Expr::Path, &*e.expr)?;
	let expr = to_ident(&expr.path)?;
	let mut arms = Vec::new();
	for arm in &e.arms {
		emit(|| {
			arms.push(parse_arm(arm)?);
			Ok(())
		});
	}
	Ok(Table { span, expr, arms })
}

fn parse_arm(arm: &Arm) -> Result<Instruction> {
	let span = arm.span();

	let pat = cast!(Pat::Lit, &arm.pat)?;
	let pat = cast!(Expr::Lit, &*pat.expr)?;
	let lit = cast!(Lit::Int, &pat.lit)?;
	let lhs = lit.clone();

	let body = cast!(Expr::Call, &*arm.body)?;
	let path = cast!(Expr::Path, &*body.func)?;
	let name = to_ident(&path.path)?;

	let mut fields = Vec::new();
	let mut tail = None;

	for a in body.args.pairs() {
		emit(|| {
			match a {
				Pair::End(Expr::Match(e)) => tail = Some(parse_table(e)?),
				Pair::Punctuated(e, _) | Pair::End(e) => fields.push(parse_field(e)?),
			}
			Ok(())
		});
	}

	Ok(Instruction { span, lhs, name, fields, tail })
}

fn parse_field(expr: &Expr) -> Result<Field> {
	let span = expr.span();
	let (expr, alias) = match expr {
		Expr::Binary(ExprBinary { left, op: BinOp::Div {..}, right, ..}) => {
			let alias = cast!(Expr::Path, &**left)?;
			let alias = to_ident(&alias.path)?;
			(&**right, Some(alias))
		}
		expr => (expr, None)
	};

	let (expr, ty) = match expr {
		// I'd prefer to use Type `{expr}: ty` instead of Cast `{expr} as ty`,
		// but rust-analyzer doesn't like that at all
		Expr::Cast(ExprCast { expr, ty, ..}) => {
			let expr = cast!(Expr::Block, &**expr)?;
			(Either::Left(expr.clone().into()), ty.clone())
		}
		Expr::Path(expr) if expr.path.get_ident().is_some() => {
			let name = to_ident(&expr.path).unwrap();
			let name = to_snake(&name);
			(Either::Right(name), parse_quote! { #expr })
		}
		expr => return Err(Error::new(expr.span(), "invalid field"))
	};

	let alias = match alias {
		Some(a) => a,
		None => {
			let path = cast!(Type::Path, &*ty)?;
			let name = to_ident(&path.path)?;
			to_snake(&name)
		}
	};

	Ok(Field { span, expr, ty, alias })
}

// {{{1 Generate parse function
macro_rules! make {
	($ty:ty, $span:expr; $($tt:tt)*) => { {
		let a: $ty = parse_quote_spanned! { $span => $($tt)* };
		a
	} }
}

struct Gen {
	insn_name: String,
	arg_name: String,
	insn_variants: Vec<Variant>,
	arg_variants: BTreeMap<Ident, Box<Type>>,
	parts_arms: Vec<Arm>,
}

impl Gen {
	fn process_table(
		&mut self,
		table: &Table,
		prefix: String,
		vars: &[(Ident, &Field)],
	) -> Expr {
		let mut arms = Vec::new();
		for arm in &table.arms {
			let lhs = &arm.lhs;
			let mut vars = Vec::from(vars);
			let mut decls = Vec::new();

			let name = format_ident!("{}{}", prefix, &arm.name);

			for field in &arm.fields {
				let field_expr = match &field.expr {
					Either::Left(expr) => expr.clone(),
					Either::Right(name) => make!(Expr, name.span(); i.#name()?),
				};
				let field_name = Ident::new(
					&format!("_{}", vars.len()),
					field_expr.span(),
				);
				decls.push(make!(Stmt, field.span; let #field_name = #field_expr;));
				vars.push((field_name, field));
			}

			let last = match &arm.tail {
				Some(tail) => self.process_table(tail, name.to_string(), &vars),
				None => {
					let insn_name = Ident::new(&self.insn_name, arm.span);
					let arg_name = Ident::new(&self.arg_name, arm.span);
					let varnames = vars.iter().map(|a|&a.0).collect::<Vec<_>>();
					let vartypes = vars.iter().map(|a|&a.1.ty).collect::<Vec<_>>();
					let aliases  = vars.iter().map(|a|&a.1.alias).collect::<Vec<_>>();

					self.insn_variants.push(make!(Variant, arm.span;
						#name(#(#vartypes),*)
					));

					for a in vars.iter().map(|a| a.1) {
						self.arg_variants.insert(a.alias.clone(), a.ty.clone());
					}

					self.parts_arms.push(make!(Arm, arm.span;
						#insn_name::#name(#(#varnames),*) => {
							(stringify!(#name), Box::new([ #(#arg_name::#aliases(#varnames),)* ]))
						}
					));

					make!(Expr, arm.span; #insn_name::#name(#(#varnames),*))
				}
			};

			arms.push(make!(Arm, arm.span; #lhs => { #(#decls)* #last }))
		}

		let description = if prefix.is_empty() {
			self.insn_name.clone()
		} else {
			format!("{}::{}*", self.insn_name, prefix)
		};
		let fallback = make!(Arm, table.span; op => eyre::bail!("Unknown {}: {:02X}", #description, op));

		let name = &table.expr;
		let expr = make!(Expr, name.span(); i.#name()?);
		make!(Expr, table.span; match #expr { #(#arms),* #fallback })
	}
}

// {{{1 Main
#[proc_macro_attribute]
pub fn bytecode(attr: TS, item: TS) -> TS {
	match emit(|| run(attr.into(), item.into())) {
		Some(ts) => ts.into(),
		None => TS::new(),
	}
}

fn run(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
	let (mut insn_enum, mut arg_enum, mut parts_fn) = Parser::parse2(move |content: ParseStream| {
		Ok((
			content.parse::<ItemEnum>()?,
			content.parse::<ItemEnum>()?,
			content.parse::<ItemFn>()?,
		))
	}, attr)?;

	let mut func = parse2::<ItemFn>(item)?;

	let (read_arg, table) = parse_fn(&func)?;

	let mut gen = Gen {
		insn_name: insn_enum.ident.to_string(),
		insn_variants: Vec::new(),
		arg_name: arg_enum.ident.to_string(),
		arg_variants: BTreeMap::new(),
		parts_arms: Vec::new(),
	};

	let body = gen.process_table(&table, String::new(), &[]);
	func.block = Box::new(make!(Block, Span::call_site(); {
		let mut i = #read_arg;
		Ok(#body)
	}));

	insn_enum.variants = gen.insn_variants.into_iter().collect();

	arg_enum.variants = gen.arg_variants.into_iter().map(|(name, ty)| {
		let lifetime = arg_enum.generics.lifetimes().next().expect("Need a lifetime");
		make!(Variant, Span::call_site(); #name(&#lifetime #ty))
	}).collect();

	let arms = gen.parts_arms;
	parts_fn.block = Box::new(make!(Block, Span::call_site(); {
		match self { #(#arms)* }
	}));

	let enum_name = &insn_enum.ident;
	Ok(quote! {
		#insn_enum
		#arg_enum
		impl #enum_name {
			#func
			#parts_fn
		}
	})
}
