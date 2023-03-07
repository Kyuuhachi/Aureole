use themelios::scena::code::{Bytecode, Insn, Expr, ExprBinop, ExprUnop};
use themelios::scena::code::decompile::{recompile, TreeInsn};

use super::*;
use crate::span::{Spanned as S, Span};

pub mod ed7;

fn parse_func(p: &mut Parse) -> Bytecode {
	let tree = parse_tree(p, false, false);
	recompile(&tree).map_err(|e| {
		Diag::error(p.head_span(), "unknown recompile error")
			.note(p.head_span(), e)
			.emit();
		Error
	}).unwrap_or_default()
}

fn parse_tree(p: &mut Parse, can_break: bool, can_continue: bool) -> Vec<TreeInsn> {
	let mut out = Vec::new();
	let mut last_if = None;
	for l in p.body().unwrap_or_default() {
		let p = &mut Parse::new(l, p.context);

		let span = p.next_span();
		let key = p.next_if(f!(Token::Ident(a) => *a)).unwrap_or_default();

		match key {
			"if" => {
				let e = parse_expr(p);
				let b = parse_tree(p, can_break, can_continue);
				out.push(TreeInsn::If(vec![(Some(e), b)]));
				let TreeInsn::If(a) = out.last_mut().unwrap() else { unreachable!() };
				last_if = Some(a);
			}

			"elif" => {
				let e = parse_expr(p);
				let b = parse_tree(p, can_break, can_continue);
				if let Some(a) = last_if {
					a.push((Some(e), b));
					last_if = Some(a)
				} else {
					Diag::error(span, "unexpected elif").emit();
				}
			},

			"else" => {
				let b = parse_tree(p, can_break, can_continue);
				if let Some(a) = last_if {
					a.push((None, b));
					last_if = None;
				} else {
					Diag::error(span, "unexpected else").emit();
				}
			}

			"while" => {
				last_if = None;
				let e = parse_expr(p);
				let b = parse_tree(p, true, true);
				out.push(TreeInsn::While(e, b));
			}

			"switch" => {
				last_if = None;
				let e = parse_expr(p);
				let mut cases = Vec::new();
				let mut seen = Many::<Option<u16>, ()>::default(); // only used for duplicate checking, not order
				for l in p.body().unwrap_or_default() {
					Parse::new(l, p.context).parse_with(|p| {
						let span = p.next_span();
						let key = p.next_if(f!(Token::Ident(a) => *a)).unwrap_or_default();
						let i = match key {
							"case" => u16::parse(p).map(Some),
							"default" => Ok(None),
							_ => {
								Diag::error(span, "expected 'case' or 'default'").emit();
								Err(Error)
							}
						};
						let b = parse_tree(p, true, can_continue);
						if let Ok(i) = i {
							seen.mark(span, i);
							cases.push((i, b))
						}
					});
				}
				out.push(TreeInsn::Switch(e, cases));
			}

			"break" => {
				last_if = None;
				if can_break {
					out.push(TreeInsn::Break);
				} else {
					Diag::error(span, "can't break here").emit();
				}
			}

			"continue" => {
				last_if = None;
				if can_continue {
					out.push(TreeInsn::Continue);
				} else {
					Diag::error(span, "can't continue here").emit();
				}
			}

			_ => {
				p.pos -= 1;
				last_if = None;
				let insn = parse_insn(p);
				out.push(TreeInsn::Insn(insn));
			}
		}
		p.finish();
	}
	out
}

fn parse_insn(p: &mut Parse) -> Insn {
	Insn::Return()
}

// fn lower_assign(ctx: &Context, term: &S<Term>, o: S<Assop>, e: &S<Expr>) -> Insn {
// 	let e = lower_expr(ctx, e);
// 	let o = match o.1 {
// 		Assop::Assign => ExprUnop::Ass,
// 		Assop::Add => ExprUnop::AddAss,
// 		Assop::Sub => ExprUnop::SubAss,
// 		Assop::Mul => ExprUnop::MulAss,
// 		Assop::Div => ExprUnop::DivAss,
// 		Assop::Mod => ExprUnop::ModAss,
// 		Assop::Or  => ExprUnop::OrAss,
// 		Assop::And => ExprUnop::AndAss,
// 		Assop::Xor => ExprUnop::XorAss,
// 	};
// 	let e = LExpr::Unop(o, Box::new(e));
//
// 	if let Term::Term(kv) = &term.1 {
// 		match kv.key.1.as_str() {
// 			"var"    => Insn::Var(Var(kv.parse(ctx).unwrap_or_default()), e),
// 			"system" => Insn::Attr(Attr(kv.parse(ctx).unwrap_or_default()), e),
// 			"char_attr" => {
// 				let (a, b) = kv.parse(ctx).unwrap_or((CharId(0), 0));
// 				Insn::CharAttr(CharAttr(a, b), e)
// 			},
// 			"global" => Insn::Global(Global(kv.parse(ctx).unwrap_or_default()), e),
// 			_ => {
// 				Diag::error(term.0, "invalid assignment target").emit();
// 				Insn::Return()
// 			}
// 		}
// 	} else {
// 		Diag::error(term.0, "invalid assignment target").emit();
// 		Insn::Return()
// 	}
// }

// fn parse_exprq(p: &mut Parse) -> Expr {
// 	match &e.1 {
// 		Expr::Binop(a, o, b) => {
// 			let a = lower_expr(ctx, a);
// 			let b = lower_expr(ctx, b);
// 			let o = match o.1 {
// 				Binop::Eq      => ExprBinop::Eq,
// 				Binop::Ne      => ExprBinop::Ne,
// 				Binop::Lt      => ExprBinop::Lt,
// 				Binop::Le      => ExprBinop::Le,
// 				Binop::Gt      => ExprBinop::Gt,
// 				Binop::Ge      => ExprBinop::Ge,
// 				Binop::BoolAnd => ExprBinop::BoolAnd,
// 				Binop::BoolOr  => ExprBinop::Or,
// 				Binop::Add     => ExprBinop::Add,
// 				Binop::Sub     => ExprBinop::Sub,
// 				Binop::Mul     => ExprBinop::Mul,
// 				Binop::Div     => ExprBinop::Div,
// 				Binop::Mod     => ExprBinop::Mod,
// 				Binop::Or      => ExprBinop::Or,
// 				Binop::And     => ExprBinop::And,
// 				Binop::Xor     => ExprBinop::Xor,
// 			};
// 			LExpr::Binop(o, Box::new(a), Box::new(b))
// 		},
// 		Expr::Unop(o, e) => {
// 			let e = lower_expr(ctx, e);
// 			let o = match o.1 {
// 				Unop::Not => ExprUnop::Not,
// 				Unop::Neg => ExprUnop::Neg,
// 				Unop::Inv => ExprUnop::Inv,
// 			};
// 			LExpr::Unop(o, Box::new(e))
// 		},
// 		Expr::Term(S(s, kv)) => {
// 			match kv {
// 				Term::Int(S(_, i), _) => {
// 					if *i < 0 {
// 						LExpr::Unop(ExprUnop::Neg, Box::new(LExpr::Const(-*i as u32)))
// 					} else {
// 						LExpr::Const(*i as u32)
// 					}
// 				}
// 				Term::Term(kv) => {
// 					match kv.key.1.as_str() {
// 						"flag"   => LExpr::Flag(Flag(kv.parse(ctx).unwrap_or_default())),
// 						"var"    => LExpr::Var(Var(kv.parse(ctx).unwrap_or_default())),
// 						"system" => LExpr::Attr(Attr(kv.parse(ctx).unwrap_or_default())),
// 						"char_attr" => {
// 							let (a, b) = kv.parse(ctx).unwrap_or((CharId(0), 0));
// 							LExpr::CharAttr(CharAttr(a, b))
// 						},
// 						"global" => LExpr::Global(Global(kv.parse(ctx).unwrap_or_default())),
// 						"random" => { kv.parse::<()>(ctx).unwrap_or_default(); LExpr::Rand },
// 						_ => {
// 							Diag::error(*s, "invalid expr").emit();
// 							LExpr::Const(0)
// 						}
// 					}
// 				},
// 				_ => {
// 					Diag::error(*s, "invalid expr").emit();
// 					LExpr::Const(0)
// 				},
// 			}
// 		},
// 		Expr::Insn(i) => LExpr::Insn(Box::new(lower_insn(ctx, i))),
// 	}
// }

fn parse_expr(p: &mut Parse) -> Expr {
	parse_expr0(p, 10).unwrap_or(Expr::Const(0))
}

fn parse_expr0(p: &mut Parse, prec: usize) -> Result<Expr> {
	let mut e = parse_atom(p)?;
	while let Some(op) = parse_binop(p, prec) {
		let e2 = parse_expr0(p, prec-1)?;
		e = Expr::Binop(op, Box::new(e), Box::new(e2));
	}
	Ok(e)
}

fn parse_atom(p: &mut Parse) -> Result<Expr> {
	if p.pos == p.tokens.len() {
		Diag::error(p.eol, "expected expression").emit();
		return Err(Error)
	}
	match p.next().unwrap() {
		Token::Paren(d) => {
			Parse {
				tokens: &d.tokens,
				pos: 0,
				body: None,
				context: p.context,
				eol: d.close,
				commas: false,
			}.parse_with(|p| parse_expr0(p, 10))
		}
		Token::Minus => Ok(Expr::Unop(ExprUnop::Neg, Box::new(parse_atom(p)?))),
		Token::Excl  => Ok(Expr::Unop(ExprUnop::Not, Box::new(parse_atom(p)?))),
		Token::Tilde => Ok(Expr::Unop(ExprUnop::Inv, Box::new(parse_atom(p)?))),
		Token::Int(..) => {
			p.pos -= 1;
			Ok(Expr::Const(Val::parse(p)?))
		}
		Token::Ident("flag") => {
			p.pos -= 1;
			Val::parse(p).map(Expr::Flag)
		}
		Token::Ident("var") => {
			p.pos -= 1;
			Val::parse(p).map(Expr::Var)
		}
		Token::Ident("system") => {
			p.pos -= 1;
			Val::parse(p).map(Expr::Attr)
		}
		Token::Ident("char_attr") => {
			p.pos -= 1;
			Val::parse(p).map(Expr::CharAttr)
		}
		Token::Ident("global") => {
			p.pos -= 1;
			Val::parse(p).map(Expr::Global)
		}
		Token::Ident("random") => Ok(Expr::Rand),
		_ => {
			p.pos -= 1;
			Ok(Expr::Insn(Box::new(parse_insn(p))))
		}
	}
}

macro op($p:ident; $t1:ident $($t:ident)* => $op:expr) {
	let pos = $p.pos;
	if $p.next_if(f!(Token::$t1 => ())).is_some() $( && $p.space().is_none() && $p.next_if(f!(Token::$t => ())).is_some())* {
		return Some($op)
	}
	$p.pos = pos;
}

fn parse_assop(p: &mut Parse) -> Option<ExprUnop> {
	op!(p;         Eq => ExprUnop::Ass);
	op!(p; Plus    Eq => ExprUnop::AddAss);
	op!(p; Minus   Eq => ExprUnop::SubAss);
	op!(p; Star    Eq => ExprUnop::MulAss);
	op!(p; Slash   Eq => ExprUnop::DivAss);
	op!(p; Percent Eq => ExprUnop::ModAss);
	op!(p; Pipe    Eq => ExprUnop::OrAss);
	op!(p; Amp     Eq => ExprUnop::AndAss);
	op!(p; Caret   Eq => ExprUnop::XorAss);

	None
}

fn parse_binop(p: &mut Parse, prec: usize) -> Option<ExprBinop> {
	macro prio($prio:literal, $p:stmt) {
		if prec >= $prio {
			$p
		}
	}
	prio!(4, op!(p; Eq Eq   => ExprBinop::Eq));
	prio!(4, op!(p; Excl Eq => ExprBinop::Ne));
	prio!(4, op!(p; Lt Eq   => ExprBinop::Le));
	prio!(4, op!(p; Lt      => ExprBinop::Lt));
	prio!(4, op!(p; Gt Eq   => ExprBinop::Ge));
	prio!(4, op!(p; Gt      => ExprBinop::Gt));

	prio!(1, op!(p; Pipe Pipe => ExprBinop::Or));
	prio!(3, op!(p; Amp  Amp  => ExprBinop::BoolAnd));

	prio!(5, op!(p; Plus    => ExprBinop::Add));
	prio!(5, op!(p; Minus   => ExprBinop::Sub));
	prio!(6, op!(p; Star    => ExprBinop::Mul));
	prio!(6, op!(p; Slash   => ExprBinop::Div));
	prio!(6, op!(p; Percent => ExprBinop::Mod));
	prio!(1, op!(p; Pipe    => ExprBinop::Or));
	prio!(3, op!(p; Amp     => ExprBinop::And));
	prio!(2, op!(p; Caret   => ExprBinop::Xor));

	None
}
