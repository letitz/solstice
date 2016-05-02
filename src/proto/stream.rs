use std::error;
use std::io;
use std::iter;
use std::mem;

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};
use mio;
use mio::TryRead;

use super::constants::*;
use super::packet::{Packet, ReadFromPacket};

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

/// This trait is implemented by packet sinks to which a parser can forward
/// the packets it reads.
pub trait SendPacket {
    type Value: ReadFromPacket;
    type Error: error::Error;

    fn send_packet(&mut self, Self::Value) -> Result<(), Self::Error>;
}

#[derive(Debug)]
struct Parser<T: SendPacket> {
    state:          State,
    num_bytes_left: usize,
    buffer:         Vec<u8>,
    packet_tx:      T,
}

impl<T: SendPacket> Parser<T> {
    pub fn new(packet_tx: T) -> Self {
        Parser {
            state:          State::ReadingLength,
            num_bytes_left: U32_SIZE,
            buffer:         vec![0; U32_SIZE],
            packet_tx:      packet_tx,
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
    fn try_read<U>(&mut self, stream: &mut U) -> io::Result<Option<Packet>>
        where U: io::Read
    {
        // Try to read as many bytes as we currently need from the underlying
        // byte stream.
        let offset = self.buffer.len() - self.num_bytes_left;
        match try!(stream.try_read(&mut self.buffer[offset..])) {
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

    pub fn read_from<U: io::Read>(&mut self, stream: &mut U) ->
        Result<(), String>
    {
        loop {
            let mut packet = match self.try_read(stream) {
                Ok(Some(packet)) => packet,
                Ok(None) => {
                    return Ok(())
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
            if let Err(e) = self.packet_tx.send_packet(value) {
                return Err(format!("Error sending parsed packet: {}", e))
            }
        }
    }

}

/// This struct wraps around an mio byte stream and reads soulseek packets
/// from it, forwarding them once parsed.
#[derive(Debug)]
pub struct Stream<T, U>
    where T: io::Read + io::Write + mio::Evented,
          U: SendPacket
{
    stream: T,
    parser: Parser<U>
}

impl<T, U> Stream<T, U>
    where T: io::Read + io::Write + mio::Evented,
          U: SendPacket
{
    /// Returns a new struct wrapping the provided byte stream, which will
    /// forward packets to the provided sink.
    pub fn new(stream: T, packet_tx: U) -> Self {
        Stream {
            stream: stream,
            parser: Parser::new(packet_tx),
        }
    }

    /// Returns a reference to the underlying byte stream, to allow it to be
    /// registered with an event loop.
    pub fn evented(&self) -> &T {
        &self.stream
    }

    /// The stream is readable.
    pub fn on_readable(&mut self) -> Intent {
        match self.parser.read_from(&mut self.stream) {
            Ok(()) => Intent::Continue(mio::EventSet::readable()),
            Err(e) => {
                error!("Stream input error: {}", e);
                Intent::Done
            }
        }
    }
}

impl<T, U> io::Write for Stream<T, U>
    where T: io::Read + io::Write + mio::Evented,
          U: SendPacket
{
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.stream.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

