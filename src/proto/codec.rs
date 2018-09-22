use std::io;
use std::marker;

use tokio_codec;
use bytes::BytesMut;

use super::base_codec::{ProtoEncode, ProtoEncoder};
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
