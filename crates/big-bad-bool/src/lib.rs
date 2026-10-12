#![forbid(unsafe_code)]
//! Public facade for Big Bad Bool packed schemas.
//!
//! Applications normally need only `big-bad-bool`. Runtime metadata lives in
//! `big-bad-bool-core`; the procedural macro implementation lives in
//! `big-bad-bool-derive`.

extern crate self as big_bad_bool;

pub use big_bad_bool_core as core;
pub use big_bad_bool_core::*;
pub use big_bad_bool_derive as derive;
pub use big_bad_bool_derive::bitpack;
