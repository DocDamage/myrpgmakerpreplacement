//! Migration V7: Classification Rules
//!
//! Adds support for storing and managing asset classification rules:
//! - Pattern-based matching rules
//! - Dimension constraints
//! - Auto-tagging configuration
//! - Priority ordering

use rusqlite::Connection;

use crate::Result;

/// Apply V7 migration - Classification Rules schema
pub fn apply(conn: &Connection) -> Result<()> {
    tracing::info!("Applying migration v7: Classification Rules schema");

    conn.execute_batch(V7_SCHEMA)?;

    // Insert default rules if table is empty
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM classification_rules",
        [],
        |row| row.get(0),
    )?;

    if count == 0 {
        insert_default_rules(conn)?;
    }

    tracing::info!("Migration v7 complete");
    Ok(())
}

/// V7 schema additions for Classification Rules
const V7_SCHEMA: &str = r#"
-- =====================================================
-- CLASSIFICATION RULES
-- =====================================================

-- Classification rules for automatic asset categorization
CREATE TABLE IF NOT EXISTS classification_rules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    file_pattern TEXT NOT NULL,
    asset_type TEXT NOT NULL,
    auto_tags_json TEXT NOT NULL DEFAULT '[]',
    priority INTEGER NOT NULL DEFAULT 50,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    
    -- Dimension constraints (NULL means no constraint)
    exact_width INTEGER,
    exact_height INTEGER,
    min_width INTEGER,
    max_width INTEGER,
    min_height INTEGER,
    max_height INTEGER,
    
    -- Classification confidence (0.0 - 1.0)
    confidence REAL NOT NULL DEFAULT 0.85,
    
    updated_at INTEGER NOT NULL
);

-- Index for priority-based ordering
CREATE INDEX IF NOT EXISTS idx_classification_rules_priority 
    ON classification_rules(priority DESC);

-- Index for enabled status
CREATE INDEX IF NOT EXISTS idx_classification_rules_enabled 
    ON classification_rules(enabled);

-- Index for asset type lookup
CREATE INDEX IF NOT EXISTS idx_classification_rules_asset_type 
    ON classification_rules(asset_type);

-- =====================================================
-- CLASSIFICATION HISTORY
-- =====================================================

-- Track classification results for auditing and improvement
CREATE TABLE IF NOT EXISTS classification_history (
    history_id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id INTEGER NOT NULL REFERENCES assets(asset_id),
    classified_at INTEGER NOT NULL,
    detected_type TEXT NOT NULL,
    confidence REAL NOT NULL,
    rules_matched_json TEXT NOT NULL DEFAULT '[]',
    auto_applied BOOLEAN NOT NULL DEFAULT 0,
    manually_corrected BOOLEAN NOT NULL DEFAULT 0,
    corrected_type TEXT,
    corrected_by TEXT,
    correction_notes TEXT
);

CREATE INDEX IF NOT EXISTS idx_classification_history_asset 
    ON classification_history(asset_id);

CREATE INDEX IF NOT EXISTS idx_classification_history_date 
    ON classification_history(classified_at);

-- =====================================================
-- AUTO-CLASSIFICATION QUEUE
-- =====================================================

-- Queue for files waiting to be classified
CREATE TABLE IF NOT EXISTS classification_queue (
    queue_id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL UNIQUE,
    file_name TEXT NOT NULL,
    file_size INTEGER,
    detected_dimensions TEXT, -- "widthxheight" or NULL
    queued_at INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending', -- pending, processing, completed, failed
    retry_count INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    processed_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_classification_queue_status 
    ON classification_queue(status);

CREATE INDEX IF NOT EXISTS idx_classification_queue_queued 
    ON classification_queue(queued_at);

-- =====================================================
-- CLASSIFICATION STATISTICS
-- =====================================================

-- Aggregate statistics for rule effectiveness
CREATE TABLE IF NOT EXISTS classification_stats (
    rule_id TEXT PRIMARY KEY REFERENCES classification_rules(id) ON DELETE CASCADE,
    times_matched INTEGER NOT NULL DEFAULT 0,
    times_applied INTEGER NOT NULL DEFAULT 0,
    times_overridden INTEGER NOT NULL DEFAULT 0,
    avg_confidence REAL,
    last_matched_at INTEGER,
    updated_at INTEGER NOT NULL
);

-- =====================================================
-- VIEWS
-- =====================================================

-- View: Active rules ordered by priority
CREATE VIEW IF NOT EXISTS v_classification_rules_active AS
SELECT * FROM classification_rules
WHERE enabled = 1
ORDER BY priority DESC;

-- View: Classification effectiveness
CREATE VIEW IF NOT EXISTS v_classification_effectiveness AS
SELECT 
    r.id,
    r.name,
    r.asset_type,
    r.priority,
    COALESCE(s.times_matched, 0) as times_matched,
    COALESCE(s.times_applied, 0) as times_applied,
    COALESCE(s.times_overridden, 0) as times_overridden,
    CASE 
        WHEN COALESCE(s.times_matched, 0) = 0 THEN 0.0
        ELSE CAST(COALESCE(s.times_applied, 0) AS REAL) / COALESCE(s.times_matched, 1)
    END as application_rate,
    s.avg_confidence
FROM classification_rules r
LEFT JOIN classification_stats s ON r.id = s.rule_id
WHERE r.enabled = 1
ORDER BY r.priority DESC;

-- View: Recent classifications
CREATE VIEW IF NOT EXISTS v_recent_classifications AS
SELECT 
    h.*,
    a.name as asset_name,
    a.file_path,
    a.file_size
FROM classification_history h
JOIN assets a ON h.asset_id = a.asset_id
ORDER BY h.classified_at DESC
LIMIT 100;

-- View: Pending classification queue
CREATE VIEW IF NOT EXISTS v_classification_queue_pending AS
SELECT * FROM classification_queue
WHERE status = 'pending'
ORDER BY queued_at;
"#;

/// Insert default classification rules
fn insert_default_rules(conn: &Connection) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();

    let default_rules: Vec<(&str, &str, &str, &str, &str, i32, Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<i64>, f64)> = vec![
        ("rule_char_32", "Character Sprites (32x32)", "character_*", "character", r#"["character", "animated"]"#, 100, Some(32), Some(32), None, None, None, None, 0.95),
        ("rule_char_64", "Character Sprites (64x64)", "character_*", "character", r#"["character", "animated", "high_res"]"#, 95, Some(64), Some(64), None, None, None, None, 0.95),
        ("rule_item", "Item Icons", "item_*", "item", r#"["item", "icon"]"#, 90, Some(32), Some(32), None, None, None, None, 0.90),
        ("rule_portrait", "Portraits (64x64)", "face_*", "portrait", r#"["portrait", "face"]"#, 85, Some(64), Some(64), None, None, None, None, 0.90),
        ("rule_effect", "Spell Effects", "effect_*", "effect", r#"["effect", "animated"]"#, 80, Some(192), Some(192), None, None, None, None, 0.85),
        ("rule_tileset", "Tilesets", "*.tsx", "tileset", r#"["tileset", "terrain"]"#, 75, None, None, None, None, None, None, 0.80),
        ("rule_spritesheet", "Sprite Sheets", "*_sheet.png", "sprite_sheet", r#"["sprite_sheet", "animated"]"#, 70, None, None, Some(128), None, Some(32), Some(128), 0.85),
        ("rule_background", "Backgrounds", "bg_*", "background", r#"["background", "parallax"]"#, 65, None, None, Some(640), None, Some(480), None, 0.80),
        ("rule_ui", "UI Elements", "ui_*", "ui", r#"["ui", "hud"]"#, 60, None, None, None, Some(512), None, Some(256), 0.75),
        ("rule_battle", "Battle Sprites", "battle_*", "battle_sprite", r#"["battle", "sv_battler"]"#, 55, None, None, Some(64), Some(192), Some(64), Some(64), 0.85),
    ];

    for rule in default_rules {
        conn.execute(
            "INSERT INTO classification_rules 
             (id, name, file_pattern, asset_type, auto_tags_json, priority, 
              enabled, exact_width, exact_height, min_width, max_width, 
              min_height, max_height, confidence, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(id) DO NOTHING",
            (
                rule.0, rule.1, rule.2, rule.3, rule.4, rule.5,
                rule.6, rule.7, rule.8, rule.9, rule.10, rule.11, rule.12, now,
            ),
        )?;
    }

    tracing::info!("Inserted default classification rules");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn in_memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(V7_SCHEMA).unwrap();
        conn
    }

    #[test]
    fn test_classification_rules_table() {
        let conn = in_memory_db();
        
        // Test insert
        conn.execute(
            "INSERT INTO classification_rules 
             (id, name, file_pattern, asset_type, auto_tags_json, priority, enabled, confidence, updated_at)
             VALUES ('test1', 'Test Rule', '*.png', 'test', '[]', 50, 1, 0.9, 1234567890)",
            [],
        ).unwrap();

        // Test select
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM classification_rules",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_classification_queue_table() {
        let conn = in_memory_db();
        
        conn.execute(
            "INSERT INTO classification_queue (file_path, file_name, queued_at, status)
             VALUES ('/test/file.png', 'file.png', 1234567890, 'pending')",
            [],
        ).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM classification_queue WHERE status = 'pending'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_classification_history_table() {
        let conn = in_memory_db();
        
        conn.execute(
            "INSERT INTO classification_history 
             (asset_id, classified_at, detected_type, confidence, rules_matched_json)
             VALUES (1, 1234567890, 'character', 0.95, '[\"rule1\", \"rule2\"]')",
            [],
        ).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM classification_history",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_default_rules_insertion() {
        let conn = in_memory_db();
        insert_default_rules(&conn).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM classification_rules",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(count > 0);

        // Check that character rule exists
        let name: String = conn
            .query_row(
                "SELECT name FROM classification_rules WHERE id = 'rule_char_32'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "Character Sprites (32x32)");
    }
}
