use std::error;
use std::fmt;
use std::io;
use std::mem;
use std::net;
use std::iter;
use std::io::{Read, Write};

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use encoding::{Encoding, DecoderTrap, EncoderTrap};
use encoding::all::ISO_8859_1;
use mio::{
    Evented, EventLoop, EventSet, Handler, PollOpt, Token, TryRead, TryWrite
};

const MAX_PACKET_SIZE: usize = 1 << 20; // 1 MiB
const U32_SIZE: usize = 4;
const MAX_MESSAGE_SIZE: usize = MAX_PACKET_SIZE - U32_SIZE;

const MAX_PORT: u32 = (1 << 16) - 1;

/*========*
 * PACKET *
 *========*/

#[derive(Debug)]
pub struct Packet {
    cursor: usize,
    bytes: Vec<u8>,
}

impl io::Read for Packet {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut slice = &self.bytes[self.cursor..];
        let result = slice.read(buf);
        if let Ok(num_bytes_read) = result {
            self.cursor += num_bytes_read
        }
        result
    }
}

impl io::Write for Packet {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.bytes.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.bytes.flush()
    }
}

impl Packet {
    /// Returns an empty packet with the given packet code.
    pub fn new(code: u32) -> Self {
        let mut bytes = Vec::new();
        bytes.write_u32::<LittleEndian>(0).unwrap();
        bytes.write_u32::<LittleEndian>(code).unwrap();
        Packet {
            cursor: 2*U32_SIZE,
            bytes: bytes,
        }
    }

    /// Returns a new packet struct, constructed from the wire representation
    /// of a packet.
    fn from_raw_parts(bytes: Vec<u8>) -> Self {
        let size = LittleEndian::read_u32(&bytes[..U32_SIZE]) as usize;
        assert!(size + U32_SIZE == bytes.len());
        Packet {
            cursor: U32_SIZE,
            bytes: bytes,
        }
    }

    /// Provides the main way to read data out of a binary packet.
    pub fn read_value<T: ReadFromPacket>(&mut self)
        -> Result<T, PacketReadError>
    {
        T::read_from_packet(self)
    }

    /// Provides the main way to write data into a binary packet.
    pub fn write_value<T: WriteToPacket>(&mut self, val: T)
        -> io::Result<()>
    {
        val.write_to_packet(self)
    }

    /// Returns the number of unread bytes remaining in the packet.
    pub fn bytes_remaining(&self) -> usize {
        self.bytes.len() - self.cursor
    }

    /// Returns a slice pointing to the entire underlying byte array, including
    /// the length prefix.
    pub fn as_slice(&mut self) -> &[u8] {
        let bytes_len = (self.bytes.len() - U32_SIZE) as u32;
        {
            let mut first_word = &mut self.bytes[..U32_SIZE];
            first_word.write_u32::<LittleEndian>(bytes_len).unwrap();
        }
        &self.bytes
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
/// Packet.
pub trait WriteToPacket {
    fn write_to_packet(self, &mut Packet) -> io::Result<()>;
}

/// 32-bit integers are serialized in 4 bytes, little-endian.
impl WriteToPacket for u32 {
    fn write_to_packet(self, packet: &mut Packet) -> io::Result<()> {
        packet.write_u32::<LittleEndian>(self)
    }
}

/// Booleans are serialized as single bytes, containing either 0 or 1.
impl WriteToPacket for bool {
    fn write_to_packet(self, packet: &mut Packet) -> io::Result<()> {
        try!(packet.write(&[self as u8]));
        Ok(())
    }
}

/// 16-bit integers are serialized as 32-bit integers.
impl WriteToPacket for u16 {
    fn write_to_packet(self, packet: &mut Packet) -> io::Result<()> {
        (self as u32).write_to_packet(packet)
    }
}

/// Strings are serialized as a length-prefixed array of ISO-8859-1 encoded
/// characters.
impl<'a> WriteToPacket for &'a str {
    fn write_to_packet(self, packet: &mut Packet) -> io::Result<()> {
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
    fn write_to_packet(self, packet: &mut Packet) -> io::Result<()> {
        packet.write_value::<&'a str>(self)
    }
}

/*===============*
 * PACKET STREAM *
 *===============*/

/// This enum defines the possible states a PacketStream state machine can be
/// in.
#[derive(Debug, Clone, Copy)]
enum State {
    /// The PacketStream is waiting to read enough bytes to determine the
    /// length of the following packet.
    ReadingLength,
    /// The PacketStream is waiting to read enough bytes to form the entire
    /// packet.
    ReadingPacket,
}

/// This struct wraps around an mio byte stream and provides the ability to
/// read 32-bit-length-prefixed packets of bytes from it.
#[derive(Debug)]
pub struct PacketStream<T: Read + Write + Evented> {
    stream: T,
    state: State,
    num_bytes_left: usize,
    buffer: Vec<u8>,
}

impl<T: Read + Write + Evented> PacketStream<T> {

    /// Returns a new PacketStream wrapping the provided byte stream.
    pub fn new(stream: T) -> Self {
        PacketStream {
            stream: stream,
            state: State::ReadingLength,
            num_bytes_left: U32_SIZE,
            buffer: vec![0; U32_SIZE],
        }
    }

    /// Attemps to read a packet in a non-blocking fashion.
    /// If enough bytes can be read from the underlying byte stream to form a
    /// complete packet `p`, returns `Ok(Some(p))`.
    /// If not enough bytes are available, returns `Ok(None)`.
    /// If an I/O error `e` arises when trying to read the underlying stream,
    /// returns `Err(e)`.
    /// Note: as long as this function returns `Ok(Some(p))`, the caller is
    /// responsible for calling it once more to ensure that all packets are
    /// read as soon as possible.
    pub fn try_read(&mut self) -> io::Result<Option<Packet>> {
        // Try to read as many bytes as we currently need from the underlying
        // byte stream.
        let offset = self.buffer.len() - self.num_bytes_left;
        match try!(self.stream.try_read(&mut self.buffer[offset..])) {
            None => (),

            Some(num_bytes_read) => {
                assert!(num_bytes_read <= self.num_bytes_left);
                self.num_bytes_left -= num_bytes_read;
            },
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
                let message_len =
                    LittleEndian::read_u32(&mut self.buffer) as usize;
                if message_len > MAX_MESSAGE_SIZE {
                    unimplemented!();
                };
                self.state = State::ReadingPacket;
                self.num_bytes_left = message_len;
                self.buffer.extend(iter::repeat(0).take(message_len));
                self.try_read()
            },

            State::ReadingPacket => {
                // If we have finished reading the packet, swap the full buffer
                // out and return the packet made from the full buffer.
                self.state = State::ReadingLength;
                self.num_bytes_left = U32_SIZE;
                let new_buffer = vec![0;U32_SIZE];
                let old_buffer = mem::replace(&mut self.buffer, new_buffer);
                Ok(Some(Packet::from_raw_parts(old_buffer)))
            }
        }
    }

    /// Tries to write a given packet to the underlying byte stream.
    /// TODO: If the packet is not entirely written in the first call, this
    /// will send garbage along the wire. Instead we should track how far we
    /// are in sending the given packet?
    pub fn try_write(&mut self, packet: &mut Packet) -> io::Result<Option<()>> {
        match try!(self.stream.try_write(packet.as_slice())) {
            None => Ok(None),
            Some(_) => Ok(Some(()))
        }
    }

    /// Register the packet stream with the given mio event loop.
    pub fn register<U: Handler>(
        &self, event_loop: &mut EventLoop<U>, token: Token,
        event_set: EventSet, poll_opt: PollOpt)
        -> io::Result<()>
    {
        event_loop.register(&self.stream, token, event_set, poll_opt)
    }

    /// Re-register the packet stream with the given mio event loop.
    pub fn reregister<U: Handler>(
        &self, event_loop: &mut EventLoop<U>, token: Token,
        event_set: EventSet, poll_opt: PollOpt)
        -> io::Result<()>
    {
        event_loop.reregister(&self.stream, token, event_set, poll_opt)
    }
}
