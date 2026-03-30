//! Migration V2: Asset OS - Asset Forge Integration
//!
//! Expands the asset system to support the full Asset Forge pipeline:
//! - Inbox: New assets awaiting classification
//! - Staging: Classified assets awaiting review
//! - Review Queue: Assets under review
//! - Approved: Production-ready assets
//! - Rejected: Discarded assets

use rusqlite::Connection;

use crate::Result;

/// Apply V2 migration - Asset OS schema
pub fn apply(conn: &Connection) -> Result<()> {
    tracing::info!("Applying migration v2: Asset OS schema");

    conn.execute_batch(V2_SCHEMA)?;

    tracing::info!("Migration v2 complete");
    Ok(())
}

/// V2 schema additions for Asset OS
const V2_SCHEMA: &str = r#"
-- =====================================================
-- ASSET OS: Expanded asset management
-- =====================================================

-- Asset status now includes full pipeline stages
-- pending -> inbox -> classified -> review -> approved/rejected

-- Asset classification results
CREATE TABLE IF NOT EXISTS asset_classification (
    asset_id INTEGER PRIMARY KEY REFERENCES assets(asset_id),
    classifier_version INTEGER NOT NULL DEFAULT 1,
    detected_type TEXT,
    confidence_score REAL NOT NULL DEFAULT 0.0,
    classification_data_json TEXT,
    classified_at INTEGER,
    classified_by TEXT DEFAULT 'system'
);

-- Asset review queue
CREATE TABLE IF NOT EXISTS asset_reviews (
    review_id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id INTEGER NOT NULL REFERENCES assets(asset_id),
    reviewer_name TEXT,
    review_status TEXT NOT NULL DEFAULT 'pending',
    review_score INTEGER,
    review_notes TEXT,
    consistency_score REAL,
    quality_issues_json TEXT,
    reviewed_at INTEGER,
    created_at INTEGER NOT NULL
);

-- Asset variants (different sizes, formats, derived assets)
CREATE TABLE IF NOT EXISTS asset_variants (
    variant_id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id INTEGER NOT NULL REFERENCES assets(asset_id),
    variant_type TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    width INTEGER,
    height INTEGER,
    metadata_json TEXT,
    created_at INTEGER NOT NULL
);

-- Enhanced asset provenance with Forge integration
CREATE TABLE IF NOT EXISTS asset_provenance_v2 (
    provenance_id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id INTEGER NOT NULL REFERENCES assets(asset_id),
    source_type TEXT NOT NULL,
    -- 'forge_generate', 'forge_derive', 'import_file', 'import_zip', 
    -- 'manual_upload', 'derived_from', 'converted'
    source_id TEXT,
    
    -- For generated assets
    generation_prompt TEXT,
    generation_negative_prompt TEXT,
    generation_model TEXT,
    generation_provider TEXT,
    generation_seed INTEGER,
    generation_cost_cents INTEGER,
    generation_params_json TEXT,
    
    -- For derived assets
    parent_asset_id INTEGER REFERENCES assets(asset_id),
    derivation_type TEXT,
    -- 'sheet_from_hero', 'portrait_from_hero', 'frame_extracted',
    -- 'bg_removed', 'upscaled', 'converted_format'
    
    -- Style profile linking
    style_profile_id TEXT,
    
    -- Chain of provenance (for multi-step generation)
    root_asset_id INTEGER REFERENCES assets(asset_id),
    generation_step INTEGER DEFAULT 1,
    
    created_at INTEGER NOT NULL
);

-- Style profiles for consistent generation
CREATE TABLE IF NOT EXISTS style_profiles (
    profile_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    
    -- Visual characteristics
    art_style TEXT,
    color_palette_json TEXT,
    line_weight TEXT,
    shading_style TEXT,
    
    -- Technical specs
    target_width INTEGER,
    target_height INTEGER,
    transparency BOOLEAN DEFAULT 1,
    
    -- Generation hints
    positive_prompt_prefix TEXT,
    negative_prompt_prefix TEXT,
    
    -- Reference assets
    reference_asset_ids_json TEXT,
    
    -- Provider preferences for this style
    preferred_provider TEXT,
    fallback_provider TEXT,
    
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Duplicate detection (hash-based and perceptual)
CREATE TABLE IF NOT EXISTS asset_duplicates (
    duplicate_id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id INTEGER NOT NULL REFERENCES assets(asset_id),
    duplicate_of_asset_id INTEGER NOT NULL REFERENCES assets(asset_id),
    match_type TEXT NOT NULL,
    -- 'exact_hash', 'perceptual_hash', 'filename', 'metadata'
    match_score REAL NOT NULL,
    match_details_json TEXT,
    detected_at INTEGER NOT NULL,
    resolved BOOLEAN DEFAULT 0,
    resolution TEXT
    -- 'confirmed_duplicate', 'false_positive', 'variant_allowed'
);

-- Asset hashes for deduplication
CREATE TABLE IF NOT EXISTS asset_hashes (
    asset_id INTEGER PRIMARY KEY REFERENCES assets(asset_id),
    sha256_hash TEXT NOT NULL,
    perceptual_hash TEXT,
    perceptual_hash_64 TEXT,
    avg_hash TEXT,
    dhash TEXT,
    phash TEXT,
    computed_at INTEGER NOT NULL
);

-- Asset import batch tracking
CREATE TABLE IF NOT EXISTS import_batches (
    batch_id INTEGER PRIMARY KEY AUTOINCREMENT,
    batch_type TEXT NOT NULL,
    -- 'zip', 'folder', 'drag_drop', 'forge_export'
    source_path TEXT,
    total_files INTEGER NOT NULL DEFAULT 0,
    processed_files INTEGER NOT NULL DEFAULT 0,
    failed_files INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'in_progress',
    -- 'in_progress', 'completed', 'failed', 'partial'
    error_log_json TEXT,
    created_at INTEGER NOT NULL,
    completed_at INTEGER
);

-- Link assets to import batches
CREATE TABLE IF NOT EXISTS import_batch_assets (
    batch_id INTEGER NOT NULL REFERENCES import_batches(batch_id),
    asset_id INTEGER NOT NULL REFERENCES assets(asset_id),
    original_filename TEXT,
    import_status TEXT DEFAULT 'success',
    error_message TEXT,
    PRIMARY KEY (batch_id, asset_id)
);

-- Asset tags expanded
CREATE INDEX IF NOT EXISTS idx_assets_status ON assets(status);
CREATE INDEX IF NOT EXISTS idx_assets_type ON assets(asset_type);
CREATE INDEX IF NOT EXISTS idx_assets_created ON assets(created_at);

CREATE INDEX IF NOT EXISTS idx_asset_hashes_sha256 ON asset_hashes(sha256_hash);
CREATE INDEX IF NOT EXISTS idx_asset_hashes_perceptual ON asset_hashes(perceptual_hash);

CREATE INDEX IF NOT EXISTS idx_provenance_parent ON asset_provenance_v2(parent_asset_id);
CREATE INDEX IF NOT EXISTS idx_provenance_root ON asset_provenance_v2(root_asset_id);
CREATE INDEX IF NOT EXISTS idx_provenance_style ON asset_provenance_v2(style_profile_id);

CREATE INDEX IF NOT EXISTS idx_duplicates_asset ON asset_duplicates(asset_id);
CREATE INDEX IF NOT EXISTS idx_duplicates_match ON asset_duplicates(duplicate_of_asset_id);

CREATE INDEX IF NOT EXISTS idx_reviews_status ON asset_reviews(review_status);
CREATE INDEX IF NOT EXISTS idx_reviews_asset ON asset_reviews(asset_id);

-- Asset OS views for common queries

-- View: Assets in inbox (awaiting classification)
CREATE VIEW IF NOT EXISTS v_assets_inbox AS
SELECT a.* FROM assets a
WHERE a.status = 'inbox'
ORDER BY a.created_at DESC;

-- View: Assets in review queue
CREATE VIEW IF NOT EXISTS v_assets_review_queue AS
SELECT 
    a.*,
    r.review_id,
    r.reviewer_name,
    r.review_score,
    r.review_notes,
    r.created_at as review_requested_at
FROM assets a
JOIN asset_reviews r ON a.asset_id = r.asset_id
WHERE r.review_status = 'pending'
ORDER BY r.created_at;

-- View: Approved production assets
CREATE VIEW IF NOT EXISTS v_assets_production AS
SELECT a.* FROM assets a
WHERE a.status = 'approved'
ORDER BY a.updated_at DESC;

-- View: Assets with full provenance
CREATE VIEW IF NOT EXISTS v_assets_with_provenance AS
SELECT 
    a.*,
    p.source_type,
    p.generation_model,
    p.generation_provider,
    p.generation_prompt,
    p.parent_asset_id,
    p.style_profile_id
FROM assets a
LEFT JOIN asset_provenance_v2 p ON a.asset_id = p.asset_id;

-- View: Potential duplicates awaiting resolution
CREATE VIEW IF NOT EXISTS v_duplicate_candidates AS
SELECT 
    d.*,
    a1.name as asset_name,
    a2.name as duplicate_of_name,
    a1.file_path as asset_path,
    a2.file_path as duplicate_of_path
FROM asset_duplicates d
JOIN assets a1 ON d.asset_id = a1.asset_id
JOIN assets a2 ON d.duplicate_of_asset_id = a2.asset_id
WHERE d.resolved = 0
ORDER BY d.match_score DESC;

-- Update assets table to allow new status values
-- Note: SQLite doesn't support ALTER TABLE for enums,
-- so we rely on application-level validation

-- Insert default style profile
INSERT OR IGNORE INTO style_profiles (
    profile_id, name, description, art_style,
    target_width, target_height, transparency,
    preferred_provider, fallback_provider,
    created_at, updated_at
) VALUES (
    'default_rpg',
    'Default RPG Style',
    'Standard RPG Maker style 32x32 pixel art characters',
    'pixel_art',
    32, 32, 1,
    'openai', 'fal',
    strftime('%s', 'now') * 1000,
    strftime('%s', 'now') * 1000
);
"#;
