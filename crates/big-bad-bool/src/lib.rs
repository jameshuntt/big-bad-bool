#![forbid(unsafe_code)]
//! Public facade for Big Bad Bool packed schemas.
//!
//! Applications normally need only `big-bad-bool`. Runtime metadata lives in
//! `big-bad-bool-core`; the procedural macro implementation lives in
//! `big-bad-bool-derive`.
//!
//! Optional: the `program` feature adds `big_bad_bool::program`, the
//! `big-bad-bool-program` crate (dimensions, contracts, verification, framing
//! and stepping over packed words), and `program-bowl` also turns on its
//! ob-serve-a-bowl adapter, which needs Rust 1.88. `#[bitpack]` and the core
//! metadata need neither.

extern crate self as big_bad_bool;

pub use big_bad_bool_core as core;
pub use big_bad_bool_core::*;
pub use big_bad_bool_derive as derive;
pub use big_bad_bool_derive::bitpack;

/// Programs over packed words: dimensions, contracts across them, bounded
/// verification, optional framing and controlled stepping. This is the
/// `big-bad-bool-program` crate, re-exported whole; enable the `program`
/// feature.
#[cfg(feature = "program")]
pub use big_bad_bool_program as program;
