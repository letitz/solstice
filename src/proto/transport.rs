use std::fmt;
use std::io;

use bytes::{Buf, BytesMut};
use tokio_io::codec::length_delimited;
use tokio_io::{AsyncRead, AsyncWrite};

use proto::peer;
use proto::{Decode, ProtoEncode, ProtoEncoder, ServerRequest, ServerResponse};

fn decode_frame<'a, T>(frame_type: &str, bytes: &'a mut BytesMut) -> io::Result<T>
where
    T: fmt::Debug,
    io::Cursor<&'a mut BytesMut>: Decode<T>,
{
    let mut cursor = io::Cursor::new(bytes);
    let frame = cursor.decode()?;
    if cursor.has_remaining() {
        warn!(
            "Received {} with trailing bytes. Frame:\n{:?}Bytes:{:?}",
            frame_type,
            frame,
            cursor.bytes()
        );
    }
    Ok(frame)
}

fn encode_frame<T: ProtoEncode>(frame: &T) -> io::Result<BytesMut> {
    let mut bytes = BytesMut::new();
    frame.encode(&mut ProtoEncoder::new(&mut bytes))?;
    Ok(bytes)
}

/// Wraps a raw byte async I/O object, providing it with the ability to read and
/// write entire frames at once.
/// The returned stream and sink of frames is intended to be combined (using
/// `Stream::and_then` and `Sink::with`) with the following decoding and
/// encoding functions to create a stream/sink of decoded messages.
pub fn new_framed<T: AsyncRead + AsyncWrite>(io: T) -> length_delimited::Framed<T, BytesMut> {
    length_delimited::Builder::new()
        .length_field_length(4)
        .little_endian()
        .new_framed(io)
}

/// Decodes a server response from the given byte buffer, which should contain
/// exactly the bytes encoding the returned response.
/// Intended to be used on the result of `new_framed` using `Stream::and_then`.
pub fn decode_server_response(bytes: &mut BytesMut) -> io::Result<ServerResponse> {
    decode_frame("server response", bytes)
}

/// Encodes the given server response into a byte buffer, then returns it.
/// Intended to be used on a sink of BytesMut objects, using `Sink::with`.
pub fn encode_server_response(response: &ServerResponse) -> io::Result<BytesMut> {
    encode_frame(response)
}

/// Decodes a server request from the given byte buffer, which should contain
/// exactly the bytes encoding the returned response.
/// Intended to be used on the result of `new_framed` using `Stream::and_then`.
pub fn decode_server_request(bytes: &mut BytesMut) -> io::Result<ServerRequest> {
    decode_frame("server request", bytes)
}

/// Encodes the given server request into a byte buffer, then returns it.
/// Intended to be used on a sink of BytesMut objects, using `Sink::with`.
pub fn encode_server_request(request: &ServerRequest) -> io::Result<BytesMut> {
    encode_frame(request)
}

/// Decodes a peer message from the given byte buffer, which should contain
/// exactly the bytes encoding the returned response.
/// Intended to be used on the result of `new_framed` using `Stream::and_then`.
pub fn decode_peer_message(bytes: &mut BytesMut) -> io::Result<peer::Message> {
    decode_frame("peer message", bytes)
}

/// Encodes the given peer message into a byte buffer, then returns it.
/// Intended to be used on a sink of BytesMut objects, using `Sink::with`.
pub fn encode_peer_message(message: &peer::Message) -> io::Result<BytesMut> {
    encode_frame(message)
}
