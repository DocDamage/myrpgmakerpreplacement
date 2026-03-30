//! Real-time collaboration for DocDamage Engine
//!
//! Provides multiplayer editing using CRDTs and WebSockets.

pub mod client;
pub mod server;
pub mod crdt;
pub mod protocol;
pub mod presence;
pub mod error;
pub mod lock;

pub use client::SyncClient;
pub use server::SyncServer;
pub use protocol::{SyncMessage, Operation};
pub use presence::{UserPresence, CursorPosition};
pub use error::{SyncError, Result};
pub use lock::LockManager;
