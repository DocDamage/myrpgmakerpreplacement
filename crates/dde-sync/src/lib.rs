//! Real-time collaboration for DocDamage Engine
//!
//! Provides multiplayer editing using CRDTs and WebSockets.

pub mod client;
pub mod crdt;
pub mod error;
pub mod lock;
pub mod presence;
pub mod protocol;
pub mod server;

// Re-export main types for convenience
pub use client::{ClientConfig, ConnectionState, SyncClient};
pub use error::{Result, SyncError};
pub use lock::{LockInfo, LockManager, LockResult, UnlockResult};
pub use presence::{
    Color32, CursorPosition, CursorRenderInfo, PresenceManager, Rect, UserPresence, UserStatus,
    collect_user_cursors, get_color_for_user, get_user_color, USER_COLORS,
};
pub use protocol::{ComponentData, ErrorCode, Operation, ProjectState, SyncMessage};
pub use server::SyncServer;

/// Version of the sync protocol
pub const PROTOCOL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default port for sync server
pub const DEFAULT_SYNC_PORT: u16 = 8080;

/// Default server URL
pub const DEFAULT_SERVER_URL: &str = "ws://localhost:8080/sync";

/// Maximum message size (16 MB)
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// Heartbeat interval in milliseconds
pub const HEARTBEAT_INTERVAL_MS: u64 = 30000;

/// Connection timeout in milliseconds
pub const CONNECTION_TIMEOUT_MS: u64 = 10000;

/// Get a brief description of the sync module
pub fn module_info() -> &'static str {
    "DocDamage Engine Real-time Collaboration Module"
}

/// Check if a message size is within limits
pub fn is_valid_message_size(size: usize) -> bool {
    size <= MAX_MESSAGE_SIZE
}

/// Calculate the recommended heartbeat interval based on network conditions
pub fn calculate_heartbeat_interval(latency_ms: u64) -> u64 {
    // Use 3x latency as heartbeat interval, but clamp between min and max
    let interval = latency_ms.saturating_mul(3);
    interval.clamp(5000, 60000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_version() {
        // Should not be empty
        assert!(!PROTOCOL_VERSION.is_empty());
    }

    #[test]
    fn test_default_port() {
        assert_eq!(DEFAULT_SYNC_PORT, 8080);
    }

    #[test]
    fn test_max_message_size() {
        assert_eq!(MAX_MESSAGE_SIZE, 16 * 1024 * 1024);
        assert!(is_valid_message_size(1024));
        assert!(!is_valid_message_size(MAX_MESSAGE_SIZE + 1));
    }

    #[test]
    fn test_heartbeat_interval() {
        // Fast connection
        assert_eq!(calculate_heartbeat_interval(100), 5000);
        
        // Normal connection
        assert_eq!(calculate_heartbeat_interval(3000), 9000);
        
        // Slow connection - should be clamped
        assert_eq!(calculate_heartbeat_interval(25000), 60000);
    }

    #[test]
    fn test_module_info() {
        assert!(module_info().contains("Collaboration"));
    }
}
