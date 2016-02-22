use std::io;
use std::net;

use super::constants::*;
use super::super::packet::Packet;

const MAX_PORT: u32 = (1 << 16) - 1;

/*=================*
 * SERVER RESPONSE *
 *=================*/

#[derive(Debug)]
pub enum ServerResponse {
    LoginResponse(LoginResponse),
    ConnectToPeerResponse(ConnectToPeerResponse),
    PrivilegedUsersResponse(PrivilegedUsersResponse),
    RoomListResponse(RoomListResponse),
    WishlistIntervalResponse(WishlistIntervalResponse),

    // Unknown purpose
    ParentMinSpeedResponse(ParentMinSpeedResponse),
    ParentSpeedRatioResponse(ParentSpeedRatioResponse),

    UnknownResponse(u32, Packet),
}

impl ServerResponse {
    pub fn from_packet(mut packet: Packet) -> io::Result<Self> {
        let code = try!(packet.read_uint());
        let resp = match code {
            CODE_CONNECT_TO_PEER =>
                ServerResponse::ConnectToPeerResponse(
                    try!(ConnectToPeerResponse::from_packet(&mut packet))
                ),

            CODE_LOGIN =>
                ServerResponse::LoginResponse(
                    try!(LoginResponse::from_packet(&mut packet))
                ),

            CODE_PRIVILEGED_USERS =>
                ServerResponse::PrivilegedUsersResponse(
                    try!(PrivilegedUsersResponse::from_packet(&mut packet))
                ),

            CODE_ROOM_LIST =>
                ServerResponse::RoomListResponse(
                    try!(RoomListResponse::from_packet(&mut packet))
                ),

            CODE_WISHLIST_INTERVAL =>
                ServerResponse::WishlistIntervalResponse(
                    try!(WishlistIntervalResponse::from_packet(&mut packet))
                    ),

            CODE_PARENT_MIN_SPEED =>
                ServerResponse::ParentMinSpeedResponse(
                    try!(ParentMinSpeedResponse::from_packet(&mut packet))
                ),

            CODE_PARENT_SPEED_RATIO =>
                ServerResponse::ParentSpeedRatioResponse(
                    try!(ParentSpeedRatioResponse::from_packet(&mut packet))
                ),

            code => return Ok(ServerResponse::UnknownResponse(code, packet)),
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

impl ConnectToPeerResponse {
    fn from_packet(packet: &mut Packet) -> io::Result<Self> {
        let username = try!(packet.read_str());
        let connection_type = try!(packet.read_str());

        let ip = net::Ipv4Addr::from(try!(packet.read_uint()));

        let port = try!(packet.read_uint());
        if port > MAX_PORT {
            return Err(
                io::Error::new(io::ErrorKind::Other, "Invalid port number"));
        }

        let token = try!(packet.read_uint());
        let is_privileged = try!(packet.read_bool());

        Ok(ConnectToPeerResponse {
            username: username,
            connection_type: connection_type,
            ip: ip,
            port: port as u16,
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

impl LoginResponse {
    pub fn from_packet(packet: &mut Packet) -> io::Result<Self> {
        let ok = try!(packet.read_bool());
        let resp = if ok {
            let motd = try!(packet.read_str());
            let ip = net::Ipv4Addr::from(try!(packet.read_uint()));
            match packet.read_bool() {
                Ok(value) => debug!("LoginResponse last field: {}", value),
                Err(e) => debug!("Error reading LoginResponse field: {:?}", e),
            }
            LoginResponse::LoginOk {
                motd: motd,
                ip: ip,
                password_md5_opt: None
            }
        } else {
            LoginResponse::LoginFail {
                reason: try!(packet.read_str())
            }
        };
        Ok(resp)
    }
}

/*==================*
 * PARENT MIN SPEED *
 *==================*/

#[derive(Debug)]
pub struct ParentMinSpeedResponse {
    pub value: u32,
}

impl ParentMinSpeedResponse {
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

impl ParentSpeedRatioResponse {
    fn from_packet(packet: &mut Packet) -> io::Result<Self> {
        let value = try!(packet.read_uint());
        Ok(ParentSpeedRatioResponse {
            value: value,
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

impl PrivilegedUsersResponse {
    fn from_packet(packet: &mut Packet) -> io::Result<Self> {
        let mut response = PrivilegedUsersResponse {
            users: Vec::new(),
        };
        try!(packet.read_array_with(Packet::read_str, &mut response.users));
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

impl RoomListResponse {
    fn from_packet(packet: &mut Packet) -> io::Result<Self> {
        let mut response = RoomListResponse {
            rooms: Vec::new(),
            owned_private_rooms: Vec::new(),
            other_private_rooms: Vec::new(),
            operated_private_room_names: Vec::new(),
        };

        try!(Self::read_rooms(packet, &mut response.rooms));

        if let Err(e) =
            Self::read_rooms(packet, &mut response.owned_private_rooms)
        {
            warn!("Error parsing owned_private_rooms: {}", e);
            return Ok(response);
        }

        if let Err(e) =
            Self::read_rooms(packet, &mut response.other_private_rooms)
        {
            warn!("Error parsing other_private_rooms: {}", e);
            return Ok(response);
        }

        if let Err(e) =
            packet.read_array_with(
                Packet::read_str, &mut response.operated_private_room_names)
        {
            warn!("Error parsing operated_private_rooms: {}", e);
        }

        Ok(response)
    }

    fn read_rooms(packet: &mut Packet, rooms: &mut Vec<(String, u32)>)
        -> io::Result<()>
    {
        let original_rooms_len = rooms.len();

        let num_rooms = try!(packet.read_uint()) as usize;
        for _ in 0..num_rooms {
            let room_name = try!(packet.read_str());
            rooms.push((room_name, 0));
        }

        let num_user_counts = try!(packet.read_uint()) as usize;
        for i in 0..num_user_counts {
            let user_count = try!(packet.read_uint());
            rooms[original_rooms_len+i].1 = user_count;
        }

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

impl WishlistIntervalResponse {
    fn from_packet(packet: &mut Packet) -> io::Result<Self> {
        let seconds = try!(packet.read_uint());
        Ok(WishlistIntervalResponse {
            seconds: seconds,
        })
    }
}

