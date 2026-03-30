//! Error types for sync operations

/// Result type for sync operations
pub type Result<T> = std::result::Result<T, SyncError>;

/// Errors that can occur during sync operations
#[derive(thiserror::Error, Debug)]
pub enum SyncError {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Not connected to server")]
    NotConnected,

    #[error("Entity locked by another user")]
    EntityLocked,

    #[error("Lock denied: entity {entity_id:?} is locked by {locked_by}")]
    LockDenied {
        entity_id: dde_core::Entity,
        locked_by: uuid::Uuid,
    },

    #[error("Database error: {0}")]
    Database(String),

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Channel send error")]
    ChannelSend,
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for SyncError {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        SyncError::ChannelSend
    }
}
