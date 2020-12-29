use std::io;

use crate::proto::peer::constants::*;
use crate::proto::{
    MutPacket, Packet, PacketReadError, ReadFromPacket, ValueDecode, ValueDecodeError,
    ValueDecoder, ValueEncode, ValueEncodeError, ValueEncoder, WriteToPacket,
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
        let code: u32 = packet.read_value()?;
        let message = match code {
            CODE_PIERCE_FIREWALL => Message::PierceFirewall(packet.read_value()?),

            CODE_PEER_INIT => Message::PeerInit(packet.read_value()?),

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

impl ValueDecode for Message {
    fn decode_from(decoder: &mut ValueDecoder) -> Result<Self, ValueDecodeError> {
        let position = decoder.position();
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
                return Err(ValueDecodeError::InvalidData {
                    value_name: "peer message code".to_string(),
                    cause: format!("unknown value {}", code),
                    position: position,
                })
            }
        };
        Ok(message)
    }
}

impl ValueEncode for Message {
    fn encode(&self, encoder: &mut ValueEncoder) -> Result<(), ValueEncodeError> {
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
                packet.write_value(&CODE_PIERCE_FIREWALL)?;
                packet.write_value(token)?;
            }

            Message::PeerInit(ref request) => {
                packet.write_value(&CODE_PEER_INIT)?;
                packet.write_value(request)?;
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
        let user_name = packet.read_value()?;
        let connection_type = packet.read_value()?;
        let token = packet.read_value()?;
        Ok(PeerInit {
            user_name,
            connection_type,
            token,
        })
    }
}

impl WriteToPacket for PeerInit {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        packet.write_value(&self.user_name)?;
        packet.write_value(&self.connection_type)?;
        packet.write_value(&self.token)?;
        Ok(())
    }
}

impl ValueEncode for PeerInit {
    fn encode(&self, encoder: &mut ValueEncoder) -> Result<(), ValueEncodeError> {
        encoder.encode_string(&self.user_name)?;
        encoder.encode_string(&self.connection_type)?;
        encoder.encode_u32(self.token)?;
        Ok(())
    }
}

impl ValueDecode for PeerInit {
    fn decode_from(decoder: &mut ValueDecoder) -> Result<Self, ValueDecodeError> {
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
    use bytes::BytesMut;

    use crate::proto::value_codec::tests::roundtrip;
    use crate::proto::{ValueDecodeError, ValueDecoder};

    use super::*;

    #[test]
    fn invalid_code() {
        let bytes = BytesMut::from(vec![57, 5, 0, 0]);

        let result = ValueDecoder::new(&bytes).decode::<Message>();

        assert_eq!(
            result,
            Err(ValueDecodeError::InvalidData {
                value_name: "peer message code".to_string(),
                cause: "unknown value 1337".to_string(),
                position: 0,
            })
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
