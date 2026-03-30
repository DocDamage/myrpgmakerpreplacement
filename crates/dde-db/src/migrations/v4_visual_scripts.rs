//! Migration V4: Visual Scripting Support
//!
//! Adds support for storing visual script (blueprint) graphs in the database.

use rusqlite::Connection;

use crate::Result;

/// Apply V4 migration - Visual scripting support
pub fn apply(conn: &Connection) -> Result<()> {
    tracing::info!("Applying migration v4: Visual scripting support");

    conn.execute_batch(V4_SCHEMA)?;

    tracing::info!("Migration v4 complete");
    Ok(())
}

/// V4 schema for visual scripting support
const V4_SCHEMA: &str = r#"
-- =====================================================
-- VISUAL SCRIPTS: Node-based visual scripting graphs
-- =====================================================

CREATE TABLE IF NOT EXISTS visual_scripts (
    script_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    graph_json TEXT NOT NULL, -- Serialized NodeGraph
    created_at INTEGER NOT NULL,
    modified_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_visual_scripts_name ON visual_scripts(name);
CREATE INDEX IF NOT EXISTS idx_visual_scripts_modified ON visual_scripts(modified_at);

-- =====================================================
-- VISUAL SCRIPT TRIGGERS: Link scripts to game events
-- =====================================================

CREATE TABLE IF NOT EXISTS visual_script_triggers (
    trigger_id INTEGER PRIMARY KEY AUTOINCREMENT,
    script_id INTEGER NOT NULL REFERENCES visual_scripts(script_id) ON DELETE CASCADE,
    trigger_type TEXT NOT NULL, -- 'on_interact', 'on_enter_region', 'on_item_use', etc.
    target_id INTEGER, -- entity_id, region_id, item_id, etc.
    target_type TEXT, -- 'entity', 'region', 'item', 'tile', etc.
    enabled BOOLEAN NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 0,
    one_shot BOOLEAN NOT NULL DEFAULT 0,
    cooldown_ms INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_vst_script ON visual_script_triggers(script_id);
CREATE INDEX IF NOT EXISTS idx_vst_type ON visual_script_triggers(trigger_type, enabled);
CREATE INDEX IF NOT EXISTS idx_vst_target ON visual_script_triggers(target_type, target_id, enabled);
"#;
