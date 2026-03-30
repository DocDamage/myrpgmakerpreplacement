//! Save system with encryption support
//!
//! This module provides secure save/load functionality:
//! - AES-256-GCM encryption for save files
//! - PBKDF2 key derivation
//! - Save slot management
//! - Automatic backups
//! - Export/import functionality

pub mod encryption;
pub mod manager;

pub use encryption::{EncryptedSave, EncryptionError, encrypt_save, decrypt_save, verify_password, generate_password};
pub use manager::{SaveManager, SaveConfig, SaveMetadata, SaveError};
