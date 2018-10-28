use std::io;
use std::marker;

use tokio_codec;
use bytes::BytesMut;

use super::base_codec::{Decode, ProtoEncode, ProtoEncoder, U32_BYTE_LEN};
use super::server::{ServerRequest,ServerResponse};
use super::peer::Message;

/*===================================*
 * TOKIO CODEC TRAIT IMPLEMENTATIONS *
 *===================================*/

// Encodes types that implement ProtoEncode with a length prefix.
struct Encoder<T> {
  phantom: marker::PhantomData<T>
}

impl<T> Encoder<T> {
    fn new() -> Self { Self{phantom: marker::PhantomData} }
}

impl<T: ProtoEncode> tokio_codec::Encoder for Encoder<T> {
    type Item = T;
    type Error = io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Split buffer into two parts: the length prefix and the message.
        dst.reserve(U32_BYTE_LEN);
        let mut msg_dst = dst.split_off(U32_BYTE_LEN);

        // Encode the message.
        item.encode(&mut ProtoEncoder::new(&mut msg_dst))?;

        // Encode the message length.
        ProtoEncoder::new(dst).encode_u32(msg_dst.len() as u32)?;

        // Reassemble both parts into one contiguous buffer.
        dst.unsplit(msg_dst);
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

mod tests {
    use bytes::BytesMut;
    use tokio_codec::Encoder;

    use proto::ProtoEncode;

    // Avoid name conflict with tokio_codec::Encoder.
    use super::Encoder as MyEncoder;

    #[test]
    fn encode_u32() {
        let val: u32 = 13 + 37*256;

        let mut bytes = BytesMut::new();
        MyEncoder::new().encode(val, &mut bytes).unwrap();

        assert_eq!(bytes, vec![
            4,   0, 0, 0,
            13, 37, 0, 0,
        ]);
    }

    #[test]
    fn encode_vec() {
        let v: Vec<u32> = vec![1, 3, 3, 7];

        let mut bytes = BytesMut::new();
        MyEncoder::new().encode(v, &mut bytes).unwrap();

        assert_eq!(bytes, vec![
            20, 0, 0, 0,  // 5 32-bit integers = 20 bytes.
            4,  0, 0, 0,
            1,  0, 0, 0,
            3,  0, 0, 0,
            3,  0, 0, 0,
            7,  0, 0, 0
        ]);
    }
}
