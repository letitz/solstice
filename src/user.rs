/// This enumeration is the list of possible user statuses.
#[derive(Clone, Copy, Debug)]
pub enum Status {
    /// The user if offline.
    Offline = 1,
    /// The user is connected, but AFK.
    Away    = 2,
    /// The user is present.
    Online  = 3,
}

/// This structure contains the last known information about a fellow user.
/// It does not store the name, as that is stored implicitly as the key in the
/// user hash table.
#[derive(Clone, Copy, Debug)]
pub struct User {
    /// The last known status of the user.
    pub status: Status,
    /// The average upload speed of the user.
    pub average_speed: usize,
    /// ???
    pub num_downloads: usize,
    /// The number of files this user shares.
    pub num_files: usize,
    /// The number of folders this user shares.
    pub num_folders: usize,
    /// True if the user has free download slots, False if the user doesn't.
    pub has_free_slots: bool,
}
