//! This module provides tokio Codec implementations for protocol messages.
//!
//! Specifically, the following types:
//!
//!  * proto::peer::Message
//!  * proto::server::ServerRequest
//!  * proto::server::ServerResponse
//!
//! This enables wrapping AsyncRead and AsyncWrite objects into Stream and Sink
//! objects using tokio_codec's FramedRead and FramedWrite adapters.

use std::io;
use std::marker;

use bytes::BytesMut;
use tokio_codec;

use super::base_codec::{ProtoDecode, ProtoDecoder, ProtoEncode, ProtoEncoder, U32_BYTE_LEN};
use super::peer::Message;
use super::server::{ServerRequest, ServerResponse};

/// Implements tokio's Encoder trait for types that implement ProtoEncode.
pub struct LengthPrefixedEncoder<T> {
    phantom: marker::PhantomData<T>,
}

impl<T> LengthPrefixedEncoder<T> {
    pub fn new() -> Self {
        Self {
            phantom: marker::PhantomData,
        }
    }
}

impl<T: ProtoEncode> tokio_codec::Encoder for LengthPrefixedEncoder<T> {
    type Item = T;
    type Error = io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Split buffer into two parts: the length prefix and the message.
        dst.reserve(U32_BYTE_LEN);
        let mut msg_dst = dst.split_off(U32_BYTE_LEN);

        // Encode the message.
        item.encode(&mut ProtoEncoder::new(&mut msg_dst))?;

        // Encode the message length.
        ProtoEncoder::new(dst).encode_u32(msg_dst.len() as u32)?;

        // Reassemble both parts into one contiguous buffer.
        dst.unsplit(msg_dst);
        Ok(())
    }
}

/// Implements tokio's Decoder trait for types that implement ProtoDecode.
pub struct LengthPrefixedDecoder<T> {
    // The length, as a number of bytes, of the next item to decode.
    // None if we have not read the length prefix yet.
    // Some(n) if we read the length prefix, and are now waiting for `n` bytes
    // to be available.
    length: Option<usize>,

    // Only here to enable parameterizing `Decoder` by `T`.
    phantom: marker::PhantomData<T>,
}

impl<T> LengthPrefixedDecoder<T> {
    pub fn new() -> Self {
        Self {
            length: None,
            phantom: marker::PhantomData,
        }
    }

    // If necessary, atempts to decode a length prefix from `src`.
    //
    // Helper for decode() below.
    //
    // If self.length is not None, returns Ok(()).
    // If there are not enough bytes in `src`, returns Ok(()).
    // Otherwise, splits off the length prefix bytes from `src`, and:
    //  - returns an error if decoding the value failed.
    //  - sets self.length to Some(length) and returns Ok(()) otherwise.
    fn maybe_decode_length(&mut self, src: &mut BytesMut) -> io::Result<()> {
        if self.length.is_some() {
            return Ok(()); // Aready read length.
        }

        if src.len() < U32_BYTE_LEN {
            return Ok(()); // Not enough bytes yet.
        }

        let prefix = src.split_to(U32_BYTE_LEN);
        let length = ProtoDecoder::new(&prefix).decode::<u32>()?;

        self.length = Some(length as usize);
        Ok(())
    }
}

impl<T: ProtoDecode> tokio_codec::Decoder for LengthPrefixedDecoder<T> {
    type Item = T;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // If we have not read the length prefix yet, attempt to do so first.
        self.maybe_decode_length(src)?;

        let length = match self.length {
            None => return Ok(None), // Not enough bytes yet.
            Some(n) => n,
        };

        if src.len() < length {
            return Ok(None); // Not enough bytes yet.
        }

        // Split off the right amount of bytes from the buffer.
        let buf = src.split_to(length);
        self.length = None;

        // Attempt to decode the value.
        let item = ProtoDecoder::new(&buf).decode()?;
        Ok(Some(item))
    }
}

mod tests {
    use bytes::BytesMut;
    use tokio_codec::{Decoder, Encoder};

    use crate::proto::ProtoEncode;

    use super::{LengthPrefixedDecoder, LengthPrefixedEncoder};

    // Test value: [1, 3, 3, 7] in little-endian.
    const U32_1337: u32 = 1 + (3 << 8) + (3 << 16) + (7 << 24);

    #[test]
    fn encode_u32() {
        let mut bytes = BytesMut::new();
        LengthPrefixedEncoder::new()
            .encode(U32_1337, &mut bytes)
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
    fn encode_vec() {
        let v: Vec<u32> = vec![1, 3, 3, 7];

        let mut bytes = BytesMut::new();
        LengthPrefixedEncoder::new().encode(v, &mut bytes).unwrap();

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
    fn decode_not_enough_data() {
        let mut bytes = BytesMut::from(vec![
            4, 0, 0, // Incomplete 32-bit length prefix.
        ]);

        let value: Option<u32> = LengthPrefixedDecoder::new().decode(&mut bytes).unwrap();

        assert_eq!(value, None);
        assert_eq!(bytes, vec![4, 0, 0]); // Untouched.
    }

    #[test]
    fn decode_u32() {
        let mut bytes = BytesMut::from(vec![
            4, 0, 0, 0, // 1 32-bit integer = 4 bytes.
            1, 3, 3, 7, // Little-endian integer.
            4, 2, // Trailing bytes.
        ]);

        let value = LengthPrefixedDecoder::new().decode(&mut bytes).unwrap();

        assert_eq!(value, Some(U32_1337));
        assert_eq!(bytes, vec![4, 2]); // Decoded bytes were split off.
    }

    #[test]
    fn decode_vec() {
        let mut bytes = BytesMut::from(vec![
            20, 0, 0, 0, // 5 32-bit integers = 20 bytes.
            4, 0, 0, 0, // 4 elements in the vector.
            1, 0, 0, 0, // Little-endian vector elements.
            3, 0, 0, 0, //
            3, 0, 0, 0, //
            7, 0, 0, 0, //
            4, 2, // Trailing bytes.
        ]);

        let value = LengthPrefixedDecoder::new().decode(&mut bytes).unwrap();

        let expected_value: Vec<u32> = vec![1, 3, 3, 7];
        assert_eq!(value, Some(expected_value));
        assert_eq!(bytes, vec![4, 2]); // Decoded bytes were split off.
    }

    #[test]
    fn decode_stateful() {
        let mut decoder = LengthPrefixedDecoder::new();

        let mut bytes = BytesMut::from(vec![
            4, 0, 0, 0, // 32-bit integer = 4 bytes.
            1, 3, // Incomplete integer.
        ]);

        let value = decoder.decode(&mut bytes).unwrap();

        assert_eq!(value, None);
        assert_eq!(bytes, vec![1, 3]); // Decoded bytes were split off.

        bytes.extend_from_slice(&[
            3, 7, // End of integer.
            4, 0, 0, 0, // Second identical message waiting to be read.
            1, 3, 3, 7, //
            4, 2, // Trailing bytes.
        ]);

        // Decoder has state, remembers that the length prefix was 4.
        let value = decoder.decode(&mut bytes).unwrap();

        assert_eq!(value, Some(U32_1337));

        // Decoder state resets after entire item is decoded.
        // Decode the second message now.
        let value = decoder.decode(&mut bytes).unwrap();

        assert_eq!(value, Some(U32_1337));
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

        LengthPrefixedEncoder::new()
            .encode(value.clone(), &mut buffer)
            .unwrap();
        let decoded = LengthPrefixedDecoder::new().decode(&mut buffer).unwrap();

        assert_eq!(decoded, Some(value));
        assert_eq!(buffer, vec![]);
    }
}
