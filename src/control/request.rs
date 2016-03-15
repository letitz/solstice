#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum Request {
    JoinRoomRequest(String),
    LoginStatusRequest,
    RoomListRequest,
}
