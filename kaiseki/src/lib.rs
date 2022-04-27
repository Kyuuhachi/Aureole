#![allow(clippy::needless_question_mark)]

pub mod ed6 {
	pub mod archive;
	pub mod magic;
	pub mod scena;
	pub mod code;
	pub use archive::{Archive,Archives};
}
pub mod image;
pub mod util;

mod decompress;
mod decompile;
