use std::io;

use tokio_codec;
use bytes::BytesMut;

use super::base_codec::{ProtoEncode, ProtoEncoder};
use super::server::ServerRequest;

/*===================================*
 * TOKIO CODEC TRAIT IMPLEMENTATIONS *
 *===================================*/

struct ServerRequestEncoder;

impl tokio_codec::Encoder for ServerRequestEncoder {
    type Item = ServerRequest;
    type Error = io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut encoder = ProtoEncoder::new(dst);
        item.encode(&mut encoder)?;
        Ok(())
    }
}
