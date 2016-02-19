use std::iter::repeat;
use std::io;
use std::io::{Read, Write};
use std::mem;

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use mio::{
    Evented, EventLoop, EventSet, Handler, PollOpt, Token, TryRead, TryWrite
};

const MAX_PACKET_SIZE: usize = 1 << 20; // 1 MiB
const U32_SIZE: usize = 4;
const MAX_MESSAGE_SIZE: usize = MAX_PACKET_SIZE - U32_SIZE;


#[derive(Debug)]
pub struct Packet {
    cursor: usize,
    bytes: Vec<u8>,
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

    pub fn write_str(&mut self, string: &str) -> io::Result<usize> {
        try!(self.write_uint(string.len() as u32));
        let n = try!(self.write(string.as_bytes()));
        Ok(n + U32_SIZE)
    }

    pub fn write_uint(&mut self, n: u32) -> io::Result<usize> {
        match self.write_u32::<LittleEndian>(n) {
            Ok(()) => Ok(U32_SIZE),
            Err(e) => Err(io::Error::from(e))
        }
    }

    pub fn write_bool(&mut self, b: bool) -> io::Result<usize> {
        self.write(&[b as u8])
    }

    // Reading convenience

    pub fn read_uint(&mut self) -> io::Result<u32> {
        self.read_u32::<LittleEndian>().map_err(io::Error::from)
    }

    pub fn read_str(&mut self) -> io::Result<String> {
        let len = try!(self.read_uint()) as usize;
        let mut buffer = vec![0; len];
        try!(self.read(&mut buffer));
        let result = String::from_utf8(buffer);
        match result {
            Ok(string) => Ok(string),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
        }
    }

    pub fn read_bool(&mut self) -> io::Result<bool> {
        let mut buffer = vec![0; 1];
        try!(self.read(&mut buffer));
        match buffer[0] {
            0 => Ok(false),
            1 => Ok(true),
            n => Err(io::Error::new(io::ErrorKind::InvalidInput,
                                    format!("{} is not a boolean", n)))

        }
    }

    pub fn bytes_remaining(&self) -> usize {
        self.bytes.len() - self.cursor
    }


    pub fn finalize(mut self) -> Vec<u8> {
        let bytes_len = (self.bytes.len() - U32_SIZE) as u32;
        {
            let mut first_word = &mut self.bytes[..U32_SIZE];
            first_word.write_u32::<LittleEndian>(bytes_len).unwrap();
        }
        self.bytes
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

#[derive(Debug, Clone, Copy)]
enum State {
    ReadingLength,
    ReadingPacket,
}

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

    pub fn try_write(&mut self, packet: Packet) -> io::Result<Option<()>> {
        match try!(self.stream.try_write(&packet.finalize())) {
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
