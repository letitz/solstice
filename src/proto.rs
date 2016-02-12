use std::iter::repeat;
use std::io;
use std::io::{Cursor, Read, Write};
use std::mem;

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};

use mio::tcp::TcpStream;

const MAX_PACKET_SIZE: usize = 1 << 20; // 1 MiB
const U32_SIZE: usize = 4;
const MAX_MESSAGE_SIZE: usize = MAX_PACKET_SIZE - U32_SIZE;

const CODE_LOGIN: u32 = 1;

/*=========*
 * MESSAGE *
 *=========*/

#[derive(Debug, Clone, Copy)]
pub enum MessageCode {
    Login,
    Unknown(u32),
}

impl MessageCode {
    fn to_u32(&self) -> u32 {
        match *self {
            MessageCode::Login => CODE_LOGIN,
            MessageCode::Unknown(code) => code,
        }
    }

    fn from_u32(code: u32) -> MessageCode {
        match code {
            CODE_LOGIN => MessageCode::Login,
            _ => MessageCode::Unknown(code),
        }
    }
}

#[derive(Debug)]
pub struct Message {
    code: MessageCode,
    bytes: Vec<u8>,
}

impl Message {
    pub fn new(code: MessageCode) -> Message {
        let mut bytes = Vec::new();
        bytes.write_u32::<LittleEndian>(0).unwrap();
        bytes.write_u32::<LittleEndian>(code.to_u32()).unwrap();
        Message {
            code: code,
            bytes: bytes,
        }
    }

    fn from_raw_parts(bytes: Vec<u8>) -> Message {
        let code_u32 = LittleEndian::read_u32(&bytes[U32_SIZE..2*U32_SIZE]);
        Message {
            code: MessageCode::from_u32(code_u32),
            bytes: bytes,
        }
    }

    pub fn code(&self) -> MessageCode {
        self.code
    }

    pub fn write_str(&mut self, string: &str) -> io::Result<usize> {
        try!(self.write_u32(string.len() as u32));
        let n = try!(self.bytes.write(string.as_bytes()));
        Ok(n + U32_SIZE)
    }

    pub fn write_u32(&mut self, n: u32) -> io::Result<usize> {
        match self.bytes.write_u32::<LittleEndian>(n) {
            Ok(()) => Ok(U32_SIZE),
            Err(e) => Err(io::Error::from(e))
        }
    }

    pub fn write_bool(&mut self, b: bool) -> io::Result<usize> {
        self.bytes.write(&[b as u8])
    }

    pub fn finalize(mut self) -> Vec<u8> {
        let bytes_len = (self.bytes.len() - U32_SIZE) as u32;
        {
            let mut first_word = &mut self.bytes[..4];
            first_word.write_u32::<LittleEndian>(bytes_len).unwrap();
        }
        self.bytes
    }
}

impl io::Write for Message {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.bytes.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.bytes.flush()
    }
}

/*======*
 * PEER *
 *======*/

pub trait Peer {
    fn read_message(&mut self) -> Option<Message>;
    fn write_message(&mut self, message: Message);
}

#[derive(Debug, Clone, Copy)]
enum State {
    ReadingLength,
    ReadingMessage,
}

#[derive(Debug)]
pub struct Connection<T: Peer> {
    state: State,
    num_bytes_left: usize,
    buffer: Vec<u8>,
    peer: T,
}

impl<T: Peer> Connection<T> {

    pub fn new(peer: T) -> Self {
        Connection {
            state: State::ReadingLength,
            num_bytes_left: U32_SIZE,
            buffer: vec![0; U32_SIZE],
            peer: peer,
        }
    }

    pub fn ready_to_read(&mut self, stream: &mut TcpStream) {
        let offset = self.buffer.len() - self.num_bytes_left;
        match stream.read(&mut self.buffer[offset..]) {
            Ok(num_bytes_read) => {
                assert!(num_bytes_read <= self.num_bytes_left);
                self.num_bytes_left -= num_bytes_read;
            },

            Err(e) => error!("Could not read stream: {:?}", e),
        }

        if self.num_bytes_left > 0 {
            return;
        }

        match self.state {
            State::ReadingLength => {
                let message_len =
                    LittleEndian::read_u32(&mut self.buffer) as usize;
                if message_len > MAX_MESSAGE_SIZE {
                    unimplemented!();
                };
                self.state = State::ReadingMessage;
                self.num_bytes_left = message_len;
                self.buffer.extend(repeat(0).take(message_len));
            },

            State::ReadingMessage => {
                self.state = State::ReadingLength;
                self.num_bytes_left = U32_SIZE;
                let new_buffer = vec![0;U32_SIZE];
                let old_buffer = mem::replace(&mut self.buffer, new_buffer);
                self.peer.write_message(Message::from_raw_parts(old_buffer));
            }
        }
    }

    pub fn ready_to_write(&mut self, stream: &mut TcpStream) {
        match self.peer.read_message() {
            Some(message) => {
                stream.write(&message.finalize()).unwrap();
                ()
            },
            None => (),
        }
    }
}
