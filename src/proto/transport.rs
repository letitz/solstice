use std::io;

use bytes::BytesMut;
use futures::{Async, AsyncSink, Poll, Sink, StartSend, Stream};
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::{Decoder, Encoder, length_delimited};

use super::codec::DecodeError;
use super::{ServerResponse, ServerRequest};

pub struct ServerTransport<T> {
    framed: length_delimited::Framed<T, BytesMut>,
}

impl<T: AsyncRead + AsyncWrite> ServerTransport<T> {
    fn new(io: T) -> ServerTransport<T> {
        ServerTransport {
            framed: length_delimited::Builder::new()
                .length_field_length(4)
                .little_endian()
                .new_framed(io),
        }
    }
}

fn decode(bytes: &mut BytesMut) -> ServerResponse {
    unimplemented!();
}

fn encode(request: &ServerRequest) -> BytesMut {
    unimplemented!();
}

impl<T: AsyncRead> Stream for ServerTransport<T> {
    type Item = ServerResponse;
    type Error = DecodeError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.framed.poll() {
            Ok(Async::Ready(Some(mut bytes))) => Ok(Async::Ready(Some(decode(&mut bytes)))),
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
        match self.framed.start_send(encode(&item)) {
            Ok(AsyncSink::Ready) => Ok(AsyncSink::Ready),
            Ok(AsyncSink::NotReady(_)) => Ok(AsyncSink::NotReady(item)),
            Err(err) => Err(err),
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.framed.poll_complete()
    }
}
