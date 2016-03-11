use room;

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum Response {
    LoginStatusResponse(LoginStatusResponse),
    RoomListResponse(RoomListResponse),
}

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum LoginStatusResponse {
    Pending {
        username: String,
    },

    Success {
        username: String,
        motd: String,
    },

    Failure {
        username: String,
        reason: String,
    }
}

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct RoomListResponse {
    pub rooms: Vec<(String, room::Room)>,
}
