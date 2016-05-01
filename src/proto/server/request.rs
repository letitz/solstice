use std::io;

use crypto::md5::Md5;
use crypto::digest::Digest;

use super::constants::*;
use super::super::packet::{MutPacket, WriteToPacket};

/*================*
 * SERVER REQUEST *
 *================*/

#[derive(Debug)]
pub enum ServerRequest {
    LoginRequest(LoginRequest),
    PeerAddressRequest(PeerAddressRequest),
    RoomJoinRequest(RoomJoinRequest),
    RoomLeaveRequest(RoomLeaveRequest),
    RoomListRequest,
    RoomMessageRequest(RoomMessageRequest),
    SetListenPortRequest(SetListenPortRequest),
    UserStatusRequest(UserStatusRequest),
}

macro_rules! try_to_packet {
    ($code: ident, $request:ident) => {
        {
            let mut packet = MutPacket::new($code);
            try!($request.write_to_packet(&mut packet));
            Ok(packet)
        }
    }
}

impl ServerRequest {
    pub fn to_packet(&self) -> io::Result<MutPacket> {
        match *self {
            ServerRequest::LoginRequest(ref request) =>
                try_to_packet!(CODE_LOGIN, request),

            ServerRequest::PeerAddressRequest(ref request) =>
                try_to_packet!(CODE_PEER_ADDRESS, request),

            ServerRequest::RoomJoinRequest(ref request) =>
                try_to_packet!(CODE_ROOM_JOIN, request),

            ServerRequest::RoomLeaveRequest(ref request) =>
                try_to_packet!(CODE_ROOM_LEAVE, request),

            ServerRequest::RoomListRequest =>
                Ok(MutPacket::new(CODE_ROOM_LIST)),

            ServerRequest::RoomMessageRequest(ref request) =>
                try_to_packet!(CODE_ROOM_MESSAGE, request),

            ServerRequest::SetListenPortRequest(ref request) =>
                try_to_packet!(CODE_SET_LISTEN_PORT, request),

            ServerRequest::UserStatusRequest(ref request) =>
                try_to_packet!(CODE_USER_STATUS, request),
        }
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

impl<'a> WriteToPacket for &'a LoginRequest {
    fn write_to_packet(self, packet: &mut MutPacket) -> io::Result<()> {
        let userpass = String::new() + &self.username + &self.password;
        let userpass_md5 = md5_str(&userpass);

        try!(packet.write_value(&self.username));
        try!(packet.write_value(&self.password));
        try!(packet.write_value(self.major));
        try!(packet.write_value(&userpass_md5));
        try!(packet.write_value(self.minor));

        Ok(())
    }
}

/*==============*
 * PEER ADDRESS *
 *==============*/

#[derive(Debug)]
pub struct PeerAddressRequest {
    pub username: String,
}

impl<'a> WriteToPacket for &'a PeerAddressRequest {
    fn write_to_packet(self, packet: &mut MutPacket) -> io::Result<()> {
        try!(packet.write_value(&self.username));
        Ok(())
    }
}

/*===========*
 * ROOM JOIN *
 *===========*/

#[derive(Debug)]
pub struct RoomJoinRequest {
    pub room_name: String
}

impl<'a> WriteToPacket for &'a RoomJoinRequest {
    fn write_to_packet(self, packet: &mut MutPacket) -> io::Result<()> {
        try!(packet.write_value(&self.room_name));
        Ok(())
    }
}

/*============*
 * ROOM LEAVE *
 *============*/

#[derive(Debug)]
pub struct RoomLeaveRequest {
    pub room_name: String
}

impl<'a> WriteToPacket for &'a RoomLeaveRequest {
    fn write_to_packet(self, packet: &mut MutPacket) -> io::Result<()> {
        try!(packet.write_value(&self.room_name));
        Ok(())
    }
}

/*==============*
 * ROOM MESSAGE *
 *==============*/

#[derive(Debug)]
pub struct RoomMessageRequest {
    pub room_name: String,
    pub message:   String,
}

impl<'a> WriteToPacket for &'a RoomMessageRequest {
    fn write_to_packet(self, packet: &mut MutPacket) -> io::Result<()> {
        try!(packet.write_value(&self.room_name));
        try!(packet.write_value(&self.message));
        Ok(())
    }
}

/*=================*
 * SET LISTEN PORT *
 *=================*/

#[derive(Debug)]
pub struct SetListenPortRequest {
    pub port: u16,
}

impl<'a> WriteToPacket for &'a SetListenPortRequest {
    fn write_to_packet(self, packet: &mut MutPacket) -> io::Result<()> {
        try!(packet.write_value(self.port));
        Ok(())
    }
}

/*=============*
 * USER STATUS *
 *=============*/

#[derive(Debug)]
pub struct UserStatusRequest {
    pub user_name: String,
}

impl<'a> WriteToPacket for &'a UserStatusRequest {
    fn write_to_packet(self, packet: &mut MutPacket) -> io::Result<()> {
        try!(packet.write_value(&self.user_name));
        Ok(())
    }
}
