use std::collections;
use std::error;
use std::fmt;

use proto::{User, UserStatus};

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

    /// Looks up the given user name in the map, returning a mutable reference
    /// to the associated data if found, or an error if not found.
    pub fn get_mut_strict(&mut self, user_name: &str) -> Result<&mut User, UserNotFoundError> {
        match self.map.get_mut(user_name) {
            Some(user) => Ok(user),
            None => Err(UserNotFoundError { user_name: user_name.to_string() }),
        }
    }

    /// Inserts the given user info for the given user name in the mapping.
    /// If there is already data under that name, it is replaced.
    pub fn insert(&mut self, user_name: String, user: User) {
        self.map.insert(user_name, user);
    }

    /// Sets the given user's status to the given value, if such a user exists.
    pub fn set_status(&mut self, user_name: &str, status: UserStatus) -> Result<(), UserNotFoundError> {
        let user = self.get_mut_strict(user_name)?;
        user.status = status;
        Ok(())
    }

    /// Returns the list of (user name, user data) representing all known users.
    pub fn get_list(&self) -> Vec<(String, User)> {
        let mut users = Vec::new();
        for (user_name, user_data) in self.map.iter() {
            users.push((user_name.clone(), user_data.clone()));
        }
        users
    }

    /// Sets the set of privileged users to the given list.
    pub fn set_all_privileged(&mut self, mut users: Vec<String>) {
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
