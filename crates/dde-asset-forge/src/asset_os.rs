//! Asset OS - Pipeline Management
//!
//! Manages the asset pipeline: Inbox -> Staging -> Review -> Production

use serde::{Deserialize, Serialize};

/// Asset pipeline stages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetPipelineStage {
    /// New asset in inbox awaiting classification
    Inbox,
    /// Classified asset awaiting review
    Staging,
    /// Asset in review queue
    Review,
    /// Approved asset in production library
    Approved,
    /// Rejected asset
    Rejected,
}

impl AssetPipelineStage {
    /// Get the display name for this stage
    pub fn display_name(&self) -> &'static str {
        match self {
            AssetPipelineStage::Inbox => "Inbox",
            AssetPipelineStage::Staging => "Staging",
            AssetPipelineStage::Review => "Review Queue",
            AssetPipelineStage::Approved => "Production",
            AssetPipelineStage::Rejected => "Rejected",
        }
    }

    /// Get database status string
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetPipelineStage::Inbox => "inbox",
            AssetPipelineStage::Staging => "staging",
            AssetPipelineStage::Review => "review",
            AssetPipelineStage::Approved => "approved",
            AssetPipelineStage::Rejected => "rejected",
        }
    }
}

impl std::str::FromStr for AssetPipelineStage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "inbox" => Ok(AssetPipelineStage::Inbox),
            "staging" => Ok(AssetPipelineStage::Staging),
            "review" => Ok(AssetPipelineStage::Review),
            "approved" => Ok(AssetPipelineStage::Approved),
            "rejected" => Ok(AssetPipelineStage::Rejected),
            _ => Err(format!("Unknown stage: {}", s)),
        }
    }
}

/// Asset review record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetReview {
    pub review_id: i64,
    pub asset_id: i64,
    pub reviewer_name: Option<String>,
    pub review_status: ReviewStatus,
    pub review_score: Option<i32>,
    pub review_notes: Option<String>,
    pub consistency_score: Option<f64>,
    pub quality_issues: Vec<String>,
    pub reviewed_at: Option<i64>,
    pub created_at: i64,
}

/// Review status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Pending,
    InProgress,
    Completed,
}

impl ReviewStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReviewStatus::Pending => "pending",
            ReviewStatus::InProgress => "in_progress",
            ReviewStatus::Completed => "completed",
        }
    }
}

/// Asset OS - manages the asset pipeline
pub struct AssetOs {
    db: dde_db::Database,
}

/// Asset record from database
#[derive(Debug, Clone)]
pub struct AssetRecord {
    pub asset_id: i64,
    pub name: String,
    pub asset_type: String,
    pub file_path: String,
    pub file_hash: String,
    pub file_size: i64,
    pub metadata: serde_json::Value,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// New asset to be ingested
#[derive(Debug, Clone)]
pub struct NewAsset {
    pub name: String,
    pub asset_type: String,
    pub file_path: String,
    pub file_hash: String,
    pub file_size: i64,
    pub metadata: serde_json::Value,
}

impl AssetOs {
    /// Create a new Asset OS instance
    pub fn new(db: dde_db::Database) -> Self {
        Self { db }
    }

    /// Get mutable reference to database (for classification)
    pub fn db_mut(&mut self) -> &mut dde_db::Database {
        &mut self.db
    }

    /// Ingest a new asset into the inbox
    pub async fn ingest_asset(&mut self, asset: NewAsset) -> crate::Result<i64> {
        let now = chrono::Utc::now().timestamp_millis();

        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO assets (name, asset_type, file_path, file_hash, file_size, metadata_json, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'inbox', ?7, ?8)",
            (
                &asset.name,
                &asset.asset_type,
                &asset.file_path,
                &asset.file_hash,
                asset.file_size,
                asset.metadata.to_string(),
                now,
                now,
            ),
        )?;

        let asset_id = conn.last_insert_rowid();

        // Create classification record
        conn.execute(
            "INSERT INTO asset_classification (asset_id, confidence_score, classified_by)
             VALUES (?1, 0.0, 'pending')",
            [asset_id],
        )?;

        tracing::info!(
            "Ingested asset {}: {} ({} bytes)",
            asset_id,
            asset.name,
            asset.file_size
        );

        Ok(asset_id)
    }

    /// Classify an asset and move to staging
    pub async fn classify_asset(
        &mut self,
        asset_id: i64,
        detected_type: &str,
        confidence: f64,
        classification_data: serde_json::Value,
    ) -> crate::Result<()> {
        let now = chrono::Utc::now().timestamp_millis();

        let conn = self.db.conn();

        // Update classification record
        conn.execute(
            "UPDATE asset_classification SET 
             detected_type = ?1, confidence_score = ?2, classification_data_json = ?3, 
             classified_at = ?4
             WHERE asset_id = ?5",
            (
                detected_type,
                confidence,
                classification_data.to_string(),
                now,
                asset_id,
            ),
        )?;

        // Update asset status and type
        conn.execute(
            "UPDATE assets SET asset_type = ?1, status = 'staging', updated_at = ?2
             WHERE asset_id = ?3",
            (detected_type, now, asset_id),
        )?;

        // Create review record if confidence is high enough
        if confidence >= 0.7 {
            conn.execute(
                "INSERT INTO asset_reviews (asset_id, review_status, created_at)
                 VALUES (?1, 'pending', ?2)",
                (asset_id, now),
            )?;
        }

        tracing::info!(
            "Classified asset {} as {} (confidence: {:.2})",
            asset_id,
            detected_type,
            confidence
        );

        Ok(())
    }

    /// Submit an asset for review
    pub async fn submit_for_review(&mut self, asset_id: i64) -> crate::Result<()> {
        let now = chrono::Utc::now().timestamp_millis();

        let conn = self.db.conn();
        conn.execute(
            "UPDATE assets SET status = 'review', updated_at = ?1 WHERE asset_id = ?2",
            (now, asset_id),
        )?;

        // Ensure review record exists
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM asset_reviews WHERE asset_id = ?1",
                [asset_id],
                |_row| Ok(true),
            )
            .unwrap_or(false);

        if !exists {
            conn.execute(
                "INSERT INTO asset_reviews (asset_id, review_status, created_at)
                 VALUES (?1, 'pending', ?2)",
                (asset_id, now),
            )?;
        }

        tracing::info!("Submitted asset {} for review", asset_id);
        Ok(())
    }

    /// Approve an asset and move to production
    pub async fn approve_asset(
        &mut self,
        asset_id: i64,
        reviewer: &str,
        score: i32,
        notes: Option<&str>,
    ) -> crate::Result<()> {
        let now = chrono::Utc::now().timestamp_millis();

        let conn = self.db.conn();

        // Update asset status
        conn.execute(
            "UPDATE assets SET status = 'approved', updated_at = ?1 WHERE asset_id = ?2",
            (now, asset_id),
        )?;

        // Update review record
        conn.execute(
            "UPDATE asset_reviews SET 
             reviewer_name = ?1, review_status = 'completed', review_score = ?2, 
             review_notes = ?3, reviewed_at = ?4
             WHERE asset_id = ?5",
            (reviewer, score, notes, now, asset_id),
        )?;

        tracing::info!(
            "Approved asset {} by {} (score: {})",
            asset_id,
            reviewer,
            score
        );
        Ok(())
    }

    /// Reject an asset
    pub async fn reject_asset(
        &mut self,
        asset_id: i64,
        reviewer: &str,
        reason: &str,
    ) -> crate::Result<()> {
        let now = chrono::Utc::now().timestamp_millis();

        let conn = self.db.conn();

        // Update asset status
        conn.execute(
            "UPDATE assets SET status = 'rejected', updated_at = ?1 WHERE asset_id = ?2",
            (now, asset_id),
        )?;

        // Update review record
        conn.execute(
            "UPDATE asset_reviews SET 
             reviewer_name = ?1, review_status = 'completed', review_notes = ?2, 
             reviewed_at = ?3
             WHERE asset_id = ?4",
            (reviewer, reason, now, asset_id),
        )?;

        tracing::info!("Rejected asset {} by {}: {}", asset_id, reviewer, reason);
        Ok(())
    }

    /// Get assets in a specific stage
    pub fn get_assets_by_stage(
        &self,
        stage: AssetPipelineStage,
        limit: i64,
    ) -> crate::Result<Vec<AssetRecord>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT asset_id, name, asset_type, file_path, file_hash, file_size, 
                    metadata_json, status, created_at, updated_at
             FROM assets WHERE status = ?1 ORDER BY created_at DESC LIMIT ?2",
        )?;

        let assets = stmt.query_map((stage.as_str(), limit), |row| {
            Ok(AssetRecord {
                asset_id: row.get(0)?,
                name: row.get(1)?,
                asset_type: row.get(2)?,
                file_path: row.get(3)?,
                file_hash: row.get(4)?,
                file_size: row.get(5)?,
                metadata: serde_json::from_str(&row.get::<_, String>(6)?)
                    .unwrap_or(serde_json::Value::Null),
                status: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        assets
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| crate::AssetForgeError::Database(e.into()))
    }

    /// Get asset by ID
    pub fn get_asset(&self, asset_id: i64) -> crate::Result<Option<AssetRecord>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT asset_id, name, asset_type, file_path, file_hash, file_size, 
                    metadata_json, status, created_at, updated_at
             FROM assets WHERE asset_id = ?1",
        )?;

        let asset = stmt.query_row([asset_id], |row| {
            Ok(AssetRecord {
                asset_id: row.get(0)?,
                name: row.get(1)?,
                asset_type: row.get(2)?,
                file_path: row.get(3)?,
                file_hash: row.get(4)?,
                file_size: row.get(5)?,
                metadata: serde_json::from_str(&row.get::<_, String>(6)?)
                    .unwrap_or(serde_json::Value::Null),
                status: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        });

        match asset {
            Ok(a) => Ok(Some(a)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(crate::AssetForgeError::Database(e.into())),
        }
    }

    /// Get counts by stage
    pub fn get_stage_counts(&self) -> crate::Result<Vec<(AssetPipelineStage, i64)>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT status, COUNT(*) FROM assets GROUP BY status")?;

        let counts = stmt.query_map([], |row| {
            let status: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            let stage = status
                .parse::<AssetPipelineStage>()
                .unwrap_or(AssetPipelineStage::Inbox);
            Ok((stage, count))
        })?;

        counts
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| crate::AssetForgeError::Database(e.into()))
    }
}
