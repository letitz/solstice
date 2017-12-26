use std::io;

use bytes::{Buf, BytesMut};
use futures::{Async, AsyncSink, Poll, Sink, StartSend, Stream};
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::length_delimited;

use proto::peer;
use proto::{DecodeError, ProtoDecode, ProtoDecoder, ProtoEncode, ProtoEncoder, ServerResponse,
            ServerRequest};

/* ------- *
 * Helpers *
 * ------- */

fn new_framed<T: AsyncRead + AsyncWrite>(io: T) -> length_delimited::Framed<T, BytesMut> {
    length_delimited::Builder::new()
        .length_field_length(4)
        .little_endian()
        .new_framed(io)
}

fn decode_server_response(bytes: &mut BytesMut) -> Result<ServerResponse, DecodeError> {
    unimplemented!();
}

fn encode_server_request(request: &ServerRequest) -> Result<BytesMut, io::Error> {
    unimplemented!();
}

fn decode_peer_message(bytes: BytesMut) -> Result<peer::Message, DecodeError> {
    let mut cursor = io::Cursor::new(bytes);
    let message = peer::Message::decode(&mut ProtoDecoder::new(&mut cursor))?;
    if cursor.has_remaining() {
        warn!(
            "Received peer message with trailing bytes. Message:\n{:?}Bytes:{:?}",
            message,
            cursor.bytes()
        );
    }
    Ok(message)
}

fn encode_peer_message(message: &peer::Message) -> Result<BytesMut, io::Error> {
    let mut bytes = BytesMut::new();
    message.encode(&mut ProtoEncoder::new(&mut bytes))?;
    Ok(bytes)
}

/* --------------- *
 * ServerTransport *
 * --------------- */

pub struct ServerTransport<T> {
    framed: length_delimited::Framed<T, BytesMut>,
}

impl<T: AsyncRead + AsyncWrite> ServerTransport<T> {
    fn new(io: T) -> ServerTransport<T> {
        ServerTransport { framed: new_framed(io) }
    }
}

impl<T: AsyncRead> Stream for ServerTransport<T> {
    type Item = ServerResponse;
    type Error = DecodeError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.framed.poll() {
            Ok(Async::Ready(Some(mut bytes))) => {
                let response = decode_server_response(&mut bytes)?;
                Ok(Async::Ready(Some(response)))
            }
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(err) => Err(DecodeError::from(err)),
        }
    }
}

impl<T: AsyncWrite> Sink for ServerTransport<T> {
    type SinkItem = ServerRequest;
    type SinkError = io::Error;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        let bytes = encode_server_request(&item)?;
        match self.framed.start_send(bytes) {
            Ok(AsyncSink::Ready) => Ok(AsyncSink::Ready),
            Ok(AsyncSink::NotReady(_)) => Ok(AsyncSink::NotReady(item)),
            Err(err) => Err(err),
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.framed.poll_complete()
    }
}

/* ------------- *
 * PeerTransport *
 * ------------- */

pub struct PeerTransport<T> {
    framed: length_delimited::Framed<T, BytesMut>,
}

impl<T: AsyncRead + AsyncWrite> PeerTransport<T> {
    fn new(io: T) -> PeerTransport<T> {
        PeerTransport { framed: new_framed(io) }
    }
}

impl<T: AsyncRead> Stream for PeerTransport<T> {
    type Item = peer::Message;
    type Error = DecodeError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.framed.poll() {
            Ok(Async::Ready(Some(bytes))) => {
                let message = decode_peer_message(bytes)?;
                Ok(Async::Ready(Some(message)))
            }
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(err) => Err(DecodeError::from(err)),
        }
    }
}

impl<T: AsyncWrite> Sink for PeerTransport<T> {
    type SinkItem = peer::Message;
    type SinkError = io::Error;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        let bytes = encode_peer_message(&item)?;
        match self.framed.start_send(bytes) {
            Ok(AsyncSink::Ready) => Ok(AsyncSink::Ready),
            Ok(AsyncSink::NotReady(_)) => Ok(AsyncSink::NotReady(item)),
            Err(err) => Err(err),
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.framed.poll_complete()
    }
}
