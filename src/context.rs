//! This module provides a central `Context` type that ties together all the
//! different bits of client state.

use parking_lot::Mutex;

use crate::room::RoomMap;
use crate::user::UserMap;

/// Contains all the different bits of client state.
pub struct Context {
    pub rooms: Mutex<RoomMap>,
    pub users: Mutex<UserMap>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            rooms: Mutex::new(RoomMap::new()),
            users: Mutex::new(UserMap::new()),
        }
    }
}
