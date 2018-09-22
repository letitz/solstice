use std::io;
use std::marker;

use tokio_codec;
use bytes::BytesMut;

use super::base_codec::{Decode, ProtoEncode, ProtoEncoder};
use super::server::{ServerRequest,ServerResponse};
use super::peer::Message;

/*===================================*
 * TOKIO CODEC TRAIT IMPLEMENTATIONS *
 *===================================*/

struct Encoder<T> {
  data: marker::PhantomData<T>
}

impl<T: ProtoEncode> tokio_codec::Encoder for Encoder<T> {
    type Item = T;
    type Error = io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut encoder = ProtoEncoder::new(dst);
        item.encode(&mut encoder)?;
        Ok(())
    }
}

pub type ServerRequestEncoder = Encoder<ServerRequest>;
pub type ServerResponseEncoder = Encoder<ServerResponse>;
pub type PeerMessageEncoder = Encoder<Message>;

struct Decoder<T> {
    data: marker::PhantomData<T>
}

impl<T> tokio_codec::Decoder for Decoder<T>
where BytesMut: Decode<T> {
    type Item = T;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(Some(src.decode()?))
    }
}

pub type ServerRequestDecoder = Decoder<ServerRequest>;
pub type ServerResponseDecoder = Decoder<ServerResponse>;
pub type PeerMessageDecoder = Decoder<Message>;
