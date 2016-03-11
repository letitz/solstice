#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum Request {
    LoginStatusRequest,
    RoomListRequest,
}
