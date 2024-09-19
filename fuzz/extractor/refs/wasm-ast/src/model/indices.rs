//! Definitions are referenced with zero-based indices.
//! Each class of definition has its own index space, as distinguished by the following classes.
//!
//! The index space for functions, tables,
//! memories and globals includes respective imports declared in the same module.
//! The indices of these imports precede the indices of other definitions in the same index space.
//!
//! Element indices reference element segments and data indices reference data segments.
//!
//! The index space for locals is only accessible inside a function and includes the parameters of that function,
//! which precede the local variables.
//!
//! Label indices reference structured control instructions inside an instruction sequence.
//!
//! See <https://webassembly.github.io/spec/core/syntax/modules.html#indices>

pub type TypeIndex = u32;
pub type FunctionIndex = u32;
pub type TableIndex = u32;
pub type MemoryIndex = u32;
pub type GlobalIndex = u32;
pub type ElementIndex = u32;
pub type DataIndex = u32;
pub type LocalIndex = u32;
pub type LabelIndex = u32;
