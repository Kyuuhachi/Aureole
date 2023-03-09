pub mod diag;
pub mod lex;
pub mod lower;

pub use diag::Diag;
use themelios::types::Game;
use themelios::lookup::Lookup;

pub fn compile(src: &str, lookup: Option<&dyn Lookup>) -> (Option<(Game, crate::Content)>, Vec<Diag>) {
	let (v, diag) = diag::diagnose(|| {
		let tok = lex::lex(src);
		lower::parse(&tok, lookup)
	});
	if diag.iter().any(|a| a.is_fatal()) {
		(None, diag)
	} else {
		(Some(v.expect("no error")), diag)
	}
}
