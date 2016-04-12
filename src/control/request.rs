/// This enumeration is the list of possible control requests made by the
/// controller client to the client.
#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum Request {
    /// Not a real request: this is to notify the client that a controller is
    /// now connected, and control responses can now be sent.
    ConnectNotification,
    /// Not a real request: this is to notify the client that the controller has
    /// now disconnected, and control responses should no longer be
    /// sent.
    DisconnectNotification,
    /// The controller wants to join a room. Contains the room name.
    JoinRoomRequest(String),
    /// The controller wants to know what the login status is.
    LoginStatusRequest,
    /// The controller wants to know the list of visible chat rooms.
    RoomListRequest,
    /// The controller wants to send a message to a chat room.
    RoomMessageRequest(RoomMessageRequest),
}

/// This structure contains the chat room message request from the controller.
#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct RoomMessageRequest {
    /// The name of the chat room in which to send the message.
    pub room_name: String,
    /// The message to be said.
    pub message: String,
}
