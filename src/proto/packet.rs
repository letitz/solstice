use std::error;
use std::fmt;
use std::io;
use std::net;
use std::io::{Read, Write};

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use encoding::{Encoding, DecoderTrap, EncoderTrap};
use encoding::all::ISO_8859_1;

use super::constants::*;

/*==================*
 * READ-ONLY PACKET *
 *==================*/

#[derive(Debug)]
pub struct Packet {
    /// The current read position in the byte buffer.
    cursor: usize,
    /// The underlying bytes.
    bytes: Vec<u8>,
}

impl io::Read for Packet {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let bytes_read = {
            let mut slice = &self.bytes[self.cursor..];
            try!(slice.read(buf))
        };
        self.cursor += bytes_read;
        Ok(bytes_read)
    }
}

impl Packet {
    /// Returns a readable packet struct from the wire representation of a
    /// packet.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        // Check that the packet is long enough to contain at least a code.
        assert!(bytes.len() >= U32_SIZE);
        // Read the purported length of the packet.
        let size = LittleEndian::read_u32(&bytes[0..U32_SIZE]) as usize;
        // Check that the packet has the right length.
        assert!(size + U32_SIZE == bytes.len());
        Packet {
            cursor: U32_SIZE,
            bytes:  bytes,
        }
    }

    /// Provides the main way to read data out of a binary packet.
    pub fn read_value<T>(&mut self) -> Result<T, PacketReadError>
        where T: ReadFromPacket
    {
        T::read_from_packet(self)
    }

    /// Returns the number of unread bytes remaining in the packet.
    pub fn bytes_remaining(&self) -> usize {
        self.bytes.len() - self.cursor
    }
}

/*===================*
 * WRITE-ONLY PACKET *
 *===================*/

#[derive(Debug)]
pub struct MutPacket {
    bytes: Vec<u8>,
}

impl MutPacket {
    /// Returns an empty packet with the given packet code.
    pub fn new() -> Self {
        // Leave space for the eventual size of the packet.
        MutPacket {
            bytes: vec![0; U32_SIZE]
        }
    }

    /// Provides the main way to write data into a binary packet.
    pub fn write_value<T>(&mut self, val: T) -> io::Result<()>
        where T: WriteToPacket
    {
        val.write_to_packet(self)
    }

    /// Consumes the mutable packet and returns its wire representation.
    pub fn into_bytes(mut self) -> Vec<u8> {
        let length = (self.bytes.len() - U32_SIZE) as u32;
        {
            let mut first_word = &mut self.bytes[..U32_SIZE];
            first_word.write_u32::<LittleEndian>(length).unwrap();
        }
        self.bytes
    }
}

impl io::Write for MutPacket {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.bytes.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.bytes.flush()
    }
}

/*===================*
 * PACKET READ ERROR *
 *===================*/

/// This enum contains an error that arose when reading data out of a Packet.
#[derive(Debug)]
pub enum PacketReadError {
    /// Attempted to read a boolean, but the value was not 0 nor 1.
    InvalidBoolError(u8),
    /// Attempted to read an unsigned 16-bit integer, but the value was too
    /// large.
    InvalidU16Error(u32),
    /// Attempted to read a string, but a character was invalid.
    InvalidStringError(Vec<u8>),
    /// Attempted to read a user::Status, but the value was not a valid
    /// representation of an enum variant.
    InvalidUserStatusError(u32),
    /// Encountered an I/O error while reading.
    IOError(io::Error),
}

impl fmt::Display for PacketReadError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PacketReadError::InvalidBoolError(n) =>
                write!(fmt, "InvalidBoolError: {}", n),
            PacketReadError::InvalidU16Error(n) =>
                write!(fmt, "InvalidU16Error: {}", n),
            PacketReadError::InvalidStringError(ref bytes) =>
                write!(fmt, "InvalidStringError: {:?}", bytes),
            PacketReadError::InvalidUserStatusError(n) =>
                write!(fmt, "InvalidUserStatusError: {}", n),
            PacketReadError::IOError(ref err) =>
                write!(fmt, "IOError: {}", err),
        }
    }
}

impl error::Error for PacketReadError {
    fn description(&self) -> &str {
        match *self {
            PacketReadError::InvalidBoolError(_) =>
                "InvalidBoolError",
            PacketReadError::InvalidU16Error(_) =>
                "InvalidU16Error",
            PacketReadError::InvalidStringError(_) =>
                "InvalidStringError",
            PacketReadError::InvalidUserStatusError(_) =>
                "InvalidUserStatusError",
            PacketReadError::IOError(_) =>
                "IOError",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            PacketReadError::InvalidBoolError(_)       => None,
            PacketReadError::InvalidU16Error(_)        => None,
            PacketReadError::InvalidStringError(_)     => None,
            PacketReadError::InvalidUserStatusError(_) => None,
            PacketReadError::IOError(ref err)          => Some(err),
        }
    }
}

impl From<io::Error> for PacketReadError {
    fn from(err: io::Error) -> Self {
        PacketReadError::IOError(err)
    }
}

/*==================*
 * READ FROM PACKET *
 *==================*/

/// This trait is implemented by types that can be deserialized from binary
/// Packets.
pub trait ReadFromPacket: Sized {
    fn read_from_packet(&mut Packet) -> Result<Self, PacketReadError>;
}

/// 32-bit integers are serialized in 4 bytes, little-endian.
impl ReadFromPacket for u32 {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        packet.read_u32::<LittleEndian>().map_err(PacketReadError::from)
    }
}

/// For convenience, usize's are deserialized as u32's then casted.
impl ReadFromPacket for usize {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let n: u32 = try!(packet.read_value());
        Ok(n as usize)
    }
}

/// Booleans are serialized as single bytes, containing either 0 or 1.
impl ReadFromPacket for bool {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let mut buffer = vec![0];
        try!(packet.read(&mut buffer));
        match buffer[0] {
            0 => Ok(false),
            1 => Ok(true),
            n => Err(PacketReadError::InvalidBoolError(n))
        }
    }
}

/// 16-bit integers are serialized as 32-bit integers.
impl ReadFromPacket for u16 {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let n: u32 = try!(packet.read_value());
        if n > MAX_PORT {
            return Err(PacketReadError::InvalidU16Error(n))
        }
        Ok(n as u16)
    }
}

/// IPv4 addresses are serialized directly as 32-bit integers.
impl ReadFromPacket for net::Ipv4Addr {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let ip: u32 = try!(packet.read_value());
        Ok(net::Ipv4Addr::from(ip))
    }
}

/// Strings are serialized as length-prefixed arrays of ISO-8859-1 encoded
/// characters.
impl ReadFromPacket for String {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let len = try!(packet.read_value());
        let mut buffer = vec![0; len];
        try!(packet.read(&mut buffer));
        match ISO_8859_1.decode(&buffer, DecoderTrap::Strict) {
            Ok(string) => Ok(string),
            Err(_) => Err(PacketReadError::InvalidStringError(buffer))
        }
    }
}

/// Vectors are serialized as length-prefixed arrays of values.
impl<T: ReadFromPacket> ReadFromPacket for Vec<T> {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let len: usize = try!(packet.read_value());
        let mut vec = Vec::new();
        for _ in 0..len {
            vec.push(try!(packet.read_value()));
        }
        Ok(vec)
    }
}

/*=================*
 * WRITE TO PACKET *
 *=================*/

/// This trait is implemented by types that can be serialized to a binary
/// MutPacket.
pub trait WriteToPacket {
    fn write_to_packet(self, &mut MutPacket) -> io::Result<()>;
}

/// 32-bit integers are serialized in 4 bytes, little-endian.
impl WriteToPacket for u32 {
    fn write_to_packet(self, packet: &mut MutPacket) -> io::Result<()> {
        packet.write_u32::<LittleEndian>(self)
    }
}

/// Booleans are serialized as single bytes, containing either 0 or 1.
impl WriteToPacket for bool {
    fn write_to_packet(self, packet: &mut MutPacket) -> io::Result<()> {
        try!(packet.write(&[self as u8]));
        Ok(())
    }
}

/// 16-bit integers are serialized as 32-bit integers.
impl WriteToPacket for u16 {
    fn write_to_packet(self, packet: &mut MutPacket) -> io::Result<()> {
        (self as u32).write_to_packet(packet)
    }
}

/// Strings are serialized as a length-prefixed array of ISO-8859-1 encoded
/// characters.
impl<'a> WriteToPacket for &'a str {
    fn write_to_packet(self, packet: &mut MutPacket) -> io::Result<()> {
        let bytes = match ISO_8859_1.encode(self, EncoderTrap::Strict) {
            Ok(bytes) => bytes,
            Err(_) => {
                let copy = self.to_string();
                return Err(io::Error::new(io::ErrorKind::Other, copy))
            }
        };
        try!(packet.write_value(bytes.len() as u32));
        try!(packet.write(&bytes));
        Ok(())
    }
}

/// Deref coercion does not happen for trait methods apparently.
impl<'a> WriteToPacket for &'a String {
    fn write_to_packet(self, packet: &mut MutPacket) -> io::Result<()> {
        packet.write_value::<&'a str>(self)
    }
}
