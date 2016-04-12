use std::net;

use super::constants::*;
use super::super::packet::Packet;

use result;
use user;

/*=============*
 * FROM PACKET *
 *=============*/

pub trait FromPacket: Sized {
    fn from_packet(&mut Packet) -> result::Result<Self>;
}

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
    UserJoinedRoomResponse(UserJoinedRoomResponse),
    WishlistIntervalResponse(WishlistIntervalResponse),

    // Unknown purpose
    ParentMinSpeedResponse(ParentMinSpeedResponse),
    ParentSpeedRatioResponse(ParentSpeedRatioResponse),

    UnknownResponse(u32),
}

impl FromPacket for ServerResponse {
    fn from_packet(packet: &mut Packet) -> result::Result<Self> {
        let code = try!(packet.read_uint());
        let resp = match code {
            CODE_CONNECT_TO_PEER =>
                ServerResponse::ConnectToPeerResponse(
                    try!(ConnectToPeerResponse::from_packet(packet))
                ),

            CODE_LOGIN =>
                ServerResponse::LoginResponse(
                    try!(LoginResponse::from_packet(packet))
                ),

            CODE_PEER_ADDRESS =>
                ServerResponse::PeerAddressResponse(
                    try!(PeerAddressResponse::from_packet(packet))
                ),

            CODE_PRIVILEGED_USERS =>
                ServerResponse::PrivilegedUsersResponse(
                    try!(PrivilegedUsersResponse::from_packet(packet))
                ),

            CODE_ROOM_JOIN =>
                ServerResponse::RoomJoinResponse(
                    try!(RoomJoinResponse::from_packet(packet))
                ),

            CODE_ROOM_LEAVE =>
                ServerResponse::RoomLeaveResponse(
                    try!(RoomLeaveResponse::from_packet(packet))
                ),

            CODE_ROOM_LIST =>
                ServerResponse::RoomListResponse(
                    try!(RoomListResponse::from_packet(packet))
                ),

            CODE_ROOM_MESSAGE =>
                ServerResponse::RoomMessageResponse(
                    try!(RoomMessageResponse::from_packet(packet))
                ),

            CODE_USER_JOINED_ROOM =>
                ServerResponse::UserJoinedRoomResponse(
                    try!(UserJoinedRoomResponse::from_packet(packet))
                ),

            CODE_WISHLIST_INTERVAL =>
                ServerResponse::WishlistIntervalResponse(
                    try!(WishlistIntervalResponse::from_packet(packet))
                    ),

            CODE_PARENT_MIN_SPEED =>
                ServerResponse::ParentMinSpeedResponse(
                    try!(ParentMinSpeedResponse::from_packet(packet))
                ),

            CODE_PARENT_SPEED_RATIO =>
                ServerResponse::ParentSpeedRatioResponse(
                    try!(ParentSpeedRatioResponse::from_packet(packet))
                ),

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
    pub username: String,
    pub connection_type: String,
    pub ip: net::Ipv4Addr,
    pub port: u16,
    pub token: u32,
    pub is_privileged: bool,
}

impl FromPacket for ConnectToPeerResponse {
    fn from_packet(packet: &mut Packet) -> result::Result<Self> {
        let username = try!(packet.read_str());
        let connection_type = try!(packet.read_str());

        let ip = try!(packet.read_ipv4_addr());
        let port = try!(packet.read_port());

        let token = try!(packet.read_uint());
        let is_privileged = try!(packet.read_bool());

        Ok(ConnectToPeerResponse {
            username: username,
            connection_type: connection_type,
            ip: ip,
            port: port,
            token: token,
            is_privileged: is_privileged,
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

impl FromPacket for LoginResponse {
    fn from_packet(packet: &mut Packet) -> result::Result<Self> {
        let ok = try!(packet.read_bool());
        if ok {
            let motd = try!(packet.read_str());
            let ip = try!(packet.read_ipv4_addr());

            match packet.read_bool() {
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
                reason: try!(packet.read_str())
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

impl FromPacket for ParentMinSpeedResponse {
    fn from_packet(packet: &mut Packet) -> result::Result<Self> {
        let value = try!(packet.read_uint());
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

impl FromPacket for ParentSpeedRatioResponse {
    fn from_packet(packet: &mut Packet) -> result::Result<Self> {
        let value = try!(packet.read_uint());
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

impl FromPacket for PeerAddressResponse {
    fn from_packet(packet: &mut Packet) -> result::Result<Self> {
        let username = try!(packet.read_str());
        let ip = try!(packet.read_ipv4_addr());
        let port = try!(packet.read_port());

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

impl FromPacket for PrivilegedUsersResponse {
    fn from_packet(packet: &mut Packet) -> result::Result<Self> {
        let mut response = PrivilegedUsersResponse {
            users: Vec::new(),
        };
        try!(packet.read_array(&mut response.users, Packet::read_str));
        Ok(response)
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

impl FromPacket for RoomJoinResponse {
    fn from_packet(packet: &mut Packet) -> result::Result<Self> {
        let mut response = RoomJoinResponse {
            room_name: try!(packet.read_str()),
            users: Vec::new(),
            owner: None,
            operators: Vec::new(),
        };

        let result: result::Result<usize> =
            packet.read_array(&mut response.users, |packet| {
                let name = try!(packet.read_str());
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
                Ok((name, user))
            });
        try!(result);

        try!(response.read_user_infos(packet));

        if packet.bytes_remaining() > 0 {
            response.owner = Some(try!(packet.read_str()));
            try!(packet.read_array(&mut response.operators, Packet::read_str));
        }

        Ok(response)
    }
}

impl RoomJoinResponse {
    fn read_user_infos(&mut self, packet: &mut Packet)
        -> result::Result<()>
    {
        let num_statuses_res: result::Result<usize> =
            packet.read_array_with(|packet, i| {
                if let Some(&mut (_, ref mut user)) = self.users.get_mut(i) {
                    let status_u32 = try!(packet.read_uint());
                    user.status = try!(user::Status::from_u32(status_u32));
                }
                Ok(())
            });
        let num_statuses = try!(num_statuses_res);

        let num_infos_res: result::Result<usize> =
            packet.read_array_with(|packet, i| {
                if let Some(&mut (_, ref mut user)) = self.users.get_mut(i) {
                    user.average_speed = try!(packet.read_uint()) as usize;
                    user.num_downloads = try!(packet.read_uint()) as usize;
                    user.unknown       = try!(packet.read_uint()) as usize;
                    user.num_files     = try!(packet.read_uint()) as usize;
                    user.num_folders   = try!(packet.read_uint()) as usize;
                }
                Ok(())
            });
        let num_infos = try!(num_infos_res);

        let num_free_slots_res: result::Result<usize> =
            packet.read_array_with(|packet, i| {
                if let Some(&mut (_, ref mut user)) = self.users.get_mut(i) {
                    user.num_free_slots = try!(packet.read_uint()) as usize;
                }
                Ok(())
            });
        let num_free_slots = try!(num_free_slots_res);

        let num_countries_res: result::Result<usize> =
            packet.read_array_with(|packet, i| {
                if let Some(&mut (_, ref mut user)) = self.users.get_mut(i) {
                    user.country = try!(packet.read_str());
                }
                Ok(())
            });
        let num_countries = try!(num_countries_res);

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

impl FromPacket for RoomLeaveResponse {
    fn from_packet(packet: &mut Packet) -> result::Result<Self> {
        Ok(RoomLeaveResponse {
            room_name: try!(packet.read_str()),
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

impl FromPacket for RoomListResponse {
    fn from_packet(packet: &mut Packet) -> result::Result<Self> {
        let mut response = RoomListResponse {
            rooms: Vec::new(),
            owned_private_rooms: Vec::new(),
            other_private_rooms: Vec::new(),
            operated_private_room_names: Vec::new(),
        };

        try!(Self::read_rooms(packet, &mut response.rooms));

        if let Err(e) = Self::read_rooms(
            packet, &mut response.owned_private_rooms)
        {
            warn!("Error parsing owned_private_rooms: {}", e);
            return Ok(response);
        }

        if let Err(e) = Self::read_rooms(
            packet, &mut response.other_private_rooms)
        {
            warn!("Error parsing other_private_rooms: {}", e);
            return Ok(response);
        }

        if let Err(e) = packet.read_array(
            &mut response.operated_private_room_names, Packet::read_str)
        {
            warn!("Error parsing operated_private_rooms: {}", e);
        }

        Ok(response)
    }
}

impl RoomListResponse {
    fn read_rooms(packet: &mut Packet, rooms: &mut Vec<(String, u32)>)
        -> result::Result<()>
    {
        let original_rooms_len = rooms.len();

        let num_rooms_res: result::Result<usize> =
            packet.read_array(rooms, |packet| {
                Ok((try!(packet.read_str()), 0))
            });
        let num_rooms = try!(num_rooms_res);

        let num_user_counts_res: result::Result<usize> =
            packet.read_array_with(|packet, i| {
                let index = original_rooms_len + i;
                if let Some(&mut (_, ref mut count)) = rooms.get_mut(index) {
                    *count = try!(packet.read_uint());
                }
                Ok(())
            });
        let num_user_counts = try!(num_user_counts_res);

        if num_rooms != num_user_counts {
            warn!("Numbers of rooms and user counts do not match: {} != {}",
                     num_rooms, num_user_counts);
        }

        Ok(())
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

impl FromPacket for RoomMessageResponse {
    fn from_packet(packet: &mut Packet) -> result::Result<Self> {
        let room_name = try!(packet.read_str());
        let user_name = try!(packet.read_str());
        let message   = try!(packet.read_str());
        Ok(RoomMessageResponse {
            room_name: room_name,
            user_name: user_name,
            message:   message,
        })
    }
}

/*==================*
 * USER JOINED ROOM *
 *==================*/

#[derive(Debug)]
pub struct UserJoinedRoomResponse {
    pub room_name: String,
    pub user_name: String,
    pub user:      user::User,
}

impl FromPacket for UserJoinedRoomResponse {
    fn from_packet(packet: &mut Packet) -> result::Result<Self> {
        let room_name = try!(packet.read_str());
        let user_name = try!(packet.read_str());

        let status_u32 = try!(packet.read_uint());
        let status     = try!(user::Status::from_u32(status_u32));

        let average_speed  = try!(packet.read_uint()) as usize;
        let num_downloads  = try!(packet.read_uint()) as usize;
        let unknown        = try!(packet.read_uint()) as usize;
        let num_files      = try!(packet.read_uint()) as usize;
        let num_folders    = try!(packet.read_uint()) as usize;
        let num_free_slots = try!(packet.read_uint()) as usize;

        let country = try!(packet.read_str());

        Ok(UserJoinedRoomResponse {
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

/*===================*
 * WISHLIST INTERVAL *
 *===================*/

#[derive(Debug)]
pub struct WishlistIntervalResponse {
    pub seconds: u32,
}

impl FromPacket for WishlistIntervalResponse {
    fn from_packet(packet: &mut Packet) -> result::Result<Self> {
        let seconds = try!(packet.read_uint());
        Ok(WishlistIntervalResponse {
            seconds: seconds,
        })
    }
}
