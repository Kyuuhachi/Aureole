#![feature(let_chains)]
#![feature(pattern)]
#![feature(decl_macro)]
#![feature(try_blocks)]

pub mod ed6;
pub mod ed7;
mod writer;
pub mod common;
pub mod util;

pub use writer::Context;

pub mod span;
pub mod parse;
