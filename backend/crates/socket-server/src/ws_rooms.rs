use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::debug;

/// Room metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    /// Human-readable room name.
    pub name: String,
    /// When the room was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Optional maximum number of members.
    pub max_members: Option<usize>,
    /// Arbitrary key-value metadata attached to the room.
    pub metadata: HashMap<String, String>,
}

/// WebSocket room management.
///
/// Maintains a bidirectional mapping between rooms and connections so that
/// both "members of room X" and "rooms that connection Y belongs to" are
/// O(1) look-ups.
pub struct RoomManager {
    /// room_id -> set of connection IDs
    rooms: DashMap<String, HashSet<String>>,
    /// room_id -> metadata
    room_info: DashMap<String, RoomInfo>,
    /// connection_id -> set of room IDs
    member_rooms: DashMap<String, HashSet<String>>,
}

impl RoomManager {
    /// Create an empty room manager.
    pub fn new() -> Self {
        Self {
            rooms: DashMap::new(),
            room_info: DashMap::new(),
            member_rooms: DashMap::new(),
        }
    }

    /// Create a room with the given ID and metadata.
    ///
    /// Returns `false` if a room with this ID already exists.
    pub fn create_room(
        &self,
        room_id: String,
        name: String,
        max_members: Option<usize>,
    ) -> bool {
        if self.rooms.contains_key(&room_id) {
            return false;
        }
        self.rooms.insert(room_id.clone(), HashSet::new());
        self.room_info.insert(
            room_id.clone(),
            RoomInfo {
                name: name.clone(),
                created_at: chrono::Utc::now(),
                max_members,
                metadata: HashMap::new(),
            },
        );
        debug!(room_id = %room_id, name = %name, "room created");
        true
    }

    /// Delete a room and remove all members from it.
    ///
    /// Returns `false` if the room did not exist.
    pub fn delete_room(&self, room_id: &str) -> bool {
        if let Some((_, members)) = self.rooms.remove(room_id) {
            // Clean up each member's reverse index
            for conn_id in &members {
                if let Some(mut rooms_set) = self.member_rooms.get_mut(conn_id) {
                    rooms_set.remove(room_id);
                }
            }
            self.room_info.remove(room_id);
            debug!(room_id = %room_id, "room deleted");
            true
        } else {
            false
        }
    }

    /// Join a room.
    ///
    /// Returns `false` if the room does not exist, is full, or the connection
    /// is already a member.
    pub fn join(&self, room_id: &str, conn_id: &str) -> bool {
        if let Some(mut members) = self.rooms.get_mut(room_id) {
            // Check capacity
            if let Some(info) = self.room_info.get(room_id) {
                if let Some(max) = info.max_members {
                    if members.len() >= max {
                        return false;
                    }
                }
            }
            if !members.insert(conn_id.to_owned()) {
                // Already a member
                return false;
            }
            // Update reverse index
            self.member_rooms
                .entry(conn_id.to_owned())
                .and_modify(|set| {
                    set.insert(room_id.to_owned());
                })
                .or_insert_with(|| {
                    let mut set = HashSet::new();
                    set.insert(room_id.to_owned());
                    set
                });
            debug!(room_id = %room_id, conn_id = %conn_id, "joined room");
            true
        } else {
            false
        }
    }

    /// Leave a room.
    ///
    /// Returns `false` if the room does not exist or the connection was not
    /// a member.
    pub fn leave(&self, room_id: &str, conn_id: &str) -> bool {
        if let Some(mut members) = self.rooms.get_mut(room_id) {
            if members.remove(conn_id) {
                // Update reverse index
                if let Some(mut rooms_set) = self.member_rooms.get_mut(conn_id) {
                    rooms_set.remove(room_id);
                }
                debug!(room_id = %room_id, conn_id = %conn_id, "left room");
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Remove a connection from all rooms it belongs to.
    ///
    /// Typically called on disconnect.
    pub fn leave_all(&self, conn_id: &str) {
        if let Some((_, room_ids)) = self.member_rooms.remove(conn_id) {
            for room_id in &room_ids {
                if let Some(mut members) = self.rooms.get_mut(room_id.as_str()) {
                    members.remove(conn_id);
                }
            }
            debug!(conn_id = %conn_id, rooms = room_ids.len(), "left all rooms");
        }
    }

    /// Get all member connection IDs of a room.
    pub fn members(&self, room_id: &str) -> Vec<String> {
        self.rooms
            .get(room_id)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all room IDs that a connection belongs to.
    pub fn rooms_for(&self, conn_id: &str) -> Vec<String> {
        self.member_rooms
            .get(conn_id)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get a clone of the room's metadata.
    pub fn get_room_info(&self, room_id: &str) -> Option<RoomInfo> {
        self.room_info.get(room_id).map(|r| r.clone())
    }

    /// Number of members in a room. Returns `0` if the room does not exist.
    pub fn member_count(&self, room_id: &str) -> usize {
        self.rooms
            .get(room_id)
            .map(|set| set.len())
            .unwrap_or(0)
    }

    /// Total number of rooms.
    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    /// Check whether a connection is a member of the given room.
    pub fn is_in_room(&self, room_id: &str, conn_id: &str) -> bool {
        self.rooms
            .get(room_id)
            .is_some_and(|set| set.contains(conn_id))
    }
}

impl Default for RoomManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_room_and_join() {
        let mgr = RoomManager::new();
        assert!(mgr.create_room("lobby".into(), "Lobby".into(), None));
        assert!(mgr.join("lobby", "conn_1"));
        assert!(mgr.join("lobby", "conn_2"));

        assert_eq!(mgr.member_count("lobby"), 2);
        assert!(mgr.is_in_room("lobby", "conn_1"));
        assert!(mgr.is_in_room("lobby", "conn_2"));

        let rooms = mgr.rooms_for("conn_1");
        assert_eq!(rooms, vec!["lobby".to_string()]);
    }

    #[test]
    fn leave_room() {
        let mgr = RoomManager::new();
        mgr.create_room("lobby".into(), "Lobby".into(), None);
        mgr.join("lobby", "conn_1");
        mgr.join("lobby", "conn_2");

        assert!(mgr.leave("lobby", "conn_1"));
        assert!(!mgr.is_in_room("lobby", "conn_1"));
        assert_eq!(mgr.member_count("lobby"), 1);

        // Leave again should return false
        assert!(!mgr.leave("lobby", "conn_1"));
    }

    #[test]
    fn room_full() {
        let mgr = RoomManager::new();
        mgr.create_room("small".into(), "Small Room".into(), Some(2));
        assert!(mgr.join("small", "conn_1"));
        assert!(mgr.join("small", "conn_2"));
        // Third member should be rejected
        assert!(!mgr.join("small", "conn_3"));
        assert_eq!(mgr.member_count("small"), 2);
    }

    #[test]
    fn leave_all_rooms() {
        let mgr = RoomManager::new();
        mgr.create_room("room_a".into(), "A".into(), None);
        mgr.create_room("room_b".into(), "B".into(), None);
        mgr.join("room_a", "conn_1");
        mgr.join("room_b", "conn_1");

        assert_eq!(mgr.rooms_for("conn_1").len(), 2);

        mgr.leave_all("conn_1");
        assert_eq!(mgr.rooms_for("conn_1").len(), 0);
        assert_eq!(mgr.member_count("room_a"), 0);
        assert_eq!(mgr.member_count("room_b"), 0);
    }

    #[test]
    fn room_info() {
        let mgr = RoomManager::new();
        mgr.create_room("lobby".into(), "Main Lobby".into(), Some(100));

        let info = mgr.get_room_info("lobby").unwrap();
        assert_eq!(info.name, "Main Lobby");
        assert_eq!(info.max_members, Some(100));

        // Duplicate creation returns false
        assert!(!mgr.create_room("lobby".into(), "Duplicate".into(), None));
        assert_eq!(mgr.room_count(), 1);

        // Delete
        assert!(mgr.delete_room("lobby"));
        assert_eq!(mgr.room_count(), 0);
        assert!(mgr.get_room_info("lobby").is_none());
    }
}
