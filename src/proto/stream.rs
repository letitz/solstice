use std::collections::VecDeque;
use std::error;
use std::io;
use std::iter;
use std::mem;

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};
use mio;
use mio::TryRead;

use super::constants::*;
use super::packet::{MutPacket, Packet, ReadFromPacket, WriteToPacket};

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
struct Parser {
    state:          State,
    num_bytes_left: usize,
    buffer:         Vec<u8>,
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            state:          State::ReadingLength,
            num_bytes_left: U32_SIZE,
            buffer:         vec![0; U32_SIZE],
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
        where U: io::Read
    {
        // Try to read as many bytes as we currently need from the underlying
        // byte stream.
        let offset = self.buffer.len() - self.num_bytes_left;
        match try!(stream.try_read(&mut self.buffer[offset..])) {
            None => (),

            Some(num_bytes_read) => {
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
                self.try_read(stream)
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
}

/*========*
 * OUTBUF *
 *========*/

/// A struct used for writing bytes to a TryWrite sink.
#[derive(Debug)]
struct OutBuf {
    cursor: usize,
    bytes: Vec<u8>
}

impl From<Vec<u8>> for OutBuf {
    fn from(bytes: Vec<u8>) -> Self {
        OutBuf {
            cursor: 0,
            bytes: bytes
        }
    }
}

impl OutBuf {
    #[inline]
    fn remaining(&self) -> usize {
        self.bytes.len() - self.cursor
    }

    #[inline]
    fn has_remaining(&self) -> bool {
        self.remaining() > 0
    }

    fn try_write_to<T>(&mut self, mut writer: T) -> io::Result<Option<usize>>
        where T: mio::TryWrite
    {
        let result = writer.try_write(&self.bytes[self.cursor..]);
        if let Ok(Some(bytes_written)) = result {
            self.cursor += bytes_written;
        }
        result
    }
}

/*========*
 * STREAM *
 *========*/

/// This trait is implemented by packet sinks to which a stream can forward
/// the packets it reads.
pub trait SendPacket {
    type Value: ReadFromPacket;
    type Error: error::Error;

    fn send_packet(&mut self, Self::Value) -> Result<(), Self::Error>;
}

/// This enum defines the possible actions the stream wants to take after
/// processing an event.
#[derive(Debug, Clone, Copy)]
pub enum Intent {
    /// The stream is done, the event loop handler can drop it.
    Done,
    /// The stream wants to wait for the next event matching the given
    /// `EventSet`.
    Continue(mio::EventSet),
}

/// This struct wraps around an mio byte stream and handles packet reads and
/// writes.
#[derive(Debug)]
pub struct Stream<T, U>
    where T: io::Read + io::Write + mio::Evented,
          U: SendPacket
{
    parser: Parser,
    queue:  VecDeque<OutBuf>,
    sender: U,
    stream: T,
}

impl<T, U> Stream<T, U>
    where T: io::Read + io::Write + mio::Evented,
          U: SendPacket
{
    /// Returns a new struct wrapping the provided byte stream, which will
    /// forward packets to the provided sink.
    pub fn new(stream: T, sender: U) -> Self {
        Stream {
            parser: Parser::new(),
            queue:  VecDeque::new(),
            sender: sender,
            stream: stream,
        }
    }

    /// Returns a reference to the underlying byte stream, to allow it to be
    /// registered with an event loop.
    pub fn evented(&self) -> &T {
        &self.stream
    }

    fn on_readable(&mut self) -> Result<(), String> {
        loop {
            let mut packet = match self.parser.try_read(&mut self.stream) {
                Ok(Some(packet)) => packet,
                Ok(None) => {
                    break
                },
                Err(e) => {
                    return Err(format!("Error reading stream: {}", e))
                }
            };
            let value = match packet.read_value() {
                Ok(value) => value,
                Err(e) => {
                    return Err(format!("Error parsing packet: {}", e))
                }
            };
            if let Err(e) = self.sender.send_packet(value) {
                return Err(format!("Error sending parsed packet: {}", e))
            }
        }
        Ok(())
    }

    fn on_writable(&mut self) -> io::Result<()> {
        loop {
            let mut outbuf = match self.queue.pop_front() {
                Some(outbuf) => outbuf,
                None => break
            };

            let option = try!(outbuf.try_write_to(&mut self.stream));
            match option {
                Some(_) => {
                    if outbuf.has_remaining() {
                        self.queue.push_front(outbuf)
                    }
                    // Continue looping
                },
                None => {
                    self.queue.push_front(outbuf);
                    break
                }
            }
        }
        Ok(())
    }

    /// The stream is ready to read, write, or both.
    pub fn on_ready(&mut self, event_set: mio::EventSet) -> Intent {
        if event_set.is_readable() {
            let result = self.on_readable();
            if let Err(e) = result {
                error!("Stream input error: {}", e);
                return Intent::Done
            }
        }
        if event_set.is_writable() {
            let result = self.on_writable();
            if let Err(e) = result {
                error!("Stream output error: {}", e);
                return Intent::Done
            }
        }

        // We're always interested in reading more.
        // If there is still stuff to write in the queue, we're interested in
        // the socket becoming writable too.
        let event_set = if self.queue.len() > 0 {
            mio::EventSet::readable() | mio::EventSet::writable()
        } else {
            mio::EventSet::readable()
        };
        Intent::Continue(event_set)
    }

    /// The stream has been notified.
    pub fn on_notify<V>(&mut self, payload: V) -> Intent
        where V: WriteToPacket
    {
        let mut packet = MutPacket::new();
        let result = packet.write_value(payload);
        if let Err(e) = result {
            error!("Error writing payload to packet: {}", e);
            return Intent::Done
        }
        self.queue.push_back(OutBuf::from(packet.into_bytes()));
        Intent::Continue(mio::EventSet::readable() | mio::EventSet::writable())
    }
}
