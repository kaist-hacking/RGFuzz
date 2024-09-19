//! Little-Endian Base 128 encoding and decoding of signed and unsigned integers.

mod errors;

pub use errors::LEB128Error;

use std::convert::TryFrom;
use std::io::Write;
use std::mem::size_of;

/// The radix (i.e. base) for LEB128 encoding.
const RADIX: u8 = 128;

/// The number of bits per LEB128 encoding group.
const GROUP_BITS: usize = 7;

/// The zero-indexed index of the sign bit per LEB128 encoding.
const SIGN_BIT: usize = 6;

/// Maximum size (in bytes) of an LEB128-encoded integer type
///
/// See <https://en.wikipedia.org/wiki/LEB128>
const fn max_leb128_size<T>() -> usize {
    let bits = size_of::<T>() * 8;

    (bits / 7) + (bits % 7 != 0) as usize
}

trait Bits: Copy + Sized {
    /// Sets the given bit to zero.
    fn zero_bit_at(&self, bit: usize) -> Self;

    /// Sets the given bit to one.
    fn one_bit_at(&self, bit: usize) -> Self;

    /// Gets the given bit at the index.
    fn bit_at(&self, bit: usize) -> bool;
}

impl Bits for u8 {
    /// Sets the given bit to zero.
    fn zero_bit_at(&self, bit: usize) -> u8 {
        self & !(1 << bit)
    }

    /// Sets the given bit to one.
    fn one_bit_at(&self, bit: usize) -> u8 {
        self | 1 << bit
    }

    /// Gets the given bit to one.
    fn bit_at(&self, bit: usize) -> bool {
        let mask = 1 << bit;
        self & mask == mask
    }
}

/// Parses an unsigned integer using LEB128 (Little-Endian Base 128) encoding.
/// Returns the parsed integer and the remaining input.
///
/// See <https://en.wikipedia.org/wiki/LEB128>
pub fn parse_unsigned<T>(input: &[u8]) -> Result<(&[u8], T), LEB128Error>
where
    T: TryFrom<u128, Error = std::num::TryFromIntError>,
{
    let end = input.iter().position(|x| x & RADIX == 0);
    let max_size = max_leb128_size::<T>();
    let length = match end {
        Some(index) if index > max_size => Err(LEB128Error::Overflow(index, max_size)),
        Some(index) => Ok(index + 1),
        None => Err(LEB128Error::Invalid),
    }?;

    let mut result = 0;
    for (index, &byte) in input[..length].iter().enumerate() {
        let group = byte.zero_bit_at(GROUP_BITS) as u128;

        result |= group << (index * GROUP_BITS);
    }

    Ok((&input[length..], T::try_from(result)?))
}

/// Encodes an unsigned integer using LEB128 (Little-Endian Base 128) encoding.
///
/// See <https://en.wikipedia.org/wiki/LEB128>
pub fn encode_unsigned<I, O: Write>(input: I, mut output: O) -> Result<usize, LEB128Error>
where
    I: Into<u128>,
{
    let mut value = input.into();
    let mut written = 0;

    loop {
        let mut byte = (value as u8).zero_bit_at(GROUP_BITS);
        value >>= GROUP_BITS;

        if value != 0 {
            byte = byte.one_bit_at(GROUP_BITS);
        }

        output.write_all(&[byte])?;
        written += 1;

        if value == 0 {
            break;
        }
    }

    Ok(written)
}

/// Parses a signed integer using LEB128 (Little-Endian Base 128) encoding.
/// Returns the parsed integer and the remaining input.
///
/// See <https://en.wikipedia.org/wiki/LEB128>
pub fn parse_signed<T>(input: &[u8]) -> Result<(&[u8], T), LEB128Error>
where
    T: TryFrom<i128, Error = std::num::TryFromIntError>,
{
    let end = input.iter().position(|x| x & RADIX == 0);
    let max_size = max_leb128_size::<T>();
    let length = match end {
        Some(index) if index > max_size => Err(LEB128Error::Overflow(index, max_size)),
        Some(index) => Ok(index + 1),
        None => Err(LEB128Error::Invalid),
    }?;

    let mut result = 0;
    let remaining = &input[length..];
    let input = &input[..length];
    for (index, &byte) in input.iter().enumerate() {
        let group = byte.zero_bit_at(GROUP_BITS) as i128;

        result |= group << (index * GROUP_BITS);
    }

    if let Some(byte) = input.iter().last() {
        if byte.bit_at(SIGN_BIT) {
            result |= !0 << (length * GROUP_BITS);
        }
    }

    Ok((remaining, T::try_from(result)?))
}

/// Encodes a signed integer using LEB128 (Little-Endian Base 128) encoding.
///
/// See <https://en.wikipedia.org/wiki/LEB128>
pub fn encode_signed<I, O: Write>(input: I, mut output: O) -> Result<usize, LEB128Error>
where
    I: Into<i128>,
{
    let mut value = input.into();
    let mut written = 0;

    loop {
        let mut byte = (value as u8).zero_bit_at(GROUP_BITS);
        value >>= GROUP_BITS;

        if value != 0 {
            byte = byte.one_bit_at(GROUP_BITS);
        }

        output.write_all(&[byte])?;
        written += 1;

        if value == 0 {
            break;
        }
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_unsigned_leb128_large() {
        let input = vec![0xE5, 0x8E, 0x26];
        let (remaining, actual): (&[u8], u32) = parse_unsigned(input.as_slice()).unwrap();

        assert_eq!(actual, 624485);
        assert!(remaining.is_empty())
    }

    #[test]
    fn parse_unsigned_leb128_small() {
        let input = vec![64, 0xFF];
        let (remaining, actual): (&[u8], u8) = parse_unsigned(input.as_slice()).unwrap();

        assert_eq!(actual, 64);
        assert_eq!(remaining, &[0xFF])
    }

    #[test]
    fn parse_unsigned_leb128_zero() {
        let input = vec![0x00, 0xFF];
        let (remaining, actual): (&[u8], usize) = parse_unsigned(input.as_slice()).unwrap();

        assert_eq!(actual, 0);
        assert_eq!(remaining, &[0xFF])
    }

    #[test]
    fn encode_unsigned_leb128_large() {
        let mut output = Vec::new();
        let written = encode_unsigned(624485u128, &mut output).unwrap();

        assert_eq!(written, 3);
        assert_eq!(output, vec![0xE5, 0x8E, 0x26]);
    }

    #[test]
    fn encode_unsigned_leb128_small() {
        let input = 64;
        let mut output = Vec::new();
        let written = encode_unsigned(input, &mut output).unwrap();

        assert_eq!(written, 1);
        assert_eq!(output, vec![input]);
    }

    #[test]
    fn encode_unsigned_leb128_zero() {
        let input = 0;
        let mut output = Vec::new();
        let written = encode_unsigned(input, &mut output).unwrap();

        assert_eq!(written, 1);
        assert_eq!(output, vec![input]);
    }

    #[test]
    fn parse_signed_leb128_large() {
        let input = vec![0xC0, 0xBB, 0x78];
        let (remaining, actual): (&[u8], i32) = parse_signed(input.as_slice()).unwrap();

        println!("{} = {:b}", -123456, -123456);
        println!("{} = {:b}", actual, actual);
        assert_eq!(actual, -123456);
        assert!(remaining.is_empty())
    }

    #[test]
    fn parse_signed_leb128_small() {
        let input = vec![32, 0xFF];
        let (remaining, actual): (&[u8], i8) = parse_signed(input.as_slice()).unwrap();

        assert_eq!(actual, 32);
        assert_eq!(remaining, &[0xFF])
    }

    #[test]
    fn parse_signed_leb128_zero() {
        let input = vec![0x00, 0xFF];
        let (remaining, actual): (&[u8], u64) = parse_signed(input.as_slice()).unwrap();

        assert_eq!(actual, 0);
        assert_eq!(remaining, &[0xFF])
    }

    #[test]
    fn encode_signed_leb128_large() {
        let mut output = Vec::new();
        let written = encode_signed(624485i128, &mut output).unwrap();

        assert_eq!(written, 3);
        assert_eq!(output, vec![0xE5, 0x8E, 0x26]);
    }

    #[test]
    fn encode_signed_leb128_small() {
        let input = 64;
        let mut output = Vec::new();
        let written = encode_signed(input, &mut output).unwrap();

        assert_eq!(written, 1);
        assert_eq!(output, vec![input]);
    }

    #[test]
    fn encode_signed_leb128_zero() {
        let input = 0;
        let mut output = Vec::new();
        let written = encode_signed(input, &mut output).unwrap();

        assert_eq!(written, 1);
        assert_eq!(output, vec![input]);
    }
}
