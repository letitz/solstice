//! This module provides a codec implementation for protocol frames.
//!
//! The goal of this codec is to transform byte streams into value streams.

use std::convert::TryInto;
use std::io;
use std::marker;

use bytes::BytesMut;
use thiserror::Error;

use super::prefix::Prefixer;
use super::u32::{decode_u32, U32_BYTE_LEN};
use super::value_codec::{
    ValueDecode, ValueDecodeError, ValueDecoder, ValueEncode, ValueEncodeError, ValueEncoder,
};

#[derive(Debug, Error, PartialEq)]
pub enum FrameEncodeError {
    #[error("encoded value length {length} is too large")]
    ValueTooLarge {
        /// The length of the encoded value.
        length: usize,
    },

    #[error("failed to encode value: {0}")]
    ValueEncodeError(#[from] ValueEncodeError),
}

impl From<FrameEncodeError> for io::Error {
    fn from(error: FrameEncodeError) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, format!("{}", error))
    }
}

/// Encodes entire protocol frames containing values of type `T`.
pub struct FrameEncoder<T: ?Sized> {
    phantom: marker::PhantomData<T>,
}

impl<T: ValueEncode + ?Sized> FrameEncoder<T> {
    pub fn new() -> Self {
        Self {
            phantom: marker::PhantomData,
        }
    }

    pub fn encode_to(&mut self, value: &T, buffer: &mut BytesMut) -> Result<(), FrameEncodeError> {
        let mut prefixer = Prefixer::new(buffer);

        ValueEncoder::new(prefixer.suffix_mut()).encode(value)?;

        if let Err(prefixer) = prefixer.finalize() {
            return Err(FrameEncodeError::ValueTooLarge {
                length: prefixer.suffix().len(),
            });
        }

        Ok(())
    }
}

/// Decodes entire protocol frames containing values of type `T`.
pub struct FrameDecoder<T> {
    // Only here to enable parameterizing `Decoder` by `T`.
    phantom: marker::PhantomData<T>,
}

impl<T: ValueDecode> FrameDecoder<T> {
    pub fn new() -> Self {
        Self {
            phantom: marker::PhantomData,
        }
    }

    /// Attempts to decode an entire frame from the given buffer.
    ///
    /// Returns `Ok(Some(frame))` if successful, in which case the frame's bytes
    /// have been split off from the left of `bytes`.
    ///
    /// Returns `Ok(None)` if not enough bytes are available to decode an entire
    /// frame yet, in which case `bytes` is untouched.
    ///
    /// Returns an error if the length prefix or the framed value are malformed,
    /// in which case `bytes` is untouched.
    pub fn decode_from(&mut self, bytes: &mut BytesMut) -> Result<Option<T>, ValueDecodeError> {
        if bytes.len() < U32_BYTE_LEN {
            return Ok(None); // Not enough bytes yet.
        }

        // Split the prefix off. After this:
        //
        //  | bytes (len 4) | suffix |
        //
        // NOTE: This method would be simpler if we could use split_to() instead
        // here such that `bytes` contained the suffix. At the end, we would not
        // have to replace `bytes` with `suffix`. However, that would require
        // calling `prefix.unsplit(*bytes)`, and that does not work since
        // `bytes` is only borrowed, and unsplit() takes its argument by value.
        let mut suffix = bytes.split_off(U32_BYTE_LEN);

        // unwrap() cannot panic because `bytes` is of the exact right length.
        let array: [u8; U32_BYTE_LEN] = bytes.as_ref().try_into().unwrap();
        let length = decode_u32(array) as usize;

        if suffix.len() < length {
            // Re-assemble `bytes` as it first was.
            bytes.unsplit(suffix);
            return Ok(None); // Not enough bytes yet.
        }

        // Split off the right amount of bytes from the buffer. After this:
        //
        //   | bytes (len 4) | contents | suffix |
        //
        let mut contents = suffix.split_to(length);

        // Attempt to decode the value.
        let item = match ValueDecoder::new(&contents).decode() {
            Ok(item) => item,
            Err(error) => {
                // Re-assemble `bytes` as it first was.
                contents.unsplit(suffix);
                bytes.unsplit(contents);
                return Err(error);
            }
        };

        // Remove the decoded bytes from the left of `bytes`.
        *bytes = suffix;
        Ok(Some(item))
    }
}

mod tests {
    use bytes::BytesMut;

    use super::{FrameDecoder, FrameEncoder};

    // Test value: [1, 3, 3, 7] in little-endian.
    const U32_1337: u32 = 1 + (3 << 8) + (3 << 16) + (7 << 24);

    #[test]
    fn encode_u32() {
        let mut bytes = BytesMut::new();

        FrameEncoder::new()
            .encode_to(&U32_1337, &mut bytes)
            .unwrap();

        assert_eq!(
            bytes,
            vec![
                4, 0, 0, 0, // 1 32-bit integer = 4 bytes.
                1, 3, 3, 7, // Little-endian integer.
            ]
        );
    }

    #[test]
    fn encode_appends() {
        let mut bytes = BytesMut::new();

        let mut encoder = FrameEncoder::new();
        encoder.encode_to(&U32_1337, &mut bytes).unwrap();
        encoder.encode_to(&U32_1337, &mut bytes).unwrap();

        assert_eq!(
            bytes,
            vec![
                4, 0, 0, 0, // 1 32-bit integer = 4 bytes.
                1, 3, 3, 7, // Little-endian integer.
                4, 0, 0, 0, // Repeated.
                1, 3, 3, 7,
            ]
        );
    }

    #[test]
    fn encode_vec() {
        let v: Vec<u32> = vec![1, 3, 3, 7];

        let mut bytes = BytesMut::new();
        FrameEncoder::new().encode_to(&v, &mut bytes).unwrap();

        assert_eq!(
            bytes,
            vec![
                20, 0, 0, 0, // 5 32-bit integers = 20 bytes.
                4, 0, 0, 0, // 4 elements in the vector.
                1, 0, 0, 0, // Little-endian vector elements.
                3, 0, 0, 0, //
                3, 0, 0, 0, //
                7, 0, 0, 0, //
            ]
        );
    }

    #[test]
    fn decode_not_enough_data_for_prefix() {
        let initial_bytes = vec![
            4, 0, 0, // Incomplete 32-bit length prefix.
        ];

        let mut bytes = BytesMut::new();
        bytes.extend_from_slice(&initial_bytes);

        let value: Option<u32> = FrameDecoder::new().decode_from(&mut bytes).unwrap();

        assert_eq!(value, None);
        assert_eq!(bytes, initial_bytes); // Untouched.
    }

    #[test]
    fn decode_not_enough_data_for_contents() {
        let initial_bytes = vec![
            4, 0, 0, 0, // Length 4.
            1, 2, 3, // But there are only 3 bytes!
        ];

        let mut bytes = BytesMut::new();
        bytes.extend_from_slice(&initial_bytes);

        let value: Option<u32> = FrameDecoder::new().decode_from(&mut bytes).unwrap();

        assert_eq!(value, None);
        assert_eq!(bytes, initial_bytes); // Untouched.
    }

    #[test]
    fn decode_u32() {
        let mut bytes = BytesMut::new();
        bytes.extend_from_slice(&[
            4, 0, 0, 0, // 1 32-bit integer = 4 bytes.
            1, 3, 3, 7, // Little-endian integer.
            4, 2, // Trailing bytes.
        ]);

        let value = FrameDecoder::new().decode_from(&mut bytes).unwrap();

        assert_eq!(value, Some(U32_1337));
        assert_eq!(bytes, vec![4, 2]); // Decoded bytes were split off.
    }

    #[test]
    fn decode_vec() {
        let mut bytes = BytesMut::new();
        bytes.extend_from_slice(&[
            20, 0, 0, 0, // 5 32-bit integers = 20 bytes.
            4, 0, 0, 0, // 4 elements in the vector.
            1, 0, 0, 0, // Little-endian vector elements.
            3, 0, 0, 0, //
            3, 0, 0, 0, //
            7, 0, 0, 0, //
            4, 2, // Trailing bytes.
        ]);

        let value = FrameDecoder::new().decode_from(&mut bytes).unwrap();

        let expected_value: Vec<u32> = vec![1, 3, 3, 7];
        assert_eq!(value, Some(expected_value));
        assert_eq!(bytes, vec![4, 2]); // Decoded bytes were split off.
    }

    #[test]
    fn roundtrip() {
        let value: Vec<String> = vec![
            "apples".to_string(),      //
            "bananas".to_string(),     //
            "oranges".to_string(),     //
            "and cheese!".to_string(), //
        ];

        let mut buffer = BytesMut::new();

        FrameEncoder::new().encode_to(&value, &mut buffer).unwrap();
        let decoded = FrameDecoder::new().decode_from(&mut buffer).unwrap();

        assert_eq!(decoded, Some(value));
        assert_eq!(buffer, vec![]);
    }
}
