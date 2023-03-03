pub use themelios::types::Game;

use crate::span::Spanned as S;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decl {
	FileType(Game, FileType),
	Function(Function),
	Data(Data),
	// Alias(Alias),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileType {
	Scena,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyVal {
	pub key: S<String>,
	pub terms: Vec<S<Term>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
	pub id: S<Term>,
	pub body: FnBody,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FnBody {
	Code(Vec<S<Code>>),
	Asm(),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Data {
	pub head: KeyVal,
	pub body: Option<Vec<Data>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
	Int(S<i64>, S<Unit>),
	String(String),
	Tuple(Vec<S<Term>>),
	Text(Vec<S<TextSegment>>),
	Ident(String),
	Sub(Box<Term>, Vec<S<Term>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextSegment {
	Text(String),
	Newline(bool),
	Hex(u8),
	Directive(KeyVal)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Unit {
	None,
	Mm,
	MmPerS,
	Ms,
	Deg,
	MDeg,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Code {
	Insn(KeyVal),
	Assign(S<Term>, S<Assop>, S<Expr>),
	If(S<Expr>, Vec<S<Code>>),
	Elif(S<Expr>, Vec<S<Code>>),
	Else(Vec<S<Code>>),
	While(S<Expr>, Vec<S<Code>>),
	Switch(S<Expr>, Vec<(S<SwitchCase>, Vec<S<Code>>)>),
	Break,
	Continue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchCase {
	Case(S<Term>),
	Default,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
	Binop(
		Box<S<Expr>>,
		S<Binop>,
		Box<S<Expr>>,
	),
	Unop(S<Unop>, Box<S<Expr>>),
	Term(Term),
	Insn(KeyVal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binop {
	Eq, Ne,
	Lt, Le,
	Gt, Ge,
	BoolAnd, BoolOr,
	Add, Sub, Mul, Div, Mod,
	Or, And, Xor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assop {
	Assign,
	Add, Sub, Mul, Div, Mod,
	Or, And, Xor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unop {
	Not,
	Neg,
	Inv,
}
