use std::io;

use proto::{DecodeError, MutPacket, Packet, PacketReadError, ProtoDecode, ProtoDecoder,
            ProtoEncode, ProtoEncoder, ReadFromPacket, WriteToPacket};
use proto::peer::constants::*;

/*=========*
 * MESSAGE *
 *=========*/

/// This enum contains all the possible messages peers can exchange.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Message {
    PierceFirewall(u32),
    PeerInit(PeerInit),
    Unknown(u32),
}

impl ReadFromPacket for Message {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let code: u32 = try!(packet.read_value());
        let message = match code {
            CODE_PIERCE_FIREWALL => Message::PierceFirewall(try!(packet.read_value())),

            CODE_PEER_INIT => Message::PeerInit(try!(packet.read_value())),

            code => Message::Unknown(code),
        };

        let bytes_remaining = packet.bytes_remaining();
        if bytes_remaining > 0 {
            warn!(
                "Peer message with code {} contains {} extra bytes",
                code,
                bytes_remaining
            )
        }

        Ok(message)
    }
}

impl ProtoDecode for Message {
    fn decode(decoder: &mut ProtoDecoder) -> Result<Self, DecodeError> {
        let code = decoder.decode_u32()?;
        let message = match code {
            CODE_PIERCE_FIREWALL => {
                let val = decoder.decode_u32()?;
                Message::PierceFirewall(val)
            },
            CODE_PEER_INIT => {
                let peer_init = PeerInit::decode(decoder)?;
                Message::PeerInit(peer_init)
            },
            _ => {
                return Err(DecodeError::UnknownCodeError(code));
            },
        };
        Ok(message)
    }
}

impl ProtoEncode for Message {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        match *self {
            Message::PierceFirewall(token) => {
                encoder.encode_u32(CODE_PIERCE_FIREWALL)?;
                encoder.encode_u32(token)?;
            },
            Message::PeerInit(ref request) => {
                encoder.encode_u32(CODE_PEER_INIT)?;
                request.encode(encoder)?;
            },
            Message::Unknown(_) => unreachable!(),
        }
        Ok(())
    }
}

impl WriteToPacket for Message {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        match *self {
            Message::PierceFirewall(ref token) => {
                try!(packet.write_value(&CODE_PIERCE_FIREWALL));
                try!(packet.write_value(token));
            }

            Message::PeerInit(ref request) => {
                try!(packet.write_value(&CODE_PEER_INIT));
                try!(packet.write_value(request));
            }

            Message::Unknown(_) => unreachable!(),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerInit {
    pub user_name: String,
    pub connection_type: String,
    pub token: u32,
}

impl ReadFromPacket for PeerInit {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let user_name = try!(packet.read_value());
        let connection_type = try!(packet.read_value());
        let token = try!(packet.read_value());
        Ok(PeerInit {
            user_name: user_name,
            connection_type: connection_type,
            token: token,
        })
    }
}

impl WriteToPacket for PeerInit {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        try!(packet.write_value(&self.user_name));
        try!(packet.write_value(&self.connection_type));
        try!(packet.write_value(&self.token));
        Ok(())
    }
}

impl ProtoEncode for PeerInit {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_string(&self.user_name)?;
        encoder.encode_string(&self.connection_type)?;
        encoder.encode_u32(self.token)?;
        Ok(())
    }
}

impl ProtoDecode for PeerInit {
    fn decode(decoder: &mut ProtoDecoder) -> Result<Self, DecodeError> {
        let user_name = decoder.decode_string()?;
        let connection_type = decoder.decode_string()?;
        let token = decoder.decode_u32()?;
        Ok(PeerInit {
            user_name: user_name,
            connection_type: connection_type,
            token: token,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use std::io;

    use bytes::BytesMut;

    use proto::{DecodeError, ProtoDecode, ProtoDecoder, ProtoEncode, ProtoEncoder};

    use super::*;

    fn roundtrip(input: Message) {
        let mut bytes = BytesMut::new();
        input.encode(&mut ProtoEncoder::new(&mut bytes)).unwrap();

        let mut cursor = io::Cursor::new(bytes);
        let output = Message::decode(&mut ProtoDecoder::new(&mut cursor)).unwrap();

        assert_eq!(output, input);
    }

    #[test]
    fn invalid_code() {
        let mut bytes = BytesMut::new();
        ProtoEncoder::new(&mut bytes).encode_u32(1337).unwrap();

        let mut cursor = io::Cursor::new(bytes);
        match Message::decode(&mut ProtoDecoder::new(&mut cursor)) {
            Err(DecodeError::UnknownCodeError(1337)) => {},
            result => panic!(result),
        }
    }

    #[test]
    fn roundtrip_pierce_firewall() {
        roundtrip(Message::PierceFirewall(1337))
    }

    #[test]
    fn roundtrip_peer_init() {
        roundtrip(Message::PeerInit(PeerInit {
            user_name: "alice".to_string(),
            connection_type: "P".to_string(),
            token: 1337,
        }));
    }
}
