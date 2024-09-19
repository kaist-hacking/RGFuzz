use crate::emitter::errors::EmitError;
use crate::leb128::{encode_signed, encode_unsigned};
use crate::model::Name;
use std::borrow::Borrow;
use std::convert::TryFrom;
use std::io::Write;
use std::iter::IntoIterator;

/// Emit a 32-bit floating point to the output.
///
/// See https://webassembly.github.io/spec/core/binary/values.html#floating-point
pub fn emit_f32<T: Borrow<f32>, O: Write + ?Sized>(
    value: T,
    output: &mut O,
) -> Result<usize, EmitError> {
    let bytes = value.borrow().to_le_bytes();
    output.write_all(&bytes)?;
    Ok(bytes.len())
}

/// Emit a 64-bit floating point to the output.
///
/// See https://webassembly.github.io/spec/core/binary/values.html#floating-point
pub fn emit_f64<T: Borrow<f64>, O: Write + ?Sized>(
    value: T,
    output: &mut O,
) -> Result<usize, EmitError> {
    let bytes = value.borrow().to_le_bytes();
    output.write_all(&bytes)?;
    Ok(bytes.len())
}

/// Emit a name to the output.
///
/// See https://webassembly.github.io/spec/core/binary/values.html#names
pub fn emit_name<O: Write + ?Sized>(value: &Name, output: &mut O) -> Result<usize, EmitError> {
    emit_bytes(value.as_bytes(), output, true)
}

/// Emits a single byte to the output.
///
/// See https://webassembly.github.io/spec/core/binary/values.html#bytes
pub fn emit_byte<T: Borrow<u8>, O: Write + ?Sized>(
    byte: T,
    output: &mut O,
) -> Result<usize, EmitError> {
    let bytes = [*byte.borrow()];
    output.write_all(&bytes)?;
    Ok(bytes.len())
}

/// Emits a slice of bytes to the output.
/// The bytes may optionally be treated as a vector.
/// Provides an optimization over using `emit_vector(value, output, emit_byte)` and `emit_repeated(value, output, emit_byte)`.
///
/// See https://webassembly.github.io/spec/core/binary/values.html#bytes
///
/// See https://webassembly.github.io/spec/core/binary/conventions.html#vectors
pub fn emit_bytes<O: Write + ?Sized>(
    value: &[u8],
    output: &mut O,
    include_length: bool,
) -> Result<usize, EmitError> {
    let prefix = if include_length {
        emit_usize(value.len(), output)?
    } else {
        0
    };

    output.write_all(value)?;

    Ok(prefix + value.len())
}

/// Emits an unsigned 32-bit integer to the output.
///
/// See https://webassembly.github.io/spec/core/binary/values.html#integers
pub fn emit_u32<T: Borrow<u32>, O: Write + ?Sized>(
    value: T,
    output: &mut O,
) -> Result<usize, EmitError> {
    Ok(encode_unsigned(*value.borrow(), output)?)
}

/// Emits an unsigned platform-specific (i.e., 32-bit or 64-bit) integer to the output.
///
/// See https://webassembly.github.io/spec/core/binary/values.html#integers
pub fn emit_usize<T: Borrow<usize>, O: Write + ?Sized>(
    size: T,
    output: &mut O,
) -> Result<usize, EmitError> {
    Ok(encode_unsigned(u128::try_from(*size.borrow())?, output)?)
}

/// Emits a signed 32-bit integer to the output.
///
/// See https://webassembly.github.io/spec/core/binary/values.html#integers
pub fn emit_i32<T: Borrow<i32>, O: Write + ?Sized>(
    value: T,
    output: &mut O,
) -> Result<usize, EmitError> {
    Ok(encode_signed(*value.borrow(), output)?)
}

/// Emits a signed 64-bit integer to the output.
///
/// See https://webassembly.github.io/spec/core/binary/values.html#integers
pub fn emit_i64<T: Borrow<i64>, O: Write + ?Sized>(
    value: T,
    output: &mut O,
) -> Result<usize, EmitError> {
    Ok(encode_signed(*value.borrow(), output)?)
}

/// Emits a i128 vector to the output
pub fn emit_i128<T: Borrow<i128>, O: Write + ?Sized>(
    value: T,
    output: &mut O,
) -> Result<usize, EmitError> {
    let bytes = value.borrow().to_le_bytes();
    output.write_all(&bytes)?;
    Ok(bytes.len())
}

/// Emit each item to the output using the given emit function.
/// Prefixes the items with the length of the slice.
///
/// See https://webassembly.github.io/spec/core/binary/conventions.html#vectors
pub fn emit_vector<I, E, O>(items: I, output: &mut O, emit: E) -> Result<usize, EmitError>
where
    I: IntoIterator,
    <I as IntoIterator>::IntoIter: Clone,
    O: Write + ?Sized,
    E: Fn(I::Item, &mut O) -> Result<usize, EmitError>,
{
    emit_iterator(items, output, true, emit)
}

/// Emit each item to the output using the given emit function.
pub fn emit_repeated<I, E, O>(items: I, output: &mut O, emit: E) -> Result<usize, EmitError>
where
    I: IntoIterator,
    <I as IntoIterator>::IntoIter: Clone,
    O: Write + ?Sized,
    E: Fn(I::Item, &mut O) -> Result<usize, EmitError>,
{
    emit_iterator(items, output, false, emit)
}

/// Emit each item to the output using the given emit function.
fn emit_iterator<I, E, O>(
    items: I,
    output: &mut O,
    include_length: bool,
    emit: E,
) -> Result<usize, EmitError>
where
    I: IntoIterator,
    <I as IntoIterator>::IntoIter: Clone,
    O: Write + ?Sized,
    E: Fn(I::Item, &mut O) -> Result<usize, EmitError>,
{
    let mut bytes = 0;
    let items = items.into_iter();

    if include_length {
        bytes += emit_usize(items.clone().count(), output)?;
    }

    for item in items {
        bytes += emit(item, output)?;
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vectored() {
        let bytes: [u8; 4] = [1, 2, 3, 4];
        let mut buffer: Vec<u8> = Vec::new();

        let emitted = emit_vector(&bytes, &mut buffer, emit_byte).unwrap();

        assert_eq!(emitted, 1 + bytes.len());
        assert_eq!(buffer[0] as usize, bytes.len());
        assert_eq!(&bytes[..], &buffer[1..]);
    }
}
