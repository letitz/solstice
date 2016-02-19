use std::io;
use std::net;

use crypto::md5::Md5;
use crypto::digest::Digest;

use super::Packet;

const VERSION_MAJOR: u32 = 181;
const VERSION_MINOR: u32 = 0;

const CODE_LOGIN: u32 = 1;
const CODE_ROOM_LIST: u32 = 64;

pub enum ServerRequest {
    LoginRequest(LoginRequest),
}

impl ServerRequest {
    pub fn to_packet(&self) -> io::Result<Packet> {
        let (mut packet, request): (Packet, &WriteToPacket) = match *self {
            ServerRequest::LoginRequest(ref request) =>
                (Packet::new(CODE_LOGIN), request),
        };
        try!(request.write_to_packet(&mut packet));
        Ok(packet)
    }
}

trait WriteToPacket {
    fn write_to_packet(&self, &mut Packet) -> io::Result<()>;
}

pub enum ServerResponse {
    LoginResponse(LoginResponse),
    UnknownResponse(u32, Packet),
}

impl ServerResponse {
    pub fn from_packet(mut packet: Packet) -> io::Result<Self> {
        let resp = match try!(packet.read_uint()) {
            CODE_LOGIN => ServerResponse::LoginResponse(
                try!(LoginResponse::from_packet(packet))),

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
            let motd = try!(packet.read_str()).to_string();
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

