use std::net;

use super::constants::*;
use super::super::packet::{Packet, PacketReadError, ReadFromPacket};

use user;

/*=================*
 * SERVER RESPONSE *
 *=================*/

#[derive(Debug)]
pub enum ServerResponse {
    ConnectToPeerResponse(ConnectToPeerResponse),
    LoginResponse(LoginResponse),
    PeerAddressResponse(PeerAddressResponse),
    PrivilegedUsersResponse(PrivilegedUsersResponse),
    RoomJoinResponse(RoomJoinResponse),
    RoomLeaveResponse(RoomLeaveResponse),
    RoomListResponse(RoomListResponse),
    RoomMessageResponse(RoomMessageResponse),
    RoomTickersResponse(RoomTickersResponse),
    RoomUserJoinedResponse(RoomUserJoinedResponse),
    RoomUserLeftResponse(RoomUserLeftResponse),
    UserStatusResponse(UserStatusResponse),
    WishlistIntervalResponse(WishlistIntervalResponse),

    // Unknown purpose
    ParentMinSpeedResponse(ParentMinSpeedResponse),
    ParentSpeedRatioResponse(ParentSpeedRatioResponse),

    UnknownResponse(u32),
}

macro_rules! try_read_from_packet {
    ($struct_name:ident, $packet:ident) => {
        ServerResponse::$struct_name(
            try!($struct_name::read_from_packet($packet))
        )
    }
}

impl ReadFromPacket for ServerResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let code = try!(packet.read_value());
        let resp = match code {
            CODE_CONNECT_TO_PEER =>
                try_read_from_packet!(ConnectToPeerResponse, packet),

            CODE_LOGIN =>
                try_read_from_packet!(LoginResponse, packet),

            CODE_PEER_ADDRESS =>
                try_read_from_packet!(PeerAddressResponse, packet),

            CODE_PRIVILEGED_USERS =>
                try_read_from_packet!(PrivilegedUsersResponse, packet),

            CODE_ROOM_JOIN =>
                try_read_from_packet!(RoomJoinResponse, packet),

            CODE_ROOM_LEAVE =>
                try_read_from_packet!(RoomLeaveResponse, packet),

            CODE_ROOM_LIST =>
                try_read_from_packet!(RoomListResponse, packet),

            CODE_ROOM_MESSAGE =>
                try_read_from_packet!(RoomMessageResponse, packet),

            CODE_ROOM_TICKERS =>
                try_read_from_packet!(RoomTickersResponse, packet),

            CODE_ROOM_USER_JOINED =>
                try_read_from_packet!(RoomUserJoinedResponse, packet),

            CODE_ROOM_USER_LEFT =>
                try_read_from_packet!(RoomUserLeftResponse, packet),

            CODE_USER_STATUS =>
                try_read_from_packet!(UserStatusResponse, packet),

            CODE_WISHLIST_INTERVAL =>
                try_read_from_packet!(WishlistIntervalResponse, packet),

            CODE_PARENT_MIN_SPEED =>
                try_read_from_packet!(ParentMinSpeedResponse, packet),

            CODE_PARENT_SPEED_RATIO =>
                try_read_from_packet!(ParentSpeedRatioResponse, packet),

            code => ServerResponse::UnknownResponse(code),
        };
        let bytes_remaining = packet.bytes_remaining();
        if bytes_remaining > 0 {
            warn!("Packet with code {} contains {} extra bytes",
                   code, bytes_remaining)
        }
        Ok(resp)
    }
}

/*=================*
 * CONNECT TO PEER *
 *=================*/

#[derive(Debug)]
pub struct ConnectToPeerResponse {
    pub username:        String,
    pub connection_type: String,
    pub ip:              net::Ipv4Addr,
    pub port:            u16,
    pub token:           u32,
    pub is_privileged:   bool,
}

impl ReadFromPacket for ConnectToPeerResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let username        = try!(packet.read_value());
        let connection_type = try!(packet.read_value());
        let ip              = try!(packet.read_value());
        let port            = try!(packet.read_value());
        let token           = try!(packet.read_value());
        let is_privileged   = try!(packet.read_value());

        Ok(ConnectToPeerResponse {
            username:        username,
            connection_type: connection_type,
            ip:              ip,
            port:            port,
            token:           token,
            is_privileged:   is_privileged,
        })
    }
}

/*=======*
 * LOGIN *
 *=======*/

#[derive(Debug)]
pub enum LoginResponse {
    LoginOk {
        motd: String,
        ip: net::Ipv4Addr,
        password_md5_opt: Option<String>
    },
    LoginFail {
        reason: String
    },
}

impl ReadFromPacket for LoginResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let ok = try!(packet.read_value());
        if ok {
            let motd = try!(packet.read_value());
            let ip = try!(packet.read_value());

            match packet.read_value::<bool>() {
                Ok(value) => debug!("LoginResponse last field: {}", value),
                Err(e) => debug!("Error reading LoginResponse field: {:?}", e),
            }

            Ok(LoginResponse::LoginOk {
                motd: motd,
                ip: ip,
                password_md5_opt: None
            })
        } else {
            Ok(LoginResponse::LoginFail {
                reason: try!(packet.read_value())
            })
        }
    }
}

/*==================*
 * PARENT MIN SPEED *
 *==================*/

#[derive(Debug)]
pub struct ParentMinSpeedResponse {
    pub value: u32,
}

impl ReadFromPacket for ParentMinSpeedResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let value = try!(packet.read_value());
        Ok(ParentMinSpeedResponse {
            value: value,
        })
    }
}

/*====================*
 * PARENT SPEED RATIO *
 *====================*/

#[derive(Debug)]
pub struct ParentSpeedRatioResponse {
    pub value: u32,
}

impl ReadFromPacket for ParentSpeedRatioResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let value = try!(packet.read_value());
        Ok(ParentSpeedRatioResponse {
            value: value,
        })
    }
}

/*==============*
 * PEER ADDRESS *
 *==============*/

#[derive(Debug)]
pub struct PeerAddressResponse {
    username: String,
    ip: net::Ipv4Addr,
    port: u16,
}

impl ReadFromPacket for PeerAddressResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let username = try!(packet.read_value());
        let ip = try!(packet.read_value());
        let port = try!(packet.read_value());

        Ok(PeerAddressResponse {
            username: username,
            ip: ip,
            port: port,
        })
    }
}

/*==================*
 * PRIVILEGED USERS *
 *==================*/

#[derive(Debug)]
pub struct PrivilegedUsersResponse {
    pub users: Vec<String>,
}

impl ReadFromPacket for PrivilegedUsersResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let users = try!(packet.read_value());
        Ok(PrivilegedUsersResponse {
            users: users
        })
    }
}

/*===========*
 * ROOM JOIN *
 *===========*/

#[derive(Debug)]
pub struct RoomJoinResponse {
    pub room_name: String,
    pub users: Vec<(String, user::User)>,
    pub owner: Option<String>,
    pub operators: Vec<String>,
}

impl ReadFromPacket for RoomJoinResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let mut response = RoomJoinResponse {
            room_name: try!(packet.read_value()),
            users: Vec::new(),
            owner: None,
            operators: Vec::new(),
        };

        let num_users: usize = try!(packet.read_value());
        for _ in 0..num_users {
            let name = try!(packet.read_value());
            let user = user::User {
                status:         user::Status::Offline,
                average_speed:  0,
                num_downloads:  0,
                unknown:        0,
                num_files:      0,
                num_folders:    0,
                num_free_slots: 0,
                country:        String::new(),
            };
            response.users.push((name, user));
        }

        try!(response.read_user_infos(packet));

        if packet.bytes_remaining() > 0 {
            response.owner = Some(try!(packet.read_value()));

            let num_operators: usize = try!(packet.read_value());
            for _ in 0..num_operators {
                response.operators.push(try!(packet.read_value()));
            }
        }

        Ok(response)
    }
}

impl RoomJoinResponse {
    fn read_user_infos(&mut self, packet: &mut Packet)
        -> Result<(), PacketReadError>
    {
        let num_statuses: usize = try!(packet.read_value());
        for i in 0..num_statuses {
            if let Some(&mut (_, ref mut user)) = self.users.get_mut(i) {
                user.status = try!(packet.read_value());
            }
        }

        let num_infos: usize = try!(packet.read_value());
        for i in 0..num_infos {
            if let Some(&mut (_, ref mut user)) = self.users.get_mut(i) {
                user.average_speed = try!(packet.read_value());
                user.num_downloads = try!(packet.read_value());
                user.unknown       = try!(packet.read_value());
                user.num_files     = try!(packet.read_value());
                user.num_folders   = try!(packet.read_value());
            }
        }

        let num_free_slots: usize = try!(packet.read_value());
        for i in 0..num_free_slots {
            if let Some(&mut (_, ref mut user)) = self.users.get_mut(i) {
                user.num_free_slots = try!(packet.read_value());
            }
        }

        let num_countries: usize = try!(packet.read_value());
        for i in 0..num_countries {
            if let Some(&mut (_, ref mut user)) = self.users.get_mut(i) {
                user.country = try!(packet.read_value());
            }
        }

        let num_users = self.users.len();
        if num_users != num_statuses ||
            num_users != num_infos ||
            num_users != num_free_slots ||
            num_users != num_countries
        {
            warn!(
                "RoomJoinResponse: mismatched vector sizes {}, {}, {}, {}, {}",
                num_users, num_statuses, num_infos, num_free_slots,
                num_countries
            );
        }

        Ok(())
    }
}

/*============*
 * ROOM LEAVE *
 *============*/

#[derive(Debug)]
pub struct RoomLeaveResponse {
    pub room_name: String,
}

impl ReadFromPacket for RoomLeaveResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        Ok(RoomLeaveResponse {
            room_name: try!(packet.read_value()),
        })
    }
}

/*===========*
 * ROOM LIST *
 *===========*/

#[derive(Debug)]
pub struct RoomListResponse {
    pub rooms: Vec<(String, u32)>,
    pub owned_private_rooms: Vec<(String, u32)>,
    pub other_private_rooms: Vec<(String, u32)>,
    pub operated_private_room_names: Vec<String>,
}

impl ReadFromPacket for RoomListResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let rooms = try!(Self::read_rooms(packet));
        let owned_private_rooms = try!(Self::read_rooms(packet));
        let other_private_rooms = try!(Self::read_rooms(packet));
        let operated_private_room_names = try!(packet.read_value());
        Ok(RoomListResponse {
            rooms: rooms,
            owned_private_rooms: owned_private_rooms,
            other_private_rooms: other_private_rooms,
            operated_private_room_names: operated_private_room_names,
        })
    }
}

impl RoomListResponse {
    fn read_rooms(packet: &mut Packet)
        -> Result<Vec<(String, u32)>, PacketReadError>
    {
        let num_rooms: usize = try!(packet.read_value());
        let mut rooms = Vec::new();
        for _ in 0..num_rooms {
            let room_name = try!(packet.read_value());
            rooms.push((room_name, 0));
        }

        let num_user_counts: usize = try!(packet.read_value());
        for i in 0..num_user_counts {
            if let Some(&mut (_, ref mut count)) = rooms.get_mut(i) {
                *count = try!(packet.read_value());
            }
        }

        if num_rooms != num_user_counts {
            warn!(
                "Numbers of rooms and user counts do not match: {} != {}",
                num_rooms, num_user_counts
            );
        }

        Ok(rooms)
    }
}

/*==============*
 * ROOM MESSAGE *
 *==============*/

#[derive(Debug)]
pub struct RoomMessageResponse {
    pub room_name: String,
    pub user_name: String,
    pub message:   String,
}

impl ReadFromPacket for RoomMessageResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let room_name = try!(packet.read_value());
        let user_name = try!(packet.read_value());
        let message   = try!(packet.read_value());
        Ok(RoomMessageResponse {
            room_name: room_name,
            user_name: user_name,
            message:   message,
        })
    }
}

/*==============*
 * ROOM MESSAGE *
 *==============*/

#[derive(Debug)]
pub struct RoomTickersResponse {
    pub room_name: String,
    pub tickers:   Vec<(String, String)>
}

impl ReadFromPacket for RoomTickersResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let room_name = try!(packet.read_value());

        let num_tickers: usize = try!(packet.read_value());
        let mut tickers = Vec::new();
        for _ in 0..num_tickers {
            let user_name = try!(packet.read_value());
            let message   = try!(packet.read_value());
            tickers.push((user_name, message))
        }

        Ok(RoomTickersResponse {
            room_name: room_name,
            tickers:   tickers,
        })
    }
}

/*==================*
 * ROOM USER JOINED *
 *==================*/

#[derive(Debug)]
pub struct RoomUserJoinedResponse {
    pub room_name: String,
    pub user_name: String,
    pub user:      user::User,
}

impl ReadFromPacket for RoomUserJoinedResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let room_name = try!(packet.read_value());
        let user_name = try!(packet.read_value());

        let status = try!(packet.read_value());

        let average_speed  = try!(packet.read_value());
        let num_downloads  = try!(packet.read_value());
        let unknown        = try!(packet.read_value());
        let num_files      = try!(packet.read_value());
        let num_folders    = try!(packet.read_value());
        let num_free_slots = try!(packet.read_value());

        let country = try!(packet.read_value());

        Ok(RoomUserJoinedResponse {
            room_name: room_name,
            user_name: user_name,
            user: user::User {
                status:         status,
                average_speed:  average_speed,
                num_downloads:  num_downloads,
                unknown:        unknown,
                num_files:      num_files,
                num_folders:    num_folders,
                num_free_slots: num_free_slots,
                country:        country,
            }
        })
    }
}

/*================*
 * ROOM USER LEFT *
 *================*/

#[derive(Debug)]
pub struct RoomUserLeftResponse {
    pub room_name: String,
    pub user_name: String,
}

impl ReadFromPacket for RoomUserLeftResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let room_name = try!(packet.read_value());
        let user_name = try!(packet.read_value());
        Ok(RoomUserLeftResponse {
            room_name: room_name,
            user_name: user_name,
        })
    }
}

/*=============*
 * USER STATUS *
 *=============*/

#[derive(Debug)]
pub struct UserStatusResponse {
    pub user_name: String,
    pub status: user::Status,
    pub is_privileged: bool,
}

impl ReadFromPacket for UserStatusResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let user_name     = try!(packet.read_value());
        let status        = try!(packet.read_value());
        let is_privileged = try!(packet.read_value());
        Ok(UserStatusResponse {
            user_name:     user_name,
            status:        status,
            is_privileged: is_privileged,
        })
    }
}

/*===================*
 * WISHLIST INTERVAL *
 *===================*/

#[derive(Debug)]
pub struct WishlistIntervalResponse {
    pub seconds: u32,
}

impl ReadFromPacket for WishlistIntervalResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let seconds = try!(packet.read_value());
        Ok(WishlistIntervalResponse {
            seconds: seconds,
        })
    }
}
