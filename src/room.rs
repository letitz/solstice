use std::collections;

use control;
use proto::server;

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

#[derive(Debug)]
pub struct RoomMap {
    map: collections::HashMap<String, Room>,
}

impl RoomMap {
    pub fn new() -> Self {
        RoomMap {
            map: collections::HashMap::new()
        }
    }

    pub fn get(&self, name: &str) -> Option<&Room> {
        self.map.get(name)
    }

    pub fn update(&mut self, mut response: server::RoomListResponse) {
        // First, clear the current map, keeping backing memory.
        self.map.clear();

        // Add all public rooms.
        for (name, user_count) in response.rooms.drain(..) {
            self.map.insert(name, Room {
                visibility: Visibility::Public,
                operated: false,
                user_count: user_count as usize,
            });
        }

        // Add all private, owned, rooms.
        for (name, user_count) in response.owned_private_rooms.drain(..) {
            let room = Room {
                visibility: Visibility::PrivateOwned,
                operated: false,
                user_count: user_count as usize,
            };
            if let Some(_) = self.map.insert(name, room) {
                error!("Room is both public and owned_private");
            }
        }

        // Add all private, unowned, rooms.
        for (name, user_count) in response.other_private_rooms.drain(..) {
            let room = Room {
                visibility: Visibility::PrivateOther,
                operated: false,
                user_count: user_count as usize,
            };
            if let Some(_) = self.map.insert(name, room) {
                error!("Room is both public and other_private");
            }
        }

        // Mark all operated rooms as necessary.
        for name in response.operated_private_room_names.drain(..) {
            match self.map.get_mut(&name) {
                None => error!("Room {} is operated but does not exist", name),
                Some(room) => room.operated = true,
            }
        }
    }

    pub fn get_room_list_response(&self)
        -> control::RoomListResponse
    {
        let mut response = control::RoomListResponse{ rooms: Vec::new() };
        for (room_name, room) in self.map.iter() {
            response.rooms.push((room_name.clone(), room.clone()));
        }
        response
    }
}
