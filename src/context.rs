//! This module provides a central `Context` type that ties together all the
//! different bits of client state.

use parking_lot::Mutex;

use crate::login::LoginStatus;
use crate::room::RoomMap;
use crate::user::UserMap;

/// Contains all the different bits of client state.
///
/// Implements `Sync`.
#[derive(Debug)]
pub struct Context {
    pub login: Mutex<LoginStatus>,
    pub rooms: Mutex<RoomMap>,
    pub users: Mutex<UserMap>,
}

impl Context {
    /// Creates a new empty context.
    pub fn new() -> Self {
        Self {
            login: Mutex::new(LoginStatus::Todo),
            rooms: Mutex::new(RoomMap::new()),
            users: Mutex::new(UserMap::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Context;

    #[test]
    fn new_context_is_empty() {
        let context = Context::new();
        assert_eq!(context.rooms.lock().get_room_list(), vec![]);
        assert_eq!(context.users.lock().get_list(), vec![]);
    }

    #[test]
    fn context_is_sync() {
        let _sync: &dyn Sync = &Context::new();
    }
}
