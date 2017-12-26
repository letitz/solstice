use std::error;
use std::fmt;
use std::io;
use std::net;
use std::u16;

use bytes::{Buf, BufMut, BytesMut, LittleEndian};
use bytes::buf::IntoBuf;
use encoding::{Encoding, EncoderTrap, DecoderTrap};
use encoding::all::WINDOWS_1252;
use tokio_core::io::EasyBuf;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::{Decoder, Encoder};
use tokio_io::codec::length_delimited;

use proto::server::ServerResponse;

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
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            DecodeError::InvalidBoolError(_) => None,
            DecodeError::InvalidU16Error(_) => None,
            DecodeError::InvalidStringError(_) => None,
            DecodeError::InvalidUserStatusError(_) => None,
            DecodeError::IOError(ref err) => Some(err),
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

/*=================*
 * DECODE / ENCODE *
 *=================*/

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
/// Only here to enable ProtoDecoder::decode_vec.
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
    // around this limitation by storing the cursor itself.
    inner: &'a mut io::Cursor<BytesMut>,
}

impl<'a> ProtoDecoder<'a> {
    fn new(cursor: &'a mut io::Cursor<BytesMut>) -> ProtoDecoder<'a> {
        ProtoDecoder { inner: cursor }
    }

    fn decode_u32(&mut self) -> Result<u32, DecodeError> {
        if self.inner.remaining() < U32_BYTE_LEN {
            return Err(unexpected_eof_error("u32"));
        }
        Ok(self.inner.get_u32::<LittleEndian>())
    }

    fn decode_u16(&mut self) -> Result<u16, DecodeError> {
        let n = self.decode_u32()?;
        if n > u16::MAX as u32 {
            return Err(DecodeError::InvalidU16Error(n));
        }
        Ok(n as u16)
    }

    fn decode_bool(&mut self) -> Result<bool, DecodeError> {
        if self.inner.remaining() < 1 {
            return Err(unexpected_eof_error("bool"));
        }
        match self.inner.get_u8() {
            0 => Ok(false),
            1 => Ok(true),
            n => Err(DecodeError::InvalidBoolError(n)),
        }
    }

    fn decode_ipv4_addr(&mut self) -> Result<net::Ipv4Addr, DecodeError> {
        let ip = self.decode_u32()?;
        Ok(net::Ipv4Addr::from(ip))
    }

    fn decode_string(&mut self) -> Result<String, DecodeError> {
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

    fn decode_vec<T: ProtoDecode>(&mut self) -> Result<Vec<T>, DecodeError> {
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
    // If bytes::BufMut was object-safe we would store an &'a BufMut. We work
    // around this limiation by using BytesMut directly.
    inner: &'a mut BytesMut,
}

impl<'a> ProtoEncoder<'a> {
    fn new(buf: &'a mut BytesMut) -> ProtoEncoder {
        ProtoEncoder { inner: buf }
    }

    fn encode_u32(&mut self, val: u32) -> io::Result<()> {
        if self.inner.remaining_mut() < U32_BYTE_LEN {
            self.inner.reserve(U32_BYTE_LEN);
        }
        self.inner.put_u32::<LittleEndian>(val);
        Ok(())
    }

    fn encode_u16(&mut self, val: u16) -> io::Result<()> {
        self.encode_u32(val as u32)
    }

    fn encode_bool(&mut self, val: bool) -> io::Result<()> {
        if !self.inner.has_remaining_mut() {
            self.inner.reserve(1);
        }
        self.inner.put_u8(val as u8);
        Ok(())
    }

    fn encode_ipv4_addr(&mut self, addr: net::Ipv4Addr) -> io::Result<()> {
        let mut octets = addr.octets();
        octets.reverse(); // Little endian.
        self.inner.extend(&octets);
        Ok(())
    }

    fn encode_string(&mut self, val: &str) -> io::Result<()> {
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

    fn encode_vec<T: ProtoEncode>(&mut self, vec: &[T]) -> io::Result<()> {
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

/*=================*
 * DECODER/ENCODER *
 *=================*/

fn new_length_prefixed_framed<T, B>(inner: T) -> length_delimited::Framed<T, B>
where
    T: AsyncRead + AsyncWrite,
    B: IntoBuf,
{
    length_delimited::Builder::new()
        .length_field_length(4)
        .little_endian()
        .new_framed(inner)
}

struct ServerResponseDecoder;

impl Decoder for ServerResponseDecoder {
    type Item = ServerResponse;
    type Error = DecodeError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        unimplemented!();
    }
}

/*=======*
 * TESTS *
 *=======*/

#[cfg(test)]
mod tests {
    use super::{ProtoDecoder, ProtoEncoder, ProtoDecode, ProtoEncode};

    use std::io;
    use std::net;
    use std::u16;
    use std::u32;

    use bytes::{Buf, BytesMut};

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
}
