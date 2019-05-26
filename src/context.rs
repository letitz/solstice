//! This module provides a central `Context` type that ties together all the
//! different bits of client state.

use parking_lot::Mutex;

use crate::room::RoomMap;
use crate::user::UserMap;

/// Contains all the different bits of client state.
///
/// Implements `Sync`.
#[derive(Debug)]
pub struct Context {
    pub rooms: Mutex<RoomMap>,
    pub users: Mutex<UserMap>,
}

impl Context {
    /// Creates a new empty context.
    pub fn new() -> Self {
        Self {
            rooms: Mutex::new(RoomMap::new()),
            users: Mutex::new(UserMap::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;

    use super::Context;

    #[test]
    fn new_context_is_empty() {
        let context = Context::new();
        assert_eq!(context.rooms.lock().get_room_list(), vec![]);
        assert_eq!(context.users.lock().get_list(), vec![]);
    }

    #[test]
    fn context_is_sync() {
        let sync: &dyn Sync = &Context::new();
    }
}
