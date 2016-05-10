use std::io;

use super::super::{
    MutPacket, Packet, PacketReadError, ReadFromPacket, WriteToPacket
};
use super::constants::*;

/*=========*
 * MESSAGE *
 *=========*/

/// This enum contains all the possible messages peers can exchange.
#[derive(Clone, Debug)]
pub enum Message {
    PierceFirewall(u32),
    PeerInit(PeerInit),
    Unknown(u32),
}

impl ReadFromPacket for Message {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let code: u32 = try!(packet.read_value());
        let message = match code {
            CODE_PIERCE_FIREWALL =>
                Message::PierceFirewall(
                    try!(packet.read_value())
                ),

            CODE_PEER_INIT =>
                Message::PeerInit(
                    try!(packet.read_value())
                ),

            code => Message::Unknown(code)
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

impl WriteToPacket for Message {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        match *self {
            Message::PierceFirewall(ref token) => {
                try!(packet.write_value(&CODE_PIERCE_FIREWALL));
                try!(packet.write_value(token));
            },

            Message::PeerInit(ref request) => {
                try!(packet.write_value(&CODE_PEER_INIT));
                try!(packet.write_value(request));
            },

            Message::Unknown(_) => unreachable!(),
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct PeerInit {
    pub user_name:       String,
    pub connection_type: String,
    pub token:           u32,
}

impl ReadFromPacket for PeerInit {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let user_name       = try!(packet.read_value());
        let connection_type = try!(packet.read_value());
        let token           = try!(packet.read_value());
        Ok(PeerInit {
            user_name:       user_name,
            connection_type: connection_type,
            token:           token,
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

