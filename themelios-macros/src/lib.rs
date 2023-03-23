#![feature(proc_macro_diagnostic)]
#![feature(let_chains)]

use std::collections::BTreeMap;

use proc_macro::{TokenStream as TokenStream0, Diagnostic, Level};
use proc_macro2::Span;
use quote::{quote, format_ident, ToTokens};
use syn::{
	*,
	spanned::Spanned,
	punctuated::*,
};

mod parse;
use parse::*;

macro_rules! q {
	(_      => $($b:tt)*) => { ::quote::quote!         {                $($b)* } };
	($a:expr=> $($b:tt)*) => { ::quote::quote_spanned! { ($a).span() => $($b)* } };
}

macro_rules! pq {
	(_      => $($b:tt)*) => { ::syn::parse_quote!         {                $($b)* } };
	($a:expr=> $($b:tt)*) => { ::syn::parse_quote_spanned! { ($a).span() => $($b)* } };
}

// {{{1 Main
#[proc_macro]
#[allow(non_snake_case)]
pub fn bytecode(tokens: TokenStream0) -> TokenStream0 {
	let input: Top = parse_macro_input!(tokens);
	let ctx = match gather_top(input) {
		Ok(ctx) => ctx,
		Err(err) => return err.into_compile_error().into()
	};

	let func_args = &ctx.func_args;
	let attrs = &ctx.attrs;
	let game_expr = &ctx.game_expr;
	let game_ty = &ctx.game_ty;

	let read: Vec<_> = ctx.reads.iter().map(|ReadArm { span, games, body }| {
		let games_name = games.iter().map(|a| &a.0).collect::<Vec<_>>();
		let games_hex  = games.iter().map(|a| &a.1).collect::<Vec<_>>();
		q!{span=>
			#((IS::#games_name, #games_hex))|* => {
				run(__f, #body)
			}
		}
	}).collect();
	let read = q!{_=>
		pub fn read(__f: &mut Reader, #func_args) -> Result<Self, ReadError> {
			fn run<A>(__f: &mut Reader, fun: impl FnOnce(&mut Reader) -> Result<A, ReadError>) -> Result<A, ReadError> {
				fun(__f)
			}
			type IS = #game_ty;
			match (#game_expr, __f.u8()?) {
				#(#read)*
				(_g, _v) => Err(format!("invalid Insn on {:?}: 0x{:02X}", _g, _v).into())
			}
		}
	};

	let write: Vec<_> = ctx.writes.iter().map(|WriteArm { span, games, pat, body, .. }| {
		let games_name = games.iter().map(|a| &a.0).collect::<Vec<_>>();
		let games_hex  = games.iter().map(|a| &a.1).collect::<Vec<_>>();
		q!{span=>
			(__iset@(#(IS::#games_name)|*), Self::#pat) => {
				__f.u8(match __iset {
					#(IS::#games_name => #games_hex,)*
					#[allow(unreachable_patterns)]
					_g => unsafe { std::hint::unreachable_unchecked() }
				});
				run(__f, #body)?;
			}
		}
	}).collect();
	let write = q!{_=>
		pub fn write(__f: &mut Writer, #func_args, __insn: &Insn) -> Result<(), WriteError> {
			fn run(__f: &mut Writer, fun: impl FnOnce(&mut Writer) -> Result<(), WriteError>) -> Result<(), WriteError> {
				fun(__f)
			}
			type IS = #game_ty;
			match (#game_expr, __insn) {
				#(#write)*
				(_is, _i) => return Err(format!("'{}' is not supported on '{:?}'", _i.name(), _is).into())
			}
			Ok(())
		}
	};

	let doc_insn_table = make_table(&ctx);

	let Insn_body: Punctuated<_, Token![,]> = ctx.defs.iter().map(|Insn { span, attrs, ident, args, .. }| q!{span=>
		#attrs
		#ident(#(#args),*)
	}).collect();

	let main = quote! {
		#[allow(non_camel_case_types)]
		#[derive(Debug, Clone, PartialEq, Eq)]
		#attrs
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

		#[allow(clippy::deref_addrof, unused_parens)]
		impl Insn {
			#read
			#write
		}
	};

	let introspect = ctx.defs.iter().map(|Insn { span, ident, args, .. }| {
		let types = args.iter();
		let arg_names = args.iter().enumerate().map(|(i, _)| format_ident!("_{i}")).collect::<Vec<_>>();
		quote::quote_spanned!{*span=> (#ident #((#arg_names #types))*) }
	});

	let introspect = q!{_=>
		pub macro introspect($m:path $({$($arg:tt)*})?) {
			$m!{ $({$($arg)*})? [#(#introspect)*] }
		}
	};

	quote! {
		#main
		#introspect
	}.into()
}

struct Ctx {
	func_args: Punctuated<PatType, Token![,]>,
	games: Vec<Ident>,
	attrs: Attributes,
	defs: Vec<Insn>,
	reads: Vec<ReadArm>,
	writes: Vec<WriteArm>,
	game_expr: Box<Expr>,
	game_ty: Box<Type>,
}

#[derive(Clone)]
struct InwardContext {
	ident: Ident,
	attrs: Attributes,
	arg_names: Punctuated<Ident, Token![,]>,
	#[allow(clippy::vec_box)]
	args: Vec<Box<Type>>,
	games: GameSpec,
	write: Vec<Stmt>,
}

type GameSpec = Vec<(Ident, u8)>;

struct Insn {
	span: Span,
	ident: Ident,
	attrs: Attributes,
	#[allow(clippy::vec_box)]
	args: Vec<Box<Type>>,
}

struct ReadArm {
	span: Span,
	games: GameSpec,
	body: Box<Expr>,
}

struct WriteArm {
	span: Span,
	games: GameSpec,
	ident: Ident,
	pat: Box<Pat>,
	body: Box<Expr>,
}

fn make_table(ctx: &Ctx) -> String {
	let doc = choubun::node("table", |n| {
		n.node("style", |n| n.text("\n\
			#insn-table { text-align: center; width: min-content; overflow-x: unset; }\n\
			#insn-table thead { position: sticky; top: 0; }\n\
			#insn-table th { writing-mode: vertical-lr; }\n\
			#insn-table td:first-child { text-align: left; }\n\
			#insn-table td:not(:first-child) { vertical-align: middle; }\n\
			#insn-table td { border-top: none; }\n\
			#insn-table td:not(:first-child) { border-left: none; }\n\
			#insn-table .insn-table-blank { background: linear-gradient(to right, transparent -75000%, currentcolor 1000000%); }\n\
			#insn-table .insn-table-right { border-right: none; }\n\
			#insn-table .insn-table-down { border-bottom: none; }\n\
		"));
		n.attr("id", "insn-table");
		let mut hex: BTreeMap<Ident, BTreeMap<Ident, u8>> = BTreeMap::new();
		for insn in &ctx.defs {
			hex.insert(insn.ident.clone(), BTreeMap::new());
		}
		for WriteArm { games, ident, .. } in &ctx.writes {
			if let Some(entry) = hex.get_mut(ident) {
				for (game, hex) in games {
					entry.insert(game.clone(), *hex);
				}
			} else {
				// will error because of unknown branch
			}
		}

		n.node("thead", |n| {
			n.indent();
			n.node("tr", |n| {
				n.node("th", |_| {});
				for game in &ctx.games {
					n.node("th", |n| {
						let ty = &ctx.game_ty;
						n.text(format_args!("[`{game}`]({ty}::{game})", ty=quote!{ #ty }))
					});
				}
			});
		});

		let mut table = Vec::new();
		let mut insns = ctx.defs.iter().peekable();
		while let Some(def) = insns.next() {
			let games = hex.get(&def.ident).unwrap();
			let mut defs = vec![def];
			while let Some(next) = insns.peek() && hex.get(&next.ident).unwrap() == games {
				defs.push(insns.next().unwrap());
			}

			let head = choubun::node("td", |n| {
				for Insn { ident, ..} in defs {
					n.node("span", |n| n.text(format_args!("[`{ident}`](Self::{ident})")));
					n.text(" ");
				}
			});

			let row = ctx.games.iter().map(|a| games.get(a)).collect::<Vec<_>>();
			table.push((head, row));
		}

		n.node("tbody", |n| {
			n.indent();
			let mut iter = table.into_iter().peekable();
			while let Some((head, row)) = iter.next() {
				n.node("tr", |n| {
					n.add(head);
					let mut row_iter = row.iter().copied().peekable();
					let next = iter.peek();
					let mut next_iter = next.iter().flat_map(|a| a.1.iter().copied());
					while let Some(hex) = row_iter.next() {
						let same_right = row_iter.peek() == Some(&hex);
						let same_below = next_iter.next().map(|a| a.map(|a| *a as u16)) == Some(hex.map(|a| *a as u16 + 1));
						n.node("td", |n| {
							if let Some(hex) = hex {
								n.text(format_args!("{hex:02X}"));
							} else {
								n.class("insn-table-blank")
							}
							if same_right {
								n.class("insn-table-right")
							}
							if same_below {
								n.class("insn-table-down")
							}
						});
					}
				});
			}
		});
	});
	let doc = choubun::node("div", |n| {
		n.class("example-wrap");
		n.add(doc)
	});
	format!("\n\n<span></span>{}\n\n", doc.render_to_string())
}

fn gather_top(input: Top) -> Result<Ctx> {
	let games = input.attrs.games;
	let all_games: Vec<Ident> = games.idents.iter().cloned().collect();

	let mut ctx = Ctx {
		func_args: input.args,
		attrs: input.attrs.other,
		games: games.idents.iter().cloned().collect(),
		defs: Vec::new(),
		reads: Vec::new(),
		writes: Vec::new(),
		game_expr: games.expr,
		game_ty: games.ty,
	};

	let mut n = vec![0; games.idents.len()];
	for item in input.defs {
		match item {
			Def::Skip(mut def) => {
				let val = def.count.base10_parse::<u8>()?;

				get_games(&mut def.attrs, &all_games, &mut n, val as usize)?;

				for attr in def.attrs.other.iter() {
					Diagnostic::spanned(attr.path().span().unwrap(), Level::Error, format!("cannot find attribute `{}` in this scope", attr.path().to_token_stream())).emit();
				}
			}
			Def::Custom(mut def) => {
				let games = get_games(&mut def.attrs, &all_games, &mut n, 1)?;

				for attr in def.attrs.other.iter() {
					Diagnostic::spanned(attr.path().span().unwrap(), Level::Error, format!("cannot find attribute `{}` in this scope", attr.path().to_token_stream())).emit();
				}

				let mut has_read = false;
				for clause in def.clauses {
					match clause {
						Clause::Read(clause) => {
							if has_read {
								Diagnostic::spanned(clause.read_token.span().unwrap(), Level::Error, "only one `read` allowed").emit();
							}
							has_read = true;

							ctx.reads.push(ReadArm {
								span: clause.span(),
								games: games.clone(),
								body: clause.expr,
							});
						}
						Clause::Write(clause) => {
							ctx.writes.push(WriteArm {
								span: clause.span(),
								games: games.clone(),
								ident: clause.ident,
								pat: clause.pat,
								body: clause.expr,
							});
						}
					}
				}
			}
			Def::Def(def) => {
				ctx.defs.push(Insn {
					span: def.span(),
					ident: def.ident,
					attrs: def.attrs,
					args: def.args.into_iter().collect(),
				});
			}
			Def::Standard(mut def) => {
				let span = def.span();
				let games = get_games(&mut def.attrs, &all_games, &mut n, 1)?;

				let ictx = InwardContext {
					ident: def.ident.clone(),
					attrs: def.attrs.other.clone(),
					arg_names: Punctuated::new(),
					args: Vec::new(),
					games: games.clone(),
					write: Vec::new(),
				};
				let read = gather_arm(&mut ctx, ictx, def);
				ctx.reads.push(ReadArm {
					span,
					games: games.clone(),
					body: pq!{span=> |__f| { #(#read)* } },
				});
			}
		}
	}

	for (game, n) in all_games.iter().zip(n.iter()) {
		if *n != 256 {
			Diagnostic::spanned(input.end_span.unwrap(), Level::Warning, format!("instructions do not sum up to 256: {game} => {n}")).emit();
		}
	}

	Ok(ctx)
}

fn get_games(attrs: &mut DefAttributes, all_games: &[Ident], n: &mut [usize], num: usize) -> Result<GameSpec> {
	let games = if let Some(attr) = &attrs.game {
		attr.idents.iter().cloned().collect()
	} else {
		all_games.to_owned()
	};

	let game_idx: Vec<usize> = games.iter().filter_map(|game| {
		if let Some(n) = all_games.iter().position(|a| a == game) {
			Some(n)
		} else {
			Diagnostic::spanned(game.span().unwrap(), Level::Error, format!("unknown game '{game}'")).emit();
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

fn gather_arm(ctx: &mut Ctx, mut ictx: InwardContext, arm: DefStandard) -> Vec<Stmt> {
	let mut read = Vec::<Stmt>::new();
	let span = arm.span();

	for pair in arm.args.into_pairs() {
		match pair.into_tuple() {
			(Arg::Source(arg), _) => {
				let varname = format_ident!("_{}", ictx.args.len(), span=arg.span());

				let read_expr = read_source(ctx, &arg);
				let write_expr = write_source(ctx, &arg, pq!{arg=> #varname });

				read.push(pq!{arg=> let #varname = #read_expr; });
				ictx.write.push(pq!{arg=> #write_expr; });

				ictx.arg_names.push(varname);
				ictx.args.push(source_ty(&arg));
			}
			(Arg::Match(arg), comma) => {
				if let Some(comma) = comma {
					Diagnostic::spanned(comma.span().unwrap(), Level::Error, "tail must be last").emit();
				}

				let mut arms = Vec::<Arm>::new();
				for arm in arg.arms {
					let mut ictx = ictx.clone();
					ictx.ident = format_ident!("{}{}", &ictx.ident, &arm.def.ident, span=arm.def.ident.span());
					ictx.attrs.extend((*arm.attrs).clone());
					let key = &arm.key;
					ictx.write.push(pq!{arm=> __f.u8(#key); });
					let span = arm.span();
					let body = gather_arm(ctx, ictx, arm.def);
					arms.push(pq!{span=> #key => { #(#body)* } });
				}

				let name = &ictx.ident.to_string();
				read.push(Stmt::Expr(pq!{span=>
					match __f.u8()? {
						#(#arms)*
						_v => Err(format!("invalid Insn::{}*: 0x{:02X}", #name, _v).into())
					}
				}, None));
				return read
			}
		};
	}

	ctx.defs.push(Insn {
		span,
		ident: ictx.ident.clone(),
		attrs: ictx.attrs,
		args: ictx.args,
	});

	let write = ictx.write;
	let ident = ictx.ident;
	let arg_names = ictx.arg_names;
	ctx.writes.push(WriteArm {
		span,
		games: ictx.games,
		ident: ident.clone(),
		pat: pq!{span=> #ident(#arg_names) },
		body: pq!{span=> |__f| { #(#write)* Ok(()) } },
	});

	read.push(Stmt::Expr(pq!{span=> Ok(Self::#ident(#arg_names)) }, None));
	read
}

fn source_ty(source: &Source) -> Box<Type> {
	match source {
		Source::Simple(source) => source.ty.clone(),
		Source::Const(source) => {
			if source.lit.suffix().is_empty() {
				Diagnostic::spanned(source.span().unwrap(), Level::Error, "constants must be suffixed").emit();
				pq!{_=> usize }
			} else {
				parse_str(source.lit.suffix()).unwrap()
			}
		}
		Source::Cast(source) => source.ty.clone(),
		Source::Block(source) => source_ty(&source.source),
		Source::If(source) => source_ty(&source.then_branch),
	}
}

fn write_source(ctx: &Ctx, source: &Source, val: Expr) -> Expr {
	match source {
		Source::Simple(source) => {
			let args = ctx.func_args.iter()
				.map(|a| { let x = &a.pat; quote!(#x) })
				.chain(Some(quote!(#val)));
			if let Some(Via { path, .. }) = &source.via {
				pq!{source=> #path::write(__f, #(#args),*)? }
			} else {
				pq!{source=> <#source as Arg>::write(__f, #(#args),*)? }
			}
		}
		Source::Const(source) => {
			let lit = &source.lit;
			pq!{source=> {
				let v = #val;
				if v != &#lit {
					return Err(format!("{:?} must be {:?}", v, #lit).into());
				}
			} }
		}
		Source::Cast(source) => {
			let ty = &source.ty;
			write_source(ctx, &source.source, pq!{source=> &cast::<#ty, _>(*#val)? })
		}
		Source::Block(source) => {
			let source = write_source(ctx, &source.source, val);
			pq!{source=> { #source } }
		}
		Source::If(SourceIf { if_token, cond, then_branch, else_token, else_branch }) => {
			let then_branch = write_source(ctx, then_branch, pq!{val=> _v});
			let else_branch = write_source(ctx, else_branch, pq!{val=> _v});
			pq!{source=> { let _v = #val; #if_token #cond #then_branch #else_token #else_branch } }
		}
	}
}

fn read_source(ctx: &Ctx, source: &Source) -> Expr {
	match source {
		Source::Simple(source) => {
			let args = ctx.func_args.iter()
				.map(|a| { let x = &a.pat; quote!(#x) });
			if let Some(Via { path, .. }) = &source.via {
				pq!{source=> #path::read(__f, #(#args),*)? }
			} else {
				pq!{source=> <#source as Arg>::read(__f, #(#args),*)? }
			}
		}
		Source::Const(source) => {
			let lit = &source.lit;
			pq!{source => #lit }
		}
		Source::Cast(source) => {
			let ty = &source.ty;
			let expr = read_source(ctx, &source.source);
			pq!{source=> cast::<_, #ty>(#expr)? }
		}
		Source::Block(source) => {
			let source = read_source(ctx, &source.source);
			pq!{source=> { #source } }
		}
		Source::If(SourceIf { if_token, cond, then_branch, else_token, else_branch }) => {
			let then_branch = read_source(ctx, then_branch);
			let else_branch = read_source(ctx, else_branch);
			pq!{source=> #if_token #cond #then_branch #else_token #else_branch }
		}
	}
}
