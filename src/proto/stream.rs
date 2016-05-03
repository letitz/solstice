use std::collections::VecDeque;
use std::error;
use std::io;

use mio;

use super::packet::{MutPacket, Parser, ReadFromPacket, WriteToPacket};

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

    /// The stream is ready to be read from.
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

    /// The stream is ready to be written to.
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
    pub fn on_notify<V>(&mut self, payload: &V) -> Intent
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
