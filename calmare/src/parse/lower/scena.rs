use themelios::scena::code::{FlatInsn, Insn, Expr as LExpr};
use themelios::scena::code::decompile::{recompile, TreeInsn};

use super::*;
use crate::span::{Spanned as S, Span};

pub mod ed7;

fn lower_func(ctx: &Context, body: &FnBody) -> Result<Vec<FlatInsn>> {
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
			Code::Assign(_, _, _) => todo!(),

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
					let i = k.parse_with(ctx, |p| {
						if let Some(i) = p.term("case")? {
							Ok(Some(i))
						} else if let Some(()) = p.term("default")? {
							Ok(None)
						} else {
							Diag::error(p.pos(), "expected 'case' or 'default'").emit();
							Err(Error)
						}
					});
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

fn lower_expr(ctx: &Context, e: &S<Expr>) -> LExpr {
	println!("{:?}", e);
	LExpr::Const(0)
}

fn lower_insn(ctx: &Context, i: &KeyVal) -> Insn {
	println!("{:?}", i);
	Insn::Return()
}
