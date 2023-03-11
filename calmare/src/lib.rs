#![feature(let_chains)]
#![feature(pattern)]
#![feature(decl_macro)]
#![feature(try_blocks)]
#![feature(array_try_map)]

pub mod ed6;
pub mod ed7;
mod writer;
pub mod common;

use themelios::{types::Game, lookup::Lookup};
pub use writer::Context;

pub mod span;
pub mod parse;

#[derive(Debug, Clone)]
pub enum Content {
	ED6Scena(themelios::scena::ed6::Scena),
	ED7Scena(themelios::scena::ed7::Scena),
}

pub fn to_string(game: Game, c: &Content, lookup: Option<&dyn Lookup>) -> String {
	let mut ctx = Context::new(game, lookup);
	match c {
		Content::ED6Scena(scena) => ed6::write(&mut ctx, scena),
		Content::ED7Scena(scena) => ed7::write(&mut ctx, scena),
	}
	ctx.finish()
}

pub fn parse(src: &str, lookup: Option<&dyn Lookup>) -> (Option<(Game, crate::Content)>, Vec<parse::Diag>) {
	let (v, diag) = parse::diag::diagnose(|| {
		let tok = parse::lex::lex(src);
		parse::lower::parse(&tok, lookup)
	});
	if diag.iter().any(|a| a.is_fatal()) {
		(None, diag)
	} else {
		(Some(v.expect("no error")), diag)
	}
}
