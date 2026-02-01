use std::{
    collections::HashMap,
    sync::RwLock,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug)]
pub struct RoomInfo {
    pub room_code: String,
    pub server_address: Option<String>,
    pub cert_hash: Option<String>,
    pub created_at: u64,
}

impl RoomInfo {
    pub fn new(room_code: String) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            room_code,
            server_address: None,
            cert_hash: None,
            created_at,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.server_address.is_some() && self.cert_hash.is_some()
    }
}

pub struct RoomStore {
    rooms: RwLock<HashMap<String, RoomInfo>>,
}

impl RoomStore {
    pub fn new() -> Self {
        Self {
            rooms: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert(&self, room_code: String, info: RoomInfo) {
        let mut rooms = self.rooms.write().unwrap();
        rooms.insert(room_code, info);
    }

    pub fn get(&self, room_code: &str) -> Option<RoomInfo> {
        let rooms = self.rooms.read().unwrap();
        rooms.get(room_code).cloned()
    }

    pub fn update_cert_hash(&self, room_code: &str, cert_hash: String) -> bool {
        let mut rooms = self.rooms.write().unwrap();
        if let Some(room) = rooms.get_mut(room_code) {
            room.cert_hash = Some(cert_hash);
            true
        } else {
            false
        }
    }

    pub fn update_server_address(&self, room_code: &str, server_address: String) -> bool {
        let mut rooms = self.rooms.write().unwrap();
        if let Some(room) = rooms.get_mut(room_code) {
            room.server_address = Some(server_address);
            true
        } else {
            false
        }
    }
}
