pub mod diag;
pub mod lex;
pub mod ast;
#[allow(clippy::module_inception)]
pub mod parse;

pub use diag::Diag;

pub fn parse(src: &str) -> (Vec<ast::Decl>, Vec<Diag>) {
	diag::diagnose(||
		parse::parse(&lex::lex(src))
	)
}
