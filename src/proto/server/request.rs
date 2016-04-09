use std::io;

use crypto::md5::Md5;
use crypto::digest::Digest;

use super::constants::*;
use super::super::packet::Packet;

trait WriteToPacket {
    fn write_to_packet(&self, &mut Packet) -> io::Result<()>;
}

/*================*
 * SERVER REQUEST *
 *================*/

#[derive(Debug)]
pub enum ServerRequest {
    JoinRoomRequest(JoinRoomRequest),
    LoginRequest(LoginRequest),
    PeerAddressRequest(PeerAddressRequest),
    RoomListRequest,
    SayRoomRequest(SayRoomRequest),
    SetListenPortRequest(SetListenPortRequest),
}

impl ServerRequest {
    pub fn to_packet(&self) -> io::Result<Packet> {
        let (mut packet, request): (Packet, &WriteToPacket) = match *self {
            ServerRequest::JoinRoomRequest(ref request) =>
                (Packet::new(CODE_JOIN_ROOM), request),

            ServerRequest::LoginRequest(ref request) =>
                (Packet::new(CODE_LOGIN), request),

            ServerRequest::PeerAddressRequest(ref request) =>
                (Packet::new(CODE_PEER_ADDRESS), request),

            ServerRequest::RoomListRequest =>
                return Ok(Packet::new(CODE_ROOM_LIST)),

            ServerRequest::SayRoomRequest(ref request) =>
                (Packet::new(CODE_SAY_ROOM), request),

            ServerRequest::SetListenPortRequest(ref request) =>
                (Packet::new(CODE_SET_LISTEN_PORT), request),
        };
        try!(request.write_to_packet(&mut packet));
        Ok(packet)
    }
}

fn md5_str(string: &str) -> String {
    let mut hasher = Md5::new();
    hasher.input_str(string);
    hasher.result_str()
}

/*===========*
 * JOIN ROOM *
 *===========*/

#[derive(Debug)]
pub struct JoinRoomRequest {
    pub room_name: String
}

impl WriteToPacket for JoinRoomRequest {
    fn write_to_packet(&self, packet: &mut Packet) -> io::Result<()> {
        try!(packet.write_str(&self.room_name));
        Ok(())
    }
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

/*==============*
 * PEER ADDRESS *
 *==============*/

#[derive(Debug)]
pub struct PeerAddressRequest {
    username: String,
}

impl PeerAddressRequest {
    fn new(username: &str) -> Self {
        PeerAddressRequest {
            username: username.to_string(),
        }
    }
}

impl WriteToPacket for PeerAddressRequest {
    fn write_to_packet(&self, packet: &mut Packet) -> io::Result<()> {
        try!(packet.write_str(&self.username));
        Ok(())
    }
}

/*==========*
 * SAY ROOM *
 *==========*/

#[derive(Debug)]
pub struct SayRoomRequest {
    pub room_name: String,
    pub message:   String,
}

impl WriteToPacket for SayRoomRequest {
    fn write_to_packet(&self, packet: &mut Packet) -> io::Result<()> {
        try!(packet.write_str(&self.room_name));
        try!(packet.write_str(&self.message));
        Ok(())
    }
}

/*=================*
 * SET LISTEN PORT *
 *=================*/

#[derive(Debug)]
pub struct SetListenPortRequest {
    port: u16,
}

impl SetListenPortRequest {
    fn new(port: u16) -> Self {
        SetListenPortRequest {
            port: port,
        }
    }
}

impl WriteToPacket for SetListenPortRequest {
    fn write_to_packet(&self, packet: &mut Packet) -> io::Result<()> {
        try!(packet.write_uint(self.port as u32));
        Ok(())
    }
}

