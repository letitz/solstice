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
    CannotConnectRequest(CannotConnectRequest),
    ConnectToPeerRequest(ConnectToPeerRequest),
    FileSearchRequest(FileSearchRequest),
    LoginRequest(LoginRequest),
    PeerAddressRequest(PeerAddressRequest),
    RoomJoinRequest(RoomJoinRequest),
    RoomLeaveRequest(RoomLeaveRequest),
    RoomListRequest,
    RoomMessageRequest(RoomMessageRequest),
    SetListenPortRequest(SetListenPortRequest),
    UserStatusRequest(UserStatusRequest),
}

impl WriteToPacket for ServerRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        match *self {
            ServerRequest::CannotConnectRequest(ref request) => {
                try!(packet.write_value(&CODE_CANNOT_CONNECT));
                try!(packet.write_value(request));
            },

            ServerRequest::ConnectToPeerRequest(ref request) => {
                try!(packet.write_value(&CODE_CONNECT_TO_PEER));
                try!(packet.write_value(request));
            },

            ServerRequest::FileSearchRequest(ref request) => {
                try!(packet.write_value(&CODE_FILE_SEARCH));
                try!(packet.write_value(request));
            },

            ServerRequest::LoginRequest(ref request) => {
                try!(packet.write_value(&CODE_LOGIN));
                try!(packet.write_value(request));
            },

            ServerRequest::PeerAddressRequest(ref request) => {
                try!(packet.write_value(&CODE_PEER_ADDRESS));
                try!(packet.write_value(request));
            },

            ServerRequest::RoomJoinRequest(ref request) => {
                try!(packet.write_value(&CODE_ROOM_JOIN));
                try!(packet.write_value(request));
            },

            ServerRequest::RoomLeaveRequest(ref request) => {
                try!(packet.write_value(&CODE_ROOM_LEAVE));
                try!(packet.write_value(request));
            },

            ServerRequest::RoomListRequest => {
                try!(packet.write_value(&CODE_ROOM_LIST));
            },

            ServerRequest::RoomMessageRequest(ref request) => {
                try!(packet.write_value(&CODE_ROOM_MESSAGE));
                try!(packet.write_value(request));
            },

            ServerRequest::SetListenPortRequest(ref request) => {
                try!(packet.write_value(&CODE_SET_LISTEN_PORT));
                try!(packet.write_value(request));
            },

            ServerRequest::UserStatusRequest(ref request) => {
                try!(packet.write_value(&CODE_USER_STATUS));
                try!(packet.write_value(request));
            }
        }
        Ok(())
    }
}

fn md5_str(string: &str) -> String {
    let mut hasher = Md5::new();
    hasher.input_str(string);
    hasher.result_str()
}

/*================*
 * CANNOT CONNECT *
 *================*/

#[derive(Debug)]
pub struct CannotConnectRequest {
    pub token:     u32,
    pub user_name: String,
}

impl WriteToPacket for CannotConnectRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        try!(packet.write_value(&self.token));
        try!(packet.write_value(&self.user_name));
        Ok(())
    }
}

/*=================*
 * CONNECT TO PEER *
 *=================*/

#[derive(Debug)]
pub struct ConnectToPeerRequest {
    pub token:           u32,
    pub user_name:       String,
    pub connection_type: String,
}

impl WriteToPacket for ConnectToPeerRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        try!(packet.write_value(&self.token));
        try!(packet.write_value(&self.user_name));
        try!(packet.write_value(&self.connection_type));
        Ok(())
    }
}

/*=============*
 * FILE SEARCH *
 *=============*/

#[derive(Debug)]
pub struct FileSearchRequest {
    pub ticket: u32,
    pub query:  String,
}

impl WriteToPacket for FileSearchRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        try!(packet.write_value(&self.ticket));
        try!(packet.write_value(&self.query));
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
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        let userpass = String::new() + &self.username + &self.password;
        let userpass_md5 = md5_str(&userpass);

        try!(packet.write_value(&self.username));
        try!(packet.write_value(&self.password));
        try!(packet.write_value(&self.major));
        try!(packet.write_value(&userpass_md5));
        try!(packet.write_value(&self.minor));

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

impl WriteToPacket for PeerAddressRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
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

impl WriteToPacket for RoomJoinRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
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

impl WriteToPacket for RoomLeaveRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
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

impl WriteToPacket for RoomMessageRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
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

impl WriteToPacket for SetListenPortRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        try!(packet.write_value(&self.port));
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

impl WriteToPacket for UserStatusRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        try!(packet.write_value(&self.user_name));
        Ok(())
    }
}
