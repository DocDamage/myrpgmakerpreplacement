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

pub use client::SyncClient;
pub use error::{Result, SyncError};
pub use lock::LockManager;
pub use presence::{CursorPosition, UserPresence};
pub use protocol::{Operation, SyncMessage};
pub use server::SyncServer;
