//! Save system manager with encryption support
//!
//! Provides high-level save/load operations with:
//! - Automatic encryption/decryption
//! - Save slot management
//! - Metadata tracking
//! - Cloud save preparation

use super::encryption::{decrypt_save, encrypt_save, verify_password, EncryptionError};
use crate::serialization::GameSave;
use chrono;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Save system errors
#[derive(Debug, Error)]
pub enum SaveError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Encryption error: {0}")]
    Encryption(#[from] EncryptionError),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Save slot {0} not found")]
    SlotNotFound(u32),
    #[error("Invalid save file")]
    InvalidSave,
    #[error("Password required")]
    PasswordRequired,
}

/// Save metadata (for save slot UI)
#[derive(Debug, Clone)]
pub struct SaveMetadata {
    /// Save slot number
    pub slot: u32,
    /// Player name
    pub player_name: String,
    /// Current map
    pub current_map: String,
    /// Play time in seconds
    pub play_time_secs: u64,
    /// Save timestamp
    pub timestamp: i64,
    /// Has screenshot
    pub has_screenshot: bool,
    /// File size in bytes
    pub file_size: u64,
    /// Is encrypted
    pub is_encrypted: bool,
}

impl SaveMetadata {
    /// Format play time as HH:MM:SS
    pub fn formatted_play_time(&self) -> String {
        let hours = self.play_time_secs / 3600;
        let minutes = (self.play_time_secs % 3600) / 60;
        let secs = self.play_time_secs % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    }

    /// Format timestamp as readable date
    pub fn formatted_date(&self) -> String {
        chrono::DateTime::from_timestamp_millis(self.timestamp)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }
}

/// Save configuration
#[derive(Debug, Clone)]
pub struct SaveConfig {
    /// Save directory path
    pub save_dir: PathBuf,
    /// Maximum number of save slots
    pub max_slots: u32,
    /// Default encryption password (optional)
    pub default_password: Option<String>,
    /// Auto-backup enabled
    pub auto_backup: bool,
    /// Max backup count per slot
    pub max_backups: u32,
}

impl Default for SaveConfig {
    fn default() -> Self {
        Self {
            save_dir: PathBuf::from("saves"),
            max_slots: 99,
            default_password: None,
            auto_backup: true,
            max_backups: 3,
        }
    }
}

/// Save system manager
pub struct SaveManager {
    config: SaveConfig,
    cache: HashMap<u32, SaveMetadata>,
}

impl SaveManager {
    /// Create new save manager with config
    pub fn new(config: SaveConfig) -> Result<Self, SaveError> {
        // Create save directory if it doesn't exist
        std::fs::create_dir_all(&config.save_dir)?;

        let mut manager = Self {
            config,
            cache: HashMap::new(),
        };

        // Build initial cache
        manager.refresh_cache()?;

        Ok(manager)
    }

    /// Get save file path for slot
    fn save_path(&self, slot: u32) -> PathBuf {
        self.config.save_dir.join(format!("save{:03}.dat", slot))
    }

    /// Get backup path for slot
    fn backup_path(&self, slot: u32, backup_num: u32) -> PathBuf {
        self.config
            .save_dir
            .join(format!("save{:03}_backup_{}.dat", slot, backup_num))
    }

    /// Refresh save metadata cache
    pub fn refresh_cache(&mut self) -> Result<(), SaveError> {
        self.cache.clear();

        for slot in 1..=self.config.max_slots {
            if let Some(metadata) = self.read_metadata(slot)? {
                self.cache.insert(slot, metadata);
            }
        }

        Ok(())
    }

    /// Read metadata from save file without full decryption
    fn read_metadata(&self, slot: u32) -> Result<Option<SaveMetadata>, SaveError> {
        let path = self.save_path(slot);

        if !path.exists() {
            return Ok(None);
        }

        let file_size = std::fs::metadata(&path)?.len();

        // For encrypted saves, we can't read metadata without password
        // So we'll store metadata in a separate file or cache after first load
        // For now, return basic info
        Ok(Some(SaveMetadata {
            slot,
            player_name: "Unknown".to_string(),
            current_map: "Unknown".to_string(),
            play_time_secs: 0,
            timestamp: 0,
            has_screenshot: false,
            file_size,
            is_encrypted: true,
        }))
    }

    /// Save game to slot
    pub fn save(
        &mut self,
        slot: u32,
        save_data: &GameSave,
        password: Option<&str>,
    ) -> Result<(), SaveError> {
        if slot == 0 || slot > self.config.max_slots {
            return Err(SaveError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid save slot: {}", slot),
            )));
        }

        // Create backup if auto-backup enabled and save exists
        if self.config.auto_backup && self.save_path(slot).exists() {
            self.create_backup(slot)?;
        }

        // Serialize save data
        let json = save_data
            .to_json()
            .map_err(|e| SaveError::Serialization(e.to_string()))?;

        // Encrypt or save as plain text
        let data = if let Some(pass) = password.or(self.config.default_password.as_deref()) {
            encrypt_save(&json, pass)?
        } else {
            json.into_bytes()
        };

        // Write to file
        std::fs::write(self.save_path(slot), &data)?;

        // Update cache
        self.cache.insert(
            slot,
            SaveMetadata {
                slot,
                player_name: save_data.player_name.clone(),
                current_map: save_data.current_map.clone(),
                play_time_secs: save_data.play_time_secs,
                timestamp: save_data.timestamp,
                has_screenshot: save_data.screenshot.is_some(),
                file_size: data.len() as u64,
                is_encrypted: password.is_some() || self.config.default_password.is_some(),
            },
        );

        Ok(())
    }

    /// Load game from slot
    pub fn load(&self, slot: u32, password: Option<&str>) -> Result<GameSave, SaveError> {
        let path = self.save_path(slot);

        if !path.exists() {
            return Err(SaveError::SlotNotFound(slot));
        }

        let data = std::fs::read(&path)?;

        // Try to parse as JSON first (unencrypted)
        let json = if let Ok(json_str) = String::from_utf8(data.clone()) {
            if json_str.starts_with('{') {
                json_str
            } else {
                // Try to decrypt
                let pass = password.ok_or(SaveError::PasswordRequired)?;
                decrypt_save(&data, pass)?
            }
        } else {
            // Try to decrypt
            let pass = password
                .or(self.config.default_password.as_deref())
                .ok_or(SaveError::PasswordRequired)?;
            decrypt_save(&data, pass)?
        };

        // Parse GameSave
        let save =
            GameSave::from_json(&json).map_err(|e| SaveError::Serialization(e.to_string()))?;

        Ok(save)
    }

    /// Check if save slot exists
    pub fn exists(&self, slot: u32) -> bool {
        self.cache.contains_key(&slot)
    }

    /// Get save metadata
    pub fn get_metadata(&self, slot: u32) -> Option<&SaveMetadata> {
        self.cache.get(&slot)
    }

    /// Get all save slots metadata
    pub fn get_all_metadata(&self) -> Vec<&SaveMetadata> {
        let mut result: Vec<_> = self.cache.values().collect();
        result.sort_by_key(|m| m.slot);
        result
    }

    /// Delete save slot
    pub fn delete(&mut self, slot: u32) -> Result<(), SaveError> {
        let path = self.save_path(slot);

        if path.exists() {
            std::fs::remove_file(&path)?;
        }

        // Also delete backups
        for i in 0..self.config.max_backups {
            let backup = self.backup_path(slot, i);
            if backup.exists() {
                std::fs::remove_file(&backup)?;
            }
        }

        self.cache.remove(&slot);
        Ok(())
    }

    /// Create backup of save slot
    fn create_backup(&self, slot: u32) -> Result<(), SaveError> {
        let source = self.save_path(slot);

        if !source.exists() {
            return Ok(());
        }

        // Rotate backups
        for i in (1..self.config.max_backups).rev() {
            let older = self.backup_path(slot, i - 1);
            let newer = self.backup_path(slot, i);

            if older.exists() {
                std::fs::copy(&older, &newer)?;
            }
        }

        // Copy current to backup_0
        std::fs::copy(&source, self.backup_path(slot, 0))?;

        Ok(())
    }

    /// Restore from backup
    pub fn restore_backup(&mut self, slot: u32, backup_num: u32) -> Result<(), SaveError> {
        if backup_num >= self.config.max_backups {
            return Err(SaveError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid backup number",
            )));
        }

        let backup = self.backup_path(slot, backup_num);
        let target = self.save_path(slot);

        if !backup.exists() {
            return Err(SaveError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Backup not found",
            )));
        }

        std::fs::copy(&backup, &target)?;
        self.refresh_cache()?;

        Ok(())
    }

    /// Get available backup slots
    pub fn get_backups(&self, slot: u32) -> Vec<u32> {
        let mut backups = Vec::new();

        for i in 0..self.config.max_backups {
            if self.backup_path(slot, i).exists() {
                backups.push(i);
            }
        }

        backups
    }

    /// Verify save file password
    pub fn verify_password(&self, slot: u32, password: &str) -> bool {
        let path = self.save_path(slot);

        if !path.exists() {
            return false;
        }

        match std::fs::read(&path) {
            Ok(data) => verify_password(&data, password),
            Err(_) => false,
        }
    }

    /// Export save to portable format
    pub fn export(
        &self,
        slot: u32,
        destination: &Path,
        password: Option<&str>,
    ) -> Result<(), SaveError> {
        let save = self.load(slot, password)?;
        let json = save
            .to_json()
            .map_err(|e| SaveError::Serialization(e.to_string()))?;

        std::fs::write(destination, json)?;
        Ok(())
    }

    /// Import save from portable format
    pub fn import(
        &mut self,
        slot: u32,
        source: &Path,
        password: Option<&str>,
    ) -> Result<(), SaveError> {
        let json = std::fs::read_to_string(source)?;
        let save =
            GameSave::from_json(&json).map_err(|e| SaveError::Serialization(e.to_string()))?;

        self.save(slot, &save, password)
    }

    /// Get next available save slot
    pub fn next_available_slot(&self) -> Option<u32> {
        (1..=self.config.max_slots).find(|&slot| !self.cache.contains_key(&slot))
    }

    /// Get save directory size
    pub fn total_size(&self) -> Result<u64, SaveError> {
        let mut total = 0u64;

        for entry in std::fs::read_dir(&self.config.save_dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_file() {
                total += metadata.len();
            }
        }

        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_save(slot: u32) -> GameSave {
        GameSave::new(slot, "Test Player", "test_map")
    }

    fn create_temp_manager() -> SaveManager {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_dir =
            std::env::temp_dir().join(format!("dde_test_{}_{}", std::process::id(), counter));
        let _ = std::fs::remove_dir_all(&temp_dir);

        SaveManager::new(SaveConfig {
            save_dir: temp_dir,
            max_slots: 10,
            default_password: None,
            auto_backup: true,
            max_backups: 3,
        })
        .unwrap()
    }

    #[test]
    fn test_save_and_load() {
        let mut manager = create_temp_manager();
        let save = create_test_save(1);

        // Save without encryption
        manager.save(1, &save, None).unwrap();

        // Load
        let loaded = manager.load(1, None).unwrap();
        assert_eq!(loaded.player_name, "Test Player");
        assert_eq!(loaded.current_map, "test_map");
    }

    #[test]
    fn test_encrypted_save() {
        let mut manager = create_temp_manager();
        let save = create_test_save(1);
        let password = "secure_password";

        // Save with encryption
        manager.save(1, &save, Some(password)).unwrap();

        // Load without password should fail
        assert!(manager.load(1, None).is_err());

        // Load with correct password
        let loaded = manager.load(1, Some(password)).unwrap();
        assert_eq!(loaded.player_name, "Test Player");

        // Verify password
        assert!(manager.verify_password(1, password));
        assert!(!manager.verify_password(1, "wrong_password"));
    }

    #[test]
    fn test_metadata_cache() {
        let mut manager = create_temp_manager();
        let save = create_test_save(1);

        manager.save(1, &save, None).unwrap();

        let metadata = manager.get_metadata(1).unwrap();
        assert_eq!(metadata.slot, 1);
        assert_eq!(metadata.player_name, "Test Player");
    }

    #[test]
    fn test_delete_save() {
        let mut manager = create_temp_manager();
        let save = create_test_save(1);

        manager.save(1, &save, None).unwrap();
        assert!(manager.exists(1));

        manager.delete(1).unwrap();
        assert!(!manager.exists(1));
    }

    #[test]
    fn test_backup_creation() {
        let mut manager = create_temp_manager();

        // Save initial version
        let save1 = create_test_save(1);
        manager.save(1, &save1, None).unwrap();

        // Save again (should create backup)
        let save2 = GameSave::new(1, "Updated Player", "updated_map");
        manager.save(1, &save2, None).unwrap();

        // Check backup exists
        let backups = manager.get_backups(1);
        assert!(!backups.is_empty());
    }

    #[test]
    fn test_next_available_slot() {
        let mut manager = create_temp_manager();

        assert_eq!(manager.next_available_slot(), Some(1));

        let save = create_test_save(1);
        manager.save(1, &save, None).unwrap();

        assert_eq!(manager.next_available_slot(), Some(2));
    }

    #[test]
    fn test_export_import() {
        let mut manager = create_temp_manager();
        let save = create_test_save(1);

        manager.save(1, &save, None).unwrap();

        // Export
        let export_path = manager.config.save_dir.join("export.json");
        manager.export(1, &export_path, None).unwrap();

        // Import to new slot
        manager.import(2, &export_path, None).unwrap();

        let loaded = manager.load(2, None).unwrap();
        assert_eq!(loaded.player_name, "Test Player");
    }
}
