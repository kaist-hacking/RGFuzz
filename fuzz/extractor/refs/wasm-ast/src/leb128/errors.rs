/// An error in LEB128 encoding or decoding.
#[derive(thiserror::Error, Debug)]
pub enum LEB128Error {
    #[error("The given integer type does not have sufficient capacity to store the parsed integer without overflow.")]
    Conversion(#[from] std::num::TryFromIntError),
    #[error("The parsed integer requires {0} bytes to be stored without overflow, but only {1} are available.")]
    Overflow(usize, usize),
    #[error("The given input does not contain a valid LEB128-encoded integer.")]
    Invalid,
    #[error("Failed to write to the given output.")]
    WriteFailure(#[from] std::io::Error),
}
