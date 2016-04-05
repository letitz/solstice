use std::collections;
use proto::server;

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
    /// The number of free download slots of this user.
    pub num_free_slots: usize,
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
