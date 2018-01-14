use std::fmt;
use std::io;
use std::net;
use std::u16;

use bytes::{Buf, BufMut, BytesMut, LittleEndian};
use encoding::{Encoding, EncoderTrap, DecoderTrap};
use encoding::all::WINDOWS_1252;

// Constants
// ---------

/// Length of an encoded 32-bit integer in bytes.
const U32_BYTE_LEN: usize = 4;

/*===================================*
 * BASIC TYPES ENCODING AND DECODING *
 *===================================*/

// The protocol is pretty basic, though quirky. Base types are serialized in
// the following way:
//
//   * 32-bit integers are serialized in 4 bytes, little-endian.
//   * 16-bit integers are serialized as 32-bit integers with upper bytes set
//     to 0.
//   * Booleans are serialized as single bytes, containing either 0 or 1.
//   * IPv4 addresses are serialized as 32-bit integers.
//   * Strings are serialized as 32-bit-length-prefixed arrays of Windows 1252
//     encoded characters.
//   * Pairs are serialized as two consecutive values.
//   * Vectors are serialized as length-prefixed arrays of serialized values.

pub trait Decode<T> {
    /// Attempts to decode an istance of `T` from `self`.
    fn decode(&mut self) -> io::Result<T>;
}

pub trait Encode<T> {
    /// Attempts to encode `value` into `self`.
    fn encode(&mut self, value: T) -> io::Result<()>;
}

// Builds an EOF error encountered when reading a value of the given type.
fn unexpected_eof_error(type_name: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::UnexpectedEof,
        format!("reading {}", type_name),
    )
}

// Builds an InvalidData error for the given value of the given type.
fn invalid_data_error<T: fmt::Debug>(type_name: &str, value: T) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid {}: {:?}", type_name, value),
    )
}

// A few helper methods for implementing Decode<T> for basic types.
trait BootstrapDecode: Buf {
    // Asserts that the buffer contains at least `n` more bytes from which to
    // read a value of the given type.
    // Returns Ok(()) if there are that many bytes, an error otherwise.
    fn expect_remaining(&self, type_name: &str, n: usize) -> io::Result<()>;

    // Decodes a u32 value as the first step in decoding a value of the given type.
    fn decode_u32_generic(&mut self, type_name: &str) -> io::Result<u32>;
}

impl<T: Buf> BootstrapDecode for T {
    fn expect_remaining(&self, type_name: &str, n: usize) -> io::Result<()> {
        if self.remaining() < n {
            Err(unexpected_eof_error(type_name))
        } else {
            Ok(())
        }
    }

    fn decode_u32_generic(&mut self, type_name: &str) -> io::Result<u32> {
        self.expect_remaining(type_name, U32_BYTE_LEN)?;
        Ok(self.get_u32::<LittleEndian>())
    }
}

impl<T: Buf> Decode<u32> for T {
    fn decode(&mut self) -> io::Result<u32> {
        self.decode_u32_generic("u32")
    }
}

impl<T: Buf> Decode<u16> for T {
    fn decode(&mut self) -> io::Result<u16> {
        let n = self.decode_u32_generic("u16")?;
        if n > u16::MAX as u32 {
            return Err(invalid_data_error("u16", n));
        }
        Ok(n as u16)
    }
}

impl<T: Buf> Decode<bool> for T {
    fn decode(&mut self) -> io::Result<bool> {
        self.expect_remaining("bool", 1)?;
        match self.get_u8() {
            0 => Ok(false),
            1 => Ok(true),
            n => Err(invalid_data_error("bool", n)),
        }
    }
}

impl<T: Buf> Decode<net::Ipv4Addr> for T {
    fn decode(&mut self) -> io::Result<net::Ipv4Addr> {
        let ip = self.decode_u32_generic("ipv4 address")?;
        Ok(net::Ipv4Addr::from(ip))
    }
}

impl<T: Buf> Decode<String> for T {
    fn decode(&mut self) -> io::Result<String> {
        let len = self.decode_u32_generic("string length")? as usize;
        self.expect_remaining("string", len)?;

        let result = {
            let bytes = &self.bytes()[..len];
            WINDOWS_1252.decode(bytes, DecoderTrap::Strict).map_err(
                |err| {
                    invalid_data_error("string", (err, bytes))
                },
            )
        };

        self.advance(len);
        result
    }
}

impl<T, U, V> Decode<(U, V)> for T
where
    T: Decode<U> + Decode<V>,
{
    fn decode(&mut self) -> io::Result<(U, V)> {
        let first = self.decode()?;
        let second = self.decode()?;
        Ok((first, second))
    }
}

impl<T, U> Decode<Vec<U>> for T
where
    T: Buf + Decode<U>,
{
    fn decode(&mut self) -> io::Result<Vec<U>> {
        let len = self.decode_u32_generic("vector length")? as usize;
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            let val = self.decode()?;
            vec.push(val);
        }
        Ok(vec)
    }
}

/// This trait is implemented by types that can be encoded into messages with
/// a `ProtoEncoder`.
/// Only here to enable `ProtoEncoder::encode_vec`.
pub trait ProtoEncode {
    /// Attempts to encode `self` with the given encoder.
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()>;
}

// A `ProtoEncoder` knows how to encode various types of values into protocol
// messages.
pub struct ProtoEncoder<'a> {
    // We would like to store an &'a BufMut instead, but not only is it not
    // object-safe yet, it does not grow the buffer on writes either... So we
    // don't want to template this struct like ProtoDecoder either.
    inner: &'a mut BytesMut,
}

impl<'a> ProtoEncoder<'a> {
    pub fn new(inner: &'a mut BytesMut) -> Self {
        ProtoEncoder { inner: inner }
    }

    pub fn encode_u32(&mut self, val: u32) -> io::Result<()> {
        if self.inner.remaining_mut() < U32_BYTE_LEN {
            self.inner.reserve(U32_BYTE_LEN);
        }
        self.inner.put_u32::<LittleEndian>(val);
        Ok(())
    }

    pub fn encode_u16(&mut self, val: u16) -> io::Result<()> {
        self.encode_u32(val as u32)
    }

    pub fn encode_bool(&mut self, val: bool) -> io::Result<()> {
        if !self.inner.has_remaining_mut() {
            self.inner.reserve(1);
        }
        self.inner.put_u8(val as u8);
        Ok(())
    }

    pub fn encode_ipv4_addr(&mut self, addr: net::Ipv4Addr) -> io::Result<()> {
        let mut octets = addr.octets();
        octets.reverse(); // Little endian.
        self.inner.extend(&octets);
        Ok(())
    }

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

    pub fn encode_pair<T, U>(&mut self, pair: &(T, U)) -> io::Result<()>
    where
        T: ProtoEncode,
        U: ProtoEncode,
    {
        pair.0.encode(self)?;
        pair.1.encode(self)
    }

    pub fn encode_vec<T: ProtoEncode>(&mut self, vec: &[T]) -> io::Result<()> {
        self.encode_u32(vec.len() as u32)?;
        for ref item in vec {
            item.encode(self)?;
        }
        Ok(())
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
        encoder.encode_u16(*self)
    }
}

impl ProtoEncode for net::Ipv4Addr {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_ipv4_addr(*self)
    }
}

// It would be nice to use AsRef<str> for the following stringy types instead
// of having to spell them out, but trying that fails because E0119:
// "upstream crates may add new impl of trait `core::convert::AsRef<str>` for
// type `bool` in future versions".
// We could probably work around this with more complex type logic (e.g.
// wrapping primitive types in a newtype for which we implement
// Proto{De,En}code) but it is not really worth the hassle.

impl ProtoEncode for str {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_string(self)
    }
}

impl<'a> ProtoEncode for &'a str {
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
        encoder.encode_pair(self)
    }
}

impl<T: ProtoEncode> ProtoEncode for Vec<T> {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_vec(self)
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

    use bytes::{Buf, BytesMut};

    use super::{Decode, ProtoEncoder, ProtoEncode};

    pub fn roundtrip<T>(input: T)
    where
        T: fmt::Debug + Eq + PartialEq + ProtoEncode,
        io::Cursor<BytesMut>: Decode<T>,
    {
        let mut bytes = BytesMut::new();
        input.encode(&mut ProtoEncoder::new(&mut bytes)).unwrap();

        let output: T = io::Cursor::new(bytes).decode().unwrap();

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

    // Helper for succinctness in tests below.
    fn new_cursor(vec: Vec<u8>) -> io::Cursor<BytesMut> {
        io::Cursor::new(BytesMut::from(vec))
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
            let mut cursor = new_cursor(bytes.to_vec());
            let val: u32 = cursor.decode().unwrap();
            assert_eq!(val, expected_val);
            assert_eq!(cursor.remaining(), 0);
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
        let result: io::Result<u32> = new_cursor(vec![13]).decode();
        expect_io_error(result, io::ErrorKind::UnexpectedEof, "reading u32");
    }

    #[test]
    fn encode_bool() {
        let mut bytes = BytesMut::from(vec![13]);
        ProtoEncoder::new(&mut bytes).encode_bool(false).unwrap();
        assert_eq!(*bytes, [13, 0]);

        bytes.truncate(1);
        ProtoEncoder::new(&mut bytes).encode_bool(true).unwrap();
        assert_eq!(*bytes, [13, 1]);
    }

    #[test]
    fn decode_bool() {
        let mut cursor = new_cursor(vec![0]);
        let val: bool = cursor.decode().unwrap();
        assert!(!val);
        assert_eq!(cursor.remaining(), 0);

        cursor = new_cursor(vec![1]);
        let val: bool = cursor.decode().unwrap();
        assert!(val);
        assert_eq!(cursor.remaining(), 0);
    }

    #[test]
    fn decode_bool_invalid() {
        let result: io::Result<bool> = new_cursor(vec![42]).decode();
        expect_io_error(result, io::ErrorKind::InvalidData, "invalid bool: 42");
    }

    #[test]
    fn decode_bool_unexpected_eof() {
        let result: io::Result<bool> = new_cursor(vec![]).decode();
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
                .encode_u16(val as u16)
                .unwrap();
            assert_eq!(bytes, expected_bytes);
        }
    }

    #[test]
    fn decode_u16() {
        for &(expected_val, ref bytes) in &U32_ENCODINGS {
            let mut cursor = new_cursor(bytes.to_vec());
            if expected_val <= u16::MAX as u32 {
                let val: u16 = cursor.decode().unwrap();
                assert_eq!(val, expected_val as u16);
                assert_eq!(cursor.remaining(), 0);
            } else {
                let result: io::Result<u16> = cursor.decode();
                expect_io_error(
                    result,
                    io::ErrorKind::InvalidData,
                    &format!("invalid u16: {}", expected_val),
                );
            }
        }
    }

    #[test]
    fn decode_u16_unexpected_eof() {
        let result: io::Result<u16> = new_cursor(vec![]).decode();
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
            let mut cursor = new_cursor(bytes.to_vec());
            let val: net::Ipv4Addr = cursor.decode().unwrap();
            assert_eq!(val, net::Ipv4Addr::from(expected_val));
            assert_eq!(cursor.remaining(), 0);
        }
    }

    #[test]
    fn roundtrip_ipv4() {
        for &(val, _) in &U32_ENCODINGS {
            roundtrip(net::Ipv4Addr::from(val))
        }
    }

    // A few strings and their corresponding encodings.
    const STRING_ENCODINGS: [(&'static str, &'static [u8]); 3] =
        [
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
            let mut cursor = new_cursor(bytes.to_vec());
            let string: String = cursor.decode().unwrap();
            assert_eq!(string, expected_string);
            assert_eq!(cursor.remaining(), 0);
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
            .encode_pair(&(integer, string))
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

        let mut cursor = new_cursor(bytes);

        let pair: (u32, String) = cursor.decode().unwrap();

        assert_eq!(pair, (expected_integer, expected_string.to_string()));
        assert_eq!(cursor.remaining(), 0);
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
        ProtoEncoder::new(&mut bytes).encode_vec(&vec).unwrap();

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

        let mut cursor = new_cursor(bytes);
        let vec: Vec<u32> = cursor.decode().unwrap();

        assert_eq!(vec, expected_vec);
        assert_eq!(cursor.remaining(), 0);
    }

    #[test]
    fn roundtrip_u32_vector() {
        roundtrip(vec![0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9])
    }
}
