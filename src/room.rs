use std::collections;
use std::mem;

use control;
use proto::server;

/// This enumeration is the list of possible membership states for a chat room.
#[derive(Clone, Copy, Debug, RustcDecodable, RustcEncodable)]
pub enum Membership {
    /// The user is not a member of this room.
    NonMember,
    /// The user has requested to join the room, but hasn't heard back from the
    /// server yet.
    Joining,
    /// The user is a member of the room.
    Member,
    /// The user has request to leave the room, but hasn't heard back from the
    /// server yet.
    Leaving,
}

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
    /// The membership state of the user for the room.
    pub membership: Membership,
    /// The visibility of the room.
    pub visibility: Visibility,
    /// True if the user is one of the room's operators, False if the user is a
    /// regular member.
    pub operated: bool,
    /// The number of users that are members of the room.
    pub user_count: usize,
}

impl Room {
    /// Merges the previous version of the room's information into the new
    /// version.
    fn merge(&mut self, old_room: &Self) {
        self.membership = old_room.membership;
    }
}

/// Contains the mapping from room names to room data and provides a clean
/// interface to interact with it.
#[derive(Debug)]
pub struct RoomMap {
    /// The actual map from room names to room data.
    map: collections::HashMap<String, Room>,
}

impl RoomMap {
    /// Creates an empty mapping.
    pub fn new() -> Self {
        RoomMap {
            map: collections::HashMap::new()
        }
    }

    /// Looks up the given room name in the map, returning an immutable
    /// reference to the associated data if found.
    pub fn get(&self, name: &str) -> Option<&Room> {
        self.map.get(name)
    }

    /// Looks up the given room name in the map, returning a mutable
    /// reference to the associated data if found.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Room> {
        self.map.get_mut(name)
    }

    /// Updates one room in the mapping.
    fn update_one(
        &mut self, name: String, mut new_room: Room,
        old_map: & collections::HashMap<String, Room>)
    {
        if let Some(old_room) = old_map.get(&name) {
            new_room.merge(old_room);
        }
        if let Some(_) = self.map.insert(name, new_room) {
            error!("Room present twice in room list response");
        }
    }

    /// Updates the map to reflect the information contained in the given
    /// server response.
    pub fn update(&mut self, mut response: server::RoomListResponse) {
        // Replace the old mapping with an empty one.
        let old_map = mem::replace(&mut self.map, collections::HashMap::new());

        // Add all public rooms.
        for (name, user_count) in response.rooms.drain(..) {
            let new_room = Room {
                membership: Membership::NonMember,
                visibility: Visibility::Public,
                operated: false,
                user_count: user_count as usize,
            };
            self.update_one(name, new_room, &old_map);
        }

        // Add all private, owned, rooms.
        for (name, user_count) in response.owned_private_rooms.drain(..) {
            let new_room = Room {
                membership: Membership::NonMember,
                visibility: Visibility::PrivateOwned,
                operated: false,
                user_count: user_count as usize,
            };
            self.update_one(name, new_room, &old_map);
        }

        // Add all private, unowned, rooms.
        for (name, user_count) in response.other_private_rooms.drain(..) {
            let new_room = Room {
                membership: Membership::NonMember,
                visibility: Visibility::PrivateOther,
                operated: false,
                user_count: user_count as usize,
            };
            self.update_one(name, new_room, &old_map);
        }

        // Mark all operated rooms as necessary.
        for name in response.operated_private_room_names.drain(..) {
            match self.map.get_mut(&name) {
                Some(room) => room.operated = true,
                None => error!("Room {} is operated but does not exist", name),
            }
        }
    }

    /// Creates a control response containing the list of visible rooms.
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

