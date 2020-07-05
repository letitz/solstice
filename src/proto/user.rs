use std::io;

use crate::proto::{
    MutPacket, Packet, PacketReadError, ReadFromPacket, ValueDecode, ValueDecodeError,
    ValueDecoder, ValueEncode, ValueEncodeError, ValueEncoder, WriteToPacket,
};

const STATUS_OFFLINE: u32 = 1;
const STATUS_AWAY: u32 = 2;
const STATUS_ONLINE: u32 = 3;

/// This enumeration is the list of possible user statuses.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, RustcDecodable, RustcEncodable)]
pub enum UserStatus {
    /// The user if offline.
    Offline,
    /// The user is connected, but AFK.
    Away,
    /// The user is present.
    Online,
}

impl ReadFromPacket for UserStatus {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let n: u32 = packet.read_value()?;
        match n {
            STATUS_OFFLINE => Ok(UserStatus::Offline),
            STATUS_AWAY => Ok(UserStatus::Away),
            STATUS_ONLINE => Ok(UserStatus::Online),
            _ => Err(PacketReadError::InvalidUserStatusError(n)),
        }
    }
}

impl WriteToPacket for UserStatus {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        let n = match *self {
            UserStatus::Offline => STATUS_OFFLINE,
            UserStatus::Away => STATUS_AWAY,
            UserStatus::Online => STATUS_ONLINE,
        };
        packet.write_value(&n)?;
        Ok(())
    }
}

impl ValueEncode for UserStatus {
    fn encode(&self, encoder: &mut ValueEncoder) -> Result<(), ValueEncodeError> {
        let value = match *self {
            UserStatus::Offline => STATUS_OFFLINE,
            UserStatus::Away => STATUS_AWAY,
            UserStatus::Online => STATUS_ONLINE,
        };
        encoder.encode_u32(value)
    }
}

impl ValueDecode for UserStatus {
    fn decode_from(decoder: &mut ValueDecoder) -> Result<Self, ValueDecodeError> {
        let position = decoder.position();
        let value: u32 = decoder.decode()?;
        match value {
            STATUS_OFFLINE => Ok(UserStatus::Offline),
            STATUS_AWAY => Ok(UserStatus::Away),
            STATUS_ONLINE => Ok(UserStatus::Online),
            _ => Err(ValueDecodeError::InvalidData {
                value_name: "user status".to_string(),
                cause: format!("unknown value {}", value),
                position: position,
            }),
        }
    }
}

/// This structure contains the last known information about a fellow user.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, RustcDecodable, RustcEncodable)]
pub struct User {
    /// The name of the user.
    pub name: String,
    /// The last known status of the user.
    pub status: UserStatus,
    /// The average upload speed of the user.
    pub average_speed: usize,
    /// ??? Nicotine calls it downloadnum.
    pub num_downloads: usize,
    /// ??? Unknown field.
    pub unknown: usize,
    /// The number of files this user shares.
    pub num_files: usize,
    /// The number of folders this user shares.
    pub num_folders: usize,
    /// The number of free download slots of this user.
    pub num_free_slots: usize,
    /// The user's country code.
    pub country: String,
}
