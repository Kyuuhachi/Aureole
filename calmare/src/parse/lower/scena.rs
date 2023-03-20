use themelios::scena::code::{Code, Insn, Expr, ExprTerm, ExprOp};
use themelios::scena::decompile::{recompile, TreeInsn};

use super::*;
use crate::span::{Spanned as S, Span};

pub mod ed6;
pub mod ed7;

themelios::types::newtype!(CharDefId(u16));
newtype!(CharDefId, "char");

themelios::types::newtype!(FuncDefId(u16));
newtype!(FuncDefId, "fn");

#[derive(Debug, Clone)]
pub enum NpcOrMonster<A, B> {
	Npc(A),
	Monster(B),
}

fn chars<A, B>(items: Many<CharDefId, NpcOrMonster<A, B>>) -> (Vec<A>, Vec<B>) {
	let misorder = items.0.iter()
		.skip_while(|a| !matches!(&a.1.1, Some(NpcOrMonster::Monster(_))))
		.find(|a| matches!(&a.1.1, Some(NpcOrMonster::Npc(_))));
	if let Some((k, S(s, _))) = misorder {
		let (_, S(prev, _)) = items.0.range(..k).last().unwrap();
		Diag::error(*prev, "monsters must come after npcs")
			.note(*s, "is before this npc")
			.emit();
	}

	let mut npcs = Vec::new();
	let mut monsters = Vec::new();
	for m in items.get(|a| a.0 as usize) {
		match m {
			NpcOrMonster::Npc(n) => npcs.push(n),
			NpcOrMonster::Monster(m) => monsters.push(m),
		}
	}

	(npcs, monsters)
}

fn parse_func(p: &mut Parse) -> Code {
	let tree = parse_tree(p, false, false);
	recompile(&tree).map_err(|e| {
		Diag::error(p.head_span(), "unknown recompile error")
			.note(p.head_span(), e)
			.emit();
		Error
	}).unwrap_or_default()
}

impl Val for Code {
	fn parse(p: &mut Parse) -> Result<Self> {
		Ok(parse_func(p))
	}
}

fn parse_tree(p: &mut Parse, can_break: bool, can_continue: bool) -> Vec<TreeInsn> {
	let mut out = Vec::new();
	let mut last_if = None;
	for l in p.body() {
		let p = &mut Parse::new(l, p.context);

		let span = p.next_span();
		match test!(p, Token::Ident(a) => *a) {
			Some("if") => {
				let e = parse_expr(p);
				let b = parse_tree(p, can_break, can_continue);
				out.push(TreeInsn::If(vec![(Some(e), b)]));
				let TreeInsn::If(a) = out.last_mut().unwrap() else { unreachable!() };
				last_if = Some(a);
			}

			Some("elif") => {
				let e = parse_expr(p);
				let b = parse_tree(p, can_break, can_continue);
				if let Some(a) = last_if {
					a.push((Some(e), b));
					last_if = Some(a)
				} else {
					Diag::error(span, "unexpected elif").emit();
				}
			},

			Some("else") => {
				let b = parse_tree(p, can_break, can_continue);
				if let Some(a) = last_if {
					a.push((None, b));
					last_if = None;
				} else {
					Diag::error(span, "unexpected else").emit();
				}
			}

			Some("while") => {
				last_if = None;
				let e = parse_expr(p);
				let b = parse_tree(p, true, true);
				out.push(TreeInsn::While(e, b));
			}

			Some("switch") => {
				last_if = None;
				let e = parse_expr(p);
				let mut cases = Vec::new();
				let mut seen = BTreeMap::<Option<u16>, Span>::default();
				for l in p.body() {
					Parse::new(l, p.context).parse_with(|p| {
						let span = p.next_span();
						let i = match test!(p, Token::Ident(a) => *a) {
							Some("case") => u16::parse(p).map(Some),
							Some("default") => Ok(None),
							_ => {
								Diag::error(span, "expected 'case' or 'default'").emit();
								Err(Error)
							}
						};
						let b = parse_tree(p, true, can_continue);
						if let Ok(i) = i {
							if let Some(prev) = seen.insert(i, span) {
								// I'd have this as an error, but the vanilla scripts do it, so...
								Diag::warn(span, "duplicate case")
									.note(prev, "previous here")
									.emit();
							}
							cases.push((i, b))
						}
					});
				}
				out.push(TreeInsn::Switch(e, cases));
			}

			Some("break") => {
				last_if = None;
				if can_break {
					out.push(TreeInsn::Break);
				} else {
					Diag::error(span, "can't break here").emit();
				}
			}

			Some("continue") => {
				last_if = None;
				if can_continue {
					out.push(TreeInsn::Continue);
				} else {
					Diag::error(span, "can't continue here").emit();
				}
			}

			a => {
				if a.is_some() {
					p.pos -= 1;
				}
				last_if = None;
				out.push(TreeInsn::Insn(parse_insn(p)));
			}
		}
		p.finish();
	}
	out
}

fn parse_insn(p: &mut Parse) -> Insn {
	let _: Result<()> = try {
		if let Some(i) = try_parse_insn(p)? {
			return i
		}
		if let Some(i) = try_parse_assign(p)? {
			return i
		}
		Diag::error(p.next_span(), "unknown instruction").emit();
	};
	p.pos = p.tokens.len();
	Insn::Return()
}

fn try_parse_insn(p: &mut Parse) -> Result<Option<Insn>> {
	if p.pos == p.tokens.len() {
		Diag::error(p.next_span(), "can't parse insn").emit();
		return Err(Error)
	}
	macro run {
		([$(($ident:ident $(($_n:ident $($ty:tt)*))*))*]) => {
			match p.tokens[p.pos].1 {
				$(Token::Ident(stringify!($ident)) => {
					p.pos += 1;
					run!($ident $(($_n $($ty)*))*);
				})*
				_ => return Ok(None)
			}
		},
		($ident:ident ($v1:ident $_:ty) ($v2:ident Expr)) => {
			Diag::error(p.prev_span(), "please use assignment syntax").emit();
			p.pos = p.tokens.len();
			return Err(Error)
		},
		($ident:ident $(($_n:ident $ty:ty))*) => {
			let s = p.prev_span();
			let i = Insn::$ident($(<$ty>::parse(p)?),*);
			validate_insn(p, s, &i);
			return Ok(Some(i))
		}
	}

	match p.tokens[p.pos].1 {
		Token::Ident("VisSet") => {
			p.pos += 1;
			let s = p.prev_span();
			let vis = VisId::parse(p)?;
			let prop = u8::parse(p)?;
			let (a, b, c, d) = match prop {
				0..=2 => (i32::parse(p)?, i32::parse(p)?, Time::parse(p)?.0 as i32, i32::parse(p)?),
				3 => (Color::parse(p)?.0 as i32, Time::parse(p)?.0 as i32, i32::parse(p)?, i32::parse(p)?),
				_ => (i32::parse(p)?, i32::parse(p)?, i32::parse(p)?, i32::parse(p)?),
			};
			let i = Insn::VisSet(vis, prop, a, b, c, d);
			validate_insn(p, s, &i);
			Ok(Some(i))
		}
		_ => {
			themelios::scena::code::introspect!(run);
		}
	}
}

fn try_parse_assign(p: &mut Parse) -> Result<Option<Insn>> {
	macro run {
		([$(($ident:ident $(($_n:ident $($ty:tt)*))*))*]) => {
			$(run!($ident $(($_n $($ty)*))*);)*
		},
		($ident:ident ($v1:ident $t:ty) ($v2:ident Expr)) => {
			if let Some(S(s, a)) = <S<$t>>::try_parse(p)? {
				let e = parse_assignment_expr(p);
				let i = Insn::$ident(a, e);
				validate_insn(p, s, &i);
				return Ok(Some(i));
			}
		},
		($ident:ident $($t:tt)*) => {}
	}
	themelios::scena::code::introspect!(run);

	Ok(None)
}

fn validate_insn(p: &Parse, s: Span, i: &Insn) {
	if let Err(e) = Insn::validate(p.context.game, i) {
		Diag::error(s, format!("invalid instruction: {}", e)).emit();
	}
}

macro test_op($p:ident, $t1:ident $($t:ident)*) { {
	let pos = $p.pos;
	let v = test!($p, Token::$t1) $( && $p.space().is_none() && test!($p, Token::$t))*;
	if !v { $p.pos = pos; }
	v
} }

fn parse_expr(p: &mut Parse) -> Expr {
	let mut e = Vec::new();
	if parse_expr0(p, &mut e, 0).is_err() {
		p.pos = p.tokens.len();
	};
	Expr(e)
}

fn parse_expr0(p: &mut Parse, e: &mut Vec<ExprTerm>, prec: usize) -> Result<()> {
	parse_atom(p, e)?;
	while let Some((op_prec, op)) = parse_binop(p, prec) {
		parse_expr0(p, e, op_prec+1)?;
		e.push(ExprTerm::Op(op));
	}
	Ok(())
}

fn parse_binop(p: &mut Parse, prec: usize) -> Option<(usize, ExprOp)> {
	macro op_prec($op_prec:expr; $q:tt => $op:expr) {
		if prec <= $op_prec && test_op! $q {
			return Some(($op_prec, $op))
		}
	}

	op_prec!(4; (p, Eq   Eq)   => ExprOp::Eq);
	op_prec!(4; (p, Excl Eq)   => ExprOp::Ne);
	op_prec!(4; (p, Lt   Eq)   => ExprOp::Le);
	op_prec!(4; (p, Lt     )   => ExprOp::Lt);
	op_prec!(4; (p, Gt   Eq)   => ExprOp::Ge);
	op_prec!(4; (p, Gt     )   => ExprOp::Gt);

	op_prec!(1; (p, Pipe Pipe) => ExprOp::Or);
	op_prec!(3; (p, Amp  Amp)  => ExprOp::BoolAnd);

	op_prec!(5; (p, Plus    )  => ExprOp::Add);
	op_prec!(5; (p, Minus   )  => ExprOp::Sub);
	op_prec!(6; (p, Star    )  => ExprOp::Mul);
	op_prec!(6; (p, Slash   )  => ExprOp::Div);
	op_prec!(6; (p, Percent )  => ExprOp::Mod);
	op_prec!(1; (p, Pipe    )  => ExprOp::Or);
	op_prec!(3; (p, Amp     )  => ExprOp::And);
	op_prec!(2; (p, Caret   )  => ExprOp::Xor);

	None
}

fn parse_assignment_expr(p: &mut Parse) -> Expr {
	let op = parse_assop(p).unwrap_or_else(|| {
		Diag::error(p.next_span(), "expected assignment operator").emit();
		ExprOp::Ass
	});
	let mut e = parse_expr(p);
	e.0.push(ExprTerm::Op(op));
	e
}

fn parse_assop(p: &mut Parse) -> Option<ExprOp> {
	if test_op!(p,         Eq) { return Some(ExprOp::Ass) }
	if test_op!(p, Plus    Eq) { return Some(ExprOp::AddAss) }
	if test_op!(p, Minus   Eq) { return Some(ExprOp::SubAss) }
	if test_op!(p, Star    Eq) { return Some(ExprOp::MulAss) }
	if test_op!(p, Slash   Eq) { return Some(ExprOp::DivAss) }
	if test_op!(p, Percent Eq) { return Some(ExprOp::ModAss) }
	if test_op!(p, Pipe    Eq) { return Some(ExprOp::OrAss) }
	if test_op!(p, Amp     Eq) { return Some(ExprOp::AndAss) }
	if test_op!(p, Caret   Eq) { return Some(ExprOp::XorAss) }
	None
}

fn parse_atom(p: &mut Parse, e: &mut Vec<ExprTerm>) -> Result<()> {
	if let Some(d) = test!(p, Token::Paren(d) => d) {
		Parse::new_inner(&d.tokens, d.close, p.context)
			.parse_with(|p| parse_expr0(p, e, 0))?
	} else if test!(p, Token::Minus) {
		parse_atom(p, e)?;
		e.push(ExprTerm::Op(ExprOp::Neg));
	} else if test!(p, Token::Excl) {
		parse_atom(p, e)?;
		e.push(ExprTerm::Op(ExprOp::Not));
	} else if test!(p, Token::Tilde) {
		parse_atom(p, e)?;
		e.push(ExprTerm::Op(ExprOp::Inv));
	} else if let Some(v) = TryVal::try_parse(p)? {
		e.push(ExprTerm::Const(v))
	} else if let Some(v) = TryVal::try_parse(p)? {
		e.push(ExprTerm::Flag(v))
	} else if let Some(v) = TryVal::try_parse(p)? {
		e.push(ExprTerm::Var(v))
	} else if let Some(v) = TryVal::try_parse(p)? {
		e.push(ExprTerm::Attr(v))
	} else if let Some(v) = TryVal::try_parse(p)? {
		e.push(ExprTerm::CharAttr(v))
	} else if let Some(v) = TryVal::try_parse(p)? {
		e.push(ExprTerm::Global(v))
	} else if p.term::<()>("random")?.is_some() {
		e.push(ExprTerm::Rand)
	} else if let Some(i) = try_parse_insn(p)? {
		e.push(ExprTerm::Insn(Box::new(i)))
	} else {
		Diag::error(p.next_span(), "invalid expression").emit();
		return Err(Error)
	}
	Ok(())
}
