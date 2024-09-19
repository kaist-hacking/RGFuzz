//! The model of the WebAssembly syntax.

mod errors;
pub mod indices;
pub mod instruction;
pub mod module;
pub mod types;
pub mod values;

pub use errors::ModelError;
pub use indices::*;
pub use instruction::*;
pub use module::*;
pub use types::*;
pub use values::*;
