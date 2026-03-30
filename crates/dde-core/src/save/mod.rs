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

pub use encryption::{
    decrypt_save, encrypt_save, generate_password, verify_password, EncryptedSave, EncryptionError,
};
pub use manager::{SaveConfig, SaveError, SaveManager, SaveMetadata};
