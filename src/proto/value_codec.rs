//! This module provides encoding and decoding functionality for basic values.
//!
//! The protocol is pretty basic, though quirky. Base types are serialized in
//! the following way:
//!
//!   * 32-bit integers are serialized in 4 bytes, little-endian.
//!   * 16-bit integers are serialized as 32-bit integers with upper bytes set
//!     to 0.
//!   * Booleans are serialized as single bytes, containing either 0 or 1.
//!   * IPv4 addresses are serialized as 32-bit integers.
//!   * Strings are serialized as 32-bit-length-prefixed arrays of Windows 1252
//!     encoded characters.
//!   * Pairs are serialized as two consecutive values.
//!   * Vectors are serialized as length-prefixed arrays of serialized values.

use std::io;
use std::net;

use crate::proto::prefix::Prefixer;
use crate::proto::u32::{decode_u32, encode_u32, U32_BYTE_LEN};
use encoding::all::WINDOWS_1252;
use encoding::{DecoderTrap, EncoderTrap, Encoding};
use std::convert::{TryFrom, TryInto};
use thiserror::Error;

pub trait Decode<T> {
    /// Attempts to decode an instance of `T` from `self`.
    fn decode(&mut self) -> io::Result<T>;
}

pub trait Encode<T> {
    /// Attempts to encode `value` into `self`.
    fn encode(&mut self, value: T) -> io::Result<()>;
}

// TODO: Add backtrace fields to each enum variant once std::backtrace is
// stabilized.
#[derive(PartialEq, Error, Debug)]
pub enum ValueDecodeError {
    #[error("at position {position}: not enough bytes to decode: expected {expected}, found {remaining}")]
    NotEnoughData {
        /// The number of bytes the decoder expected to read.
        ///
        /// Invariant: `remaining < expected`.
        expected: usize,

        /// The number of bytes remaining in the input buffer.
        ///
        /// Invariant: `remaining < expected`.
        remaining: usize,

        /// The decoder's position in the input buffer.
        position: usize,
    },
    #[error("at position {position}: invalid boolean value: {value}")]
    InvalidBool {
        /// The invalid value. Never equal to 0 nor 1.
        value: u8,

        /// The decoder's position in the input buffer.
        position: usize,
    },
    #[error("at position {position}: invalid u16 value: {value}")]
    InvalidU16 {
        /// The invalid value. Always greater than u16::max_value().
        value: u32,

        /// The decoder's position in the input buffer.
        position: usize,
    },
    #[error("at position {position}: failed to decode string: {cause}")]
    InvalidString {
        /// The cause of the error, as reported by the encoding library.
        cause: String,

        /// The decoder's position in the input buffer.
        position: usize,
    },
    #[error("at position {position}: invalid {value_name}: {cause}")]
    InvalidData {
        /// The name of the value the decoder failed to decode.
        value_name: String,

        /// The cause of the error.
        cause: String,

        /// The decoder's position in the input buffer.
        position: usize,
    },
}

impl From<ValueDecodeError> for io::Error {
    fn from(error: ValueDecodeError) -> Self {
        let kind = match &error {
            &ValueDecodeError::NotEnoughData { .. } => io::ErrorKind::UnexpectedEof,
            _ => io::ErrorKind::InvalidData,
        };
        let message = format!("{}", &error);
        io::Error::new(kind, message)
    }
}

/// A type for decoding various types of values from protocol messages.
pub struct ValueDecoder<'a> {
    // The buffer we are decoding from.
    //
    // Invariant: `position <= buffer.len()`.
    buffer: &'a [u8],

    // Our current position within `buffer`.
    //
    // We could instead maintain this implicitly in `buffer` by splitting off
    // decoded bytes from the start of the buffer, but we would then be unable
    // to remember how many bytes we had decoded. This information is useful to
    // have in error messages when encountering decoding errors.
    //
    // Invariant: `position <= buffer.len()`.
    position: usize,
}

/// This trait is implemented by types that can be decoded from messages using
/// a `ValueDecoder`.
pub trait ValueDecode: Sized {
    /// Attempts to decode a value of this type with the given decoder.
    fn decode_from(decoder: &mut ValueDecoder) -> Result<Self, ValueDecodeError>;
}

impl<'a> ValueDecoder<'a> {
    /// Wraps the given byte buffer.
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            buffer: buffer,
            position: 0,
        }
    }

    /// The current position of this decoder in the input buffer.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Returns the number of bytes remaining to decode.
    pub fn remaining(&self) -> usize {
        self.buffer.len() - self.position
    }

    /// Returns whether the underlying buffer has remaining bytes to decode.
    ///
    /// Shorthand for `remaining() > 0`.
    pub fn has_remaining(&self) -> bool {
        self.remaining() > 0
    }

    /// Returns a read-only view of the remaining bytes to decode.
    ///
    /// The returned slice is of size `remaining()`.
    pub fn bytes(&self) -> &[u8] {
        &self.buffer[self.position..]
    }

    /// Attempts to consume the next `n` bytes from this buffer.
    ///
    /// Returns a slice of size `n` if successful, in which case this decoder
    /// advances its internal position by `n`.
    fn consume(&mut self, n: usize) -> Result<&[u8], ValueDecodeError> {
        if self.remaining() < n {
            return Err(ValueDecodeError::NotEnoughData {
                expected: n,
                remaining: self.remaining(),
                position: self.position,
            });
        }

        // Cannot use bytes() here as it borrows self immutably, which
        // prevents us from mutating self.position afterwards.
        let end = self.position + n;
        let bytes = &self.buffer[self.position..end];
        self.position = end;
        Ok(bytes)
    }

    /// Attempts to decode a u32 value.
    fn decode_u32(&mut self) -> Result<u32, ValueDecodeError> {
        let bytes = self.consume(U32_BYTE_LEN)?;
        // The conversion from slice to fixed-size array cannot fail, because
        // consume() guarantees that its return value is of size n.
        let array: [u8; U32_BYTE_LEN] = bytes.try_into().unwrap();
        Ok(decode_u32(array))
    }

    fn decode_u16(&mut self) -> Result<u16, ValueDecodeError> {
        let position = self.position;
        let n = self.decode_u32()?;
        match u16::try_from(n) {
            Ok(value) => Ok(value),
            Err(_) => Err(ValueDecodeError::InvalidU16 {
                value: n,
                position: position,
            }),
        }
    }

    /// Attempts to decode a boolean value.
    fn decode_bool(&mut self) -> Result<bool, ValueDecodeError> {
        let position = self.position;
        let bytes = self.consume(1)?;
        match bytes[0] {
            0 => Ok(false),
            1 => Ok(true),
            n => Err(ValueDecodeError::InvalidBool {
                value: n,
                position: position,
            }),
        }
    }

    /// Attempts to decode a string value.
    fn decode_string(&mut self) -> Result<String, ValueDecodeError> {
        let length = self.decode_u32()? as usize;

        let position = self.position;
        let bytes = self.consume(length)?;

        let result = WINDOWS_1252.decode(bytes, DecoderTrap::Strict);
        match result {
            Ok(string) => Ok(string),
            Err(error) => Err(ValueDecodeError::InvalidString {
                cause: error.to_string(),
                position: position,
            }),
        }
    }

    /// Attempts to decode a value of the given type.
    ///
    /// Allows easy decoding of complex values using type inference:
    ///
    /// ```
    /// let val: Foo = decoder.decode()?;
    /// ```
    pub fn decode<T: ValueDecode>(&mut self) -> Result<T, ValueDecodeError> {
        T::decode_from(self)
    }
}

impl ValueDecode for u32 {
    fn decode_from(decoder: &mut ValueDecoder) -> Result<Self, ValueDecodeError> {
        decoder.decode_u32()
    }
}

impl ValueDecode for u16 {
    fn decode_from(decoder: &mut ValueDecoder) -> Result<Self, ValueDecodeError> {
        decoder.decode_u16()
    }
}

impl ValueDecode for bool {
    fn decode_from(decoder: &mut ValueDecoder) -> Result<Self, ValueDecodeError> {
        decoder.decode_bool()
    }
}

impl ValueDecode for net::Ipv4Addr {
    fn decode_from(decoder: &mut ValueDecoder) -> Result<Self, ValueDecodeError> {
        let ip = decoder.decode_u32()?;
        Ok(net::Ipv4Addr::from(ip))
    }
}

impl ValueDecode for String {
    fn decode_from(decoder: &mut ValueDecoder) -> Result<Self, ValueDecodeError> {
        decoder.decode_string()
    }
}

impl<T: ValueDecode, U: ValueDecode> ValueDecode for (T, U) {
    fn decode_from(decoder: &mut ValueDecoder) -> Result<Self, ValueDecodeError> {
        let first = decoder.decode()?;
        let second = decoder.decode()?;
        Ok((first, second))
    }
}

impl<T: ValueDecode> ValueDecode for Vec<T> {
    fn decode_from(decoder: &mut ValueDecoder) -> Result<Self, ValueDecodeError> {
        let len = decoder.decode_u32()? as usize;
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            let val = decoder.decode()?;
            vec.push(val);
        }
        Ok(vec)
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum ValueEncodeError {
    #[error("encoded string length {length} is too large: {string:?}")]
    StringTooLong {
        /// The string that is too long to encode.
        string: String,

        /// The length of `string` in the Windows-1252 encoding.
        /// Always larger than `u32::max_value()`.
        length: usize,
    },
}

impl From<ValueEncodeError> for io::Error {
    fn from(error: ValueEncodeError) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, format!("{}", error))
    }
}

/// A type for encoding various types of values into protocol messages.
pub struct ValueEncoder<'a> {
    /// The buffer to which the encoder appends encoded bytes.
    buffer: &'a mut Vec<u8>,
}

/// This trait is implemented by types that can be encoded into messages using
/// a `ValueEncoder`.
pub trait ValueEncode {
    // TODO: Rename to encode_to().
    /// Attempts to encode `self` with the given encoder.
    fn encode(&self, encoder: &mut ValueEncoder) -> Result<(), ValueEncodeError>;
}

impl<'a> ValueEncoder<'a> {
    /// Wraps the given buffer for encoding values into.
    ///
    /// Encoded bytes are appended. The buffer is not pre-cleared.
    pub fn new(buffer: &'a mut Vec<u8>) -> Self {
        ValueEncoder { buffer: buffer }
    }

    /// Encodes the given u32 value into the underlying buffer.
    pub fn encode_u32(&mut self, val: u32) -> Result<(), ValueEncodeError> {
        self.buffer.extend_from_slice(&encode_u32(val));
        Ok(())
    }

    /// Encodes the given u16 value into the underlying buffer.
    pub fn encode_u16(&mut self, val: u16) -> Result<(), ValueEncodeError> {
        self.encode_u32(val as u32)
    }

    /// Encodes the given boolean value into the underlying buffer.
    pub fn encode_bool(&mut self, val: bool) -> Result<(), ValueEncodeError> {
        self.buffer.push(val as u8);
        Ok(())
    }

    /// Encodes the given string into the underlying buffer.
    pub fn encode_string(&mut self, val: &str) -> Result<(), ValueEncodeError> {
        // Reserve space for the length prefix.
        let prefixer = Prefixer::new(self.buffer);

        // Encode the string. We are quite certain this cannot fail because we
        // use EncoderTrap::Replace to replace any unencodable characters with
        // '?' which is always encodable in Windows-1252.
        WINDOWS_1252
            .encode_to(val, EncoderTrap::Replace, self.buffer)
            .unwrap();

        // Write the length prefix in the space we initially reserved for it.
        prefixer
            .finalize(self.buffer)
            .map_err(|length| ValueEncodeError::StringTooLong {
                string: val.to_string(),
                length: length,
            })
    }

    /// Encodes the given value into the underlying buffer.
    ///
    /// Allows for easy encoding with type inference:
    /// ```
    /// encoder.encode(&Foo::new(bar))?;
    /// ```
    pub fn encode<T: ValueEncode>(&mut self, val: &T) -> Result<(), ValueEncodeError> {
        val.encode(self)
    }
}

impl ValueEncode for u32 {
    fn encode(&self, encoder: &mut ValueEncoder) -> Result<(), ValueEncodeError> {
        encoder.encode_u32(*self)
    }
}

impl ValueEncode for u16 {
    fn encode(&self, encoder: &mut ValueEncoder) -> Result<(), ValueEncodeError> {
        encoder.encode_u16(*self)
    }
}

impl ValueEncode for bool {
    fn encode(&self, encoder: &mut ValueEncoder) -> Result<(), ValueEncodeError> {
        encoder.encode_bool(*self)
    }
}

impl ValueEncode for net::Ipv4Addr {
    fn encode(&self, encoder: &mut ValueEncoder) -> Result<(), ValueEncodeError> {
        encoder.encode_u32(u32::from(*self))
    }
}

// It would be nice to use AsRef<str>, or Deref<Target=str> for the following
// stringy types instead of having to spell them out, but trying that fails
// because E0119: "upstream crates may add new impl of trait
// `core::convert::AsRef<str>` for type `bool` in future versions".
// We could probably work around this with more complex type logic (e.g.
// wrapping primitive types in a newtype for which we implement
// Proto{De,En}code) but it is not really worth the hassle.

impl ValueEncode for str {
    fn encode(&self, encoder: &mut ValueEncoder) -> Result<(), ValueEncodeError> {
        encoder.encode_string(self)
    }
}

impl ValueEncode for String {
    fn encode(&self, encoder: &mut ValueEncoder) -> Result<(), ValueEncodeError> {
        encoder.encode_string(self)
    }
}

impl<'a> ValueEncode for &'a String {
    fn encode(&self, encoder: &mut ValueEncoder) -> Result<(), ValueEncodeError> {
        encoder.encode_string(*self)
    }
}

impl<T: ValueEncode, U: ValueEncode> ValueEncode for (T, U) {
    fn encode(&self, encoder: &mut ValueEncoder) -> Result<(), ValueEncodeError> {
        self.0.encode(encoder)?;
        self.1.encode(encoder)
    }
}

impl<T: ValueEncode> ValueEncode for [T] {
    fn encode(&self, encoder: &mut ValueEncoder) -> Result<(), ValueEncodeError> {
        encoder.encode_u32(self.len() as u32)?;
        for ref item in self {
            item.encode(encoder)?;
        }
        Ok(())
    }
}

impl<T: ValueEncode> ValueEncode for Vec<T> {
    fn encode(&self, encoder: &mut ValueEncoder) -> Result<(), ValueEncodeError> {
        let slice: &[T] = &*self;
        slice.encode(encoder)
    }
}

/*=======*
 * TESTS *
 *=======*/

#[cfg(test)]
pub mod tests {
    use std::fmt;
    use std::io;
    use std::net;
    use std::u16;
    use std::u32;

    use super::{ValueDecode, ValueDecodeError, ValueDecoder, ValueEncode, ValueEncoder};

    // Declared here because assert_eq!(bytes, &[]) fails to infer types.
    const EMPTY_BYTES: &'static [u8] = &[];

    pub fn roundtrip<T>(input: T)
    where
        T: fmt::Debug + Eq + PartialEq + ValueEncode + ValueDecode,
    {
        let mut bytes = vec![];

        ValueEncoder::new(&mut bytes).encode(&input).unwrap();
        let output = ValueDecoder::new(&bytes).decode::<T>().unwrap();

        assert_eq!(output, input);
    }

    // A few integers and their corresponding byte encodings.
    const U32_ENCODINGS: [(u32, [u8; 4]); 8] = [
        (0, [0, 0, 0, 0]),
        (255, [255, 0, 0, 0]),
        (256, [0, 1, 0, 0]),
        (65535, [255, 255, 0, 0]),
        (65536, [0, 0, 1, 0]),
        (16777215, [255, 255, 255, 0]),
        (16777216, [0, 0, 0, 1]),
        (u32::MAX, [255, 255, 255, 255]),
    ];

    #[test]
    fn encode_u32() {
        for &(val, ref encoded_bytes) in &U32_ENCODINGS {
            let mut bytes = vec![13];
            let mut expected_bytes = vec![13];
            expected_bytes.extend(encoded_bytes);

            ValueEncoder::new(&mut bytes).encode_u32(val).unwrap();
            assert_eq!(bytes, expected_bytes);
        }
    }

    #[test]
    fn decode_u32() {
        for &(expected_val, ref bytes) in &U32_ENCODINGS {
            let buffer = bytes.to_vec();
            let mut decoder = ValueDecoder::new(&buffer);

            let val = decoder.decode::<u32>().unwrap();

            assert_eq!(val, expected_val);
            assert_eq!(decoder.bytes(), EMPTY_BYTES);
        }
    }

    #[test]
    fn roundtrip_u32() {
        for &(val, _) in &U32_ENCODINGS {
            roundtrip(val)
        }
    }

    #[test]
    fn decode_u32_unexpected_eof() {
        let buffer = vec![13];
        let mut decoder = ValueDecoder::new(&buffer);

        let result = decoder.decode::<u32>();

        assert_eq!(
            result,
            Err(ValueDecodeError::NotEnoughData {
                expected: 4,
                remaining: 1,
                position: 0,
            })
        );
        assert_eq!(decoder.bytes(), &[13]);
    }

    #[test]
    fn encode_bool_false() {
        let mut bytes = vec![13];
        ValueEncoder::new(&mut bytes).encode_bool(false).unwrap();
        assert_eq!(bytes, vec![13, 0]);
    }

    #[test]
    fn encode_bool_true() {
        let mut bytes = vec![13];
        ValueEncoder::new(&mut bytes).encode_bool(true).unwrap();
        assert_eq!(bytes, vec![13, 1]);
    }

    #[test]
    fn decode_bool_false() {
        let buffer = vec![0];
        let mut decoder = ValueDecoder::new(&buffer);

        let val = decoder.decode::<bool>().unwrap();

        assert!(!val);
        assert_eq!(decoder.bytes(), EMPTY_BYTES);
    }

    #[test]
    fn decode_bool_true() {
        let buffer = vec![1];
        let mut decoder = ValueDecoder::new(&buffer);

        let val = decoder.decode::<bool>().unwrap();

        assert!(val);
        assert_eq!(decoder.bytes(), EMPTY_BYTES);
    }

    #[test]
    fn decode_bool_invalid() {
        let buffer = vec![42];

        let result = ValueDecoder::new(&buffer).decode::<bool>();

        assert_eq!(
            result,
            Err(ValueDecodeError::InvalidBool {
                value: 42,
                position: 0,
            })
        );
    }

    #[test]
    fn decode_bool_unexpected_eof() {
        let buffer = vec![];

        let result = ValueDecoder::new(&buffer).decode::<bool>();

        assert_eq!(
            result,
            Err(ValueDecodeError::NotEnoughData {
                expected: 1,
                remaining: 0,
                position: 0,
            })
        );
    }

    #[test]
    fn roundtrip_bool() {
        roundtrip(false);
        roundtrip(true);
    }

    #[test]
    fn encode_u16() {
        for &(val, ref encoded_bytes) in &U32_ENCODINGS {
            if val > u16::MAX as u32 {
                continue;
            }

            let mut bytes = vec![13];
            let mut expected_bytes = vec![13];
            expected_bytes.extend(encoded_bytes);

            ValueEncoder::new(&mut bytes).encode(&(val as u16)).unwrap();
            assert_eq!(bytes, expected_bytes);
        }
    }

    #[test]
    fn decode_u16() {
        for &(expected_val, ref buffer) in &U32_ENCODINGS {
            let mut decoder = ValueDecoder::new(buffer);

            if expected_val <= u16::MAX as u32 {
                let val = decoder.decode::<u16>().unwrap();
                assert_eq!(val, expected_val as u16);
                assert_eq!(decoder.bytes(), EMPTY_BYTES);
            } else {
                assert_eq!(
                    decoder.decode::<u16>(),
                    Err(ValueDecodeError::InvalidU16 {
                        value: expected_val,
                        position: 0,
                    })
                );
            }
        }
    }

    #[test]
    fn decode_u16_unexpected_eof() {
        let buffer = vec![];
        let mut decoder = ValueDecoder::new(&buffer);

        let result = decoder.decode::<u16>();

        assert_eq!(
            result,
            Err(ValueDecodeError::NotEnoughData {
                expected: 4,
                remaining: 0,
                position: 0,
            })
        );
    }

    #[test]
    fn roundtrip_u16() {
        for &(val, _) in &U32_ENCODINGS {
            if val <= u16::MAX as u32 {
                roundtrip(val)
            }
        }
    }

    #[test]
    fn encode_ipv4() {
        for &(val, ref encoded_bytes) in &U32_ENCODINGS {
            let mut bytes = vec![13];
            let mut expected_bytes = vec![13];
            expected_bytes.extend(encoded_bytes);

            let addr = net::Ipv4Addr::from(val);
            ValueEncoder::new(&mut bytes).encode(&addr).unwrap();
            assert_eq!(bytes, expected_bytes);
        }
    }

    #[test]
    fn decode_ipv4() {
        for &(expected_val, ref buffer) in &U32_ENCODINGS {
            let mut decoder = ValueDecoder::new(buffer);

            let val = decoder.decode::<net::Ipv4Addr>().unwrap();

            assert_eq!(val, net::Ipv4Addr::from(expected_val));
            assert_eq!(decoder.bytes(), EMPTY_BYTES);
        }
    }

    #[test]
    fn roundtrip_ipv4() {
        for &(val, _) in &U32_ENCODINGS {
            roundtrip(net::Ipv4Addr::from(val))
        }
    }

    // A few strings and their corresponding encodings.
    const STRING_ENCODINGS: [(&'static str, &'static [u8]); 3] = [
        ("", &[0, 0, 0, 0]),
        ("hey!", &[4, 0, 0, 0, 104, 101, 121, 33]),
        // Windows 1252 specific codepoints.
        ("‘’“”€", &[5, 0, 0, 0, 145, 146, 147, 148, 128]),
    ];

    #[test]
    fn encode_string() {
        for &(string, encoded_bytes) in &STRING_ENCODINGS {
            let mut bytes = vec![13];
            let mut expected_bytes = vec![13];
            expected_bytes.extend(encoded_bytes);

            ValueEncoder::new(&mut bytes).encode_string(string).unwrap();
            assert_eq!(bytes, expected_bytes);
        }
    }

    #[test]
    fn encode_string_with_unencodable_characters() {
        let mut bytes = vec![];

        ValueEncoder::new(&mut bytes)
            .encode_string("忠犬ハチ公")
            .unwrap();

        // Characters not in the Windows 1252 codepage are rendered as '?'.
        assert_eq!(bytes, &[5, 0, 0, 0, 63, 63, 63, 63, 63]);

        assert_eq!(ValueDecoder::new(&bytes).decode_string().unwrap(), "?????");
    }

    #[test]
    fn decode_string() {
        for &(expected_string, buffer) in &STRING_ENCODINGS {
            let mut decoder = ValueDecoder::new(&buffer);

            let string = decoder.decode::<String>().unwrap();

            assert_eq!(string, expected_string);
            assert_eq!(decoder.bytes(), EMPTY_BYTES);
        }
    }

    #[test]
    fn roundtrip_string() {
        for &(string, _) in &STRING_ENCODINGS {
            roundtrip(string.to_string())
        }
    }

    #[test]
    fn encode_pair_u32_string() {
        let mut bytes = vec![13];
        let mut expected_bytes = vec![13];

        let (integer, ref expected_integer_bytes) = U32_ENCODINGS[0];
        let (string, expected_string_bytes) = STRING_ENCODINGS[0];

        expected_bytes.extend(expected_integer_bytes);
        expected_bytes.extend(expected_string_bytes);

        ValueEncoder::new(&mut bytes)
            .encode(&(integer, string.to_string()))
            .unwrap();

        assert_eq!(bytes, expected_bytes);
    }

    #[test]
    fn decode_pair_u32_string() {
        let mut buffer = vec![];

        let (expected_integer, ref integer_bytes) = U32_ENCODINGS[0];
        let (expected_string, string_bytes) = STRING_ENCODINGS[0];

        buffer.extend(integer_bytes);
        buffer.extend(string_bytes);

        let mut decoder = ValueDecoder::new(&buffer);

        let pair = decoder.decode::<(u32, String)>().unwrap();

        assert_eq!(pair, (expected_integer, expected_string.to_string()));
        assert_eq!(decoder.bytes(), EMPTY_BYTES);
    }

    #[test]
    fn roundtrip_pair_u32_string() {
        roundtrip((42u32, "hello world!".to_string()))
    }

    #[test]
    fn encode_u32_vector() {
        let mut vec = vec![];
        let mut expected_bytes = vec![13, U32_ENCODINGS.len() as u8, 0, 0, 0];
        for &(val, ref encoded_bytes) in &U32_ENCODINGS {
            vec.push(val);
            expected_bytes.extend(encoded_bytes);
        }

        let mut bytes = vec![13];
        ValueEncoder::new(&mut bytes).encode(&vec).unwrap();

        assert_eq!(bytes, expected_bytes);
    }

    #[test]
    fn decode_u32_vector() {
        let mut expected_vec = vec![];
        let mut buffer = vec![U32_ENCODINGS.len() as u8, 0, 0, 0];
        for &(expected_val, ref encoded_bytes) in &U32_ENCODINGS {
            expected_vec.push(expected_val);
            buffer.extend(encoded_bytes);
        }

        let mut decoder = ValueDecoder::new(&buffer);

        let vec = decoder.decode::<Vec<u32>>().unwrap();

        assert_eq!(vec, expected_vec);
        assert_eq!(decoder.bytes(), EMPTY_BYTES);
    }

    #[test]
    fn roundtrip_u32_vector() {
        roundtrip(vec![0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9])
    }
}
