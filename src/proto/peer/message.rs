use std::io;

use bytes;

use proto::peer::constants::*;
use proto::{
    Decode, MutPacket, Packet, PacketReadError, ProtoEncode, ProtoEncoder, ReadFromPacket,
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

impl<T: bytes::Buf> Decode<Message> for T {
    fn decode(&mut self) -> io::Result<Message> {
        let code: u32 = self.decode()?;
        let message = match code {
            CODE_PIERCE_FIREWALL => {
                let val = self.decode()?;
                Message::PierceFirewall(val)
            }
            CODE_PEER_INIT => {
                let peer_init = self.decode()?;
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

impl<T: bytes::Buf> Decode<PeerInit> for T {
    fn decode(&mut self) -> io::Result<PeerInit> {
        let user_name = self.decode()?;
        let connection_type = self.decode()?;
        let token = self.decode()?;
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

    use proto::codec::tests::{expect_io_error, roundtrip};
    use proto::{Decode, ProtoEncoder};

    use super::*;

    #[test]
    fn invalid_code() {
        let mut bytes = BytesMut::new();
        ProtoEncoder::new(&mut bytes).encode_u32(1337).unwrap();

        let result: io::Result<Message> = io::Cursor::new(bytes).decode();

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
