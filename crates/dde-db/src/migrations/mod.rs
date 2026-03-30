//! Database Migrations
//!
//! Manages schema versioning and migrations for the SQLite database.

use rusqlite::Connection;

use crate::Result;

pub mod v2_asset_os;
pub mod v3_screenshots;
pub mod v4_visual_scripts;
pub mod v4_cutscenes;

/// Current schema version
pub const CURRENT_SCHEMA_VERSION: i32 = 5;

/// Run all pending migrations
pub fn run_migrations(conn: &Connection) -> Result<()> {
    // Create migrations table if not exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _migrations (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL
        )",
        [],
    )?;

    // Get current version
    let current_version: i32 = conn
        .query_row("SELECT MAX(version) FROM _migrations", [], |row| {
            row.get::<_, Option<i32>>(0).map(|v| v.unwrap_or(0))
        })
        .unwrap_or(0);

    // Apply pending migrations
    if current_version < 1 {
        migration_v1_initial_schema(conn)?;
        record_migration(conn, 1)?;
    }

    if current_version < 2 {
        v2_asset_os::apply(conn)?;
        record_migration(conn, 2)?;
    }

    if current_version < 3 {
        v3_screenshots::apply(conn)?;
        record_migration(conn, 3)?;
    }

    if current_version < 4 {
        v4_cutscenes::apply(conn)?;
        record_migration(conn, 4)?;
    }

    if current_version < 5 {
        v4_visual_scripts::apply(conn)?;
        record_migration(conn, 5)?;
    }

    tracing::info!("Database schema at version {}", CURRENT_SCHEMA_VERSION);
    Ok(())
}

/// Record a migration as applied
fn record_migration(conn: &Connection, version: i32) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "INSERT INTO _migrations (version, applied_at) VALUES (?1, ?2)",
        (&version, &now),
    )?;
    Ok(())
}

/// Initial schema v1 - creates all tables from the blueprint
fn migration_v1_initial_schema(conn: &Connection) -> Result<()> {
    tracing::info!("Applying migration v1: Initial schema");

    conn.execute_batch(V1_SCHEMA)?;
    Ok(())
}

/// Complete v1 schema from the blueprint
const V1_SCHEMA: &str = r#"
-- =====================================================
-- PROJECT METADATA
-- =====================================================
CREATE TABLE IF NOT EXISTS project_meta (
    project_id TEXT PRIMARY KEY,
    project_name TEXT NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1,
    world_seed INTEGER NOT NULL,
    tick_count INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- =====================================================
-- TILESETS
-- =====================================================
CREATE TABLE IF NOT EXISTS tilesets (
    tileset_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    image_hash TEXT NOT NULL,
    image_path TEXT NOT NULL,
    tile_width INTEGER NOT NULL DEFAULT 32,
    tile_height INTEGER NOT NULL DEFAULT 32,
    columns INTEGER NOT NULL,
    rows INTEGER NOT NULL,
    passability_json TEXT NOT NULL DEFAULT '[]',
    autotile_rules_json TEXT,
    tags TEXT,
    updated_at INTEGER NOT NULL
);

-- =====================================================
-- MAPS
-- =====================================================
CREATE TABLE IF NOT EXISTS maps (
    map_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    map_type TEXT NOT NULL DEFAULT 'overworld',
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    parent_map_id INTEGER REFERENCES maps(map_id),
    entry_x INTEGER NOT NULL DEFAULT 0,
    entry_y INTEGER NOT NULL DEFAULT 0,
    bgm_id TEXT,
    ambient_id TEXT,
    encounter_rate REAL NOT NULL DEFAULT 0.0,
    encounter_table_id INTEGER,
    mode7_enabled BOOLEAN NOT NULL DEFAULT 0,
    camera_bounds_json TEXT,
    updated_at INTEGER NOT NULL
);

-- =====================================================
-- TILES
-- =====================================================
CREATE TABLE IF NOT EXISTS tiles (
    tile_id INTEGER PRIMARY KEY AUTOINCREMENT,
    map_id INTEGER NOT NULL REFERENCES maps(map_id),
    x INTEGER NOT NULL,
    y INTEGER NOT NULL,
    z INTEGER NOT NULL DEFAULT 0,
    tileset_id INTEGER REFERENCES tilesets(tileset_id),
    tile_index INTEGER NOT NULL DEFAULT 0,
    world_state INTEGER NOT NULL DEFAULT 0,
    biome TEXT NOT NULL DEFAULT 'grassland',
    passable BOOLEAN NOT NULL DEFAULT 1,
    event_trigger_id INTEGER,
    updated_at INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_tiles_map_xyz ON tiles(map_id, x, y, z);
CREATE INDEX IF NOT EXISTS idx_tiles_world_state ON tiles(map_id, world_state);
CREATE INDEX IF NOT EXISTS idx_tiles_biome ON tiles(map_id, biome);
CREATE INDEX IF NOT EXISTS idx_tiles_updated ON tiles(updated_at);

-- =====================================================
-- ENTITIES
-- =====================================================
CREATE TABLE IF NOT EXISTS entities (
    entity_id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL DEFAULT 'npc',
    name TEXT NOT NULL,
    map_id INTEGER NOT NULL REFERENCES maps(map_id),
    x INTEGER NOT NULL,
    y INTEGER NOT NULL,
    sprite_sheet_id INTEGER,
    direction INTEGER NOT NULL DEFAULT 0,
    logic_prompt TEXT,
    dialogue_tree_id INTEGER,
    stats_json TEXT NOT NULL DEFAULT '{}',
    equipment_json TEXT,
    inventory_json TEXT DEFAULT '[]',
    patrol_path_json TEXT,
    schedule_json TEXT,
    faction_id INTEGER,
    is_interactable BOOLEAN NOT NULL DEFAULT 1,
    is_collidable BOOLEAN NOT NULL DEFAULT 1,
    respawn_ticks INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_entities_map ON entities(map_id, x, y);
CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);
CREATE INDEX IF NOT EXISTS idx_entities_map_pos ON entities(map_id, x, y);
CREATE INDEX IF NOT EXISTS idx_entities_type_map ON entities(entity_type, map_id);
CREATE INDEX IF NOT EXISTS idx_entities_updated ON entities(updated_at);

-- =====================================================
-- SIMULATION STATS
-- =====================================================
CREATE TABLE IF NOT EXISTS simulation_stats (
    stat_key TEXT PRIMARY KEY,
    value REAL NOT NULL DEFAULT 0.0,
    raw_value REAL NOT NULL DEFAULT 0.0,
    display_value TEXT NOT NULL DEFAULT '0',
    min_value REAL NOT NULL DEFAULT 0.0,
    max_value REAL NOT NULL DEFAULT 1.0,
    decay_rate REAL NOT NULL DEFAULT 0.0,
    updated_at INTEGER NOT NULL
);

-- =====================================================
-- FACTIONS
-- =====================================================
CREATE TABLE IF NOT EXISTS factions (
    faction_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    reputation REAL NOT NULL DEFAULT 0.0,
    color_hex TEXT,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS faction_relations (
    faction_a INTEGER NOT NULL REFERENCES factions(faction_id),
    faction_b INTEGER NOT NULL REFERENCES factions(faction_id),
    relation REAL NOT NULL DEFAULT 0.0,
    PRIMARY KEY (faction_a, faction_b)
);

-- =====================================================
-- DIALOGUE
-- =====================================================
CREATE TABLE IF NOT EXISTS dialogue_trees (
    tree_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    root_node_id INTEGER,
    context_tags TEXT,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS dialogue_nodes (
    node_id INTEGER PRIMARY KEY AUTOINCREMENT,
    tree_id INTEGER NOT NULL REFERENCES dialogue_trees(tree_id),
    speaker TEXT NOT NULL,
    text TEXT NOT NULL,
    portrait_asset_id INTEGER,
    expression TEXT,
    voice_sfx_id TEXT,
    auto_advance_ms INTEGER NOT NULL DEFAULT 0,
    condition_json TEXT,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS dialogue_choices (
    choice_id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id INTEGER NOT NULL REFERENCES dialogue_nodes(node_id),
    text TEXT NOT NULL,
    next_node_id INTEGER REFERENCES dialogue_nodes(node_id),
    condition_json TEXT,
    effect_json TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0
);

-- =====================================================
-- BATTLE
-- =====================================================
CREATE TABLE IF NOT EXISTS encounter_tables (
    table_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    entries_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS enemy_groups (
    group_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    members_json TEXT NOT NULL,
    bgm_override TEXT,
    is_boss BOOLEAN NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS enemy_templates (
    template_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    stats_json TEXT NOT NULL,
    skills_json TEXT DEFAULT '[]',
    loot_table_json TEXT DEFAULT '[]',
    exp_reward INTEGER NOT NULL DEFAULT 0,
    gold_reward INTEGER NOT NULL DEFAULT 0,
    sprite_asset_id INTEGER,
    element_weak_json TEXT DEFAULT '[]',
    element_resist_json TEXT DEFAULT '[]',
    element_absorb_json TEXT DEFAULT '[]',
    status_immune_json TEXT DEFAULT '[]',
    ai_behavior TEXT DEFAULT 'aggressive',
    ai_script_json TEXT,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS skills (
    skill_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    mp_cost INTEGER NOT NULL DEFAULT 0,
    target_type TEXT NOT NULL DEFAULT 'single_enemy',
    damage_formula TEXT,
    element TEXT DEFAULT 'none',
    status_inflict_json TEXT,
    status_cure_json TEXT,
    animation_id INTEGER,
    sfx_id TEXT,
    icon_asset_id INTEGER,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS items (
    item_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    item_type TEXT NOT NULL DEFAULT 'consumable',
    effect_json TEXT,
    equip_stats_json TEXT,
    equip_slot TEXT,
    element_grant TEXT,
    status_resist_json TEXT,
    price_buy INTEGER NOT NULL DEFAULT 0,
    price_sell INTEGER NOT NULL DEFAULT 0,
    stackable BOOLEAN NOT NULL DEFAULT 1,
    max_stack INTEGER NOT NULL DEFAULT 99,
    icon_asset_id INTEGER,
    rarity TEXT DEFAULT 'common',
    updated_at INTEGER NOT NULL
);

-- =====================================================
-- QUESTS
-- =====================================================
CREATE TABLE IF NOT EXISTS quests (
    quest_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    quest_type TEXT NOT NULL DEFAULT 'side',
    state TEXT NOT NULL DEFAULT 'inactive',
    objectives_json TEXT DEFAULT '[]',
    rewards_json TEXT,
    prerequisite_json TEXT,
    on_complete_effects_json TEXT,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_quests_state ON quests(state);

-- =====================================================
-- EVENT TRIGGERS
-- =====================================================
CREATE TABLE IF NOT EXISTS event_triggers (
    trigger_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT,
    trigger_type TEXT NOT NULL DEFAULT 'step_on',
    condition_json TEXT,
    actions_json TEXT NOT NULL,
    one_shot BOOLEAN NOT NULL DEFAULT 0,
    cooldown_ticks INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_triggers_type ON event_triggers(trigger_type, enabled);

-- =====================================================
-- GAME FLAGS
-- =====================================================
CREATE TABLE IF NOT EXISTS game_flags (
    flag_key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

-- =====================================================
-- TIMELINE / DIRECTOR
-- =====================================================
CREATE TABLE IF NOT EXISTS timelines (
    timeline_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    total_ticks INTEGER NOT NULL,
    loop_mode TEXT NOT NULL DEFAULT 'once',
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS timeline_clips (
    clip_id INTEGER PRIMARY KEY AUTOINCREMENT,
    timeline_id INTEGER NOT NULL REFERENCES timelines(timeline_id),
    start_tick INTEGER NOT NULL,
    duration_ticks INTEGER NOT NULL,
    entity_id INTEGER,
    clip_type TEXT NOT NULL,
    data_json TEXT NOT NULL,
    easing TEXT NOT NULL DEFAULT 'linear',
    sort_order INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL
);

-- =====================================================
-- AI CACHE
-- =====================================================
CREATE TABLE IF NOT EXISTS llm_cache (
    prompt_hash TEXT PRIMARY KEY,
    model TEXT NOT NULL,
    response TEXT NOT NULL,
    token_cost INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_llm_cache_model ON llm_cache(model);
CREATE INDEX IF NOT EXISTS idx_llm_cache_expires ON llm_cache(expires_at);

-- =====================================================
-- ASSETS
-- =====================================================
CREATE TABLE IF NOT EXISTS assets (
    asset_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    asset_type TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    metadata_json TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS asset_tags (
    asset_id INTEGER NOT NULL REFERENCES assets(asset_id),
    tag TEXT NOT NULL,
    PRIMARY KEY (asset_id, tag)
);

CREATE TABLE IF NOT EXISTS asset_provenance (
    asset_id INTEGER PRIMARY KEY REFERENCES assets(asset_id),
    source_type TEXT NOT NULL,
    source_id TEXT,
    generation_prompt TEXT,
    generation_model TEXT,
    generation_seed INTEGER,
    parent_asset_id INTEGER,
    created_at INTEGER NOT NULL
);

-- =====================================================
-- SCRIPTS
-- =====================================================
CREATE TABLE IF NOT EXISTS scripts (
    script_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    source TEXT NOT NULL,
    attachment_type TEXT NOT NULL DEFAULT 'event',
    updated_at INTEGER NOT NULL
);

-- =====================================================
-- SAVE SLOTS (for player save games)
-- =====================================================
CREATE TABLE IF NOT EXISTS save_slots (
    slot_number INTEGER PRIMARY KEY,
    saved_at INTEGER NOT NULL,
    play_time_ms INTEGER NOT NULL DEFAULT 0,
    screenshot_data BLOB
);

-- =====================================================
-- SAVEPOINTS (for undo/redo)
-- =====================================================
CREATE TABLE IF NOT EXISTS savepoints (
    savepoint_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
"#;
