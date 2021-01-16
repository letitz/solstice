/// This enumeration is the list of possible control requests made by the
/// controller client to the client.
#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum Request {
  /// The controller wants to join a room. Contains the room name.
  RoomJoinRequest(String),
  /// The controller wants to leave a rom. Contains the room name.
  RoomLeaveRequest(String),
  /// The controller wants to know what the login status is.
  LoginStatusRequest,
  /// The controller wants to know the list of visible chat rooms.
  RoomListRequest,
  /// The controller wants to send a message to a chat room.
  RoomMessageRequest(RoomMessageRequest),
  /// The controller wants to know the list of known users.
  UserListRequest,
}

/// This structure contains the chat room message request from the controller.
#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct RoomMessageRequest {
  /// The name of the chat room in which to send the message.
  pub room_name: String,
  /// The message to be said.
  pub message: String,
}
