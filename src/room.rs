use std::collections;
use std::error;
use std::fmt;
use std::mem;

use crate::proto::{server, User};

/// This enumeration is the list of possible membership states for a chat room.
#[derive(Clone, Copy, Debug, Eq, PartialEq, RustcDecodable, RustcEncodable)]
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
#[derive(Clone, Copy, Debug, Eq, PartialEq, RustcDecodable, RustcEncodable)]
pub enum Visibility {
  /// This room is visible to any user.
  Public,
  /// This room is visible only to members, and the user owns it.
  PrivateOwned,
  /// This room is visible only to members, and someone else owns it.
  PrivateOther,
}

/// This structure contains a chat room message.
#[derive(Clone, Debug, Eq, PartialEq, RustcDecodable, RustcEncodable)]
pub struct Message {
  pub user_name: String,
  pub message: String,
}

/// This structure contains the last known information about a chat room.
/// It does not store the name, as that is stored implicitly as the key in the
/// room hash table.
#[derive(Clone, Debug, Eq, PartialEq, RustcDecodable, RustcEncodable)]
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
  /// The tickers displayed in this room.
  pub tickers: Vec<(String, String)>,
}

impl Room {
  /// Creates a new room with the given visibility and user count.
  fn new(visibility: Visibility, user_count: usize) -> Self {
    Room {
      membership: Membership::NonMember,
      visibility: visibility,
      operated: false,
      user_count: user_count,
      owner: None,
      operators: collections::HashSet::new(),
      members: collections::HashSet::new(),
      messages: Vec::new(),
      tickers: Vec::new(),
    }
  }
}

/// The error returned by RoomMap functions.
#[derive(Debug)]
pub enum Error {
  RoomNotFound(String),
  MembershipChangeInvalid(Membership, Membership),
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      Error::RoomNotFound(ref room_name) => {
        write!(f, "room {:?} not found", room_name)
      }

      Error::MembershipChangeInvalid(old_membership, new_membership) => {
        write!(
          f,
          "cannot change membership from {:?} to {:?}",
          old_membership, new_membership
        )
      }
    }
  }
}

impl error::Error for Error {
  fn description(&self) -> &str {
    match *self {
      Error::RoomNotFound(_) => "room not found",
      Error::MembershipChangeInvalid(_, _) => "cannot change membership",
    }
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
      map: collections::HashMap::new(),
    }
  }

  /// Looks up the given room name in the map, returning an immutable
  /// reference to the associated data if found, or an error if not found.
  fn get_strict(&self, room_name: &str) -> Result<&Room, Error> {
    match self.map.get(room_name) {
      Some(room) => Ok(room),
      None => Err(Error::RoomNotFound(room_name.to_string())),
    }
  }

  /// Looks up the given room name in the map, returning a mutable
  /// reference to the associated data if found, or an error if not found.
  fn get_mut_strict(&mut self, room_name: &str) -> Result<&mut Room, Error> {
    match self.map.get_mut(room_name) {
      Some(room) => Ok(room),
      None => Err(Error::RoomNotFound(room_name.to_string())),
    }
  }

  /// Updates one room in the map based on the information received in
  /// a RoomListResponse and the potential previously stored information.
  fn update_one(
    &mut self,
    name: String,
    visibility: Visibility,
    user_count: u32,
    old_map: &mut collections::HashMap<String, Room>,
  ) {
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
    let mut old_map = mem::replace(&mut self.map, collections::HashMap::new());

    // Add all public rooms.
    for (name, user_count) in response.rooms.drain(..) {
      self.update_one(name, Visibility::Public, user_count, &mut old_map);
    }

    // Add all private, owned, rooms.
    for (name, user_count) in response.owned_private_rooms.drain(..) {
      self.update_one(name, Visibility::PrivateOwned, user_count, &mut old_map);
    }

    // Add all private, unowned, rooms.
    for (name, user_count) in response.other_private_rooms.drain(..) {
      self.update_one(name, Visibility::PrivateOther, user_count, &mut old_map);
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
  pub fn get_room_list(&self) -> Vec<(String, Room)> {
    let mut rooms = Vec::new();
    for (room_name, room) in self.map.iter() {
      rooms.push((room_name.clone(), room.clone()));
    }
    rooms
  }

  /// Records that we are now trying to join the given room.
  /// If the room is not found, or if its membership is not `NonMember`,
  /// returns an error.
  pub fn start_joining(&mut self, room_name: &str) -> Result<(), Error> {
    let room = self.get_mut_strict(room_name)?;

    match room.membership {
      Membership::NonMember => {
        room.membership = Membership::Joining;
        Ok(())
      }

      membership => Err(Error::MembershipChangeInvalid(
        membership,
        Membership::Joining,
      )),
    }
  }

  /// Records that we are now a member of the given room and updates the room
  /// information.
  pub fn join(
    &mut self,
    room_name: &str,
    owner: Option<String>,
    mut operators: Vec<String>,
    members: &[User],
  ) -> Result<(), Error> {
    // First look up the room struct.
    let room = self.get_mut_strict(room_name)?;

    // Log what's happening.
    if let Membership::Joining = room.membership {
      info!("Joined room {:?}", room_name);
    } else {
      warn!(
        "Joined room {:?} but membership was already {:?}",
        room_name, room.membership
      );
    }

    // Update the room struct.
    room.membership = Membership::Member;
    room.user_count = members.len();
    room.owner = owner;

    room.operators.clear();
    for user_name in operators.drain(..) {
      room.operators.insert(user_name);
    }

    room.members.clear();
    for user in members {
      room.members.insert(user.name.clone());
    }

    Ok(())
  }

  /// Records that we are now trying to leave the given room.
  /// If the room is not found, or if its membership status is not `Member`,
  /// returns an error.
  pub fn start_leaving(&mut self, room_name: &str) -> Result<(), Error> {
    let room = self.get_mut_strict(room_name)?;

    match room.membership {
      Membership::Member => {
        room.membership = Membership::Leaving;
        Ok(())
      }

      membership => Err(Error::MembershipChangeInvalid(
        membership,
        Membership::Leaving,
      )),
    }
  }

  /// Records that we have now left the given room.
  pub fn leave(&mut self, room_name: &str) -> Result<(), Error> {
    let room = self.get_mut_strict(room_name)?;

    match room.membership {
      Membership::Leaving => info!("Left room {:?}", room_name),

      membership => warn!(
        "Left room {:?} with wrong membership: {:?}",
        room_name, membership
      ),
    }

    room.membership = Membership::NonMember;
    Ok(())
  }

  /// Saves the given message as the last one in the given room.
  pub fn add_message(
    &mut self,
    room_name: &str,
    message: Message,
  ) -> Result<(), Error> {
    let room = self.get_mut_strict(room_name)?;
    room.messages.push(message);
    Ok(())
  }

  /// Inserts the given user in the given room's set of members.
  /// Returns an error if the room is not found.
  pub fn insert_member(
    &mut self,
    room_name: &str,
    user_name: String,
  ) -> Result<(), Error> {
    let room = self.get_mut_strict(room_name)?;
    room.members.insert(user_name);
    Ok(())
  }

  /// Removes the given user from the given room's set of members.
  /// Returns an error if the room is not found.
  pub fn remove_member(
    &mut self,
    room_name: &str,
    user_name: &str,
  ) -> Result<(), Error> {
    let room = self.get_mut_strict(room_name)?;
    room.members.remove(user_name);
    Ok(())
  }

  /*---------*
   * Tickers *
   *---------*/

  pub fn set_tickers(
    &mut self,
    room_name: &str,
    tickers: Vec<(String, String)>,
  ) -> Result<(), Error> {
    let room = self.get_mut_strict(room_name)?;
    room.tickers = tickers;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use crate::proto::server::RoomListResponse;

  use super::{Room, RoomMap, Visibility};

  #[test]
  fn room_map_new_is_empty() {
    assert_eq!(RoomMap::new().get_room_list(), vec![]);
  }

  #[test]
  fn room_map_get_strict() {
    let mut rooms = RoomMap::new();
    rooms.set_room_list(RoomListResponse {
      rooms: vec![("room a".to_string(), 42), ("room b".to_string(), 1337)],
      owned_private_rooms: vec![],
      other_private_rooms: vec![],
      operated_private_room_names: vec![],
    });

    assert_eq!(
      rooms.get_strict("room a").unwrap(),
      &Room::new(Visibility::Public, 42)
    );
  }
}
