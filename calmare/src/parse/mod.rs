pub mod diag;
pub mod lex;
#[allow(clippy::module_inception)]
pub mod parse;

pub use diag::Diag;

pub fn parse(src: &str) -> (Vec<crate::ast::Decl>, Vec<Diag>) {
	diag::diagnose(||
		parse::parse(&lex::lex(src))
	)
}
