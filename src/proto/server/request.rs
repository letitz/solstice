use std::io;

use crypto::digest::Digest;
use crypto::md5::Md5;

use crate::proto::packet::{MutPacket, WriteToPacket};
use crate::proto::server::constants::*;
use crate::proto::{ProtoDecode, ProtoDecoder, ProtoEncode, ProtoEncoder};

/* ------- *
 * Helpers *
 * ------- */

fn md5_str(string: &str) -> String {
    let mut hasher = Md5::new();
    hasher.input_str(string);
    hasher.result_str()
}

/*================*
 * SERVER REQUEST *
 *================*/

#[derive(Debug, Eq, PartialEq)]
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
                packet.write_value(&CODE_CANNOT_CONNECT)?;
                packet.write_value(request)?;
            }

            ServerRequest::ConnectToPeerRequest(ref request) => {
                packet.write_value(&CODE_CONNECT_TO_PEER)?;
                packet.write_value(request)?;
            }

            ServerRequest::FileSearchRequest(ref request) => {
                packet.write_value(&CODE_FILE_SEARCH)?;
                packet.write_value(request)?;
            }

            ServerRequest::LoginRequest(ref request) => {
                packet.write_value(&CODE_LOGIN)?;
                packet.write_value(request)?;
            }

            ServerRequest::PeerAddressRequest(ref request) => {
                packet.write_value(&CODE_PEER_ADDRESS)?;
                packet.write_value(request)?;
            }

            ServerRequest::RoomJoinRequest(ref request) => {
                packet.write_value(&CODE_ROOM_JOIN)?;
                packet.write_value(request)?;
            }

            ServerRequest::RoomLeaveRequest(ref request) => {
                packet.write_value(&CODE_ROOM_LEAVE)?;
                packet.write_value(request)?;
            }

            ServerRequest::RoomListRequest => {
                packet.write_value(&CODE_ROOM_LIST)?;
            }

            ServerRequest::RoomMessageRequest(ref request) => {
                packet.write_value(&CODE_ROOM_MESSAGE)?;
                packet.write_value(request)?;
            }

            ServerRequest::SetListenPortRequest(ref request) => {
                packet.write_value(&CODE_SET_LISTEN_PORT)?;
                packet.write_value(request)?;
            }

            ServerRequest::UserStatusRequest(ref request) => {
                packet.write_value(&CODE_USER_STATUS)?;
                packet.write_value(request)?;
            }
        }
        Ok(())
    }
}

impl ProtoEncode for ServerRequest {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        match *self {
            ServerRequest::CannotConnectRequest(ref request) => {
                encoder.encode_u32(CODE_CANNOT_CONNECT)?;
                request.encode(encoder)?;
            }
            ServerRequest::ConnectToPeerRequest(ref request) => {
                encoder.encode_u32(CODE_CONNECT_TO_PEER)?;
                request.encode(encoder)?;
            }
            ServerRequest::FileSearchRequest(ref request) => {
                encoder.encode_u32(CODE_FILE_SEARCH)?;
                request.encode(encoder)?;
            }
            ServerRequest::LoginRequest(ref request) => {
                encoder.encode_u32(CODE_LOGIN)?;
                request.encode(encoder)?;
            }
            ServerRequest::PeerAddressRequest(ref request) => {
                encoder.encode_u32(CODE_PEER_ADDRESS)?;
                request.encode(encoder)?;
            }
            ServerRequest::RoomJoinRequest(ref request) => {
                encoder.encode_u32(CODE_ROOM_JOIN)?;
                request.encode(encoder)?;
            }
            ServerRequest::RoomLeaveRequest(ref request) => {
                encoder.encode_u32(CODE_ROOM_LEAVE)?;
                request.encode(encoder)?;
            }
            ServerRequest::RoomListRequest => {
                encoder.encode_u32(CODE_ROOM_LIST)?;
            }
            ServerRequest::RoomMessageRequest(ref request) => {
                encoder.encode_u32(CODE_ROOM_MESSAGE)?;
                request.encode(encoder)?;
            }
            ServerRequest::SetListenPortRequest(ref request) => {
                encoder.encode_u32(CODE_SET_LISTEN_PORT)?;
                request.encode(encoder)?;
            }
            ServerRequest::UserStatusRequest(ref request) => {
                encoder.encode_u32(CODE_USER_STATUS)?;
                request.encode(encoder)?;
            }
        }
        Ok(())
    }
}

impl ProtoDecode for ServerRequest {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let code: u32 = decoder.decode()?;
        let request = match code {
            CODE_CANNOT_CONNECT => {
                let request = decoder.decode()?;
                ServerRequest::CannotConnectRequest(request)
            }
            CODE_CONNECT_TO_PEER => {
                let request = decoder.decode()?;
                ServerRequest::ConnectToPeerRequest(request)
            }
            CODE_FILE_SEARCH => {
                let request = decoder.decode()?;
                ServerRequest::FileSearchRequest(request)
            }
            CODE_LOGIN => {
                let request = decoder.decode()?;
                ServerRequest::LoginRequest(request)
            }
            CODE_PEER_ADDRESS => {
                let request = decoder.decode()?;
                ServerRequest::PeerAddressRequest(request)
            }
            CODE_ROOM_JOIN => {
                let request = decoder.decode()?;
                ServerRequest::RoomJoinRequest(request)
            }
            CODE_ROOM_LEAVE => {
                let request = decoder.decode()?;
                ServerRequest::RoomLeaveRequest(request)
            }
            CODE_ROOM_LIST => ServerRequest::RoomListRequest,
            CODE_ROOM_MESSAGE => {
                let request = decoder.decode()?;
                ServerRequest::RoomMessageRequest(request)
            }
            CODE_SET_LISTEN_PORT => {
                let request = decoder.decode()?;
                ServerRequest::SetListenPortRequest(request)
            }
            CODE_USER_STATUS => {
                let request = decoder.decode()?;
                ServerRequest::UserStatusRequest(request)
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unknown server request code: {}", code),
                ));
            }
        };
        Ok(request)
    }
}

/*================*
 * CANNOT CONNECT *
 *================*/

#[derive(Debug, Eq, PartialEq)]
pub struct CannotConnectRequest {
    pub token: u32,
    pub user_name: String,
}

impl WriteToPacket for CannotConnectRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        packet.write_value(&self.token)?;
        packet.write_value(&self.user_name)?;
        Ok(())
    }
}

impl ProtoEncode for CannotConnectRequest {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        encoder.encode_u32(self.token)?;
        encoder.encode_string(&self.user_name)
    }
}

impl ProtoDecode for CannotConnectRequest {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let token = decoder.decode()?;
        let user_name = decoder.decode()?;
        Ok(CannotConnectRequest { token, user_name })
    }
}

/*=================*
 * CONNECT TO PEER *
 *=================*/

#[derive(Debug, Eq, PartialEq)]
pub struct ConnectToPeerRequest {
    pub token: u32,
    pub user_name: String,
    pub connection_type: String,
}

impl WriteToPacket for ConnectToPeerRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        packet.write_value(&self.token)?;
        packet.write_value(&self.user_name)?;
        packet.write_value(&self.connection_type)?;
        Ok(())
    }
}

impl ProtoEncode for ConnectToPeerRequest {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        encoder.encode_u32(self.token)?;
        encoder.encode_string(&self.user_name)?;
        encoder.encode_string(&self.connection_type)
    }
}

impl ProtoDecode for ConnectToPeerRequest {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let token = decoder.decode()?;
        let user_name = decoder.decode()?;
        let connection_type = decoder.decode()?;
        Ok(ConnectToPeerRequest {
            token,
            user_name,
            connection_type,
        })
    }
}

/*=============*
 * FILE SEARCH *
 *=============*/

#[derive(Debug, Eq, PartialEq)]
pub struct FileSearchRequest {
    pub ticket: u32,
    pub query: String,
}

impl WriteToPacket for FileSearchRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        packet.write_value(&self.ticket)?;
        packet.write_value(&self.query)?;
        Ok(())
    }
}

impl ProtoEncode for FileSearchRequest {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        encoder.encode_u32(self.ticket)?;
        encoder.encode_string(&self.query)
    }
}

impl ProtoDecode for FileSearchRequest {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let ticket = decoder.decode()?;
        let query = decoder.decode()?;
        Ok(FileSearchRequest { ticket, query })
    }
}

/*=======*
 * LOGIN *
 *=======*/

#[derive(Debug, Eq, PartialEq)]
pub struct LoginRequest {
    username: String,
    password: String,
    digest: String,
    major: u32,
    minor: u32,
}

fn userpass_md5(username: &str, password: &str) -> String {
    let userpass = String::new() + username + password;
    md5_str(&userpass)
}

impl LoginRequest {
    pub fn new(
        username: &str,
        password: &str,
        major: u32,
        minor: u32,
    ) -> Result<Self, &'static str> {
        if password.len() > 0 {
            Ok(LoginRequest {
                username: username.to_string(),
                password: password.to_string(),
                digest: userpass_md5(username, password),
                major,
                minor,
            })
        } else {
            Err("Empty password")
        }
    }

    fn has_correct_digest(&self) -> bool {
        self.digest == userpass_md5(&self.username, &self.password)
    }
}

impl WriteToPacket for LoginRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        packet.write_value(&self.username)?;
        packet.write_value(&self.password)?;
        packet.write_value(&self.major)?;
        packet.write_value(&self.digest)?;
        packet.write_value(&self.minor)?;
        Ok(())
    }
}

impl ProtoEncode for LoginRequest {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        encoder.encode_string(&self.username)?;
        encoder.encode_string(&self.password)?;
        encoder.encode_u32(self.major)?;
        encoder.encode_string(&self.digest)?;
        encoder.encode_u32(self.minor)
    }
}

impl ProtoDecode for LoginRequest {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let username = decoder.decode()?;
        let password = decoder.decode()?;
        let major = decoder.decode()?;
        let digest = decoder.decode()?;
        let minor = decoder.decode()?;
        Ok(LoginRequest {
            username,
            password,
            digest,
            major,
            minor,
        })
    }
}

/*==============*
 * PEER ADDRESS *
 *==============*/

#[derive(Debug, Eq, PartialEq)]
pub struct PeerAddressRequest {
    pub username: String,
}

impl WriteToPacket for PeerAddressRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        packet.write_value(&self.username)?;
        Ok(())
    }
}

impl ProtoEncode for PeerAddressRequest {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        encoder.encode_string(&self.username)
    }
}

impl ProtoDecode for PeerAddressRequest {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let username = decoder.decode()?;
        Ok(PeerAddressRequest { username: username })
    }
}

/*===========*
 * ROOM JOIN *
 *===========*/

#[derive(Debug, Eq, PartialEq)]
pub struct RoomJoinRequest {
    pub room_name: String,
}

impl WriteToPacket for RoomJoinRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        packet.write_value(&self.room_name)?;
        Ok(())
    }
}

impl ProtoEncode for RoomJoinRequest {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        encoder.encode_string(&self.room_name)
    }
}

impl ProtoDecode for RoomJoinRequest {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let room_name = decoder.decode()?;
        Ok(RoomJoinRequest {
            room_name: room_name,
        })
    }
}

/*============*
 * ROOM LEAVE *
 *============*/

#[derive(Debug, Eq, PartialEq)]
pub struct RoomLeaveRequest {
    pub room_name: String,
}

impl WriteToPacket for RoomLeaveRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        packet.write_value(&self.room_name)?;
        Ok(())
    }
}

impl ProtoEncode for RoomLeaveRequest {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        encoder.encode_string(&self.room_name)
    }
}

impl ProtoDecode for RoomLeaveRequest {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let room_name = decoder.decode()?;
        Ok(RoomLeaveRequest {
            room_name: room_name,
        })
    }
}

/*==============*
 * ROOM MESSAGE *
 *==============*/

#[derive(Debug, Eq, PartialEq)]
pub struct RoomMessageRequest {
    pub room_name: String,
    pub message: String,
}

impl WriteToPacket for RoomMessageRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        packet.write_value(&self.room_name)?;
        packet.write_value(&self.message)?;
        Ok(())
    }
}

impl ProtoEncode for RoomMessageRequest {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        encoder.encode_string(&self.room_name)?;
        encoder.encode_string(&self.message)
    }
}

impl ProtoDecode for RoomMessageRequest {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let room_name = decoder.decode()?;
        let message = decoder.decode()?;
        Ok(RoomMessageRequest { room_name, message })
    }
}

/*=================*
 * SET LISTEN PORT *
 *=================*/

#[derive(Debug, Eq, PartialEq)]
pub struct SetListenPortRequest {
    pub port: u16,
}

impl WriteToPacket for SetListenPortRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        packet.write_value(&self.port)?;
        Ok(())
    }
}

impl ProtoEncode for SetListenPortRequest {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        encoder.encode(&self.port)
    }
}

impl ProtoDecode for SetListenPortRequest {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let port = decoder.decode()?;
        Ok(SetListenPortRequest { port: port })
    }
}

/*=============*
 * USER STATUS *
 *=============*/

#[derive(Debug, Eq, PartialEq)]
pub struct UserStatusRequest {
    pub user_name: String,
}

impl WriteToPacket for UserStatusRequest {
    fn write_to_packet(&self, packet: &mut MutPacket) -> io::Result<()> {
        packet.write_value(&self.user_name)?;
        Ok(())
    }
}

impl ProtoEncode for UserStatusRequest {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        encoder.encode_string(&self.user_name)
    }
}

impl ProtoDecode for UserStatusRequest {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let user_name = decoder.decode()?;
        Ok(UserStatusRequest {
            user_name: user_name,
        })
    }
}

/*=======*
 * TESTS *
 *=======*/

#[cfg(test)]
mod tests {
    use std::io;

    use bytes::BytesMut;

    use crate::proto::base_codec::tests::{expect_io_error, roundtrip};
    use crate::proto::ProtoDecoder;

    use super::*;

    #[test]
    fn invalid_code() {
        let bytes = BytesMut::from(vec![57, 5, 0, 0]);

        let result = ProtoDecoder::new(&bytes).decode::<ServerRequest>();

        expect_io_error(
            result,
            io::ErrorKind::InvalidData,
            "decoding value at position 0: unknown server request code: 1337",
        );
    }

    #[test]
    fn roundtrip_cannot_connect_request() {
        roundtrip(ServerRequest::CannotConnectRequest(CannotConnectRequest {
            token: 1337,
            user_name: "alice".to_string(),
        }))
    }

    #[test]
    fn roundtrip_connect_to_peer_request() {
        roundtrip(ServerRequest::ConnectToPeerRequest(ConnectToPeerRequest {
            token: 1337,
            user_name: "alice".to_string(),
            connection_type: "P".to_string(),
        }))
    }

    #[test]
    fn roundtrip_file_search_request() {
        roundtrip(ServerRequest::FileSearchRequest(FileSearchRequest {
            ticket: 1337,
            query: "foo.txt".to_string(),
        }))
    }

    #[test]
    #[should_panic]
    fn new_login_request_with_empty_password() {
        LoginRequest::new("alice", "", 1337, 42).unwrap();
    }

    #[test]
    fn new_login_request_has_correct_digest() {
        let request = LoginRequest::new("alice", "password1234", 1337, 42).unwrap();
        assert!(request.has_correct_digest());
    }

    #[test]
    fn roundtrip_login_request() {
        roundtrip(ServerRequest::LoginRequest(
            LoginRequest::new("alice", "password1234", 1337, 42).unwrap(),
        ))
    }

    #[test]
    fn roundtrip_peer_address_request() {
        roundtrip(ServerRequest::PeerAddressRequest(PeerAddressRequest {
            username: "alice".to_string(),
        }))
    }

    #[test]
    fn roundtrip_room_join_request() {
        roundtrip(ServerRequest::RoomJoinRequest(RoomJoinRequest {
            room_name: "best room ever".to_string(),
        }))
    }

    #[test]
    fn roundtrip_room_leave_request() {
        roundtrip(ServerRequest::RoomLeaveRequest(RoomLeaveRequest {
            room_name: "best room ever".to_string(),
        }))
    }

    #[test]
    fn roundtrip_room_list_request() {
        roundtrip(ServerRequest::RoomListRequest)
    }

    #[test]
    fn roundtrip_room_message_request() {
        roundtrip(ServerRequest::RoomMessageRequest(RoomMessageRequest {
            room_name: "best room ever".to_string(),
            message: "hello world!".to_string(),
        }))
    }

    #[test]
    fn roundtrip_set_listen_port_request() {
        roundtrip(ServerRequest::SetListenPortRequest(SetListenPortRequest {
            port: 1337,
        }))
    }

    #[test]
    fn roundtrip_user_status_request() {
        roundtrip(ServerRequest::UserStatusRequest(UserStatusRequest {
            user_name: "alice".to_string(),
        }))
    }
}
