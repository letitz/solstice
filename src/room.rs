/// This enumeration is the list of visibility types for rooms that the user is
/// a member of.
#[derive(Clone, Copy, Debug, RustcDecodable, RustcEncodable)]
pub enum Visibility {
    /// This room is visible to any user.
    Public,
    /// This room is visible only to members, and the user owns it.
    PrivateOwned,
    /// This room is visible only to members, and someone else owns it.
    PrivateOther,
}

/// This structure contains the last known information about a chat room.
/// It does not store the name, as that is stored implicitly as the key in the
/// room hash table.
#[derive(Clone, Copy, Debug, RustcDecodable, RustcEncodable)]
pub struct Room {
    /// The visibility of the room.
    pub visibility: Visibility,
    /// True if the user is one of the room's operators, False if the user is a
    /// regular member.
    pub operated: bool,
    /// The number of users that are members of the room.
    pub user_count: usize,
}
