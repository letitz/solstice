use std::error;
use std::fmt;
use std::io;
use std::io::{Read, Write};
use std::mem;
use std::net;

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use encoding::all::ISO_8859_1;
use encoding::{DecoderTrap, EncoderTrap, Encoding};
#[allow(deprecated)]
use mio::deprecated::TryRead;

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
            slice.read(buf)?
        };
        self.cursor += bytes_read;
        Ok(bytes_read)
    }
}

impl Packet {
    /// Returns a readable packet struct from the wire representation of a
    /// packet.
    /// Assumes that the given vector is a valid length-prefixed packet.
    fn from_wire(bytes: Vec<u8>) -> Self {
        Packet {
            cursor: U32_SIZE,
            bytes: bytes,
        }
    }

    /// Provides the main way to read data out of a binary packet.
    pub fn read_value<T>(&mut self) -> Result<T, PacketReadError>
    where
        T: ReadFromPacket,
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
            bytes: vec![0; U32_SIZE],
        }
    }

    /// Provides the main way to write data into a binary packet.
    pub fn write_value<T>(&mut self, val: &T) -> io::Result<()>
    where
        T: WriteToPacket,
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
            PacketReadError::InvalidBoolError(n) => write!(fmt, "InvalidBoolError: {}", n),
            PacketReadError::InvalidU16Error(n) => write!(fmt, "InvalidU16Error: {}", n),
            PacketReadError::InvalidStringError(ref bytes) => {
                write!(fmt, "InvalidStringError: {:?}", bytes)
            }
            PacketReadError::InvalidUserStatusError(n) => {
                write!(fmt, "InvalidUserStatusError: {}", n)
            }
            PacketReadError::IOError(ref err) => write!(fmt, "IOError: {}", err),
        }
    }
}

impl error::Error for PacketReadError {
    fn description(&self) -> &str {
        match *self {
            PacketReadError::InvalidBoolError(_) => "InvalidBoolError",
            PacketReadError::InvalidU16Error(_) => "InvalidU16Error",
            PacketReadError::InvalidStringError(_) => "InvalidStringError",
            PacketReadError::InvalidUserStatusError(_) => "InvalidUserStatusError",
            PacketReadError::IOError(_) => "IOError",
        }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            PacketReadError::InvalidBoolError(_) => None,
            PacketReadError::InvalidU16Error(_) => None,
            PacketReadError::InvalidStringError(_) => None,
            PacketReadError::InvalidUserStatusError(_) => None,
            PacketReadError::IOError(ref err) => Some(err),
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
    fn read_from_packet(_: &mut Packet) -> Result<Self, PacketReadError>;
}

/// 32-bit integers are serialized in 4 bytes, little-endian.
impl ReadFromPacket for u32 {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        Ok(packet.read_u32::<LittleEndian>()?)
    }
}

/// For convenience, usize's are deserialized as u32's then casted.
impl ReadFromPacket for usize {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        Ok(u32::read_from_packet(packet)? as usize)
    }
}

/// Booleans are serialized as single bytes, containing either 0 or 1.
impl ReadFromPacket for bool {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        match packet.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            n => Err(PacketReadError::InvalidBoolError(n)),
        }
    }
}

/// 16-bit integers are serialized as 32-bit integers.
impl ReadFromPacket for u16 {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let n = u32::read_from_packet(packet)?;
        if n > MAX_PORT {
            return Err(PacketReadError::InvalidU16Error(n));
        }
        Ok(n as u16)
    }
}

/// IPv4 addresses are serialized directly as 32-bit integers.
impl ReadFromPacket for net::Ipv4Addr {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let ip = u32::read_from_packet(packet)?;
        Ok(net::Ipv4Addr::from(ip))
    }
}

/// Strings are serialized as length-prefixed arrays of ISO-8859-1 encoded
/// characters.
impl ReadFromPacket for String {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let len = usize::read_from_packet(packet)?;

        let mut buffer = vec![0; len];
        packet.read_exact(&mut buffer)?;

        match ISO_8859_1.decode(&buffer, DecoderTrap::Strict) {
            Ok(string) => Ok(string),
            Err(_) => Err(PacketReadError::InvalidStringError(buffer)),
        }
    }
}

/// Vectors are serialized as length-prefixed arrays of values.
impl<T: ReadFromPacket> ReadFromPacket for Vec<T> {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let len = usize::read_from_packet(packet)?;

        let mut vec = Vec::new();
        for _ in 0..len {
            vec.push(T::read_from_packet(packet)?);
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
    fn write_to_packet(&self, _: &mut MutPacket) -> io::Result<()>;
}

/// 32-bit integers are serialized in 4 bytes, little-endian.
impl WriteToPacket for u32 {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        packet.write_u32::<LittleEndian>(*self)
    }
}

/// Booleans are serialized as single bytes, containing either 0 or 1.
impl WriteToPacket for bool {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        packet.write_u8(*self as u8)?;
        Ok(())
    }
}

/// 16-bit integers are serialized as 32-bit integers.
impl WriteToPacket for u16 {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        (*self as u32).write_to_packet(packet)
    }
}

/// Strings are serialized as a length-prefixed array of ISO-8859-1 encoded
/// characters.
impl WriteToPacket for str {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        // Encode the string.
        let bytes = match ISO_8859_1.encode(self, EncoderTrap::Strict) {
            Ok(bytes) => bytes,
            Err(_) => {
                let copy = self.to_string();
                return Err(io::Error::new(io::ErrorKind::Other, copy));
            }
        };
        // Then write the bytes to the packet.
        (bytes.len() as u32).write_to_packet(packet)?;
        packet.write(&bytes)?;
        Ok(())
    }
}

/// Deref coercion does not happen for trait methods apparently.
impl WriteToPacket for String {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        (self as &str).write_to_packet(packet)
    }
}

/*========*
 * PARSER *
 *========*/

/// This enum defines the possible states of a packet parser state machine.
#[derive(Debug, Clone, Copy)]
enum State {
    /// The parser is waiting to read enough bytes to determine the
    /// length of the following packet.
    ReadingLength,
    /// The parser is waiting to read enough bytes to form the entire
    /// packet.
    ReadingPacket,
}

#[derive(Debug)]
pub struct Parser {
    state: State,
    num_bytes_left: usize,
    buffer: Vec<u8>,
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            state: State::ReadingLength,
            num_bytes_left: U32_SIZE,
            buffer: vec![0; U32_SIZE],
        }
    }

    /// Attemps to read a packet in a non-blocking fashion.
    /// If enough bytes can be read from the given byte stream to form a
    /// complete packet `p`, returns `Ok(Some(p))`.
    /// If not enough bytes are available, returns `Ok(None)`.
    /// If an I/O error `e` arises when trying to read the underlying stream,
    /// returns `Err(e)`.
    /// Note: as long as this function returns `Ok(Some(p))`, the caller is
    /// responsible for calling it once more to ensure that all packets are
    /// read as soon as possible.
    pub fn try_read<U>(&mut self, stream: &mut U) -> io::Result<Option<Packet>>
    where
        U: io::Read,
    {
        // Try to read as many bytes as we currently need from the underlying
        // byte stream.
        let offset = self.buffer.len() - self.num_bytes_left;

        #[allow(deprecated)]
        match stream.try_read(&mut self.buffer[offset..])? {
            None => (),

            Some(num_bytes_read) => {
                self.num_bytes_left -= num_bytes_read;
            }
        }

        // If we haven't read enough bytes, return.
        if self.num_bytes_left > 0 {
            return Ok(None);
        }

        // Otherwise, the behavior depends on what state we were in.
        match self.state {
            State::ReadingLength => {
                // If we have finished reading the length prefix, then
                // deserialize it, switch states and try to read the packet
                // bytes.
                let message_len = LittleEndian::read_u32(&mut self.buffer) as usize;
                if message_len > MAX_MESSAGE_SIZE {
                    unimplemented!();
                };
                self.state = State::ReadingPacket;
                self.num_bytes_left = message_len;
                self.buffer.resize(message_len + U32_SIZE, 0);
                self.try_read(stream)
            }

            State::ReadingPacket => {
                // If we have finished reading the packet, swap the full buffer
                // out and return the packet made from the full buffer.
                self.state = State::ReadingLength;
                self.num_bytes_left = U32_SIZE;
                let new_buffer = vec![0; U32_SIZE];
                let old_buffer = mem::replace(&mut self.buffer, new_buffer);
                Ok(Some(Packet::from_wire(old_buffer)))
            }
        }
    }
}
