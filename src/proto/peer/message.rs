use std::io;

use proto::peer::constants::*;
use proto::{
    MutPacket, Packet, PacketReadError, ProtoDecode, ProtoDecoder, ProtoEncode, ProtoEncoder, ReadFromPacket,
    WriteToPacket,
};

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
                code, bytes_remaining
            )
        }

        Ok(message)
    }
}

impl ProtoDecode for Message {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let code: u32 = decoder.decode()?;
        let message = match code {
            CODE_PIERCE_FIREWALL => {
                let val = decoder.decode()?;
                Message::PierceFirewall(val)
            }
            CODE_PEER_INIT => {
                let peer_init = decoder.decode()?;
                Message::PeerInit(peer_init)
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unknown peer message code: {}", code),
                ))
            }
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
            }
            Message::PeerInit(ref request) => {
                encoder.encode_u32(CODE_PEER_INIT)?;
                request.encode(encoder)?;
            }
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
            user_name,
            connection_type,
            token,
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
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let user_name = decoder.decode()?;
        let connection_type = decoder.decode()?;
        let token = decoder.decode()?;
        Ok(PeerInit {
            user_name,
            connection_type,
            token,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use bytes::BytesMut;

    use proto::base_codec::tests::{expect_io_error, roundtrip};
    use proto::ProtoDecoder;

    use super::*;

    #[test]
    fn invalid_code() {
        let bytes = BytesMut::from(vec![57, 5, 0, 0]);

        let result = ProtoDecoder::new(&bytes).decode::<Message>();

        expect_io_error(
            result,
            io::ErrorKind::InvalidData,
            "unknown peer message code: 1337",
        );
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
