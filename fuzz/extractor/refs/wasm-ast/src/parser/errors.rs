use thiserror::Error;

/// An error in parser a WebAssembly module.
#[derive(Error, Debug)]
pub enum ParseError {
    #[cfg(feature = "text")]
    #[error(
        "The WebAssembly module in text format could not be transformed to the binary format."
    )]
    InvalidText(#[from] wat::Error),
    #[error("The WebAssembly module is not a valid binary format.")]
    InvalidBinary,
    #[error(
        "The module's type and code sections have different lengths (type: {0:?}, code: {1:?})."
    )]
    MismatchedFunctionParts(Option<usize>, Option<usize>),
}

/// Create a parse error from a nom error.
impl<T> From<nom::Err<nom::error::Error<T>>> for ParseError {
    fn from(_: nom::Err<nom::error::Error<T>>) -> Self {
        ParseError::InvalidBinary
    }
}
