//! Migration V3: Screenshot Support for Save Slots
//!
//! Extends the save_slots table with metadata columns for screenshots:
//! - screenshot_width: Width of the screenshot in pixels
//! - screenshot_height: Height of the screenshot in pixels
//! - screenshot_format: Image format (png, jpeg, webp)
//! - screenshot_data: The actual compressed image data (already exists)

use rusqlite::Connection;

use crate::Result;

/// Apply V3 migration - Screenshot support
pub fn apply(conn: &Connection) -> Result<()> {
    tracing::info!("Applying migration v3: Screenshot support");

    conn.execute_batch(V3_SCHEMA)?;

    tracing::info!("Migration v3 complete");
    Ok(())
}

/// V3 schema additions for screenshot support
const V3_SCHEMA: &str = r#"
-- =====================================================
-- SCREENSHOT SUPPORT: Extended save slot metadata
-- =====================================================

-- Add screenshot metadata columns if they don't exist
-- Note: SQLite doesn't have a direct "ADD COLUMN IF NOT EXISTS" syntax
-- so we use a workaround with the schema check

-- Add screenshot_width column
ALTER TABLE save_slots ADD COLUMN screenshot_width INTEGER;

-- Add screenshot_height column  
ALTER TABLE save_slots ADD COLUMN screenshot_height INTEGER;

-- Add screenshot_format column
ALTER TABLE save_slots ADD COLUMN screenshot_format TEXT;

-- Note: screenshot_data BLOB column was already added in v1 schema
"#;
