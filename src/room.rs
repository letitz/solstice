use std::collections;
use std::error;
use std::fmt;
use std::mem;

use proto::server;
use user;

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

/// This structure contains a chat room message.
#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
pub struct Message {
    pub user_name: String,
    pub message:   String,
}

/// This structure contains the last known information about a chat room.
/// It does not store the name, as that is stored implicitly as the key in the
/// room hash table.
#[derive(Clone, Debug, RustcDecodable, RustcEncodable)]
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
    /// The name of the room's owner, if any.
    pub owner: Option<String>,
    /// The names of the room's operators.
    pub operators: collections::HashSet<String>,
    /// The names of the room's members.
    pub members: collections::HashSet<String>,
    /// The messages sent to this chat room, in chronological order.
    pub messages: Vec<Message>,
}

impl Room {
    /// Creates a new room with the given visibility and user count.
    fn new(visibility: Visibility, user_count: usize) -> Self {
        Room {
            membership: Membership::NonMember,
            visibility: visibility,
            operated:   false,
            user_count: user_count,
            owner:      None,
            operators:  collections::HashSet::new(),
            members:    collections::HashSet::new(),
            messages:   Vec::new(),
        }
    }
}

/// The error returned when a room name was not found in the room map.
#[derive(Debug)]
pub struct RoomNotFoundError {
    room_name: String,
}

impl fmt::Display for RoomNotFoundError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "room \"{}\" not found", self.room_name)
    }
}

impl error::Error for RoomNotFoundError {
    fn description(&self) -> &str {
        "room not found"
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

    /// Updates one room in the map based on the information received in
    /// a RoomListResponse and the potential previously stored information.
    fn update_one(
        &mut self, name: String, visibility: Visibility, user_count: u32,
        old_map: &mut collections::HashMap<String, Room>)
    {
        let room = match old_map.remove(&name) {
            None => Room::new(Visibility::Public, user_count as usize),
            Some(mut room) => {
                room.visibility = visibility;
                room.user_count = user_count as usize;
                room
            }
        };
        if let Some(_) = self.map.insert(name, room) {
            error!("Room present twice in room list response");
        }
    }

    /// Updates the map to reflect the information contained in the given
    /// server response.
    pub fn set_room_list(&mut self, mut response: server::RoomListResponse) {
        // Replace the old mapping with an empty one.
        let mut old_map =
            mem::replace(&mut self.map, collections::HashMap::new());

        // Add all public rooms.
        for (name, user_count) in response.rooms.drain(..) {
            self.update_one(
                name, Visibility::Public, user_count, &mut old_map);
        }

        // Add all private, owned, rooms.
        for (name, user_count) in response.owned_private_rooms.drain(..) {
            self.update_one(
                name, Visibility::PrivateOwned, user_count, &mut old_map);
        }

        // Add all private, unowned, rooms.
        for (name, user_count) in response.other_private_rooms.drain(..) {
            self.update_one(
                name, Visibility::PrivateOther, user_count, &mut old_map);
        }

        // Mark all operated rooms as necessary.
        for name in response.operated_private_room_names.iter() {
            match self.map.get_mut(name) {
                Some(room) => room.operated = true,
                None => error!("Room {} is operated but does not exist", name),
            }
        }
    }

    /// Returns the list of (room name, room data) representing all known rooms.
    pub fn get_room_list(&self) -> Vec<(String, Room)>
    {
        let mut rooms = Vec::new();
        for (room_name, room) in self.map.iter() {
            rooms.push((room_name.clone(), room.clone()));
        }
        rooms
    }

    pub fn join(
        &mut self, room_name: &str,
        owner: Option<String>,
        mut operators: Vec<String>,
        members: &Vec<(String, user::User)>)
        -> Result<(), RoomNotFoundError>
    {
        // First look up the room struct.
        let room = match self.map.get_mut(room_name) {
            Some(room) => room,
            None => return Err(
                RoomNotFoundError{ room_name: room_name.to_string() }
            ),
        };

        // Log what's happening.
        if let Membership::Joining = room.membership {
            info!("Joined room \"{}\"", room_name);
        } else {
            warn!(
                "Joined room \"{}\" but membership was already {:?}",
                room_name, room.membership
            );
        }

        // Update the room struct.
        room.membership = Membership::Member;
        room.user_count = members.len();
        room.owner      = owner;

        room.operators.clear();
        for user_name in operators.drain(..) {
            room.operators.insert(user_name);
        }

        room.members.clear();
        for &(ref user_name, _) in members.iter() {
            room.members.insert(user_name.clone());
        }

        Ok(())
    }

    /// Saves the given message as the last one in the given room.
    pub fn add_message(&mut self, room_name: &str, message: Message) {
        match self.get_mut(room_name) {
            None => {
                error!(
                    "RoomMap::add_message: unknown room \"{}\"", room_name
                );
                return;
            },
            Some(room) => room.messages.push(message),
        }
    }
}

