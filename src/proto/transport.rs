use std::io;

use bytes::BytesMut;
use futures::{Async, AsyncSink, Poll, Sink, StartSend, Stream};
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::{Decoder, Encoder, length_delimited};

use super::peer;
use super::codec::DecodeError;
use super::{ServerResponse, ServerRequest};

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

fn encode_server_request(request: &ServerRequest) -> BytesMut {
    unimplemented!();
}

fn decode_peer_message(bytes: &mut BytesMut) -> Result<peer::Message, DecodeError> {
    unimplemented!();
}

fn encode_peer_message(message: &peer::Message) -> BytesMut {
    unimplemented!();
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
        match self.framed.start_send(encode_server_request(&item)) {
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
            Ok(Async::Ready(Some(mut bytes))) => {
                let message = decode_peer_message(&mut bytes)?;
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
        match self.framed.start_send(encode_peer_message(&item)) {
            Ok(AsyncSink::Ready) => Ok(AsyncSink::Ready),
            Ok(AsyncSink::NotReady(_)) => Ok(AsyncSink::NotReady(item)),
            Err(err) => Err(err),
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.framed.poll_complete()
    }
}
