use std::io;
use std::net;

use super::constants::*;
use super::super::packet::Packet;

/*=============*
 * FROM PACKET *
 *=============*/

pub trait FromPacket: Sized {
    fn from_packet(&mut Packet) -> io::Result<Self>;
}

/*=================*
 * SERVER RESPONSE *
 *=================*/

#[derive(Debug)]
pub enum ServerResponse {
    ConnectToPeerResponse(ConnectToPeerResponse),
    JoinRoomResponse(JoinRoomResponse),
    LoginResponse(LoginResponse),
    PeerAddressResponse(PeerAddressResponse),
    PrivilegedUsersResponse(PrivilegedUsersResponse),
    RoomListResponse(RoomListResponse),
    WishlistIntervalResponse(WishlistIntervalResponse),

    // Unknown purpose
    ParentMinSpeedResponse(ParentMinSpeedResponse),
    ParentSpeedRatioResponse(ParentSpeedRatioResponse),

    UnknownResponse(u32),
}

impl FromPacket for ServerResponse {
    fn from_packet(packet: &mut Packet) -> io::Result<Self> {
        let code = try!(packet.read_uint());
        let resp = match code {
            CODE_CONNECT_TO_PEER =>
                ServerResponse::ConnectToPeerResponse(
                    try!(ConnectToPeerResponse::from_packet(packet))
                ),

            CODE_JOIN_ROOM =>
                ServerResponse::JoinRoomResponse(
                    try!(JoinRoomResponse::from_packet(packet))
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

            CODE_ROOM_LIST =>
                ServerResponse::RoomListResponse(
                    try!(RoomListResponse::from_packet(packet))
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
    fn from_packet(packet: &mut Packet) -> io::Result<Self> {
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

/*===========*
 * JOIN ROOM *
 *===========*/

#[derive(Debug)]
pub struct JoinRoomResponse {
    room_name: String,
    user_names: Vec<String>,
    user_statuses: Vec<u32>,
}

impl FromPacket for JoinRoomResponse {
    fn from_packet(packet: &mut Packet) -> io::Result<Self> {
        let room_name = try!(packet.read_str());

        let mut user_names = Vec::new();
        try!(packet.read_array(&mut user_names, Packet::read_str));

        let mut user_statuses = Vec::new();
        try!(packet.read_array(&mut user_statuses, Packet::read_uint));

        Ok(JoinRoomResponse {
            room_name: room_name,
            user_names: user_names,
            user_statuses: user_statuses,
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
    fn from_packet(packet: &mut Packet) -> io::Result<Self> {
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
    fn from_packet(packet: &mut Packet) -> io::Result<Self> {
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
    fn from_packet(packet: &mut Packet) -> io::Result<Self> {
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
    fn from_packet(packet: &mut Packet) -> io::Result<Self> {
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
    fn from_packet(packet: &mut Packet) -> io::Result<Self> {
        let mut response = PrivilegedUsersResponse {
            users: Vec::new(),
        };
        try!(packet.read_array(&mut response.users, Packet::read_str));
        Ok(response)
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
    fn from_packet(packet: &mut Packet) -> io::Result<Self> {
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
        -> io::Result<()>
    {
        let original_rooms_len = rooms.len();

        let num_rooms = try!(packet.read_array(rooms, |packet| {
            Ok((try!(packet.read_str()), 0))
        }));

        let num_user_counts = try!(packet.read_array_with(|packet, i| {
            let user_count = try!(packet.read_uint());
            rooms[original_rooms_len+i].1 = user_count;
            Ok(())
        }));

        if num_rooms != num_user_counts {
            warn!("Numbers of rooms and user counts do not match: {} != {}",
                     num_rooms, num_user_counts);
        }

        Ok(())
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
    fn from_packet(packet: &mut Packet) -> io::Result<Self> {
        let seconds = try!(packet.read_uint());
        Ok(WishlistIntervalResponse {
            seconds: seconds,
        })
    }
}
