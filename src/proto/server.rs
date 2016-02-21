use std::io;
use std::net;

use crypto::md5::Md5;
use crypto::digest::Digest;

use super::Packet;

const CODE_LOGIN: u32 = 1;
const CODE_ROOM_LIST: u32 = 64;
const CODE_PARENT_MIN_SPEED: u32 = 83;
const CODE_PARENT_SPEED_RATIO: u32 = 84;
const CODE_WISHLIST_INTERVAL: u32 = 104;

trait WriteToPacket {
    fn write_to_packet(&self, &mut Packet) -> io::Result<()>;
}

/*================*
 * SERVER REQUEST *
 *================*/

#[derive(Debug)]
pub enum ServerRequest {
    LoginRequest(LoginRequest),
    RoomListRequest(RoomListRequest),
}

impl ServerRequest {
    pub fn to_packet(&self) -> io::Result<Packet> {
        let (mut packet, request): (Packet, &WriteToPacket) = match *self {
            ServerRequest::LoginRequest(ref request) =>
                (Packet::new(CODE_LOGIN), request),

            ServerRequest::RoomListRequest(ref request) =>
                (Packet::new(CODE_ROOM_LIST), request),
        };
        try!(request.write_to_packet(&mut packet));
        Ok(packet)
    }
}

/*=================*
 * SERVER RESPONSE *
 *=================*/

#[derive(Debug)]
pub enum ServerResponse {
    LoginResponse(LoginResponse),
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
            CODE_LOGIN =>
                ServerResponse::LoginResponse(
                    try!(LoginResponse::from_packet(&mut packet))
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

fn md5_str(string: &str) -> String {
    let mut hasher = Md5::new();
    hasher.input_str(string);
    hasher.result_str()
}

/*=======*
 * LOGIN *
 *=======*/

#[derive(Debug)]
pub struct LoginRequest {
    username: String,
    password: String,
    major: u32,
    minor: u32,
}

impl LoginRequest {
    pub fn new(username: &str, password: &str, major: u32, minor: u32)
        -> Result<Self, &'static str> {
        if password.len() > 0 {
            Ok(LoginRequest {
                username: username.to_string(),
                password: password.to_string(),
                major: major,
                minor: minor,
            })
        } else {
            Err("Empty password")
        }
    }
}

impl WriteToPacket for LoginRequest {
    fn write_to_packet(&self, packet: &mut Packet) -> io::Result<()> {
        let userpass = String::new() + &self.username + &self.password;
        let userpass_md5 = md5_str(&userpass);

        try!(packet.write_str(&self.username));
        try!(packet.write_str(&self.password));
        try!(packet.write_uint(self.major));
        try!(packet.write_str(&userpass_md5));
        try!(packet.write_uint(self.minor));

        Ok(())
    }
}

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

/*===========*
 * ROOM LIST *
 *===========*/

#[derive(Debug)]
pub struct RoomListRequest;

impl RoomListRequest {
    pub fn new() -> Self {
        RoomListRequest
    }
}

impl WriteToPacket for RoomListRequest {
    fn write_to_packet(&self, _: &mut Packet) -> io::Result<()> {
        Ok(())
    }
}

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
            Self::read_rooms(packet, &mut response.owned_private_rooms) {
            warn!("Error parsing owned_private_rooms: {}", e);
            return Ok(response);
        }

        if let Err(e) =
            Self::read_rooms(packet, &mut response.other_private_rooms) {
            warn!("Error parsing other_private_rooms: {}", e);
            return Ok(response);
        }

        if let Err(e) =
            Self::read_room_names(packet,
                                  &mut response.operated_private_room_names)
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

    fn read_room_names(packet: &mut Packet, room_names: &mut Vec<String>)
        -> io::Result<()>
    {
        let num_rooms = try!(packet.read_uint()) as usize;
        for _ in 0..num_rooms {
            let room_name = try!(packet.read_str());
            room_names.push(room_name);
        }
        Ok(())
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
