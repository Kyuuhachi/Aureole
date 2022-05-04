#![allow(clippy::needless_question_mark)]

pub mod ed6 {
	pub mod archive;
	pub mod magic;
	pub mod item;
	pub mod quest;
	pub mod scena;
	pub use archive::{Archive,Archives};
}
pub mod image;
pub mod util;

mod decompress;
mod decompile;
