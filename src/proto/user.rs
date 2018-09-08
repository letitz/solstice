use std::io;

use bytes;

use proto::{
    Decode, MutPacket, Packet, PacketReadError, ProtoEncode, ProtoEncoder, ReadFromPacket,
    WriteToPacket,
};

const STATUS_OFFLINE: u32 = 1;
const STATUS_AWAY: u32 = 2;
const STATUS_ONLINE: u32 = 3;

/// This enumeration is the list of possible user statuses.
#[derive(Clone, Copy, Debug, Eq, PartialEq, RustcDecodable, RustcEncodable)]
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
        let n: u32 = try!(packet.read_value());
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
        try!(packet.write_value(&n));
        Ok(())
    }
}

impl ProtoEncode for UserStatus {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        let value = match *self {
            UserStatus::Offline => STATUS_OFFLINE,
            UserStatus::Away => STATUS_AWAY,
            UserStatus::Online => STATUS_ONLINE,
        };
        encoder.encode_u32(value)
    }
}

impl<T: bytes::Buf> Decode<UserStatus> for T {
    fn decode(&mut self) -> io::Result<UserStatus> {
        let value: u32 = self.decode()?;
        match value {
            STATUS_OFFLINE => Ok(UserStatus::Offline),
            STATUS_AWAY => Ok(UserStatus::Away),
            STATUS_ONLINE => Ok(UserStatus::Online),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid user status: {}", value),
            )),
        }
    }
}

/// This structure contains the last known information about a fellow user.
#[derive(Clone, Debug, Eq, PartialEq, RustcDecodable, RustcEncodable)]
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
