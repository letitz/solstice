#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum Request {
    ConnectNotification,
    DisconnectNotification,
    JoinRoomRequest(String),
    LoginStatusRequest,
    RoomListRequest,
}
