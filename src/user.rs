use std::collections;
use std::error;
use std::fmt;
use std::io;

use proto;

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

impl proto::ReadFromPacket for Status {
    fn read_from_packet(packet: &mut proto::Packet)
        -> Result<Self, proto::PacketReadError>
    {
        let n: u32 = try!(packet.read_value());
        match n {
            STATUS_OFFLINE => Ok(Status::Offline),
            STATUS_AWAY    => Ok(Status::Away),
            STATUS_ONLINE  => Ok(Status::Online),
            _              => {
                Err(proto::PacketReadError::InvalidUserStatusError(n))
            }
        }
    }
}

impl<'a> proto::WriteToPacket for &'a Status {
    fn write_to_packet(self, packet: &mut proto::MutPacket) -> io::Result<()> {
        let n = match *self {
            Status::Offline => STATUS_OFFLINE,
            Status::Away    => STATUS_AWAY,
            Status::Online  => STATUS_ONLINE,
        };
        try!(packet.write_value(n));
        Ok(())
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
    /// The user's country code.
    pub country: String,
}

/// The error returned when a user name was not found in the user map.
#[derive(Debug)]
pub struct UserNotFoundError {
    /// The name of the user that wasn't found.
    user_name: String,
}

impl fmt::Display for UserNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "user \"{}\" not found", self.user_name)
    }
}

impl error::Error for UserNotFoundError {
    fn description(&self) -> &str {
        "user not found"
    }
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
    pub fn get(&self, user_name: &str) -> Option<&User> {
        self.map.get(user_name)
    }

    /// Inserts the given user info for the given user name in the mapping.
    /// If there is already data under that name, it is replaced.
    pub fn insert(&mut self, user_name: String, user: User) {
        self.map.insert(user_name, user);
    }

    /// Sets the given user's status to the given value, if such a user exists.
    pub fn set_status(&mut self, user_name: &str, status: Status)
        -> Result<(), UserNotFoundError>
    {
        if let Some(user) = self.map.get_mut(user_name) {
            user.status = status;
            Ok(())
        } else {
            Err(UserNotFoundError {
                user_name: user_name.to_string(),
            })
        }
    }

    /// Sets the set of privileged users to the given list.
    pub fn set_all_privileged(&mut self, mut users: Vec<String>)
    {
        self.privileged.clear();
        for user_name in users.drain(..) {
            self.privileged.insert(user_name);
        }
    }

    /// Marks the given user as privileged.
    pub fn insert_privileged(&mut self, user_name: String) {
        self.privileged.insert(user_name);
    }

    /// Marks the given user as not privileged.
    pub fn remove_privileged(&mut self, user_name: &str) {
        self.privileged.remove(user_name);
    }

    /// Checks if the given user is privileged.
    pub fn is_privileged(&self, user_name: &str) -> bool {
        self.privileged.contains(user_name)
    }
}
