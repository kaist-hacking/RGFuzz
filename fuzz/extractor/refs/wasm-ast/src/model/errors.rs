use thiserror::Error;

/// An error in a WebAssembly module model.
#[derive(Error, Debug)]
pub enum ModelError {
    #[error("The module does not have enough space to add the given component. The indices in a WebAssembly module are limited by the capacity of a u32.")]
    IndexOverflow(#[from] std::num::TryFromIntError),
}
