#![feature(array_try_map, array_try_from_fn, array_methods)]

pub mod tables;
#[doc(inline)]
pub use themelios_common::types as types;
pub use themelios_scena::text;
pub mod scena;
pub mod lookup;

pub use themelios_common::util::{ReadError, WriteError};
