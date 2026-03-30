//! Migration v6: Status Effect Templates
//!
//! Adds support for storing status effect templates in the database.

use rusqlite::Connection;
use crate::Result;

/// Apply v6 migration
pub fn apply(conn: &Connection) -> Result<()> {
    tracing::info!("Applying migration v6: Status Effect Templates");

    conn.execute_batch(V6_SCHEMA)?;
    Ok(())
}

const V6_SCHEMA: &str = r#"
-- =====================================================
-- STATUS EFFECT TEMPLATES
-- =====================================================
CREATE TABLE IF NOT EXISTS status_effect_templates (
    template_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    status_type TEXT NOT NULL,
    duration INTEGER NOT NULL DEFAULT 3,
    potency INTEGER NOT NULL DEFAULT 10,
    tick_interval INTEGER NOT NULL DEFAULT 1,
    stack_behavior TEXT NOT NULL DEFAULT 'Replace',
    resistance_category TEXT NOT NULL DEFAULT 'Magical',
    visual_effect TEXT,
    icon_path TEXT,
    dispellable BOOLEAN NOT NULL DEFAULT 1,
    custom_description TEXT,
    tags TEXT DEFAULT '[]',
    created_at INTEGER NOT NULL,
    modified_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_status_templates_type ON status_effect_templates(status_type);
CREATE INDEX IF NOT EXISTS idx_status_templates_category ON status_effect_templates(resistance_category);
CREATE INDEX IF NOT EXISTS idx_status_templates_modified ON status_effect_templates(modified_at);
"#;
