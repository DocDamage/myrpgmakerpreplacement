//! DocDamage Engine - Database Layer
//!
//! SQLite-based persistence for world state, entities, and all game data.
//! The entire game project is a SQLite database file.

use std::path::Path;

use rusqlite::{Connection, Transaction};

pub mod migrations;
pub mod models;
pub mod queries;
pub mod screenshot;
pub mod sync;

pub use models::*;
pub use queries::{
    DialogueQueries, EntityQueries, StatusEffectQueries, StatusEffectTemplateModel, TileQueries
};
pub use screenshot::*;

/// Database error types
#[derive(thiserror::Error, Debug)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Entity not found: {0}")]
    NotFound(u64),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, DbError>;

/// Database wrapper with connection management
pub struct Database {
    conn: Connection,
    project_path: String,
}

impl Database {
    /// Create/open a project database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let conn = Connection::open(path)?;

        // Configure pragmas for performance
        conn.execute_batch(PRAGMAS)?;

        // Run migrations
        migrations::run_migrations(&conn)?;

        Ok(Self {
            conn,
            project_path: path.to_string_lossy().to_string(),
        })
    }

    /// Create a new project database
    pub fn create_new<P: AsRef<Path>>(path: P, project_name: &str) -> Result<Self> {
        let path = path.as_ref();

        // Remove existing file if present
        if path.exists() {
            std::fs::remove_file(path)?;
        }

        let conn = Connection::open(path)?;

        // Configure pragmas
        conn.execute_batch(PRAGMAS)?;

        // Run migrations
        migrations::run_migrations(&conn)?;

        // Initialize project metadata
        let project_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp_millis();
        let seed = rand::random::<u64>();
        let _ = seed;

        conn.execute(
            "INSERT INTO project_meta (project_id, project_name, schema_version, world_seed, tick_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (&project_id, project_name, migrations::CURRENT_SCHEMA_VERSION, seed as i64, 0i64, now, now),
        )?;

        tracing::info!("Created new project '{}' at {:?}", project_name, path);

        Ok(Self {
            conn,
            project_path: path.to_string_lossy().to_string(),
        })
    }

    /// Get a reference to the connection
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Start a transaction
    pub fn transaction(&mut self) -> Result<Transaction<'_>> {
        Ok(self.conn.transaction()?)
    }

    /// Execute integrity check
    pub fn integrity_check(&self) -> Result<bool> {
        let valid: String = self
            .conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        Ok(valid == "ok")
    }

    /// Get project metadata
    pub fn get_project_meta(&self) -> Result<ProjectMeta> {
        let meta = self.conn.query_row(
            "SELECT project_id, project_name, schema_version, world_seed, tick_count, created_at, updated_at FROM project_meta LIMIT 1",
            [],
            |row| {
                Ok(ProjectMeta {
                    project_id: row.get(0)?,
                    project_name: row.get(1)?,
                    schema_version: row.get(2)?,
                    world_seed: row.get::<_, i64>(3)? as u64,
                    tick_count: row.get::<_, i64>(4)? as u64,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            }
        )?;
        Ok(meta)
    }

    /// Save the database to a slot
    ///
    /// Creates a backup copy of the current database as a save slot.
    /// Slots are stored as `<project_path>.slot<N>.dde` files.
    pub fn save_to_slot(&self, slot: u32) -> Result<()> {
        if slot == 0 || slot > 99 {
            return Err(DbError::InvalidData(
                "Save slot must be between 1 and 99".to_string(),
            ));
        }

        let slot_path = format!("{}.slot{:02}.dde", self.project_path, slot);

        // Checkpoint WAL to ensure all data is in the main database file
        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;

        // Copy the database file
        std::fs::copy(&self.project_path, &slot_path)?;

        // Update slot metadata in the current database
        let timestamp = chrono::Utc::now().timestamp();
        self.conn.execute(
            "INSERT OR REPLACE INTO save_slots (slot_number, saved_at, play_time_ms) 
             VALUES (?1, ?2, COALESCE((SELECT play_time_ms FROM save_slots WHERE slot_number = ?1), 0))",
            (slot as i64, timestamp),
        )?;

        tracing::info!("Saved to slot {}: {}", slot, slot_path);
        Ok(())
    }

    /// Save the database to a slot with screenshot
    ///
    /// Creates a backup copy of the current database and stores the screenshot.
    ///
    /// # Arguments
    /// * `slot` - Slot number (1-99)
    /// * `play_time_ms` - Total play time in milliseconds
    /// * `screenshot` - Optional screenshot data to store
    pub fn save_to_slot_with_screenshot(
        &self,
        slot: u32,
        play_time_ms: u64,
        screenshot: Option<&ScreenshotData>,
    ) -> Result<()> {
        if slot == 0 || slot > 99 {
            return Err(DbError::InvalidData(
                "Save slot must be between 1 and 99".to_string(),
            ));
        }

        let slot_path = format!("{}.slot{:02}.dde", self.project_path, slot);

        // Checkpoint WAL to ensure all data is in the main database file
        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;

        // Copy the database file
        std::fs::copy(&self.project_path, &slot_path)?;

        // Update slot metadata with play time and screenshot
        let timestamp = chrono::Utc::now().timestamp();

        if let Some(screenshot_data) = screenshot {
            let format_str = match screenshot_data.format {
                ScreenshotFormat::Png => "png",
                ScreenshotFormat::Jpeg => "jpeg",
                ScreenshotFormat::Webp => "webp",
            };

            self.conn.execute(
                "INSERT OR REPLACE INTO save_slots (slot_number, saved_at, play_time_ms, screenshot_width, screenshot_height, screenshot_format, screenshot_data) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                (
                    slot as i64,
                    timestamp,
                    play_time_ms as i64,
                    screenshot_data.width as i64,
                    screenshot_data.height as i64,
                    format_str,
                    &screenshot_data.data,
                ),
            )?;
        } else {
            self.conn.execute(
                "INSERT OR REPLACE INTO save_slots (slot_number, saved_at, play_time_ms, screenshot_data) 
                 VALUES (?1, ?2, ?3, NULL)",
                (slot as i64, timestamp, play_time_ms as i64),
            )?;
        }

        tracing::info!("Saved to slot {} with screenshot: {}", slot, slot_path);
        Ok(())
    }

    /// Load from a save slot
    ///
    /// Returns the path to the slot file if it exists.
    pub fn load_from_slot(&self, slot: u32) -> Result<std::path::PathBuf> {
        if slot == 0 || slot > 99 {
            return Err(DbError::InvalidData(
                "Save slot must be between 1 and 99".to_string(),
            ));
        }

        let slot_path =
            std::path::PathBuf::from(format!("{}.slot{:02}.dde", self.project_path, slot));

        if !slot_path.exists() {
            return Err(DbError::NotFound(slot as u64));
        }

        Ok(slot_path)
    }

    /// Check if a save slot exists
    pub fn slot_exists(&self, slot: u32) -> bool {
        if slot == 0 || slot > 99 {
            return false;
        }
        let slot_path = format!("{}.slot{:02}.dde", self.project_path, slot);
        std::path::Path::new(&slot_path).exists()
    }

    /// Delete a save slot
    pub fn delete_slot(&self, slot: u32) -> Result<()> {
        if slot == 0 || slot > 99 {
            return Err(DbError::InvalidData(
                "Save slot must be between 1 and 99".to_string(),
            ));
        }

        let slot_path = format!("{}.slot{:02}.dde", self.project_path, slot);
        if std::path::Path::new(&slot_path).exists() {
            std::fs::remove_file(&slot_path)?;
        }

        // Remove metadata
        self.conn.execute(
            "DELETE FROM save_slots WHERE slot_number = ?1",
            (slot as i64,),
        )?;

        tracing::info!("Deleted save slot {}", slot);
        Ok(())
    }

    /// List all save slots with metadata
    pub fn list_slots(&self) -> Result<Vec<SaveSlotInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT slot_number, saved_at, play_time_ms, screenshot_data IS NOT NULL as has_screenshot
             FROM save_slots 
             ORDER BY slot_number"
        )?;

        let slots = stmt.query_map([], |row| {
            Ok(SaveSlotInfo {
                slot_number: row.get::<_, i64>(0)? as u32,
                saved_at: row.get(1)?,
                play_time_ms: row.get::<_, i64>(2)? as u64,
                exists: true,
                has_screenshot: row.get::<_, i64>(3)? != 0,
            })
        })?;

        slots
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(DbError::from)
    }

    /// Get detailed information about a specific save slot
    ///
    /// Returns slot metadata including whether it has a screenshot.
    /// If the slot doesn't exist, returns `Ok(None)`.
    pub fn get_slot_info(&self, slot: u32) -> Result<Option<SaveSlotInfo>> {
        if slot == 0 || slot > 99 {
            return Err(DbError::InvalidData(
                "Save slot must be between 1 and 99".to_string(),
            ));
        }

        let slot_file_exists = self.slot_exists(slot);

        let result = self.conn.query_row(
            "SELECT slot_number, saved_at, play_time_ms, screenshot_data IS NOT NULL as has_screenshot
             FROM save_slots 
             WHERE slot_number = ?1",
            (slot as i64,),
            |row| {
                Ok(SaveSlotInfo {
                    slot_number: row.get::<_, i64>(0)? as u32,
                    saved_at: row.get(1)?,
                    play_time_ms: row.get::<_, i64>(2)? as u64,
                    exists: slot_file_exists,
                    has_screenshot: row.get::<_, i64>(3)? != 0,
                })
            }
        );

        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                if slot_file_exists {
                    // Slot file exists but no metadata - create basic info
                    Ok(Some(SaveSlotInfo {
                        slot_number: slot,
                        saved_at: 0,
                        play_time_ms: 0,
                        exists: true,
                        has_screenshot: false,
                    }))
                } else {
                    Ok(None)
                }
            }
            Err(e) => Err(DbError::from(e)),
        }
    }

    /// Save a screenshot to a slot
    ///
    /// This updates the screenshot data for an existing slot.
    /// Does not create the slot if it doesn't exist.
    ///
    /// # Arguments
    /// * `slot` - Slot number (1-99)
    /// * `screenshot` - Screenshot data to save
    pub fn save_screenshot_to_slot(&self, slot: u32, screenshot: &ScreenshotData) -> Result<()> {
        if slot == 0 || slot > 99 {
            return Err(DbError::InvalidData(
                "Save slot must be between 1 and 99".to_string(),
            ));
        }

        let format_str = match screenshot.format {
            ScreenshotFormat::Png => "png",
            ScreenshotFormat::Jpeg => "jpeg",
            ScreenshotFormat::Webp => "webp",
        };

        let rows_affected = self.conn.execute(
            "UPDATE save_slots 
             SET screenshot_width = ?1, screenshot_height = ?2, screenshot_format = ?3, screenshot_data = ?4
             WHERE slot_number = ?5",
            (
                screenshot.width as i64,
                screenshot.height as i64,
                format_str,
                &screenshot.data,
                slot as i64,
            ),
        )?;

        if rows_affected == 0 {
            return Err(DbError::InvalidData(format!(
                "Save slot {} does not exist, cannot save screenshot",
                slot
            )));
        }

        tracing::debug!("Saved screenshot to slot {}", slot);
        Ok(())
    }

    /// Load a screenshot from a slot
    ///
    /// # Arguments
    /// * `slot` - Slot number (1-99)
    ///
    /// # Returns
    /// * `Ok(Some(ScreenshotData))` - Screenshot found and loaded
    /// * `Ok(None)` - Slot exists but has no screenshot
    /// * `Err` - Database error or invalid slot
    pub fn load_screenshot_from_slot(&self, slot: u32) -> Result<Option<ScreenshotData>> {
        if slot == 0 || slot > 99 {
            return Err(DbError::InvalidData(
                "Save slot must be between 1 and 99".to_string(),
            ));
        }

        let result = self.conn.query_row(
            "SELECT screenshot_width, screenshot_height, screenshot_format, screenshot_data
             FROM save_slots 
             WHERE slot_number = ?1 AND screenshot_data IS NOT NULL",
            (slot as i64,),
            |row| {
                let format_str: String = row.get(2)?;
                let format = match format_str.as_str() {
                    "png" => ScreenshotFormat::Png,
                    "jpeg" | "jpg" => ScreenshotFormat::Jpeg,
                    "webp" => ScreenshotFormat::Webp,
                    _ => ScreenshotFormat::Webp, // Default fallback
                };

                Ok(ScreenshotData {
                    width: row.get::<_, i64>(0)? as u32,
                    height: row.get::<_, i64>(1)? as u32,
                    format,
                    data: row.get(3)?,
                })
            },
        );

        match result {
            Ok(screenshot) => Ok(Some(screenshot)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DbError::from(e)),
        }
    }

    /// Get the project path
    pub fn path(&self) -> &str {
        &self.project_path
    }

    /// Clone the database connection
    /// 
    /// Creates a new connection to the same database file.
    /// This is useful when multiple components need independent connections.
    pub fn clone_connection(&self) -> Result<Self> {
        Self::open(&self.project_path)
    }

    // =====================================================
    // Status Effect Template Methods
    // =====================================================

    /// Get all status effect templates
    pub fn get_status_effect_templates(&self) -> Result<Vec<StatusEffectTemplateModel>> {
        StatusEffectQueries::get_all_templates(self)
    }

    /// Get a single status effect template by ID
    pub fn get_status_effect_template(&self, template_id: &str) -> Result<Option<StatusEffectTemplateModel>> {
        StatusEffectQueries::get_template(self, template_id)
    }

    /// Save a status effect template
    pub fn save_status_effect_template(&mut self, template: &StatusEffectTemplateModel) -> Result<()> {
        StatusEffectQueries::save_template(self, template)
    }

    /// Delete a status effect template
    pub fn delete_status_effect_template(&mut self, template_id: &str) -> Result<bool> {
        StatusEffectQueries::delete_template(self, template_id)
    }

    /// Get templates by status type
    pub fn get_status_effect_templates_by_type(&self, status_type: &str) -> Result<Vec<StatusEffectTemplateModel>> {
        StatusEffectQueries::get_templates_by_type(self, status_type)
    }

    // =====================================================
    // Classification Rules Methods
    // =====================================================

    /// Get all classification rules ordered by priority
    pub fn get_classification_rules(&self) -> Result<Vec<ClassificationRuleModel>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, file_pattern, asset_type, auto_tags_json, priority, 
                    enabled, exact_width, exact_height, min_width, max_width, 
                    min_height, max_height, confidence
             FROM classification_rules 
             ORDER BY priority DESC"
        )?;

        let rules = stmt.query_map([], |row| {
            let exact_w: Option<i64> = row.get(7)?;
            let exact_h: Option<i64> = row.get(8)?;
            let exact_dimensions = match (exact_w, exact_h) {
                (Some(w), Some(h)) => Some((w as u32, h as u32)),
                _ => None,
            };

            Ok(ClassificationRuleModel {
                id: row.get(0)?,
                name: row.get(1)?,
                file_pattern: row.get(2)?,
                asset_type: row.get(3)?,
                auto_tags_json: row.get(4)?,
                priority: row.get(5)?,
                enabled: row.get(6)?,
                exact_dimensions,
                min_width: row.get::<_, Option<i64>>(9)?.map(|v| v as u32),
                max_width: row.get::<_, Option<i64>>(10)?.map(|v| v as u32),
                min_height: row.get::<_, Option<i64>>(11)?.map(|v| v as u32),
                max_height: row.get::<_, Option<i64>>(12)?.map(|v| v as u32),
                confidence: row.get(13)?,
            })
        })?;

        rules.collect::<std::result::Result<Vec<_>, rusqlite::Error>>()
            .map_err(DbError::from)
    }

    /// Get a single classification rule by ID
    pub fn get_classification_rule(&self, rule_id: &str) -> Result<Option<ClassificationRuleModel>> {
        let result = self.conn.query_row(
            "SELECT id, name, file_pattern, asset_type, auto_tags_json, priority, 
                    enabled, exact_width, exact_height, min_width, max_width, 
                    min_height, max_height, confidence
             FROM classification_rules 
             WHERE id = ?1",
            [rule_id],
            |row| {
                let exact_w: Option<i64> = row.get(7)?;
                let exact_h: Option<i64> = row.get(8)?;
                let exact_dimensions = match (exact_w, exact_h) {
                    (Some(w), Some(h)) => Some((w as u32, h as u32)),
                    _ => None,
                };

                Ok(ClassificationRuleModel {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    file_pattern: row.get(2)?,
                    asset_type: row.get(3)?,
                    auto_tags_json: row.get(4)?,
                    priority: row.get(5)?,
                    enabled: row.get(6)?,
                    exact_dimensions,
                    min_width: row.get::<_, Option<i64>>(9)?.map(|v| v as u32),
                    max_width: row.get::<_, Option<i64>>(10)?.map(|v| v as u32),
                    min_height: row.get::<_, Option<i64>>(11)?.map(|v| v as u32),
                    max_height: row.get::<_, Option<i64>>(12)?.map(|v| v as u32),
                    confidence: row.get(13)?,
                })
            },
        );

        match result {
            Ok(rule) => Ok(Some(rule)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Save a classification rule (insert or update)
    pub fn save_classification_rule(&mut self, rule: &ClassificationRuleModel) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();
        
        self.conn.execute(
            "INSERT INTO classification_rules 
             (id, name, file_pattern, asset_type, auto_tags_json, priority, 
              enabled, exact_width, exact_height, min_width, max_width, 
              min_height, max_height, confidence, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(id) DO UPDATE SET
             name = excluded.name,
             file_pattern = excluded.file_pattern,
             asset_type = excluded.asset_type,
             auto_tags_json = excluded.auto_tags_json,
             priority = excluded.priority,
             enabled = excluded.enabled,
             exact_width = excluded.exact_width,
             exact_height = excluded.exact_height,
             min_width = excluded.min_width,
             max_width = excluded.max_width,
             min_height = excluded.min_height,
             max_height = excluded.max_height,
             confidence = excluded.confidence,
             updated_at = excluded.updated_at",
            (
                &rule.id,
                &rule.name,
                &rule.file_pattern,
                &rule.asset_type,
                &rule.auto_tags_json,
                rule.priority,
                rule.enabled,
                rule.exact_dimensions.map(|(w, _)| w as i64),
                rule.exact_dimensions.map(|(_, h)| h as i64),
                rule.min_width.map(|v| v as i64),
                rule.max_width.map(|v| v as i64),
                rule.min_height.map(|v| v as i64),
                rule.max_height.map(|v| v as i64),
                rule.confidence,
                now,
            ),
        )?;
        
        Ok(())
    }

    /// Delete a classification rule
    pub fn delete_classification_rule(&mut self, rule_id: &str) -> Result<bool> {
        let rows = self.conn.execute(
            "DELETE FROM classification_rules WHERE id = ?1",
            [rule_id],
        )?;
        Ok(rows > 0)
    }

    /// Get classification statistics for all rules
    pub fn get_classification_stats(&self) -> Result<Vec<ClassificationStatsModel>> {
        let mut stmt = self.conn.prepare(
            "SELECT rule_id, times_matched, times_applied, times_overridden, 
                    avg_confidence, last_matched_at
             FROM classification_stats"
        )?;

        let stats = stmt.query_map([], |row| {
            Ok(ClassificationStatsModel {
                rule_id: row.get(0)?,
                times_matched: row.get(1)?,
                times_applied: row.get(2)?,
                times_overridden: row.get(3)?,
                avg_confidence: row.get(4)?,
                last_matched_at: row.get(5)?,
            })
        })?;

        stats.collect::<std::result::Result<Vec<_>, rusqlite::Error>>()
            .map_err(DbError::from)
    }

    /// Add a file to the classification queue
    pub fn queue_file_for_classification(&mut self, file_path: &str, file_name: &str, file_size: Option<i64>, dimensions: Option<(u32, u32)>) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();
        let dims_str = dimensions.map(|(w, h)| format!("{}x{}", w, h));
        
        self.conn.execute(
            "INSERT INTO classification_queue (file_path, file_name, file_size, detected_dimensions, queued_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending')
             ON CONFLICT(file_path) DO UPDATE SET
             queued_at = excluded.queued_at,
             status = 'pending',
             retry_count = 0",
            (file_path, file_name, file_size, dims_str, now),
        )?;
        
        Ok(())
    }

    /// Get pending files from classification queue
    pub fn get_pending_classification_queue(&self, limit: i64) -> Result<Vec<ClassificationQueueItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT queue_id, file_path, file_name, file_size, detected_dimensions, 
                    queued_at, retry_count
             FROM classification_queue 
             WHERE status = 'pending'
             ORDER BY queued_at
             LIMIT ?1"
        )?;

        let items = stmt.query_map([limit], |row| {
            let dims_str: Option<String> = row.get(4)?;
            let dimensions = dims_str.and_then(|s| {
                let parts: Vec<&str> = s.split('x').collect();
                if parts.len() == 2 {
                    Some((parts[0].parse().unwrap_or(0), parts[1].parse().unwrap_or(0)))
                } else {
                    None
                }
            });

            Ok(ClassificationQueueItem {
                queue_id: row.get(0)?,
                file_path: row.get(1)?,
                file_name: row.get(2)?,
                file_size: row.get(3)?,
                dimensions,
                queued_at: row.get(5)?,
                retry_count: row.get(6)?,
            })
        })?;

        items.collect::<std::result::Result<Vec<_>, rusqlite::Error>>()
            .map_err(DbError::from)
    }

    /// Update classification queue item status
    pub fn update_classification_queue_status(&mut self, queue_id: i64, status: &str, error_message: Option<&str>) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();
        
        self.conn.execute(
            "UPDATE classification_queue 
             SET status = ?1, processed_at = ?2, error_message = ?3
             WHERE queue_id = ?4",
            (status, now, error_message, queue_id),
        )?;
        
        Ok(())
    }
}

/// SQLite pragmas for performance
const PRAGMAS: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = -64000;
PRAGMA mmap_size = 268435456;
PRAGMA temp_store = MEMORY;
"#;

/// Project metadata
#[derive(Debug, Clone)]
pub struct ProjectMeta {
    pub project_id: String,
    pub project_name: String,
    pub schema_version: i32,
    pub world_seed: u64,
    pub tick_count: u64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_db_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("dde_test_{}.db", uuid::Uuid::new_v4()));
        path
    }

    #[test]
    fn test_create_new_project() {
        let path = test_db_path();
        let db = Database::create_new(&path, "Test Project").unwrap();

        let meta = db.get_project_meta().unwrap();
        assert_eq!(meta.project_name, "Test Project");
        assert!(db.integrity_check().unwrap());

        // Cleanup
        drop(db);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_open_existing() {
        let path = test_db_path();

        // Create
        {
            let db = Database::create_new(&path, "Existing Project").unwrap();
            drop(db);
        }

        // Open
        {
            let db = Database::open(&path).unwrap();
            let meta = db.get_project_meta().unwrap();
            assert_eq!(meta.project_name, "Existing Project");
        }

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_save_to_slot_with_screenshot() {
        let path = test_db_path();
        let db = Database::create_new(&path, "Test Project").unwrap();

        // First create a basic slot (saves the db file)
        db.save_to_slot(1).unwrap();

        // Create and save screenshot
        let manager = ScreenshotManager::new();
        let screenshot = manager.generate_placeholder(1);

        db.save_screenshot_to_slot(1, &screenshot).unwrap();

        // Verify we can load it back
        let loaded = db.load_screenshot_from_slot(1).unwrap();
        assert!(loaded.is_some());

        let loaded_data = loaded.unwrap();
        assert_eq!(loaded_data.width, screenshot.width);
        assert_eq!(loaded_data.height, screenshot.height);
        assert!(matches!(loaded_data.format, ScreenshotFormat::Webp));

        // Cleanup
        drop(db);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}.slot01.dde", path.display()));
    }

    #[test]
    fn test_get_slot_info() {
        let path = test_db_path();
        let db = Database::create_new(&path, "Test Project").unwrap();

        // Save a slot with screenshot
        let manager = ScreenshotManager::new();
        let screenshot = manager.generate_placeholder(1);
        db.save_to_slot_with_screenshot(1, 12345678, Some(&screenshot))
            .unwrap();

        // Get slot info
        let info = db.get_slot_info(1).unwrap();
        assert!(info.is_some());

        let slot_info = info.unwrap();
        assert_eq!(slot_info.slot_number, 1);
        assert_eq!(slot_info.play_time_ms, 12345678);
        assert!(slot_info.exists);
        assert!(slot_info.has_screenshot);

        // Non-existent slot
        let no_info = db.get_slot_info(99).unwrap();
        assert!(no_info.is_none());

        // Cleanup
        drop(db);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}.slot01.dde", path.display()));
    }

    #[test]
    fn test_list_slots() {
        let path = test_db_path();
        let db = Database::create_new(&path, "Test Project").unwrap();

        // Save multiple slots
        db.save_to_slot_with_screenshot(1, 1000, None).unwrap();

        let manager = ScreenshotManager::new();
        let screenshot = manager.generate_placeholder(2);
        db.save_to_slot_with_screenshot(2, 2000, Some(&screenshot))
            .unwrap();

        db.save_to_slot_with_screenshot(3, 3000, None).unwrap();

        // List slots
        let slots = db.list_slots().unwrap();
        assert_eq!(slots.len(), 3);

        // Check each slot
        assert_eq!(slots[0].slot_number, 1);
        assert!(!slots[0].has_screenshot);

        assert_eq!(slots[1].slot_number, 2);
        assert!(slots[1].has_screenshot);

        assert_eq!(slots[2].slot_number, 3);
        assert!(!slots[2].has_screenshot);

        // Cleanup
        drop(db);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}.slot01.dde", path.display()));
        let _ = std::fs::remove_file(format!("{}.slot02.dde", path.display()));
        let _ = std::fs::remove_file(format!("{}.slot03.dde", path.display()));
    }
}
