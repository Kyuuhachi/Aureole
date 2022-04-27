#![allow(clippy::needless_question_mark)]

pub mod ed6 {
	pub mod archive;
	pub mod magic;
	pub mod scena;
	pub mod code;
	pub use archive::{Archive,Archives};
}
pub mod image;
mod decompress;
mod util;
mod decompile;

pub use util::{ByteString, Text};
