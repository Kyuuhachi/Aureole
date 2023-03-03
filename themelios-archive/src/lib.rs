//! ED6 and ED7 often use 32-bit numbers to denote files, probably for performance reasons.
//! This crate contains functions for converting between these file ids and filenames.

pub mod lookup;
pub mod dirdat;
pub use lookup::{Lookup, ED6Lookup, ED7Lookup, NullLookup};
