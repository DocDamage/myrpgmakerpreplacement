//! Migration v4: Add cutscene/timeline tables
//!
//! Creates tables for storing cutscene timeline data.

use rusqlite::Connection;
use crate::Result;

/// Apply v4 migration
pub fn apply(conn: &Connection) -> Result<()> {
    tracing::info!("Applying migration v4: Cutscene timeline tables");

    conn.execute_batch(V4_CUTSCENE_SCHEMA)?;
    
    Ok(())
}

/// V4 schema - Cutscene timeline tables
const V4_CUTSCENE_SCHEMA: &str = r#"
-- =====================================================
-- CUTSCENES
-- =====================================================
CREATE TABLE IF NOT EXISTS cutscenes (
    cutscene_id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    duration REAL NOT NULL DEFAULT 60.0,
    timeline_json TEXT NOT NULL DEFAULT '{}',
    tags_json TEXT DEFAULT '[]',
    created_at INTEGER NOT NULL,
    modified_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cutscenes_name ON cutscenes(name);
CREATE INDEX IF NOT EXISTS idx_cutscenes_modified ON cutscenes(modified_at);

-- =====================================================
-- CUTSCENE EVENTS (runtime events generated from timeline)
-- =====================================================
CREATE TABLE IF NOT EXISTS cutscene_events (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    cutscene_id INTEGER NOT NULL REFERENCES cutscenes(cutscene_id) ON DELETE CASCADE,
    time REAL NOT NULL,
    event_type TEXT NOT NULL,
    data_json TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_cutscene_events_cutscene ON cutscene_events(cutscene_id, time);

-- =====================================================
-- CUTSCENE TRIGGERS (when to play cutscenes)
-- =====================================================
CREATE TABLE IF NOT EXISTS cutscene_triggers (
    trigger_id INTEGER PRIMARY KEY AUTOINCREMENT,
    cutscene_id INTEGER NOT NULL REFERENCES cutscenes(cutscene_id) ON DELETE CASCADE,
    trigger_type TEXT NOT NULL DEFAULT 'manual',
    condition_json TEXT,
    map_id INTEGER,
    entity_id INTEGER,
    one_shot BOOLEAN NOT NULL DEFAULT 1,
    enabled BOOLEAN NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_cutscene_triggers_map ON cutscene_triggers(map_id, enabled);
CREATE INDEX IF NOT EXISTS idx_cutscene_triggers_entity ON cutscene_triggers(entity_id, enabled);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_v4_migration() {
        let conn = Connection::open_in_memory().unwrap();
        
        // Apply migration
        apply(&conn).unwrap();
        
        // Verify tables exist
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='cutscenes'",
            [],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(count, 1);
        
        // Verify we can insert
        conn.execute(
            "INSERT INTO cutscenes (uuid, name, duration, timeline_json, created_at, modified_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            ("test-uuid", "Test Cutscene", 30.0, "{}", 0i64, 0i64),
        ).unwrap();
        
        // Verify insert worked
        let name: String = conn.query_row(
            "SELECT name FROM cutscenes WHERE uuid = ?1",
            ["test-uuid"],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(name, "Test Cutscene");
    }
}
