#[derive(Clone, Copy, Debug, RustcDecodable, RustcEncodable)]
pub enum RoomKind {
    Public,
    PrivateOwned,
    PrivateOther,
}

#[derive(Clone, Copy, Debug, RustcDecodable, RustcEncodable)]
pub struct Room {
    pub kind: RoomKind,
    pub operated: bool,
    pub user_count: usize,
}
