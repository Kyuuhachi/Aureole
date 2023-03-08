use themelios::scena::code::{Code, Insn, Expr, ExprTerm, ExprOp};
use themelios::scena::code::decompile::{recompile, TreeInsn};

use super::*;
use crate::span::{Spanned as S, Span};

pub mod ed7;

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
	for l in p.body().unwrap_or_default() {
		let p = &mut Parse::new(l, p.context);

		let span = p.next_span();
		let key = test!(p, Token::Ident(a) => *a).unwrap_or_default();

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
						let key = test!(p, Token::Ident(a) => *a).unwrap_or_default();
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
	themelios::scena::code::introspect!(run);
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

fn parse_expr(p: &mut Parse) -> Expr {
	let mut e = Vec::new();
	if parse_expr0(p, &mut e, 10).is_err() {
		p.pos = p.tokens.len();
	};
	Expr(e)
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

fn parse_expr0(p: &mut Parse, e: &mut Vec<ExprTerm>, prec: usize) -> Result<()> {
	parse_atom(p, e)?;
	while let Some(op) = parse_binop(p, prec) {
		parse_expr0(p, e, prec-1)?;
		e.push(ExprTerm::Op(op));
	}
	Ok(())
}

fn parse_atom(p: &mut Parse, e: &mut Vec<ExprTerm>) -> Result<()> {
	if let Some(d) = test!(p, Token::Paren(d) => d) {
		Parse::new_inner(&d.tokens, d.close, p.context)
			.parse_with(|p| parse_expr0(p, e, 10))?
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

macro op($p:ident; $t1:ident $($t:ident)* => $op:expr) {
	let pos = $p.pos;
	if test!($p, Token::$t1) $( && $p.space().is_none() && test!($p, Token::$t))* {
		return Some($op)
	}
	$p.pos = pos;
}

fn parse_assop(p: &mut Parse) -> Option<ExprOp> {
	op!(p;         Eq => ExprOp::Ass);
	op!(p; Plus    Eq => ExprOp::AddAss);
	op!(p; Minus   Eq => ExprOp::SubAss);
	op!(p; Star    Eq => ExprOp::MulAss);
	op!(p; Slash   Eq => ExprOp::DivAss);
	op!(p; Percent Eq => ExprOp::ModAss);
	op!(p; Pipe    Eq => ExprOp::OrAss);
	op!(p; Amp     Eq => ExprOp::AndAss);
	op!(p; Caret   Eq => ExprOp::XorAss);

	None
}

fn parse_binop(p: &mut Parse, prec: usize) -> Option<ExprOp> {
	macro prio($prio:literal, $p:stmt) {
		if prec >= $prio {
			$p
		}
	}
	prio!(4, op!(p; Eq Eq   => ExprOp::Eq));
	prio!(4, op!(p; Excl Eq => ExprOp::Ne));
	prio!(4, op!(p; Lt Eq   => ExprOp::Le));
	prio!(4, op!(p; Lt      => ExprOp::Lt));
	prio!(4, op!(p; Gt Eq   => ExprOp::Ge));
	prio!(4, op!(p; Gt      => ExprOp::Gt));

	prio!(1, op!(p; Pipe Pipe => ExprOp::Or));
	prio!(3, op!(p; Amp  Amp  => ExprOp::BoolAnd));

	prio!(5, op!(p; Plus    => ExprOp::Add));
	prio!(5, op!(p; Minus   => ExprOp::Sub));
	prio!(6, op!(p; Star    => ExprOp::Mul));
	prio!(6, op!(p; Slash   => ExprOp::Div));
	prio!(6, op!(p; Percent => ExprOp::Mod));
	prio!(1, op!(p; Pipe    => ExprOp::Or));
	prio!(3, op!(p; Amp     => ExprOp::And));
	prio!(2, op!(p; Caret   => ExprOp::Xor));

	None
}
