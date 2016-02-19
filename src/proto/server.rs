use std::io;
use std::net;

use crypto::md5::Md5;
use crypto::digest::Digest;

use super::Packet;

const VERSION_MAJOR: u32 = 181;
const VERSION_MINOR: u32 = 0;

const CODE_LOGIN: u32 = 1;
const CODE_ROOM_LIST: u32 = 64;

trait WriteToPacket {
    fn write_to_packet(&self, &mut Packet) -> io::Result<()>;
}

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

pub enum ServerResponse {
    LoginResponse(LoginResponse),
    RoomListResponse(RoomListResponse),

    UnknownResponse(u32, Packet),
}

impl ServerResponse {
    pub fn from_packet(mut packet: Packet) -> io::Result<Self> {
        let resp = match try!(packet.read_uint()) {
            CODE_LOGIN => ServerResponse::LoginResponse(
                try!(LoginResponse::from_packet(packet))
                ),

            CODE_ROOM_LIST => ServerResponse::RoomListResponse(
                try!(RoomListResponse::from_packet(packet))
                ),

            code => ServerResponse::UnknownResponse(code, packet),
        };
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
        try!(packet.write_str(&md5_str(&userpass)));
        try!(packet.write_uint(self.minor));

        Ok(())
    }
}

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
    pub fn from_packet(mut packet: Packet) -> io::Result<Self> {
        let ok = try!(packet.read_bool());
        let resp = if ok {
            let motd = try!(packet.read_str());
            let ip = net::Ipv4Addr::from(try!(packet.read_uint()));
            LoginResponse::LoginOk {
                motd: motd,
                ip: ip,
                password_md5_opt: None
            }
        } else {
            LoginResponse::LoginFail {
                reason: try!(packet.read_str()).to_string()
            }
        };
        Ok(resp)
    }
}

/*===========*
 * ROOM LIST *
 *===========*/

pub struct RoomListRequest;

impl RoomListRequest {
    pub fn new() -> Self {
        RoomListRequest
    }
}

impl WriteToPacket for RoomListRequest {
    fn write_to_packet(&self, packet: &mut Packet) -> io::Result<()> {
        Ok(())
    }
}

pub struct RoomListResponse {
    pub rooms: Vec<(String, u32)>,
    pub owned_private_rooms: Vec<(String, u32)>,
    pub other_private_rooms: Vec<(String, u32)>,
}

impl RoomListResponse {
    fn from_packet(mut packet: Packet) -> io::Result<Self> {
        let rooms = try!(Self::read_rooms(&mut packet));

        let (owned_private_rooms, other_private_rooms) =
            match Self::read_rooms(&mut packet) {

            Err(e) => (Vec::new(), Vec::new()),

            Ok(owned_private_rooms) => match Self::read_rooms(&mut packet) {
                Err(e) => (owned_private_rooms, Vec::new()),

                Ok(other_private_rooms) =>
                    (owned_private_rooms, other_private_rooms)
            },
        };

        Ok(RoomListResponse {
            rooms: rooms,
            owned_private_rooms: owned_private_rooms,
            other_private_rooms: other_private_rooms,
        })
    }

    fn read_rooms(packet: &mut Packet) -> io::Result<Vec<(String, u32)>> {
        let mut rooms = Vec::new();

        let num_rooms = try!(packet.read_uint()) as usize;
        for i in 0..num_rooms {
            let room_name = try!(packet.read_str());
            rooms.push((room_name, 0));
        }

        let num_user_counts = try!(packet.read_uint()) as usize;
        for i in 0..num_user_counts {
            let user_count = try!(packet.read_uint());
            rooms[i].1 = user_count;
        }

        if num_rooms != num_user_counts {
            warn!("Numbers of rooms and user counts do not match: {} != {}",
                     num_rooms, num_user_counts);
        }

        Ok(rooms)
    }
}
