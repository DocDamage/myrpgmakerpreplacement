//! Migration v8: Formation System Tables
//!
//! Adds tables for saving and loading battle formations.

use rusqlite::Connection;
use crate::Result;

/// Apply v8 migration - add formation tables
pub fn apply(conn: &Connection) -> Result<()> {
    tracing::info!("Applying migration v7: Formation system tables");

    conn.execute_batch(
        r#"
        -- =====================================================
        -- FORMATIONS
        -- =====================================================
        -- Saved battle formations for parties
        CREATE TABLE IF NOT EXISTS formations (
            formation_id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT,
            slots_json TEXT NOT NULL DEFAULT '[]',
            default_layout TEXT NOT NULL DEFAULT 'Balanced',
            is_template BOOLEAN NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_formations_updated ON formations(updated_at);
        CREATE INDEX IF NOT EXISTS idx_formations_template ON formations(is_template);

        -- =====================================================
        -- FORMATION PRESETS
        -- =====================================================
        -- User-defined custom presets beyond built-in ones
        CREATE TABLE IF NOT EXISTS formation_presets (
            preset_id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            icon TEXT,
            layout_type TEXT NOT NULL DEFAULT 'Custom',
            slots_json TEXT,
            is_custom BOOLEAN NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL
        );

        -- =====================================================
        -- PARTY FORMATION ASSIGNMENTS
        -- =====================================================
        -- Links formations to specific parties/encounters
        CREATE TABLE IF NOT EXISTS party_formations (
            party_id INTEGER PRIMARY KEY AUTOINCREMENT,
            party_name TEXT NOT NULL,
            formation_id INTEGER REFERENCES formations(formation_id),
            is_player_party BOOLEAN NOT NULL DEFAULT 1,
            updated_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_party_formations_party ON party_formations(party_id);
        "#,
    )?;

    Ok(())
}
