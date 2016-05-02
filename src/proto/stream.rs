use std::io;
use std::iter;
use std::mem;

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};
use mio;
use mio::TryRead;

use super::constants::*;
use super::packet::Packet;

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
pub struct PacketStream<T: io::Read + io::Write + mio::Evented> {
    stream: T,
    state: State,
    num_bytes_left: usize,
    buffer: Vec<u8>,
}

impl<T: io::Read + io::Write + mio::Evented> PacketStream<T> {

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
                Ok(Some(Packet::from_bytes(old_buffer)))
            }
        }
    }

    /// Register the packet stream with the given mio event loop.
    pub fn register<U: mio::Handler>(
        &self, event_loop: &mut mio::EventLoop<U>, token: mio::Token,
        event_set: mio::EventSet, poll_opt: mio::PollOpt)
        -> io::Result<()>
    {
        event_loop.register(&self.stream, token, event_set, poll_opt)
    }

    /// Re-register the packet stream with the given mio event loop.
    pub fn reregister<U: mio::Handler>(
        &self, event_loop: &mut mio::EventLoop<U>, token: mio::Token,
        event_set: mio::EventSet, poll_opt: mio::PollOpt)
        -> io::Result<()>
    {
        event_loop.reregister(&self.stream, token, event_set, poll_opt)
    }
}

impl<T: io::Read + io::Write + mio::Evented> io::Write for PacketStream<T> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.stream.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

