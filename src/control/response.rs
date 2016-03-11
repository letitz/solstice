use room;

#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum ControlResponse {
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
    rooms: Vec<room::Room>,
}
