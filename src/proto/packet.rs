use std::error;
use std::fmt;
use std::io;
use std::mem;
use std::net;
use std::iter::repeat;
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
    pub fn new(code: u32) -> Self {
        let mut bytes = Vec::new();
        bytes.write_u32::<LittleEndian>(0).unwrap();
        bytes.write_u32::<LittleEndian>(code).unwrap();
        Packet {
            cursor: 2*U32_SIZE,
            bytes: bytes,
        }
    }

    fn from_raw_parts(bytes: Vec<u8>) -> Self {
        let size = LittleEndian::read_u32(&bytes[..U32_SIZE]) as usize;
        assert!(size + U32_SIZE == bytes.len());
        Packet {
            cursor: U32_SIZE,
            bytes: bytes,
        }
    }

    // Writing convenience

    pub fn write_port(&mut self, port: u16) -> io::Result<()> {
        self.write_value(port as u32)
    }

    /// This function is necessary because not all u16 values are encoded in
    /// 4 bytes.
    pub fn read_port(&mut self) -> Result<u16, PacketReadError> {
        let port: u32 = try!(self.read_value());
        if port > MAX_PORT {
            return Err(PacketReadError::InvalidPortError(port))
        }
        Ok(port as u16)
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

#[derive(Debug)]
pub enum PacketReadError {
    InvalidBoolError(u8),
    InvalidPortError(u32),
    InvalidStringError(Vec<u8>),
    InvalidUserStatusError(u32),
    IOError(io::Error),
}

impl fmt::Display for PacketReadError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PacketReadError::InvalidBoolError(n) =>
                write!(fmt, "InvalidBoolError: {}", n),
            PacketReadError::InvalidPortError(n) =>
                write!(fmt, "InvalidPortError: {}", n),
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
            PacketReadError::InvalidPortError(_) =>
                "InvalidPortError",
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
            PacketReadError::InvalidPortError(_)       => None,
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

pub trait ReadFromPacket: Sized {
    fn read_from_packet(&mut Packet) -> Result<Self, PacketReadError>;
}

impl ReadFromPacket for u32 {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        packet.read_u32::<LittleEndian>().map_err(PacketReadError::from)
    }
}

impl ReadFromPacket for usize {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let n: u32 = try!(packet.read_value());
        Ok(n as usize)
    }
}

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

impl ReadFromPacket for net::Ipv4Addr {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let ip: u32 = try!(packet.read_value());
        Ok(net::Ipv4Addr::from(ip))
    }
}

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

pub trait WriteToPacket: Sized {
    fn write_to_packet(self, &mut Packet) -> io::Result<()>;
}

impl WriteToPacket for u32 {
    fn write_to_packet(self, packet: &mut Packet) -> io::Result<()> {
        packet.write_u32::<LittleEndian>(self)
    }
}

impl WriteToPacket for bool {
    fn write_to_packet(self, packet: &mut Packet) -> io::Result<()> {
        try!(packet.write(&[self as u8]));
        Ok(())
    }
}

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

    pub fn new(stream: T) -> Self {
        PacketStream {
            stream: stream,
            state: State::ReadingLength,
            num_bytes_left: U32_SIZE,
            buffer: vec![0; U32_SIZE],
        }
    }

    pub fn try_read(&mut self) -> io::Result<Option<Packet>> {
        let offset = self.buffer.len() - self.num_bytes_left;
        match try!(self.stream.try_read(&mut self.buffer[offset..])) {
            None => (),

            Some(num_bytes_read) => {
                assert!(num_bytes_read <= self.num_bytes_left);
                self.num_bytes_left -= num_bytes_read;
            },
        }

        if self.num_bytes_left > 0 {
            return Ok(None);
        }

        match self.state {
            State::ReadingLength => {
                let message_len =
                    LittleEndian::read_u32(&mut self.buffer) as usize;
                if message_len > MAX_MESSAGE_SIZE {
                    unimplemented!();
                };
                self.state = State::ReadingPacket;
                self.num_bytes_left = message_len;
                self.buffer.extend(repeat(0).take(message_len));
                self.try_read()
            },

            State::ReadingPacket => {
                self.state = State::ReadingLength;
                self.num_bytes_left = U32_SIZE;
                let new_buffer = vec![0;U32_SIZE];
                let old_buffer = mem::replace(&mut self.buffer, new_buffer);
                Ok(Some(Packet::from_raw_parts(old_buffer)))
            }
        }
    }

    pub fn try_write(&mut self, packet: &mut Packet) -> io::Result<Option<()>> {
        match try!(self.stream.try_write(packet.as_slice())) {
            None => Ok(None),
            Some(_) => Ok(Some(()))
        }
    }

    pub fn register<U: Handler>(
        &self, event_loop: &mut EventLoop<U>, token: Token,
        event_set: EventSet, poll_opt: PollOpt)
        -> io::Result<()>
    {
        event_loop.register(&self.stream, token, event_set, poll_opt)
    }

    pub fn reregister<U: Handler>(
        &self, event_loop: &mut EventLoop<U>, token: Token,
        event_set: EventSet, poll_opt: PollOpt)
        -> io::Result<()>
    {
        event_loop.reregister(&self.stream, token, event_set, poll_opt)
    }
}
