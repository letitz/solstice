use room;

/// This enumeration is the list of possible control responses from the client
/// to the controller.
#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum Response {
    RoomJoinResponse(RoomJoinResponse),
    LoginStatusResponse(LoginStatusResponse),
    RoomListResponse(RoomListResponse),
    RoomMessageResponse(RoomMessageResponse),
}

#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct RoomJoinResponse {
    pub room_name: String,
}

/// This enumeration is the list of possible login states, and the associated
/// information.
#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum LoginStatusResponse {
    /// The login request has been sent to the server, but the response hasn't
    /// been received yet.
    Pending {
        /// The username used to log in.
        username: String,
    },

    /// Login was successful.
    Success {
        /// The username used to log in.
        username: String,
        /// The message of the day sent by the server.
        motd: String,
    },

    /// Login failed.
    Failure {
        /// The username used to log in.
        username: String,
        /// The reason the server gave for refusing the login request.
        reason: String,
    }
}

/// This structure contains the list of all visible rooms, and their associated
/// data.
#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct RoomListResponse {
    /// The list of (room name, room data) pairs.
    pub rooms: Vec<(String, room::Room)>,
}

/// This structure contains a message said in a chat room the user is a member
/// of.
#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct RoomMessageResponse {
    /// The name of the room in which the message was said.
    pub room_name: String,
    /// The name of the user who said the message.
    pub user_name: String,
    /// The message itself.
    pub message: String,
}
