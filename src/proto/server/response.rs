use std::io;
use std::net;

use proto::packet::{Packet, PacketReadError, ReadFromPacket};
use proto::server::constants::*;
use proto::{ProtoDecode, ProtoDecoder, ProtoEncode, ProtoEncoder, User, UserStatus};

/*=================*
 * SERVER RESPONSE *
 *=================*/

#[derive(Debug, Eq, PartialEq)]
pub enum ServerResponse {
    ConnectToPeerResponse(ConnectToPeerResponse),
    FileSearchResponse(FileSearchResponse),
    LoginResponse(LoginResponse),
    PeerAddressResponse(PeerAddressResponse),
    PrivilegedUsersResponse(PrivilegedUsersResponse),
    RoomJoinResponse(RoomJoinResponse),
    RoomLeaveResponse(RoomLeaveResponse),
    RoomListResponse(RoomListResponse),
    RoomMessageResponse(RoomMessageResponse),
    RoomTickersResponse(RoomTickersResponse),
    RoomUserJoinedResponse(RoomUserJoinedResponse),
    RoomUserLeftResponse(RoomUserLeftResponse),
    UserInfoResponse(UserInfoResponse),
    UserStatusResponse(UserStatusResponse),
    WishlistIntervalResponse(WishlistIntervalResponse),

    // Unknown purpose
    ParentMinSpeedResponse(ParentMinSpeedResponse),
    ParentSpeedRatioResponse(ParentSpeedRatioResponse),

    UnknownResponse(u32),
}

impl ReadFromPacket for ServerResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let code: u32 = try!(packet.read_value());
        let resp = match code {
            CODE_CONNECT_TO_PEER => {
                ServerResponse::ConnectToPeerResponse(try!(packet.read_value()))
            }

            CODE_FILE_SEARCH => ServerResponse::FileSearchResponse(try!(packet.read_value())),

            CODE_LOGIN => ServerResponse::LoginResponse(try!(packet.read_value())),

            CODE_PEER_ADDRESS => ServerResponse::PeerAddressResponse(try!(packet.read_value())),

            CODE_PRIVILEGED_USERS => {
                ServerResponse::PrivilegedUsersResponse(try!(packet.read_value()))
            }

            CODE_ROOM_JOIN => ServerResponse::RoomJoinResponse(try!(packet.read_value())),

            CODE_ROOM_LEAVE => ServerResponse::RoomLeaveResponse(try!(packet.read_value())),

            CODE_ROOM_LIST => ServerResponse::RoomListResponse(try!(packet.read_value())),

            CODE_ROOM_MESSAGE => ServerResponse::RoomMessageResponse(try!(packet.read_value())),

            CODE_ROOM_TICKERS => ServerResponse::RoomTickersResponse(try!(packet.read_value())),

            CODE_ROOM_USER_JOINED => {
                ServerResponse::RoomUserJoinedResponse(try!(packet.read_value()))
            }

            CODE_ROOM_USER_LEFT => ServerResponse::RoomUserLeftResponse(try!(packet.read_value())),

            CODE_USER_INFO => ServerResponse::UserInfoResponse(try!(packet.read_value())),

            CODE_USER_STATUS => ServerResponse::UserStatusResponse(try!(packet.read_value())),

            CODE_WISHLIST_INTERVAL => {
                ServerResponse::WishlistIntervalResponse(try!(packet.read_value()))
            }

            CODE_PARENT_MIN_SPEED => {
                ServerResponse::ParentMinSpeedResponse(try!(packet.read_value()))
            }

            CODE_PARENT_SPEED_RATIO => {
                ServerResponse::ParentSpeedRatioResponse(try!(packet.read_value()))
            }

            code => ServerResponse::UnknownResponse(code),
        };
        let bytes_remaining = packet.bytes_remaining();
        if bytes_remaining > 0 {
            warn!(
                "Packet with code {} contains {} extra bytes",
                code, bytes_remaining
            )
        }
        Ok(resp)
    }
}

impl ProtoEncode for ServerResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        match *self {
            ServerResponse::ConnectToPeerResponse(ref response) => {
                encoder.encode_u32(CODE_CONNECT_TO_PEER)?;
                response.encode(encoder)?;
            }
            ServerResponse::FileSearchResponse(ref response) => {
                encoder.encode_u32(CODE_FILE_SEARCH)?;
                response.encode(encoder)?;
            }
            ServerResponse::LoginResponse(ref response) => {
                encoder.encode_u32(CODE_LOGIN)?;
                response.encode(encoder)?;
            }
            ServerResponse::ParentMinSpeedResponse(ref response) => {
                encoder.encode_u32(CODE_PARENT_MIN_SPEED)?;
                response.encode(encoder)?;
            }
            ServerResponse::ParentSpeedRatioResponse(ref response) => {
                encoder.encode_u32(CODE_PARENT_SPEED_RATIO)?;
                response.encode(encoder)?;
            }
            ServerResponse::PeerAddressResponse(ref response) => {
                encoder.encode_u32(CODE_PEER_ADDRESS)?;
                response.encode(encoder)?;
            }
            ServerResponse::PrivilegedUsersResponse(ref response) => {
                encoder.encode_u32(CODE_PRIVILEGED_USERS)?;
                response.encode(encoder)?;
            }
            ServerResponse::RoomJoinResponse(ref response) => {
                encoder.encode_u32(CODE_ROOM_JOIN)?;
                response.encode(encoder)?;
            }
            ServerResponse::RoomLeaveResponse(ref response) => {
                encoder.encode_u32(CODE_ROOM_LEAVE)?;
                response.encode(encoder)?;
            }
            ServerResponse::RoomListResponse(ref response) => {
                encoder.encode_u32(CODE_ROOM_LIST)?;
                response.encode(encoder)?;
            }
            ServerResponse::RoomMessageResponse(ref response) => {
                encoder.encode_u32(CODE_ROOM_MESSAGE)?;
                response.encode(encoder)?;
            }
            ServerResponse::RoomTickersResponse(ref response) => {
                encoder.encode_u32(CODE_ROOM_TICKERS)?;
                response.encode(encoder)?;
            }
            ServerResponse::RoomUserJoinedResponse(ref response) => {
                encoder.encode_u32(CODE_ROOM_USER_JOINED)?;
                response.encode(encoder)?;
            }
            ServerResponse::RoomUserLeftResponse(ref response) => {
                encoder.encode_u32(CODE_ROOM_USER_LEFT)?;
                response.encode(encoder)?;
            }
            ServerResponse::UserInfoResponse(ref response) => {
                encoder.encode_u32(CODE_USER_INFO)?;
                response.encode(encoder)?;
            }
            ServerResponse::UserStatusResponse(ref response) => {
                encoder.encode_u32(CODE_USER_STATUS)?;
                response.encode(encoder)?;
            }
            ServerResponse::WishlistIntervalResponse(ref response) => {
                encoder.encode_u32(CODE_WISHLIST_INTERVAL)?;
                response.encode(encoder)?;
            }
            _ => {
                unimplemented!();
            }
        };
        Ok(())
    }
}

impl ProtoDecode for ServerResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let code: u32 = decoder.decode()?;
        let response = match code {
            CODE_CONNECT_TO_PEER => {
                let response = decoder.decode()?;
                ServerResponse::ConnectToPeerResponse(response)
            }
            CODE_FILE_SEARCH => {
                let response = decoder.decode()?;
                ServerResponse::FileSearchResponse(response)
            }
            CODE_LOGIN => {
                let response = decoder.decode()?;
                ServerResponse::LoginResponse(response)
            }
            CODE_PARENT_MIN_SPEED => {
                let response = decoder.decode()?;
                ServerResponse::ParentMinSpeedResponse(response)
            }
            CODE_PARENT_SPEED_RATIO => {
                let response = decoder.decode()?;
                ServerResponse::ParentSpeedRatioResponse(response)
            }
            CODE_PEER_ADDRESS => {
                let response = decoder.decode()?;
                ServerResponse::PeerAddressResponse(response)
            }
            CODE_PRIVILEGED_USERS => {
                let response = decoder.decode()?;
                ServerResponse::PrivilegedUsersResponse(response)
            }
            CODE_ROOM_JOIN => {
                let response = decoder.decode()?;
                ServerResponse::RoomJoinResponse(response)
            }
            CODE_ROOM_LEAVE => {
                let response = decoder.decode()?;
                ServerResponse::RoomLeaveResponse(response)
            }
            CODE_ROOM_LIST => {
                let response = decoder.decode()?;
                ServerResponse::RoomListResponse(response)
            }
            CODE_ROOM_MESSAGE => {
                let response = decoder.decode()?;
                ServerResponse::RoomMessageResponse(response)
            }
            CODE_ROOM_TICKERS => {
                let response = decoder.decode()?;
                ServerResponse::RoomTickersResponse(response)
            }
            CODE_ROOM_USER_JOINED => {
                let response = decoder.decode()?;
                ServerResponse::RoomUserJoinedResponse(response)
            }
            CODE_ROOM_USER_LEFT => {
                let response = decoder.decode()?;
                ServerResponse::RoomUserLeftResponse(response)
            }
            CODE_USER_INFO => {
                let response = decoder.decode()?;
                ServerResponse::UserInfoResponse(response)
            }
            CODE_USER_STATUS => {
                let response = decoder.decode()?;
                ServerResponse::UserStatusResponse(response)
            }
            CODE_WISHLIST_INTERVAL => {
                let response = decoder.decode()?;
                ServerResponse::WishlistIntervalResponse(response)
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unknown server response code: {}", code),
                ));
            }
        };
        Ok(response)
    }
}

/*=================*
 * CONNECT TO PEER *
 *=================*/

#[derive(Debug, Eq, PartialEq)]
pub struct ConnectToPeerResponse {
    pub user_name: String,
    pub connection_type: String,
    pub ip: net::Ipv4Addr,
    pub port: u16,
    pub token: u32,
    pub is_privileged: bool,
}

impl ReadFromPacket for ConnectToPeerResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let user_name = try!(packet.read_value());
        let connection_type = try!(packet.read_value());
        let ip = try!(packet.read_value());
        let port = try!(packet.read_value());
        let token = try!(packet.read_value());
        let is_privileged = try!(packet.read_value());

        Ok(ConnectToPeerResponse {
            user_name,
            connection_type,
            ip,
            port,
            token,
            is_privileged,
        })
    }
}

impl ProtoEncode for ConnectToPeerResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        encoder.encode_string(&self.user_name)?;
        encoder.encode_string(&self.connection_type)?;
        encoder.encode_ipv4_addr(self.ip)?;
        encoder.encode(&self.port)?;
        encoder.encode_u32(self.token)?;
        encoder.encode_bool(self.is_privileged)
    }
}

impl ProtoDecode for ConnectToPeerResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let user_name = decoder.decode()?;
        let connection_type = decoder.decode()?;
        let ip = decoder.decode()?;
        let port = decoder.decode()?;
        let token = decoder.decode()?;
        let is_privileged = decoder.decode()?;

        Ok(ConnectToPeerResponse {
            user_name,
            connection_type,
            ip,
            port,
            token,
            is_privileged,
        })
    }
}

/*=============*
 * FILE SEARCH *
 *=============*/

#[derive(Debug, Eq, PartialEq)]
pub struct FileSearchResponse {
    pub user_name: String,
    pub ticket: u32,
    pub query: String,
}

impl ReadFromPacket for FileSearchResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let user_name = try!(packet.read_value());
        let ticket = try!(packet.read_value());
        let query = try!(packet.read_value());

        Ok(FileSearchResponse {
            user_name,
            ticket,
            query,
        })
    }
}

impl ProtoEncode for FileSearchResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        encoder.encode_string(&self.user_name)?;
        encoder.encode_u32(self.ticket)?;
        encoder.encode_string(&self.query)
    }
}

impl ProtoDecode for FileSearchResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let user_name = decoder.decode()?;
        let ticket = decoder.decode()?;
        let query = decoder.decode()?;

        Ok(FileSearchResponse {
            user_name,
            ticket,
            query,
        })
    }
}

/*=======*
 * LOGIN *
 *=======*/

#[derive(Debug, Eq, PartialEq)]
pub enum LoginResponse {
    LoginOk {
        motd: String,
        ip: net::Ipv4Addr,
        password_md5_opt: Option<String>,
    },
    LoginFail {
        reason: String,
    },
}

impl ReadFromPacket for LoginResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let ok = try!(packet.read_value());
        if ok {
            let motd = try!(packet.read_value());
            let ip = try!(packet.read_value());

            match packet.read_value::<bool>() {
                Ok(value) => debug!("LoginResponse last field: {}", value),
                Err(e) => debug!("Error reading LoginResponse field: {:?}", e),
            }

            Ok(LoginResponse::LoginOk {
                motd,
                ip,
                password_md5_opt: None,
            })
        } else {
            Ok(LoginResponse::LoginFail {
                reason: try!(packet.read_value()),
            })
        }
    }
}

impl ProtoEncode for LoginResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        match *self {
            LoginResponse::LoginOk {
                ref motd,
                ip,
                password_md5_opt: _,
            } => {
                encoder.encode_bool(true)?;
                encoder.encode_string(motd)?;
                encoder.encode_ipv4_addr(ip)?;
            }
            LoginResponse::LoginFail { ref reason } => {
                encoder.encode_bool(false)?;
                encoder.encode_string(reason)?;
            }
        };
        Ok(())
    }
}

impl ProtoDecode for LoginResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let ok: bool = decoder.decode()?;
        if !ok {
            let reason = decoder.decode()?;
            return Ok(LoginResponse::LoginFail { reason });
        }

        let motd = decoder.decode()?;
        let ip = decoder.decode()?;

        let result: io::Result<bool> = decoder.decode();
        match result {
            Ok(value) => debug!("LoginResponse last field: {}", value),
            Err(e) => debug!("Error reading LoginResponse field: {:?}", e),
        }

        Ok(LoginResponse::LoginOk {
            motd,
            ip,
            password_md5_opt: None,
        })
    }
}

/*==================*
 * PARENT MIN SPEED *
 *==================*/

#[derive(Debug, Eq, PartialEq)]
pub struct ParentMinSpeedResponse {
    pub value: u32,
}

impl ReadFromPacket for ParentMinSpeedResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let value = try!(packet.read_value());
        Ok(ParentMinSpeedResponse { value })
    }
}

impl ProtoEncode for ParentMinSpeedResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        encoder.encode_u32(self.value)
    }
}

impl ProtoDecode for ParentMinSpeedResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let value = decoder.decode()?;
        Ok(ParentMinSpeedResponse { value })
    }
}

/*====================*
 * PARENT SPEED RATIO *
 *====================*/

#[derive(Debug, Eq, PartialEq)]
pub struct ParentSpeedRatioResponse {
    pub value: u32,
}

impl ReadFromPacket for ParentSpeedRatioResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let value = try!(packet.read_value());
        Ok(ParentSpeedRatioResponse { value })
    }
}

impl ProtoEncode for ParentSpeedRatioResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        encoder.encode_u32(self.value)
    }
}

impl ProtoDecode for ParentSpeedRatioResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let value = decoder.decode()?;
        Ok(ParentSpeedRatioResponse { value })
    }
}

/*==============*
 * PEER ADDRESS *
 *==============*/

#[derive(Debug, Eq, PartialEq)]
pub struct PeerAddressResponse {
    username: String,
    ip: net::Ipv4Addr,
    port: u16,
}

impl ReadFromPacket for PeerAddressResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let username = try!(packet.read_value());
        let ip = try!(packet.read_value());
        let port = try!(packet.read_value());

        Ok(PeerAddressResponse { username, ip, port })
    }
}

impl ProtoEncode for PeerAddressResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        encoder.encode_string(&self.username)?;
        encoder.encode_ipv4_addr(self.ip)?;
        encoder.encode(&self.port)
    }
}

impl ProtoDecode for PeerAddressResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let username = decoder.decode()?;
        let ip = decoder.decode()?;
        let port = decoder.decode()?;
        Ok(PeerAddressResponse { username, ip, port })
    }
}

/*==================*
 * PRIVILEGED USERS *
 *==================*/

#[derive(Debug, Eq, PartialEq)]
pub struct PrivilegedUsersResponse {
    pub users: Vec<String>,
}

impl ReadFromPacket for PrivilegedUsersResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let users = try!(packet.read_value());
        Ok(PrivilegedUsersResponse { users })
    }
}

impl ProtoEncode for PrivilegedUsersResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        encoder.encode(&self.users)
    }
}

impl ProtoDecode for PrivilegedUsersResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let users = decoder.decode()?;
        Ok(PrivilegedUsersResponse { users })
    }
}

/*===========*
 * ROOM JOIN *
 *===========*/

#[derive(Debug, Eq, PartialEq)]
pub struct RoomJoinResponse {
    pub room_name: String,
    pub users: Vec<User>,
    pub owner: Option<String>,
    pub operators: Vec<String>,
}

impl ReadFromPacket for RoomJoinResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let mut response = RoomJoinResponse {
            room_name: try!(packet.read_value()),
            users: Vec::new(),
            owner: None,
            operators: Vec::new(),
        };

        let num_users: usize = try!(packet.read_value());
        for _ in 0..num_users {
            let name: String = try!(packet.read_value());
            let user = User {
                name,
                status: UserStatus::Offline,
                average_speed: 0,
                num_downloads: 0,
                unknown: 0,
                num_files: 0,
                num_folders: 0,
                num_free_slots: 0,
                country: String::new(),
            };
            response.users.push(user);
        }

        try!(response.read_user_infos(packet));

        if packet.bytes_remaining() > 0 {
            response.owner = Some(try!(packet.read_value()));

            let num_operators: usize = try!(packet.read_value());
            for _ in 0..num_operators {
                response.operators.push(try!(packet.read_value()));
            }
        }

        Ok(response)
    }
}

impl RoomJoinResponse {
    fn read_user_infos(&mut self, packet: &mut Packet) -> Result<(), PacketReadError> {
        let num_statuses: usize = try!(packet.read_value());
        for i in 0..num_statuses {
            if let Some(user) = self.users.get_mut(i) {
                user.status = try!(packet.read_value());
            }
        }

        let num_infos: usize = try!(packet.read_value());
        for i in 0..num_infos {
            if let Some(user) = self.users.get_mut(i) {
                user.average_speed = try!(packet.read_value());
                user.num_downloads = try!(packet.read_value());
                user.unknown = try!(packet.read_value());
                user.num_files = try!(packet.read_value());
                user.num_folders = try!(packet.read_value());
            }
        }

        let num_free_slots: usize = try!(packet.read_value());
        for i in 0..num_free_slots {
            if let Some(user) = self.users.get_mut(i) {
                user.num_free_slots = try!(packet.read_value());
            }
        }

        let num_countries: usize = try!(packet.read_value());
        for i in 0..num_countries {
            if let Some(user) = self.users.get_mut(i) {
                user.country = try!(packet.read_value());
            }
        }

        let num_users = self.users.len();
        if num_users != num_statuses
            || num_users != num_infos
            || num_users != num_free_slots
            || num_users != num_countries
        {
            warn!(
                "RoomJoinResponse: mismatched vector sizes {}, {}, {}, {}, {}",
                num_users, num_statuses, num_infos, num_free_slots, num_countries
            );
        }

        Ok(())
    }
}

// This struct is defined to enable decoding a vector of such values for
// `RoomJoinResponse`, but its data is inlined in the `User` struct.
// For details about individual fields, see said `User` struct.
#[derive(Debug, Eq, PartialEq)]
struct UserInfo {
    average_speed: u32,
    num_downloads: u32,
    unknown: u32,
    num_files: u32,
    num_folders: u32,
}

impl UserInfo {
    fn from_user(user: &User) -> Self {
        Self {
            average_speed: user.average_speed as u32,
            num_downloads: user.num_downloads as u32,
            unknown: user.unknown as u32,
            num_files: user.num_files as u32,
            num_folders: user.num_folders as u32,
        }
    }
}

fn build_user(
    name: String,
    status: UserStatus,
    info: UserInfo,
    num_free_slots: u32,
    country: String,
) -> User {
    User {
        name,
        status,
        average_speed: info.average_speed as usize,
        num_downloads: info.num_downloads as usize,
        unknown: info.unknown as usize,
        num_files: info.num_files as usize,
        num_folders: info.num_folders as usize,
        num_free_slots: num_free_slots as usize,
        country,
    }
}

impl ProtoEncode for UserInfo {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        encoder.encode_u32(self.average_speed)?;
        encoder.encode_u32(self.num_downloads)?;
        encoder.encode_u32(self.unknown)?;
        encoder.encode_u32(self.num_files)?;
        encoder.encode_u32(self.num_folders)
    }
}

impl ProtoDecode for UserInfo {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let average_speed = decoder.decode()?;
        let num_downloads = decoder.decode()?;
        let unknown = decoder.decode()?;
        let num_files = decoder.decode()?;
        let num_folders = decoder.decode()?;
        Ok(UserInfo {
            average_speed,
            num_downloads,
            unknown,
            num_files,
            num_folders,
        })
    }
}

impl ProtoEncode for RoomJoinResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> Result<(), io::Error> {
        let mut user_names = vec![];
        let mut user_statuses = vec![];
        let mut user_infos = vec![];
        let mut user_free_slots = vec![];
        let mut user_countries = vec![];
        for user in &self.users {
            user_names.push(&user.name);
            user_statuses.push(user.status);
            user_infos.push(UserInfo::from_user(user));
            user_free_slots.push(user.num_free_slots as u32);
            user_countries.push(&user.country);
        }

        encoder.encode_string(&self.room_name)?;
        encoder.encode(&user_names)?;
        encoder.encode(&user_statuses)?;
        encoder.encode(&user_infos)?;
        encoder.encode(&user_free_slots)?;
        encoder.encode(&user_countries)?;

        if let Some(ref owner) = self.owner {
            encoder.encode_string(owner)?;
            encoder.encode(&self.operators)?;
        }

        Ok(())
    }
}

fn build_users(
    mut names: Vec<String>,
    mut statuses: Vec<UserStatus>,
    mut infos: Vec<UserInfo>,
    mut free_slots: Vec<u32>,
    mut countries: Vec<String>,
) -> Vec<User> {
    let mut users = vec![];

    loop {
        let name_opt = names.pop();
        let status_opt = statuses.pop();
        let info_opt = infos.pop();
        let slots_opt = free_slots.pop();
        let country_opt = countries.pop();

        match (name_opt, status_opt, info_opt, slots_opt, country_opt) {
            (Some(name), Some(status), Some(info), Some(slots), Some(country)) => {
                users.push(build_user(name, status, info, slots, country))
            }
            _ => break,
        }
    }

    users.reverse();
    users
}

impl ProtoDecode for RoomJoinResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let room_name = decoder.decode()?;
        let user_names = decoder.decode()?;
        let user_statuses = decoder.decode()?;
        let user_infos = decoder.decode()?;
        let user_free_slots = decoder.decode()?;
        let user_countries = decoder.decode()?;

        let mut owner = None;
        let mut operators = vec![];
        if decoder.has_remaining() {
            owner = Some(decoder.decode()?);
            operators = decoder.decode()?;
        }

        let users = build_users(
            user_names,
            user_statuses,
            user_infos,
            user_free_slots,
            user_countries,
        );

        Ok(RoomJoinResponse {
            room_name,
            users,
            owner,
            operators,
        })
    }
}

/*============*
 * ROOM LEAVE *
 *============*/

#[derive(Debug, Eq, PartialEq)]
pub struct RoomLeaveResponse {
    pub room_name: String,
}

impl ReadFromPacket for RoomLeaveResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        Ok(RoomLeaveResponse {
            room_name: try!(packet.read_value()),
        })
    }
}

impl ProtoEncode for RoomLeaveResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_string(&self.room_name)
    }
}

impl ProtoDecode for RoomLeaveResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let room_name = decoder.decode()?;
        Ok(RoomLeaveResponse { room_name })
    }
}

/*===========*
 * ROOM LIST *
 *===========*/

#[derive(Debug, Eq, PartialEq)]
pub struct RoomListResponse {
    pub rooms: Vec<(String, u32)>,
    pub owned_private_rooms: Vec<(String, u32)>,
    pub other_private_rooms: Vec<(String, u32)>,
    pub operated_private_room_names: Vec<String>,
}

impl ReadFromPacket for RoomListResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let rooms = try!(Self::read_rooms(packet));
        let owned_private_rooms = try!(Self::read_rooms(packet));
        let other_private_rooms = try!(Self::read_rooms(packet));
        let operated_private_room_names = try!(packet.read_value());
        Ok(RoomListResponse {
            rooms,
            owned_private_rooms,
            other_private_rooms,
            operated_private_room_names,
        })
    }
}

impl RoomListResponse {
    fn read_rooms(packet: &mut Packet) -> Result<Vec<(String, u32)>, PacketReadError> {
        let num_rooms: usize = try!(packet.read_value());
        let mut rooms = Vec::new();
        for _ in 0..num_rooms {
            let room_name = try!(packet.read_value());
            rooms.push((room_name, 0));
        }

        let num_user_counts: usize = try!(packet.read_value());
        for i in 0..num_user_counts {
            if let Some(&mut (_, ref mut count)) = rooms.get_mut(i) {
                *count = try!(packet.read_value());
            }
        }

        if num_rooms != num_user_counts {
            warn!(
                "Numbers of rooms and user counts do not match: {} != {}",
                num_rooms, num_user_counts
            );
        }

        Ok(rooms)
    }

    fn build_rooms(mut room_names: Vec<String>, mut user_counts: Vec<u32>) -> Vec<(String, u32)> {
        let mut rooms = vec![];

        loop {
            let room_name_opt = room_names.pop();
            let user_count_opt = user_counts.pop();

            match (room_name_opt, user_count_opt) {
                (Some(room_name), Some(user_count)) => rooms.push((room_name, user_count)),
                _ => break,
            }
        }

        if !room_names.is_empty() {
            warn!(
                "Unmatched room names in room list response: {:?}",
                room_names
            )
        }
        if !user_counts.is_empty() {
            warn!(
                "Unmatched user counts in room list response: {:?}",
                user_counts
            )
        }

        rooms.reverse();
        rooms
    }

    fn decode_rooms(decoder: &mut ProtoDecoder) -> io::Result<Vec<(String, u32)>> {
        let room_names = decoder.decode()?;
        let user_counts = decoder.decode()?;
        Ok(Self::build_rooms(room_names, user_counts))
    }

    fn encode_rooms(rooms: &[(String, u32)], encoder: &mut ProtoEncoder) -> io::Result<()> {
        let mut room_names = vec![];
        let mut user_counts = vec![];

        for &(ref room_name, user_count) in rooms {
            room_names.push(room_name);
            user_counts.push(user_count);
        }

        encoder.encode(&room_names)?;
        encoder.encode(&user_counts)
    }
}

impl ProtoEncode for RoomListResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        Self::encode_rooms(&self.rooms, encoder)?;
        Self::encode_rooms(&self.owned_private_rooms, encoder)?;
        Self::encode_rooms(&self.other_private_rooms, encoder)?;
        encoder.encode(&self.operated_private_room_names)
    }
}

impl ProtoDecode for RoomListResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let rooms = RoomListResponse::decode_rooms(decoder)?;
        let owned_private_rooms = RoomListResponse::decode_rooms(decoder)?;
        let other_private_rooms = RoomListResponse::decode_rooms(decoder)?;
        let operated_private_room_names = decoder.decode()?;
        Ok(RoomListResponse {
            rooms,
            owned_private_rooms,
            other_private_rooms,
            operated_private_room_names,
        })
    }
}

/*==============*
 * ROOM MESSAGE *
 *==============*/

#[derive(Debug, Eq, PartialEq)]
pub struct RoomMessageResponse {
    pub room_name: String,
    pub user_name: String,
    pub message: String,
}

impl ReadFromPacket for RoomMessageResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let room_name = try!(packet.read_value());
        let user_name = try!(packet.read_value());
        let message = try!(packet.read_value());
        Ok(RoomMessageResponse {
            room_name,
            user_name,
            message,
        })
    }
}

impl ProtoEncode for RoomMessageResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_string(&self.room_name)?;
        encoder.encode_string(&self.user_name)?;
        encoder.encode_string(&self.message)
    }
}

impl ProtoDecode for RoomMessageResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let room_name = decoder.decode()?;
        let user_name = decoder.decode()?;
        let message = decoder.decode()?;
        Ok(RoomMessageResponse {
            room_name,
            user_name,
            message,
        })
    }
}

/*==============*
 * ROOM TICKERS *
 *==============*/

#[derive(Debug, Eq, PartialEq)]
pub struct RoomTickersResponse {
    pub room_name: String,
    pub tickers: Vec<(String, String)>,
}

impl ReadFromPacket for RoomTickersResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let room_name = try!(packet.read_value());

        let num_tickers: usize = try!(packet.read_value());
        let mut tickers = Vec::new();
        for _ in 0..num_tickers {
            let user_name = try!(packet.read_value());
            let message = try!(packet.read_value());
            tickers.push((user_name, message))
        }

        Ok(RoomTickersResponse { room_name, tickers })
    }
}

impl ProtoEncode for RoomTickersResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_string(&self.room_name)?;
        encoder.encode(&self.tickers)
    }
}

impl ProtoDecode for RoomTickersResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let room_name = decoder.decode()?;
        let tickers = decoder.decode()?;
        Ok(RoomTickersResponse { room_name, tickers })
    }
}

/*==================*
 * ROOM USER JOINED *
 *==================*/

#[derive(Debug, Eq, PartialEq)]
pub struct RoomUserJoinedResponse {
    pub room_name: String,
    pub user: User,
}

impl ReadFromPacket for RoomUserJoinedResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let room_name = try!(packet.read_value());
        let user_name = try!(packet.read_value());

        let status = try!(packet.read_value());

        let average_speed = try!(packet.read_value());
        let num_downloads = try!(packet.read_value());
        let unknown = try!(packet.read_value());
        let num_files = try!(packet.read_value());
        let num_folders = try!(packet.read_value());
        let num_free_slots = try!(packet.read_value());

        let country = try!(packet.read_value());

        Ok(RoomUserJoinedResponse {
            room_name,
            user: User {
                name: user_name,
                status,
                average_speed,
                num_downloads,
                unknown,
                num_files,
                num_folders,
                num_free_slots,
                country,
            },
        })
    }
}

impl ProtoEncode for RoomUserJoinedResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_string(&self.room_name)?;
        encoder.encode_string(&self.user.name)?;
        self.user.status.encode(encoder)?;
        UserInfo::from_user(&self.user).encode(encoder)?;
        encoder.encode_u32(self.user.num_free_slots as u32)?;
        encoder.encode_string(&self.user.country)
    }
}

impl ProtoDecode for RoomUserJoinedResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let room_name = decoder.decode()?;
        let user_name = decoder.decode()?;
        let status = decoder.decode()?;
        let info = decoder.decode()?;
        let num_free_slots = decoder.decode()?;
        let country = decoder.decode()?;
        Ok(RoomUserJoinedResponse {
            room_name,
            user: build_user(user_name, status, info, num_free_slots, country),
        })
    }
}

/*================*
 * ROOM USER LEFT *
 *================*/

#[derive(Debug, Eq, PartialEq)]
pub struct RoomUserLeftResponse {
    pub room_name: String,
    pub user_name: String,
}

impl ReadFromPacket for RoomUserLeftResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let room_name = try!(packet.read_value());
        let user_name = try!(packet.read_value());
        Ok(RoomUserLeftResponse {
            room_name,
            user_name,
        })
    }
}

impl ProtoEncode for RoomUserLeftResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_string(&self.room_name)?;
        encoder.encode_string(&self.user_name)
    }
}

impl ProtoDecode for RoomUserLeftResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let room_name = decoder.decode()?;
        let user_name = decoder.decode()?;
        Ok(RoomUserLeftResponse {
            room_name,
            user_name,
        })
    }
}

/*===========*
 * USER INFO *
 *===========*/

#[derive(Debug, Eq, PartialEq)]
pub struct UserInfoResponse {
    pub user_name: String,
    pub average_speed: usize,
    pub num_downloads: usize,
    pub num_files: usize,
    pub num_folders: usize,
}

impl ReadFromPacket for UserInfoResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let user_name = try!(packet.read_value());
        let average_speed = try!(packet.read_value());
        let num_downloads = try!(packet.read_value());
        let num_files = try!(packet.read_value());
        let num_folders = try!(packet.read_value());
        Ok(UserInfoResponse {
            user_name,
            average_speed,
            num_downloads,
            num_files,
            num_folders,
        })
    }
}

impl ProtoEncode for UserInfoResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_string(&self.user_name)?;
        encoder.encode_u32(self.average_speed as u32)?;
        encoder.encode_u32(self.num_downloads as u32)?;
        encoder.encode_u32(self.num_files as u32)?;
        encoder.encode_u32(self.num_folders as u32)
    }
}

impl ProtoDecode for UserInfoResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let user_name = decoder.decode()?;
        let average_speed: u32 = decoder.decode()?;
        let num_downloads: u32 = decoder.decode()?;
        let num_files: u32 = decoder.decode()?;
        let num_folders: u32 = decoder.decode()?;
        Ok(UserInfoResponse {
            user_name,
            average_speed: average_speed as usize,
            num_downloads: num_downloads as usize,
            num_files: num_files as usize,
            num_folders: num_folders as usize,
        })
    }
}

/*=============*
 * USER STATUS *
 *=============*/

#[derive(Debug, Eq, PartialEq)]
pub struct UserStatusResponse {
    pub user_name: String,
    pub status: UserStatus,
    pub is_privileged: bool,
}

impl ReadFromPacket for UserStatusResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let user_name = try!(packet.read_value());
        let status = try!(packet.read_value());
        let is_privileged = try!(packet.read_value());
        Ok(UserStatusResponse {
            user_name,
            status,
            is_privileged,
        })
    }
}

impl ProtoEncode for UserStatusResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_string(&self.user_name)?;
        self.status.encode(encoder)?;
        encoder.encode_bool(self.is_privileged)
    }
}

impl ProtoDecode for UserStatusResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let user_name = decoder.decode()?;
        let status = decoder.decode()?;
        let is_privileged = decoder.decode()?;
        Ok(UserStatusResponse {
            user_name,
            status,
            is_privileged,
        })
    }
}

/*===================*
 * WISHLIST INTERVAL *
 *===================*/

#[derive(Debug, Eq, PartialEq)]
pub struct WishlistIntervalResponse {
    pub seconds: u32,
}

impl ReadFromPacket for WishlistIntervalResponse {
    fn read_from_packet(packet: &mut Packet) -> Result<Self, PacketReadError> {
        let seconds = try!(packet.read_value());
        Ok(WishlistIntervalResponse { seconds })
    }
}

impl ProtoEncode for WishlistIntervalResponse {
    fn encode(&self, encoder: &mut ProtoEncoder) -> io::Result<()> {
        encoder.encode_u32(self.seconds)
    }
}

impl ProtoDecode for WishlistIntervalResponse {
    fn decode_from(decoder: &mut ProtoDecoder) -> io::Result<Self> {
        let seconds = decoder.decode()?;
        Ok(WishlistIntervalResponse { seconds })
    }
}

/*=======*
 * TESTS *
 *=======*/

#[cfg(test)]
mod tests {
    use std::io;
    use std::net;

    use bytes::BytesMut;

    use proto::base_codec::tests::{expect_io_error, roundtrip};
    use proto::ProtoDecoder;

    use super::*;

    #[test]
    fn invalid_code() {
        let bytes = BytesMut::from(vec![57, 5, 0, 0]);

        let result = ProtoDecoder::new(&bytes).decode::<ServerResponse>();

        expect_io_error(
            result,
            io::ErrorKind::InvalidData,
            "unknown server response code: 1337",
        );
    }

    #[test]
    fn roundtrip_connect_to_peer() {
        roundtrip(ServerResponse::ConnectToPeerResponse(
            ConnectToPeerResponse {
                user_name: "alice".to_string(),
                connection_type: "P".to_string(),
                ip: net::Ipv4Addr::new(192, 168, 254, 1),
                port: 1337,
                token: 42,
                is_privileged: true,
            },
        ))
    }

    #[test]
    fn roundtrip_file_search() {
        roundtrip(ServerResponse::FileSearchResponse(FileSearchResponse {
            user_name: "alice".to_string(),
            ticket: 1337,
            query: "foo.txt".to_string(),
        }))
    }

    #[test]
    fn roundtrip_login_ok() {
        roundtrip(ServerResponse::LoginResponse(LoginResponse::LoginOk {
            motd: "welcome one welcome all!".to_string(),
            ip: net::Ipv4Addr::new(127, 0, 0, 1),
            password_md5_opt: None,
        }))
    }

    #[test]
    fn roundtrip_login_fail() {
        roundtrip(ServerResponse::LoginResponse(LoginResponse::LoginFail {
            reason: "I just don't like you".to_string(),
        }))
    }

    #[test]
    fn roundtrip_parent_min_speed() {
        roundtrip(ServerResponse::ParentMinSpeedResponse(
            ParentMinSpeedResponse { value: 1337 },
        ))
    }

    #[test]
    fn roundtrip_parent_speed_ratio() {
        roundtrip(ServerResponse::ParentSpeedRatioResponse(
            ParentSpeedRatioResponse { value: 1337 },
        ))
    }

    #[test]
    fn roundtrip_peer_address() {
        roundtrip(ServerResponse::PeerAddressResponse(PeerAddressResponse {
            username: "alice".to_string(),
            ip: net::Ipv4Addr::new(127, 0, 0, 1),
            port: 1337,
        }))
    }

    #[test]
    fn roundtrip_privileged_users() {
        roundtrip(ServerResponse::PrivilegedUsersResponse(
            PrivilegedUsersResponse {
                users: vec![
                    "alice".to_string(),
                    "bob".to_string(),
                    "chris".to_string(),
                    "dory".to_string(),
                ],
            },
        ))
    }

    #[test]
    fn roundtrip_room_join() {
        roundtrip(ServerResponse::RoomJoinResponse(RoomJoinResponse {
            room_name: "red".to_string(),
            users: vec![
                User {
                    name: "alice".to_string(),
                    status: UserStatus::Online,
                    average_speed: 1000,
                    num_downloads: 1001,
                    unknown: 1002,
                    num_files: 1003,
                    num_folders: 1004,
                    num_free_slots: 1005,
                    country: "US".to_string(),
                },
                User {
                    name: "barbara".to_string(),
                    status: UserStatus::Away,
                    average_speed: 2000,
                    num_downloads: 2001,
                    unknown: 2002,
                    num_files: 2003,
                    num_folders: 2004,
                    num_free_slots: 2005,
                    country: "DE".to_string(),
                },
            ],
            owner: Some("carol".to_string()),
            operators: vec!["deirdre".to_string(), "erica".to_string()],
        }))
    }

    #[test]
    fn roundtrip_room_join_no_owner() {
        roundtrip(ServerResponse::RoomJoinResponse(RoomJoinResponse {
            room_name: "red".to_string(),
            users: vec![],
            owner: None,
            operators: vec![],
        }))
    }

    #[test]
    fn roundtrip_room_leave() {
        roundtrip(ServerResponse::RoomLeaveResponse(RoomLeaveResponse {
            room_name: "red".to_string(),
        }))
    }

    #[test]
    fn roundtrip_room_list() {
        roundtrip(ServerResponse::RoomListResponse(RoomListResponse {
            rooms: vec![("red".to_string(), 12), ("blue".to_string(), 13)],
            owned_private_rooms: vec![("green".to_string(), 14), ("purple".to_string(), 15)],
            other_private_rooms: vec![("yellow".to_string(), 16), ("orange".to_string(), 17)],
            operated_private_room_names: vec!["brown".to_string(), "pink".to_string()],
        }))
    }

    #[test]
    fn roundtrip_room_message() {
        roundtrip(ServerResponse::RoomMessageResponse(RoomMessageResponse {
            room_name: "red".to_string(),
            user_name: "alice".to_string(),
            message: "hello world!".to_string(),
        }))
    }

    #[test]
    fn roundtrip_room_tickers() {
        roundtrip(ServerResponse::RoomTickersResponse(RoomTickersResponse {
            room_name: "red".to_string(),
            tickers: vec![
                ("alice".to_string(), "hello world!".to_string()),
                ("bob".to_string(), "hi alice :)".to_string()),
            ],
        }))
    }

    #[test]
    fn roundtrip_room_user_joined() {
        roundtrip(ServerResponse::RoomUserJoinedResponse(
            RoomUserJoinedResponse {
                room_name: "red".to_string(),
                user: User {
                    name: "alice".to_string(),
                    status: UserStatus::Online,
                    average_speed: 1000,
                    num_downloads: 1001,
                    unknown: 1002,
                    num_files: 1003,
                    num_folders: 1004,
                    num_free_slots: 1005,
                    country: "AR".to_string(),
                },
            },
        ))
    }

    #[test]
    fn roundtrip_room_user_left() {
        roundtrip(ServerResponse::RoomUserLeftResponse(RoomUserLeftResponse {
            room_name: "red".to_string(),
            user_name: "alice".to_string(),
        }))
    }

    #[test]
    fn roundtrip_user_info() {
        roundtrip(ServerResponse::UserInfoResponse(UserInfoResponse {
            user_name: "alice".to_string(),
            average_speed: 1000,
            num_downloads: 1001,
            num_files: 1002,
            num_folders: 1003,
        }))
    }

    #[test]
    fn roundtrip_user_status() {
        roundtrip(ServerResponse::UserStatusResponse(UserStatusResponse {
            user_name: "alice".to_string(),
            status: UserStatus::Offline,
            is_privileged: true,
        }))
    }

    #[test]
    fn roundtrip_wishlist_interval() {
        roundtrip(ServerResponse::WishlistIntervalResponse(
            WishlistIntervalResponse { seconds: 1337 },
        ))
    }
}
