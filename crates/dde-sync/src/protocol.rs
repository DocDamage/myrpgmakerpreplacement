//! Protocol messages for real-time collaboration

use dde_core::Entity;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Messages exchanged between clients and server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncMessage {
    // Connection
    Hello {
        client_id: Uuid,
        username: String,
        project_id: String,
    },
    Welcome {
        session_id: Uuid,
        collaborators: Vec<crate::presence::UserPresence>,
    },
    ClientJoined {
        client: crate::presence::UserPresence,
    },
    ClientLeft {
        client_id: Uuid,
    },

    // Operations
    Operation {
        client_id: Uuid,
        timestamp: u64,
        op: Operation,
    },
    OperationAck {
        timestamp: u64,
    },

    // Entity changes
    EntityCreate {
        entity_id: Entity,
        components: Vec<ComponentData>,
    },
    EntityDelete {
        entity_id: Entity,
    },
    ComponentUpdate {
        entity_id: Entity,
        component: ComponentData,
    },

    // Map changes
    TileUpdate {
        map_id: u32,
        x: i32,
        y: i32,
        z: i32,
        tile_id: u32,
    },

    // Presence
    CursorMove {
        client_id: Uuid,
        position: crate::presence::CursorPosition,
    },
    SelectionChange {
        client_id: Uuid,
        selected_entities: Vec<Entity>,
    },
    ViewportChange {
        client_id: Uuid,
        rect: Rect,
    },

    // Locking
    LockEntity {
        client_id: Uuid,
        entity_id: Entity,
    },
    UnlockEntity {
        client_id: Uuid,
        entity_id: Entity,
    },
    LockGranted {
        entity_id: Entity,
    },
    LockDenied {
        entity_id: Entity,
        locked_by: Uuid,
    },
    EntityUnlocked {
        entity_id: Entity,
        unlocked_by: Uuid,
    },

    // Chat
    ChatMessage {
        client_id: Uuid,
        username: String,
        text: String,
        timestamp: u64,
    },

    // Sync
    RequestSync,
    SyncState {
        state: ProjectState,
    },
    
    // Heartbeat
    Ping {
        timestamp: u64,
    },
    Pong {
        timestamp: u64,
    },
    
    // Error
    Error {
        code: ErrorCode,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    Insert,
    Update { field: String, value: serde_json::Value },
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentData {
    pub component_type: String,
    pub data: serde_json::Value,
}

/// Rectangle for viewport representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    pub fn contains(&self, point: (f32, f32)) -> bool {
        point.0 >= self.x
            && point.0 <= self.x + self.width
            && point.1 >= self.y
            && point.1 <= self.y + self.height
    }
}

/// Full project state for initial sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    pub project_id: String,
    pub entities: Vec<EntityState>,
    pub tile_maps: Vec<TileMapState>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityState {
    pub entity_id: Entity,
    pub components: Vec<ComponentData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileMapState {
    pub map_id: u32,
    pub width: i32,
    pub height: i32,
    pub layers: Vec<TileLayerState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileLayerState {
    pub layer_id: u32,
    pub z: i32,
    pub tiles: Vec<TileData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileData {
    pub x: i32,
    pub y: i32,
    pub tile_id: u32,
}

/// Error codes for sync protocol
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ErrorCode {
    InvalidMessage,
    Unauthorized,
    ProjectNotFound,
    EntityLocked,
    RateLimited,
    ServerError,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::InvalidMessage => write!(f, "invalid_message"),
            ErrorCode::Unauthorized => write!(f, "unauthorized"),
            ErrorCode::ProjectNotFound => write!(f, "project_not_found"),
            ErrorCode::EntityLocked => write!(f, "entity_locked"),
            ErrorCode::RateLimited => write!(f, "rate_limited"),
            ErrorCode::ServerError => write!(f, "server_error"),
        }
    }
}
