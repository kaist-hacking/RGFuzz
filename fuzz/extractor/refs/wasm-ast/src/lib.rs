//! A Rust-native WebAssembly syntax model useful for generating, parsing, and emitting WebAssembly code.

pub mod leb128;
pub mod model;

#[cfg(feature = "emitter")]
pub mod emitter;

#[cfg(feature = "parser")]
pub mod parser;

pub use model::*;

#[cfg(feature = "emitter")]
pub use emitter::*;

#[cfg(feature = "parser")]
pub use parser::*;
