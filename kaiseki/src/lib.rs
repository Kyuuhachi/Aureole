pub mod ed6 {
	pub mod archive;
	pub mod magic;
	pub use archive::{Archive,Archives};
}
mod decompress;
mod util;
