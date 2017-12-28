use std::error;
use std::fmt;
use std::io;
use std::net;
use std::u16;

use bytes::{Buf, BufMut, BytesMut, LittleEndian};
use encoding::{Encoding, EncoderTrap, DecoderTrap};
use encoding::all::WINDOWS_1252;

/// Length of an encoded 32-bit integer in bytes.
const U32_BYTE_LEN: usize = 4;

/*==============*
 * DECODE ERROR *
 *==============*/

/// An error that arose when decoding a protocol message.
#[derive(Debug)]
pub enum DecodeError {
    /// Attempted to decode a boolean, but the value was neither 0 nor 1.
    /// The invalid value is enclosed.
    InvalidBoolError(u8),
    /// Attempted to decode an unsigned 16-bit integer, but the value did not
    /// fit in 16 bits. The invalid value is enclosed.
    InvalidU16Error(u32),
    /// Attempted to decode the enclosed bytes as an Windows 1252 encoded
    /// string, but one of the bytes was not a valid character encoding.
    InvalidStringError(Vec<u8>),
    /// Attempted to decode a user::Status, but the value was not a valid
    /// representation of an enum variant. The invalid value is enclosed.
    InvalidUserStatusError(u32),
    /// Encountered the enclosed I/O error while decoding.
    IOError(io::Error),
    /// Attempted to decode a message with the enclosed code, unknown to this
    /// library.
    UnknownCodeError(u32),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DecodeError::InvalidBoolError(n) => write!(fmt, "InvalidBoolError: {}", n),
            DecodeError::InvalidU16Error(n) => write!(fmt, "InvalidU16Error: {}", n),
            DecodeError::InvalidStringError(ref bytes) => {
                write!(fmt, "InvalidStringError: {:?}", bytes)
            }
            DecodeError::InvalidUserStatusError(n) => write!(fmt, "InvalidUserStatusError: {}", n),
            DecodeError::IOError(ref err) => write!(fmt, "IOError: {}", err),
            DecodeError::UnknownCodeError(code) => write!(fmt, "UnknownCodeError: {}", code),
        }
    }
}

impl error::Error for DecodeError {
    fn description(&self) -> &str {
        match *self {
            DecodeError::InvalidBoolError(_) => "InvalidBoolError",
            DecodeError::InvalidU16Error(_) => "InvalidU16Error",
            DecodeError::InvalidStringError(_) => "InvalidStringError",
            DecodeError::InvalidUserStatusError(_) => "InvalidUserStatusError",
            DecodeError::IOError(_) => "IOError",
            DecodeError::UnknownCodeError(code) => "UnknownCodeError",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            DecodeError::InvalidBoolError(_) => None,
            DecodeError::InvalidU16Error(_) => None,
            DecodeError::InvalidStringError(_) => None,
            DecodeError::InvalidUserStatusError(_) => None,
            DecodeError::IOError(ref err) => Some(err),
            DecodeError::UnknownCodeError(_) => None,
        }
    }
}

impl From<io::Error> for DecodeError {
    fn from(err: io::Error) -> Self {
        DecodeError::IOError(err)
    }
}

fn unexpected_eof_error(value_type: &str) -> DecodeError {
    DecodeError::from(io::Error::new(io::ErrorKind::UnexpectedEof, value_type))
}

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
//   * Vectors are serialized as length-prefixed arrays of serialized values.

/// This trait is implemented by types that can be decoded from messages with
/// a `ProtoDecoder`.
/// Only here to enable `ProtoDecoder::decode_vec`.
pub trait ProtoDecode: Sized {
    /// Attempts to decode an instance of `Self` using the given decoder.
    fn decode(decoder: &mut ProtoDecoder) -> Result<Self, DecodeError>;
}

/// This trait is implemented by types that can be encoded into messages with
/// a `ProtoEncoder`.
/// Only here to enable `ProtoEncoder::encode_vec`.
pub trait ProtoEncode {
    /// Attempts to encode `self` with the given encoder.
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()>;
}

// A `ProtoDecoder` knows how to decode various types of values from protocol
// messages.
pub struct ProtoDecoder<'a> {
    // If bytes::Buf was object-safe we would just store &'a Buf. We work
    // around this limitation by explicitly naming the implementing type.
    inner: &'a mut io::Cursor<BytesMut>,
}

impl<'a> ProtoDecoder<'a> {
    pub fn new(inner: &'a mut io::Cursor<BytesMut>) -> Self {
        ProtoDecoder { inner: inner }
    }

    pub fn decode_u32(&mut self) -> Result<u32, DecodeError> {
        if self.inner.remaining() < U32_BYTE_LEN {
            return Err(unexpected_eof_error("u32"));
        }
        Ok(self.inner.get_u32::<LittleEndian>())
    }

    pub fn decode_u16(&mut self) -> Result<u16, DecodeError> {
        let n = self.decode_u32()?;
        if n > u16::MAX as u32 {
            return Err(DecodeError::InvalidU16Error(n));
        }
        Ok(n as u16)
    }

    pub fn decode_bool(&mut self) -> Result<bool, DecodeError> {
        if self.inner.remaining() < 1 {
            return Err(unexpected_eof_error("bool"));
        }
        match self.inner.get_u8() {
            0 => Ok(false),
            1 => Ok(true),
            n => Err(DecodeError::InvalidBoolError(n)),
        }
    }

    pub fn decode_ipv4_addr(&mut self) -> Result<net::Ipv4Addr, DecodeError> {
        let ip = self.decode_u32()?;
        Ok(net::Ipv4Addr::from(ip))
    }

    pub fn decode_string(&mut self) -> Result<String, DecodeError> {
        let len = self.decode_u32()? as usize;
        if self.inner.remaining() < len {
            return Err(unexpected_eof_error("string"));
        }
        let result = {
            let bytes = &self.inner.bytes()[..len];
            WINDOWS_1252.decode(bytes, DecoderTrap::Strict).map_err(
                |_| {
                    DecodeError::InvalidStringError(bytes.to_vec())
                },
            )
        };
        self.inner.advance(len);
        result
    }

    pub fn decode_vec<T: ProtoDecode>(&mut self) -> Result<Vec<T>, DecodeError> {
        let len = self.decode_u32()? as usize;
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            let val = T::decode(self)?;
            vec.push(val);
        }
        Ok(vec)
    }
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

    pub fn encode_vec<T: ProtoEncode>(&mut self, vec: &[T]) -> io::Result<()> {
        self.encode_u32(vec.len() as u32)?;
        for ref item in vec {
            item.encode(self)?;
        }
        Ok(())
    }
}

impl ProtoDecode for u32 {
    fn decode(decoder: &mut ProtoDecoder) -> Result<Self, DecodeError> {
        decoder.decode_u32()
    }
}

impl ProtoEncode for u32 {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_u32(*self)
    }
}

impl ProtoDecode for bool {
    fn decode(decoder: &mut ProtoDecoder) -> Result<Self, DecodeError> {
        decoder.decode_bool()
    }
}

impl ProtoEncode for bool {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_bool(*self)
    }
}

impl ProtoDecode for u16 {
    fn decode(decoder: &mut ProtoDecoder) -> Result<Self, DecodeError> {
        decoder.decode_u16()
    }
}

impl ProtoEncode for u16 {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_u16(*self)
    }
}

impl ProtoDecode for net::Ipv4Addr {
    fn decode(decoder: &mut ProtoDecoder) -> Result<Self, DecodeError> {
        decoder.decode_ipv4_addr()
    }
}

impl ProtoEncode for net::Ipv4Addr {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_ipv4_addr(*self)
    }
}

impl ProtoDecode for String {
    fn decode(decoder: &mut ProtoDecoder) -> Result<Self, DecodeError> {
        decoder.decode_string()
    }
}

impl ProtoEncode for str {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_string(self)
    }
}

// Apparently deref coercion does not work for trait methods.
impl ProtoEncode for String {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_string(self)
    }
}

impl<T: ProtoDecode> ProtoDecode for Vec<T> {
    fn decode(decoder: &mut ProtoDecoder) -> Result<Self, DecodeError> {
        decoder.decode_vec::<T>()
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
    use super::{ProtoDecoder, ProtoEncoder, ProtoDecode, ProtoEncode};

    use std::fmt;
    use std::io;
    use std::net;
    use std::u16;
    use std::u32;

    use bytes::{Buf, BytesMut};

    pub fn roundtrip<T: fmt::Debug + Eq + PartialEq + ProtoDecode + ProtoEncode>(input: T) {
        let mut bytes = BytesMut::new();
        input.encode(&mut ProtoEncoder::new(&mut bytes)).unwrap();

        let mut cursor = io::Cursor::new(bytes);
        let output = T::decode(&mut ProtoDecoder::new(&mut cursor)).unwrap();

        assert_eq!(output, input);
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
            let val = ProtoDecoder::new(&mut cursor).decode_u32().unwrap();
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
        let mut val = ProtoDecoder::new(&mut cursor).decode_bool().unwrap();
        assert!(!val);
        assert_eq!(cursor.remaining(), 0);

        cursor = new_cursor(vec![1]);
        val = ProtoDecoder::new(&mut cursor).decode_bool().unwrap();
        assert!(val);
        assert_eq!(cursor.remaining(), 0);
    }

    #[test]
    #[should_panic]
    fn decode_bool_invalid() {
        let mut cursor = new_cursor(vec![42]);
        ProtoDecoder::new(&mut cursor).decode_bool().unwrap();
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
                let val = ProtoDecoder::new(&mut cursor).decode_u16().unwrap();
                assert_eq!(val, expected_val as u16);
                assert_eq!(cursor.remaining(), 0);
            } else {
                assert!(ProtoDecoder::new(&mut cursor).decode_u16().is_err());
            }
        }
    }

    #[test]
    #[should_panic]
    fn decode_u16_invalid() {
        let mut cursor = new_cursor(vec![0, 0, 1, 0]);
        ProtoDecoder::new(&mut cursor).decode_u16().unwrap();
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
            let val = ProtoDecoder::new(&mut cursor).decode_ipv4_addr().unwrap();
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
            let string = ProtoDecoder::new(&mut cursor).decode_string().unwrap();
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
        let vec = ProtoDecoder::new(&mut cursor).decode_vec::<u32>().unwrap();

        assert_eq!(vec, expected_vec);
        assert_eq!(cursor.remaining(), 0);
    }

    #[test]
    fn roundtrip_u32_vector() {
        roundtrip(vec![0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9])
    }
}
