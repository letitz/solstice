//! This module provides encoding and decoding functionality for basic types.
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

use std::fmt;
use std::io;
use std::net;
use std::u16;

use bytes::{Buf, BufMut, BytesMut};
use encoding::all::WINDOWS_1252;
use encoding::{DecoderTrap, EncoderTrap, Encoding};

// Constants
// ---------

/// Length of an encoded 32-bit integer in bytes.
pub const U32_BYTE_LEN: usize = 4;

pub trait Decode<T> {
    /// Attempts to decode an istance of `T` from `self`.
    fn decode(&mut self) -> io::Result<T>;
}

pub trait Encode<T> {
    /// Attempts to encode `value` into `self`.
    fn encode(&mut self, value: T) -> io::Result<()>;
}

/// Builds an EOF error encountered when reading a value of the given type.
fn unexpected_eof_error(type_name: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::UnexpectedEof,
        format!("reading {}", type_name),
    )
}

/// Builds an InvalidData error for the given value of the given type.
fn invalid_data_error<T: fmt::Debug>(type_name: &str, value: T) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid {}: {:?}", type_name, value),
    )
}

/// A type for decoding various types of values from protocol messages.
pub struct ProtoDecoder<'a> {
    inner: io::Cursor<&'a BytesMut>,
}

/// This trait is implemented by types that can be decoded from messages using
/// a `ProtoDecoder`.
pub trait ProtoDecode: Sized {
    /// Attempts to decode a value of this type with the given decoder.
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self>;
}

impl<'a> ProtoDecoder<'a> {
    /// Wraps the given byte buffer.
    pub fn new(bytes: &'a BytesMut) -> Self {
        Self{
            inner: io::Cursor::new(bytes),
        }
    }

    /// Returns whether the underlying buffer has remaining bytes to decode.
    pub fn has_remaining(&self) -> bool {
        self.inner.has_remaining()
    }

    /// Returns a read-only view of the remaining bytes to decode.
    pub fn bytes(&self) -> &[u8] {
        self.inner.bytes()
    }

    /// Asserts that the buffer contains at least `n` more bytes from which to
    /// read a value of the named type.
    /// Returns Ok(()) if there are that many bytes, otherwise returns a
    /// descriptive error.
    fn expect_remaining(&self, type_name: &str, n: usize) -> io::Result<()> {
        if self.inner.remaining() < n {
            Err(unexpected_eof_error(type_name))
        } else {
            Ok(())
        }
    }

    /// Attempts to decode a u32 value in the context of decoding a value of
    /// the named type.
    fn decode_u32_generic(&mut self, type_name: &str) -> io::Result<u32> {
        self.expect_remaining(type_name, U32_BYTE_LEN)?;
        Ok(self.inner.get_u32_le())
    }

    /// Attempts to decode a boolean value.
    fn decode_bool(&mut self) -> io::Result<bool> {
        self.expect_remaining("bool", 1)?;
        match self.inner.get_u8() {
            0 => Ok(false),
            1 => Ok(true),
            n => Err(invalid_data_error("bool", n)),
        }
    }

    /// Attempts to decode a string value.
    fn decode_string(&mut self) -> io::Result<String> {
        let len = self.decode_u32_generic("string length")? as usize;
        self.expect_remaining("string", len)?;

        let result = {
            let bytes = &self.inner.bytes()[..len];
            WINDOWS_1252
                .decode(bytes, DecoderTrap::Strict)
                .map_err(|err| invalid_data_error("string", (err, bytes)))
        };

        self.inner.advance(len);
        result
    }

    /// Attempts to decode a value of the given type.
    ///
    /// Allows easy decoding of complex values using type inference:
    ///
    /// ```
    /// let val : Foo = decoder.decode()?;
    /// ```
    pub fn decode<T: ProtoDecode>(&mut self) -> io::Result<T> {
        T::decode_from(self)
    }
}

impl ProtoDecode for u32 {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        decoder.decode_u32_generic("u32")
    }
}

impl ProtoDecode for u16 {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let n = decoder.decode_u32_generic("u16")?;
        if n > u16::MAX as u32 {
            return Err(invalid_data_error("u16", n));
        }
        Ok(n as u16)
    }
}

impl ProtoDecode for bool {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        decoder.decode_bool()
    }
}

impl ProtoDecode for net::Ipv4Addr {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let ip = decoder.decode_u32_generic("ipv4 address")?;
        Ok(net::Ipv4Addr::from(ip))
    }
}

impl ProtoDecode for String {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        decoder.decode_string()
    }
}

impl<T: ProtoDecode, U: ProtoDecode> ProtoDecode for (T, U)
{
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let first = decoder.decode()?;
        let second = decoder.decode()?;
        Ok((first, second))
    }
}

impl<T: ProtoDecode> ProtoDecode for Vec<T>
{
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let len = decoder.decode_u32_generic("vector length")? as usize;
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            let val = decoder.decode()?;
            vec.push(val);
        }
        Ok(vec)
    }
}

/// A type for encoding various types of values into protocol messages.
pub struct ProtoEncoder<'a> {
    inner: &'a mut BytesMut,
}

/// This trait is implemented by types that can be encoded into messages using
/// a `ProtoEncoder`.
pub trait ProtoEncode {
    /// Attempts to encode `self` with the given encoder.
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()>;
}

impl<'a> ProtoEncoder<'a> {
    /// Wraps the given buffer for encoding values into.
    ///
    /// The buffer is grown as required.
    pub fn new(inner: &'a mut BytesMut) -> Self {
        ProtoEncoder { inner: inner }
    }

    /// Attempts to encode the given u32 value.
    pub fn encode_u32(&mut self, val: u32) -> io::Result<()> {
        self.inner.reserve(U32_BYTE_LEN);
        self.inner.put_u32_le(val);
        Ok(())
    }

    /// Attempts to encode the given boolean value.
    pub fn encode_bool(&mut self, val: bool) -> io::Result<()> {
        self.inner.reserve(1);
        self.inner.put_u8(val as u8);
        Ok(())
    }

    /// Attempts to encode the given IPv4 address.
    pub fn encode_ipv4_addr(&mut self, addr: net::Ipv4Addr) -> io::Result<()> {
        let mut octets = addr.octets();
        octets.reverse(); // Little endian.
        self.inner.extend(&octets);
        Ok(())
    }

    /// Attempts to encode the given string.
    pub fn encode_string(&mut self, val: &str) -> io::Result<()> {
        // Encode the string.
        let bytes = match WINDOWS_1252.encode(val, EncoderTrap::Strict) {
            Ok(bytes) => bytes,
            Err(_) => {
                return Err(io::Error::new(io::ErrorKind::InvalidData, val.to_string()));
            }
        };
        // Prefix the bytes with the length.
        self.encode_u32(bytes.len() as u32)?;
        self.inner.extend(bytes);
        Ok(())
    }

    /// Attempts to encode the given value.
    ///
    /// Allows for easy encoding with type inference:
    /// ```
    /// let val : Foo = Foo::new(bar);
    /// encoder.encode(&val)?;
    /// ```
    pub fn encode<T: ProtoEncode>(&mut self, val: &T) -> io::Result<()> {
        val.encode(self)
    }
}

impl ProtoEncode for u32 {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_u32(*self)
    }
}

impl ProtoEncode for bool {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_bool(*self)
    }
}

impl ProtoEncode for u16 {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_u32(*self as u32)
    }
}

impl ProtoEncode for net::Ipv4Addr {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_ipv4_addr(*self)
    }
}

// It would be nice to use AsRef<str>, or Deref<Target=str> for the following
// stringy types instead of having to spell them out, but trying that fails
// because E0119: "upstream crates may add new impl of trait
// `core::convert::AsRef<str>` for type `bool` in future versions".
// We could probably work around this with more complex type logic (e.g.
// wrapping primitive types in a newtype for which we implement
// Proto{De,En}code) but it is not really worth the hassle.

impl ProtoEncode for str {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_string(self)
    }
}

impl ProtoEncode for String {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_string(self)
    }
}

impl<'a> ProtoEncode for &'a String {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_string(*self)
    }
}

impl<T: ProtoEncode, U: ProtoEncode> ProtoEncode for (T, U) {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        self.0.encode(encoder)?;
        self.1.encode(encoder)
    }
}

impl<T: ProtoEncode> ProtoEncode for [T] {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_u32(self.len() as u32)?;
        for ref item in self {
            item.encode(encoder)?;
        }
        Ok(())
    }
}

impl<T: ProtoEncode> ProtoEncode for Vec<T> {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
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

    use bytes::BytesMut;

    use super::{ProtoDecode, ProtoDecoder, ProtoEncode, ProtoEncoder};

    // Declared here because assert_eq!(bytes, &[]) fails to infer types.
    const EMPTY_BYTES: &'static [u8] = &[];

    pub fn roundtrip<T>(input: T)
    where
        T: fmt::Debug + Eq + PartialEq + ProtoEncode + ProtoDecode,
    {
        let mut bytes = BytesMut::new();

        ProtoEncoder::new(&mut bytes).encode(&input).unwrap();
        let output = ProtoDecoder::new(&bytes).decode::<T>().unwrap();

        assert_eq!(output, input);
    }

    pub fn expect_io_error<T>(result: io::Result<T>, kind: io::ErrorKind, message: &str)
    where
        T: fmt::Debug + Send + 'static,
    {
        match result {
            Err(e) => {
                assert_eq!(e.kind(), kind);
                let ok = match e.get_ref() {
                    Some(e) => {
                        assert_eq!(e.description(), message);
                        true
                    }
                    None => false,
                };
                if !ok {
                    panic!(e)
                }
            }
            Ok(message) => panic!(message),
        }
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
    fn expect_io_error_success() {
        let kind = io::ErrorKind::InvalidInput;
        let message = "some message";
        let result: io::Result<()> = Err(io::Error::new(kind, message));
        expect_io_error(result, kind, message);
    }

    #[test]
    #[should_panic]
    fn expect_io_error_not_err() {
        expect_io_error(Ok(()), io::ErrorKind::InvalidInput, "some message");
    }

    #[test]
    #[should_panic]
    fn expect_io_error_wrong_kind() {
        let result: io::Result<()> =
            Err(io::Error::new(io::ErrorKind::UnexpectedEof, "some message"));
        expect_io_error(result, io::ErrorKind::InvalidInput, "some message");
    }

    #[test]
    #[should_panic]
    fn expect_io_error_wrong_message() {
        let result: io::Result<()> =
            Err(io::Error::new(io::ErrorKind::InvalidInput, "some message"));
        expect_io_error(result, io::ErrorKind::InvalidInput, "other message");
    }

    #[test]
    fn encode_u32() {
        for &(val, ref encoded_bytes) in &U32_ENCODINGS {
            let mut bytes = BytesMut::from(vec![13]);
            let mut expected_bytes = vec![13];
            expected_bytes.extend(encoded_bytes);

            ProtoEncoder::new(&mut bytes).encode_u32(val).unwrap();
            assert_eq!(bytes, expected_bytes);
        }
    }

    #[test]
    fn decode_u32() {
        for &(expected_val, ref bytes) in &U32_ENCODINGS {
            let buffer = BytesMut::from(bytes.to_vec());
            let mut decoder = ProtoDecoder::new(&buffer);

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
        let buffer = BytesMut::from(vec![13]);
        let mut decoder = ProtoDecoder::new(&buffer);

        let result = decoder.decode::<u32>();

        expect_io_error(result, io::ErrorKind::UnexpectedEof, "reading u32");
        assert_eq!(decoder.bytes(), &[13]);
    }

    #[test]
    fn encode_bool() {
        let mut bytes = BytesMut::from(vec![13]);
        ProtoEncoder::new(&mut bytes).encode_bool(false).unwrap();
        assert_eq!(bytes, vec![13, 0]);

        bytes.truncate(1);
        ProtoEncoder::new(&mut bytes).encode_bool(true).unwrap();
        assert_eq!(bytes, vec![13, 1]);
    }

    #[test]
    fn decode_bool_false() {
        let buffer = BytesMut::from(vec![0]);
        let mut decoder = ProtoDecoder::new(&buffer);

        let val = decoder.decode::<bool>().unwrap();

        assert!(!val);
        assert_eq!(decoder.bytes(), EMPTY_BYTES);
    }

    #[test]
    fn decode_bool_true() {
        let buffer = BytesMut::from(vec![1]);
        let mut decoder = ProtoDecoder::new(&buffer);

        let val = decoder.decode::<bool>().unwrap();

        assert!(val);
        assert_eq!(decoder.bytes(), EMPTY_BYTES);
    }

    #[test]
    fn decode_bool_invalid() {
        let buffer = BytesMut::from(vec![42]);

        let result = ProtoDecoder::new(&buffer).decode::<bool>();

        expect_io_error(result, io::ErrorKind::InvalidData, "invalid bool: 42");
    }

    #[test]
    fn decode_bool_unexpected_eof() {
        let buffer = BytesMut::new();

        let result = ProtoDecoder::new(&buffer).decode::<bool>();

        expect_io_error(result, io::ErrorKind::UnexpectedEof, "reading bool");
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

            let mut bytes = BytesMut::from(vec![13]);
            let mut expected_bytes = vec![13];
            expected_bytes.extend(encoded_bytes);

            ProtoEncoder::new(&mut bytes)
                .encode(&(val as u16))
                .unwrap();
            assert_eq!(bytes, expected_bytes);
        }
    }

    #[test]
    fn decode_u16() {
        for &(expected_val, ref bytes) in &U32_ENCODINGS {
            let buffer = BytesMut::from(bytes.to_vec());
            let mut decoder = ProtoDecoder::new(&buffer);

            if expected_val <= u16::MAX as u32 {
                let val = decoder.decode::<u16>().unwrap();
                assert_eq!(val, expected_val as u16);
                assert_eq!(decoder.bytes(), EMPTY_BYTES);
            } else {
                expect_io_error(
                    decoder.decode::<u16>(),
                    io::ErrorKind::InvalidData,
                    &format!("invalid u16: {}", expected_val),
                );
            }
        }
    }

    #[test]
    fn decode_u16_unexpected_eof() {
        let buffer = BytesMut::new();
        let mut decoder = ProtoDecoder::new(&buffer);

        let result = decoder.decode::<u16>();

        expect_io_error(result, io::ErrorKind::UnexpectedEof, "reading u16");
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
            let mut bytes = BytesMut::from(vec![13]);
            let mut expected_bytes = vec![13];
            expected_bytes.extend(encoded_bytes);

            let addr = net::Ipv4Addr::from(val);
            ProtoEncoder::new(&mut bytes)
                .encode_ipv4_addr(addr)
                .unwrap();
            assert_eq!(bytes, expected_bytes);
        }
    }

    #[test]
    fn decode_ipv4() {
        for &(expected_val, ref bytes) in &U32_ENCODINGS {
            let buffer = BytesMut::from(bytes.to_vec());
            let mut decoder = ProtoDecoder::new(&buffer);

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
            let mut bytes = BytesMut::from(vec![13]);
            let mut expected_bytes = vec![13];
            expected_bytes.extend(encoded_bytes);

            ProtoEncoder::new(&mut bytes).encode_string(string).unwrap();
            assert_eq!(bytes, expected_bytes);
        }
    }

    #[test]
    #[should_panic]
    fn encode_invalid_string() {
        let mut bytes = BytesMut::with_capacity(100);
        ProtoEncoder::new(&mut bytes)
            .encode_string("忠犬ハチ公")
            .unwrap();
    }

    #[test]
    fn decode_string() {
        for &(expected_string, bytes) in &STRING_ENCODINGS {
            let buffer = BytesMut::from(bytes);
            let mut decoder = ProtoDecoder::new(&buffer);

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
        let mut bytes = BytesMut::from(vec![13]);
        let mut expected_bytes = BytesMut::from(vec![13]);

        let (integer, ref expected_integer_bytes) = U32_ENCODINGS[0];
        let (string, expected_string_bytes) = STRING_ENCODINGS[0];

        expected_bytes.extend(expected_integer_bytes);
        expected_bytes.extend(expected_string_bytes);

        ProtoEncoder::new(&mut bytes)
            .encode(&(integer, string.to_string()))
            .unwrap();

        assert_eq!(bytes, expected_bytes);
    }

    #[test]
    fn decode_pair_u32_string() {
        let mut bytes = vec![];

        let (expected_integer, ref integer_bytes) = U32_ENCODINGS[0];
        let (expected_string, string_bytes) = STRING_ENCODINGS[0];

        bytes.extend(integer_bytes);
        bytes.extend(string_bytes);

        let buffer = BytesMut::from(bytes);
        let mut decoder = ProtoDecoder::new(&buffer);

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

        let mut bytes = BytesMut::from(vec![13]);
        ProtoEncoder::new(&mut bytes).encode(&vec).unwrap();

        assert_eq!(bytes, expected_bytes);
    }

    #[test]
    fn decode_u32_vector() {
        let mut expected_vec = vec![];
        let mut bytes = vec![U32_ENCODINGS.len() as u8, 0, 0, 0];
        for &(expected_val, ref encoded_bytes) in &U32_ENCODINGS {
            expected_vec.push(expected_val);
            bytes.extend(encoded_bytes);
        }

        let buffer = BytesMut::from(bytes);
        let mut decoder = ProtoDecoder::new(&buffer);

        let vec = decoder.decode::<Vec<u32>>().unwrap();

        assert_eq!(vec, expected_vec);
        assert_eq!(decoder.bytes(), EMPTY_BYTES);
    }

    #[test]
    fn roundtrip_u32_vector() {
        roundtrip(vec![0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9])
    }
}
