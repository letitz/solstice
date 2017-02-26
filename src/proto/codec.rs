use std::error;
use std::fmt;
use std::io;
use std::net;
use std::u16;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use encoding::{Encoding, EncoderTrap, DecoderTrap};
use encoding::all::WINDOWS_1252;
use tokio_core::io::EasyBuf;

/// Length of an encoded 32-bit integer in bytes.
const U32_BYTE_LEN : usize = 4;

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
            DecodeError::InvalidBoolError(n) =>
                write!(fmt, "InvalidBoolError: {}", n),
            DecodeError::InvalidU16Error(n) =>
                write!(fmt, "InvalidU16Error: {}", n),
            DecodeError::InvalidStringError(ref bytes) =>
                write!(fmt, "InvalidStringError: {:?}", bytes),
            DecodeError::InvalidUserStatusError(n) =>
                write!(fmt, "InvalidUserStatusError: {}", n),
            DecodeError::IOError(ref err) =>
                write!(fmt, "IOError: {}", err),
        }
    }
}

impl error::Error for DecodeError {
    fn description(&self) -> &str {
        match *self {
            DecodeError::InvalidBoolError(_) =>
                "InvalidBoolError",
            DecodeError::InvalidU16Error(_) =>
                "InvalidU16Error",
            DecodeError::InvalidStringError(_)    =>
                "InvalidStringError",
            DecodeError::InvalidUserStatusError(_) =>
                "InvalidUserStatusError",
            DecodeError::IOError(_) =>
                "IOError",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            DecodeError::InvalidBoolError(_)       => None,
            DecodeError::InvalidU16Error(_)        => None,
            DecodeError::InvalidStringError(_)     => None,
            DecodeError::InvalidUserStatusError(_) => None,
            DecodeError::IOError(ref err)          => Some(err),
        }
    }
}

impl From<io::Error> for DecodeError {
    fn from(err: io::Error) -> Self {
        DecodeError::IOError(err)
    }
}

/*=================*
 * DECODE / ENCODE *
 *=================*/

/// This trait is implemented by types that can be decoded from messages.
/// Decoding values from messages is attempted only after an entire frame has
/// been received, so it is an error if not enough data is available.
pub trait Decode: Sized {
    /// Attempts to decode an instance of `Self` from the bytes in `buf`.
    fn decode(buf: &mut EasyBuf) -> Result<Self, DecodeError>;
}

/// This trait is implemented by types that can be encoded into messages.
pub trait Encode {
    /// Attempts to encode `self` to the given byte buffer.
    fn encode(&self, &mut Vec<u8>) -> io::Result<()>;
}

// 32-bit integers are serialized in 4 bytes, little-endian.

impl Decode for u32 {
    fn decode(buf: &mut EasyBuf) -> Result<Self, DecodeError> {
        if buf.len() < U32_BYTE_LEN {
            return Err(DecodeError::from(
                    io::Error::new(io::ErrorKind::UnexpectedEof, "u32")));
        }
        buf.drain_to(U32_BYTE_LEN).as_slice().read_u32::<LittleEndian>()
            .map_err(DecodeError::from)
    }
}

impl Encode for u32 {
    fn encode(&self, buf: &mut Vec<u8>) -> io::Result<()> {
        buf.write_u32::<LittleEndian>(*self)
    }
}

// Booleans are serialized as single bytes, containing either 0 or 1.

impl Decode for bool {
    fn decode(buf: &mut EasyBuf) -> Result<Self, DecodeError> {
        if buf.len() < 1 {
            return Err(DecodeError::from(
                    io::Error::new(io::ErrorKind::UnexpectedEof, "bool")));
        }
        match buf.drain_to(1).as_slice()[0] {
            0 => Ok(false),
            1 => Ok(true),
            n => Err(DecodeError::InvalidBoolError(n))
        }
    }
}

impl Encode for bool {
    fn encode(&self, buf: &mut Vec<u8>) -> io::Result<()> {
        buf.push(*self as u8);
        Ok(())
    }
}

// 16-bit integers are serialized as 32-bit integers with upper bytes set to 0.

impl Decode for u16 {
    fn decode(buf: &mut EasyBuf) -> Result<Self, DecodeError> {
        let n = try!(u32::decode(buf));
        if n > u16::MAX as u32 {
            return Err(DecodeError::InvalidU16Error(n))
        }
        Ok(n as u16)
    }
}

impl Encode for u16 {
    fn encode(&self, buf: &mut Vec<u8>) -> io::Result<()> {
        (*self as u32).encode(buf)
    }
}

// IPv4 addresses are serialized just as 32-bit integers.

impl Decode for net::Ipv4Addr {
    fn decode(buf: &mut EasyBuf) -> Result<Self, DecodeError> {
        let ip = try!(u32::decode(buf));
        Ok(net::Ipv4Addr::from(ip))
    }
}

impl Encode for net::Ipv4Addr {
    fn encode(&self, buf: &mut Vec<u8>) -> io::Result<()> {
        let mut octets = self.octets();
        octets.reverse();  // Little endian.
        buf.extend(&octets);
        Ok(())
    }
}

// Strings are serialized as 32-bit-length-prefixed arrays of Windows 1252
// encoded characters.

impl Decode for String {
    fn decode(buf: &mut EasyBuf) -> Result<Self, DecodeError> {
        let len = try!(u32::decode(buf)) as usize;
        let contents = buf.drain_to(len);
        match WINDOWS_1252.decode(contents.as_slice(), DecoderTrap::Strict) {
            Ok(string) => Ok(string),
            Err(_) =>
                Err(DecodeError::InvalidStringError(
                        contents.as_slice().to_vec()))
        }
    }
}

impl Encode for str {
    fn encode(&self, buf: &mut Vec<u8>) -> io::Result<()> {
        // Encode the string.
        let bytes = match WINDOWS_1252.encode(self, EncoderTrap::Strict) {
            Ok(bytes) => bytes,
            Err(_) => {
                let copy = self.to_string();
                return Err(io::Error::new(io::ErrorKind::InvalidData, copy));
            }
        };
        // Prefix the bytes with the length.
        (bytes.len() as u32).encode(buf)?;
        buf.extend(bytes);
        Ok(())
    }
}

// Apparently deref coercion does not work for trait methods.
impl Encode for String {
    fn encode(&self, buf: &mut Vec<u8>) -> io::Result<()> {
        (self as &str).encode(buf)
    }
}

// Vectors are serialized as length-prefixed arrays of serialized values.

impl<T: Decode> Decode for Vec<T> {
    fn decode(buf: &mut EasyBuf) -> Result<Self, DecodeError> {
        let len = try!(u32::decode(buf)) as usize;
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            vec.push(try!(T::decode(buf)));
        }
        Ok(vec)
    }
}

impl<T: Encode> Encode for Vec<T> {
    fn encode(&self, buf: &mut Vec<u8>) -> io::Result<()> {
        (self.len() as u32).encode(buf)?;
        for ref item in self {
            item.encode(buf)?;
        }
        Ok(())
    }
}

/*=======*
 * TESTS *
 *=======*/

#[cfg(test)]
mod tests {
    use super::{Decode, Encode};

    use std::net;
    use std::u16;
    use std::u32;

    use tokio_core::io::EasyBuf;

    // A few integers and their corresponding byte encodings.
    const U32_ENCODINGS : [(u32, [u8; 4]); 8] = [
        (0,        [  0,   0,   0,   0]),
        (255,      [255,   0,   0,   0]),
        (256,      [  0,   1,   0,   0]),
        (65535,    [255, 255,   0,   0]),
        (65536,    [  0,   0,   1,   0]),
        (16777215, [255, 255, 255,   0]),
        (16777216, [  0,   0,   0,   1]),
        (u32::MAX, [255, 255, 255, 255]),
    ];

    #[test]
    fn encode_u32() {
        for &(val, ref encoded_bytes) in &U32_ENCODINGS {
            let mut bytes = vec![13];
            val.encode(&mut bytes).unwrap();
            let mut expected_bytes = vec![13];
            expected_bytes.extend(encoded_bytes);
            assert_eq!(bytes, expected_bytes);
        }
    }

    #[test]
    fn decode_u32() {
        for &(expected_val, ref bytes) in &U32_ENCODINGS {
            let mut buf = EasyBuf::from(bytes.to_vec());
            let val = u32::decode(&mut buf).unwrap();
            assert_eq!(val, expected_val);
            assert_eq!(buf.len(), 0);
        }
    }

    #[test]
    fn encode_bool() {
        let mut bytes = vec![13];
        false.encode(&mut bytes);
        assert_eq!(bytes, [13, 0]);

        bytes.truncate(1);
        true.encode(&mut bytes);
        assert_eq!(bytes, [13, 1]);
    }

    #[test]
    fn decode_bool() {
        let mut buf = EasyBuf::from(vec![0]);
        let mut val = bool::decode(&mut buf).unwrap();
        assert!(!val);
        assert_eq!(buf.len(), 0);

        buf = EasyBuf::from(vec![1]);
        val = bool::decode(&mut buf).unwrap();
        assert!(val);
        assert_eq!(buf.len(), 0);

        buf = EasyBuf::from(vec![42]);
        assert!(!bool::decode(&mut buf).is_ok());
    }

    #[test]
    fn encode_u16() {
        for &(val, ref encoded_bytes) in &U32_ENCODINGS {
            if val > u16::MAX as u32 {
                continue;
            }
            let mut bytes = vec![13];
            (val as u16).encode(&mut bytes).unwrap();

            let mut expected_bytes = vec![13];
            expected_bytes.extend(encoded_bytes);
            assert_eq!(bytes, expected_bytes);
        }
    }

    #[test]
    fn decode_u16() {
        for &(expected_val, ref bytes) in &U32_ENCODINGS {
            if expected_val <= u16::MAX as u32 {
                let mut buf = EasyBuf::from(bytes.to_vec());
                let val = u16::decode(&mut buf).unwrap();
                assert_eq!(val, expected_val as u16);
                assert_eq!(buf.len(), 0);
            } else {
                let mut buf = EasyBuf::from(bytes.to_vec());
                assert!(!u16::decode(&mut buf).is_ok());
            }
        }
    }

    #[test]
    fn encode_ipv4() {
        for &(val, ref encoded_bytes) in &U32_ENCODINGS {
            let mut bytes = vec![13];
            net::Ipv4Addr::from(val).encode(&mut bytes).unwrap();

            let mut expected_bytes = vec![13];
            expected_bytes.extend(encoded_bytes);
            assert_eq!(bytes, expected_bytes);
        }
    }

    #[test]
    fn decode_ipv4() {
        for &(expected_val, ref bytes) in &U32_ENCODINGS {
            let mut buf = EasyBuf::from(bytes.to_vec());
            let val = net::Ipv4Addr::decode(&mut buf).unwrap();
            assert_eq!(val, net::Ipv4Addr::from(expected_val));
            assert_eq!(buf.len(), 0);
        }
    }

    // A few strings and their corresponding encodings.
    const STRING_ENCODINGS: [(&'static str, &'static [u8]); 3] = [
        ("",      &[0, 0, 0, 0]),
        ("hey!",  &[4, 0, 0, 0, 104, 101, 121, 33]),
        // Windows 1252 specific codepoints.
        ("‘’“”€", &[5, 0, 0, 0, 145, 146, 147, 148, 128]),
    ];

    #[test]
    fn encode_string() {
        for &(string, encoded_bytes) in &STRING_ENCODINGS {
            let mut bytes = vec![13];
            string.encode(&mut bytes).unwrap();

            let mut expected_bytes = vec![13];
            expected_bytes.extend(encoded_bytes);
            assert_eq!(bytes, expected_bytes);
        }
    }

    #[test]
    #[should_panic]
    fn encode_invalid_string() {
        let mut bytes = vec![];
        "忠犬ハチ公".encode(&mut bytes).unwrap();
    }

    #[test]
    fn decode_string() {
        for &(expected_string, bytes) in &STRING_ENCODINGS {
            let mut buf = EasyBuf::from(bytes.to_vec());
            let string = String::decode(&mut buf).unwrap();
            assert_eq!(string, expected_string);
            assert_eq!(buf.len(), 0);
        }
    }
}
