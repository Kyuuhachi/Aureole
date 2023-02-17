pub use super::lex::Spanned;
use Spanned as S;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decl {
	FileType(Game, FileType),
	Function(Function),
	Data(Data),
	// Alias(Alias),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Game {
	Ed61, Ed62, Ed63, Ed71, Ed72,
	Ed61e, Ed62e, Ed63e, Ed71e, Ed72e,
	Ed61k, Ed62k, Ed63k, Ed71k, Ed72k,
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
	// Basics
	Int(S<i64>, S<Unit>),
	String(String),
	Tuple(Vec<S<Term>>),

	Text(Vec<S<TextSegment>>),
	Attr(Box<Term>, S<u64>),

	// Expr
	Random,
	Flag(S<u64>),
	System(S<u64>),
	Var(S<u64>),
	Global(S<u64>),

	// Tuples
	Emote(S<u64>, S<u64>, Box<S<Term>>),

	// Mainly chars
	Null, // Though null can be useful for other cases too
	Self_,
	Custom(S<u64>),
	Party(S<u64>),
	FieldParty(S<u64>),

	// Scena specific
	Fn(S<u64>, S<u64>),
	Char(S<u64>),
	Entrance(S<u64>), // defined externally
	Object(S<u64>),   // defined externally
	LookPoint(S<u64>),
	Chcp(S<u64>),

	// Script resource ids
	Fork(S<u64>),
	Menu(S<u64>),
	Select(S<u64>),
	Vis(S<u64>),
	Eff(S<u64>),
	EffInstance(S<u64>),

	// Global tables
	Name(S<u64>),
	Battle(S<u64>), // This one is scena-specific in ed7, but whatever
	Bgm(S<u64>),
	Sound(S<u64>),
	Item(S<u64>),
	Magic(S<u64>),
	Quest(S<u64>),
	Shop(S<u64>),
	Town(S<u64>),
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
