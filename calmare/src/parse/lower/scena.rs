use themelios::scena::code::{Bytecode, Insn, Expr as LExpr, ExprBinop, ExprUnop};
use themelios::scena::code::decompile::{recompile, TreeInsn};

use super::*;
use crate::span::{Spanned as S, Span};

pub mod ed7;

fn lower_func(ctx: &Context, body: &FnBody) -> Result<Bytecode> {
	match body {
		FnBody::Code(insns) => {
			let tree = lower_tree(ctx, insns.as_slice(), false, false);
			recompile(&tree).map_err(|e| {
				Diag::error(insns[0].0, "unknown recompile error")
					.note(insns[0].0, e)
					.emit();
				Error
			})
		},
		FnBody::Asm() => todo!(),
	}
}

fn lower_tree(ctx: &Context, insns: &[S<Code>], can_break: bool, can_continue: bool) -> Vec<TreeInsn> {
	let mut out = Vec::new();
	let mut it = insns.iter().peekable();
	while let Some(S(s, i)) = it.next() {
		match i {
			Code::Insn(i) => {
				out.push(TreeInsn::Insn(lower_insn(ctx, i)));
			}
			Code::Assign(term, op, expr) => {
				out.push(TreeInsn::Insn(lower_assign(ctx, term, *op, expr)));
			}

			Code::If(c, b) => {
				let mut cases = Vec::new();

				let c = lower_expr(ctx, c);
				let b = lower_tree(ctx, b.as_slice(), can_break, can_continue);
				cases.push((Some(c), b));

				while let Some(S(_, Code::Elif(..))) = it.peek() {
					let S(_, Code::Elif(c1, b1)) = it.next().unwrap() else { unreachable!() };
					let c1 = lower_expr(ctx, c1);
					let b1 = lower_tree(ctx, b1.as_slice(), can_break, can_continue);
					cases.push((Some(c1), b1));
				}

				if let Some(S(_, Code::Else(..))) = it.peek() {
					let S(_, Code::Else(b1)) = it.next().unwrap() else { unreachable!() };
					let b1 = lower_tree(ctx, b1.as_slice(), can_break, can_continue);
					cases.push((None, b1));
				}

				out.push(TreeInsn::If(cases))
			}
			Code::Elif(c, b) => {
				let _ = lower_expr(ctx, c);
				let _ = lower_tree(ctx, b.as_slice(), can_break, can_continue);
				Diag::error(*s, "unexpected elif").emit();
			},
			Code::Else(b) => {
				let _ = lower_tree(ctx, b.as_slice(), can_break, can_continue);
				Diag::error(*s, "unexpected else").emit();
			}

			Code::While(c, b) => {
				let c = lower_expr(ctx, c);
				let b = lower_tree(ctx, b.as_slice(), true, true);
				out.push(TreeInsn::While(c, b));
			}

			Code::Switch(c, bs) => {
				let c = lower_expr(ctx, c);
				let mut cases = Vec::new();
				let mut seen = Many::default(); // only used for duplicate checking, not order
				for (k, b) in bs {
					let b = lower_tree(ctx, b.as_slice(), true, can_continue);
					let i = match k.key.1.as_str() {
						"case" => k.parse(ctx).map(Some),
						"default" => k.parse(ctx).map(|()| None),
						_ => {
							Diag::error(k.span(), "expected 'case' or 'default'").emit();
							Err(Error)
						}
					};
					if let Ok(i) = i {
						seen.insert(*s, i, ());
						cases.push((i, b))
					}
				}
				out.push(TreeInsn::Switch(c, cases));
			}

			Code::Break => {
				if can_break {
					out.push(TreeInsn::Break);
				} else {
					Diag::error(*s, "can't break here").emit();
				}
			}

			Code::Continue => {
				if can_continue {
					out.push(TreeInsn::Continue);
				} else {
					Diag::error(*s, "can't continue here").emit();
				}
			}
		}
	}
	out
}

fn lower_insn(ctx: &Context, i: &KeyVal) -> Insn {
	Insn::Return()
}

fn lower_assign(ctx: &Context, term: &S<Term>, o: S<Assop>, e: &S<Expr>) -> Insn {
	let e = lower_expr(ctx, e);
	let o = match o.1 {
		Assop::Assign => ExprUnop::Ass,
		Assop::Add => ExprUnop::AddAss,
		Assop::Sub => ExprUnop::SubAss,
		Assop::Mul => ExprUnop::MulAss,
		Assop::Div => ExprUnop::DivAss,
		Assop::Mod => ExprUnop::ModAss,
		Assop::Or  => ExprUnop::OrAss,
		Assop::And => ExprUnop::AndAss,
		Assop::Xor => ExprUnop::XorAss,
	};
	let e = LExpr::Unop(o, Box::new(e));

	if let Term::Term(kv) = &term.1 {
		match kv.key.1.as_str() {
			"var"    => Insn::Var(Var(kv.parse(ctx).unwrap_or_default()), e),
			"system" => Insn::Attr(Attr(kv.parse(ctx).unwrap_or_default()), e),
			"char_attr" => {
				let (a, b) = kv.parse(ctx).unwrap_or((CharId(0), 0));
				Insn::CharAttr(CharAttr(a, b), e)
			},
			"global" => Insn::Global(Global(kv.parse(ctx).unwrap_or_default()), e),
			_ => {
				Diag::error(term.0, "invalid assignment target").emit();
				Insn::Return()
			}
		}
	} else {
		Diag::error(term.0, "invalid assignment target").emit();
		Insn::Return()
	}
}

fn lower_expr(ctx: &Context, e: &S<Expr>) -> LExpr {
	match &e.1 {
		Expr::Binop(a, o, b) => {
			let a = lower_expr(ctx, a);
			let b = lower_expr(ctx, b);
			let o = match o.1 {
				Binop::Eq      => ExprBinop::Eq,
				Binop::Ne      => ExprBinop::Ne,
				Binop::Lt      => ExprBinop::Lt,
				Binop::Le      => ExprBinop::Le,
				Binop::Gt      => ExprBinop::Gt,
				Binop::Ge      => ExprBinop::Ge,
				Binop::BoolAnd => ExprBinop::BoolAnd,
				Binop::BoolOr  => ExprBinop::Or,
				Binop::Add     => ExprBinop::Add,
				Binop::Sub     => ExprBinop::Sub,
				Binop::Mul     => ExprBinop::Mul,
				Binop::Div     => ExprBinop::Div,
				Binop::Mod     => ExprBinop::Mod,
				Binop::Or      => ExprBinop::Or,
				Binop::And     => ExprBinop::And,
				Binop::Xor     => ExprBinop::Xor,
			};
			LExpr::Binop(o, Box::new(a), Box::new(b))
		},
		Expr::Unop(o, e) => {
			let e = lower_expr(ctx, e);
			let o = match o.1 {
				Unop::Not => ExprUnop::Not,
				Unop::Neg => ExprUnop::Neg,
				Unop::Inv => ExprUnop::Inv,
			};
			LExpr::Unop(o, Box::new(e))
		},
		Expr::Term(S(s, kv)) => {
			match kv {
				Term::Int(S(_, i), _) => {
					if *i < 0 {
						LExpr::Unop(ExprUnop::Neg, Box::new(LExpr::Const(-*i as u32)))
					} else {
						LExpr::Const(*i as u32)
					}
				}
				Term::Term(kv) => {
					match kv.key.1.as_str() {
						"flag"   => LExpr::Flag(Flag(kv.parse(ctx).unwrap_or_default())),
						"var"    => LExpr::Var(Var(kv.parse(ctx).unwrap_or_default())),
						"system" => LExpr::Attr(Attr(kv.parse(ctx).unwrap_or_default())),
						"char_attr" => {
							let (a, b) = kv.parse(ctx).unwrap_or((CharId(0), 0));
							LExpr::CharAttr(CharAttr(a, b))
						},
						"global" => LExpr::Global(Global(kv.parse(ctx).unwrap_or_default())),
						"random" => { kv.parse::<()>(ctx).unwrap_or_default(); LExpr::Rand },
						_ => {
							Diag::error(*s, "invalid expr").emit();
							LExpr::Const(0)
						}
					}
				},
				_ => {
					Diag::error(*s, "invalid expr").emit();
					LExpr::Const(0)
				},
			}
		},
		Expr::Insn(i) => LExpr::Insn(Box::new(lower_insn(ctx, i))),
	}
}
