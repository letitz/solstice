use std::io;
use std::io::Write;

use byteorder::{LittleEndian, WriteBytesExt};

const MAX_MESSAGE_SIZE : usize = 2048;

pub struct Message {
    bytes: Vec<u8>,
}

impl Message {
    pub fn new(msg_code: u32) -> Message {
        let mut bytes = Vec::with_capacity(MAX_MESSAGE_SIZE);
        bytes.write_u32::<LittleEndian>(msg_code);
        Message{ bytes: bytes }
    }

    pub fn write_str(&mut self, string: &str) -> io::Result<usize> {
        try!(self.write_u32::<LittleEndian>(string.len() as u32));
        let n = try!(self.write(string.as_bytes()));
        Ok(n + 4)
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

