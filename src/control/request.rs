#[derive(Debug, RustcDecodable, RustcEncodable)]
pub enum ControlRequest {
    LoginStatusRequest,
    RoomListRequest,
}
