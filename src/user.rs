use std::collections;

use proto::server;
use result;

const STATUS_OFFLINE: u32 = 1;
const STATUS_AWAY:    u32 = 2;
const STATUS_ONLINE:  u32 = 3;

/// This enumeration is the list of possible user statuses.
#[derive(Clone, Copy, Debug)]
pub enum Status {
    /// The user if offline.
    Offline,
    /// The user is connected, but AFK.
    Away,
    /// The user is present.
    Online,
}

impl Status {
    pub fn from_u32(n: u32) -> result::Result<Status> {
        match n {
            STATUS_OFFLINE => Ok(Status::Offline),
            STATUS_AWAY    => Ok(Status::Away),
            STATUS_ONLINE  => Ok(Status::Online),
            _              => Err(result::Error::InvalidEnumError(n as usize))
        }
    }

    pub fn to_u32(&self) -> u32 {
        match *self {
            Status::Offline => STATUS_OFFLINE,
            Status::Away    => STATUS_AWAY,
            Status::Online  => STATUS_ONLINE,
        }
    }
}

/// This structure contains the last known information about a fellow user.
/// It does not store the name, as that is stored implicitly as the key in the
/// user hash table.
#[derive(Clone, Debug)]
pub struct User {
    /// The last known status of the user.
    pub status: Status,
    /// The average upload speed of the user.
    pub average_speed: usize,
    /// ??? Nicotine calls it downloadnum.
    pub num_downloads: usize,
    /// ??? Unknown field.
    pub unknown: usize,
    /// The number of files this user shares.
    pub num_files: usize,
    /// The number of folders this user shares.
    pub num_folders: usize,
    /// The number of free download slots of this user.
    pub num_free_slots: usize,
    /// The user's country code. If unknown, set to None.
    pub country: Option<String>,
}

/// Contains the mapping from user names to user data and provides a clean
/// interface to interact with it.
#[derive(Debug)]
pub struct UserMap {
    /// The actual map from user names to user data and privileged status.
    map: collections::HashMap<String, User>,
    /// The set of privileged users.
    privileged: collections::HashSet<String>,
}

impl UserMap {
    /// Creates an empty mapping.
    pub fn new() -> Self {
        UserMap {
            map: collections::HashMap::new(),
            privileged: collections::HashSet::new(),
        }
    }

    /// Looks up the given user name in the map, returning an immutable
    /// reference to the associated data if found.
    pub fn get(&self, name: &str) -> Option<&User> {
        self.map.get(name)
    }

    /// Inserts the given user info for the given user name in the mapping.
    /// If there is already data under that name, it is replaced.
    pub fn insert(&mut self, name: String, user: User) {
        self.map.insert(name, user);
    }

    /// Update the set of privileged users based on the last server response.
    pub fn update_privileges(
        &mut self, mut response: server::PrivilegedUsersResponse)
    {
        self.privileged.clear();
        for name in response.users.drain(..) {
            self.privileged.insert(name);
        }
    }

    /// Marks the given user as privileged.
    pub fn add_privileged(&mut self, name: String) {
        self.privileged.insert(name);
    }

    /// Checks if the given user is privileged.
    pub fn is_privileged(&self, name: &str) -> bool {
        self.privileged.contains(name)
    }
}
