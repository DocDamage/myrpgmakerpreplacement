//! Duplicate Detection
//!
//! Uses SHA-256 for exact duplicates and perceptual hashing
//! for visually similar images.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Type of hash for duplicate detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashType {
    Sha256,
    Perceptual,
    Average,
    Difference,
}

impl HashType {
    pub fn as_str(&self) -> &'static str {
        match self {
            HashType::Sha256 => "sha256",
            HashType::Perceptual => "perceptual_hash",
            HashType::Average => "avg_hash",
            HashType::Difference => "dhash",
        }
    }
}

/// Computed hashes for an asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetHashes {
    pub asset_id: i64,
    pub sha256: String,
    pub perceptual: Option<String>,
    pub perceptual_64: Option<String>,
    pub avg_hash: Option<String>,
    pub dhash: Option<String>,
    pub phash: Option<String>,
}

/// Duplicate detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateMatch {
    pub asset_id: i64,
    pub duplicate_of_asset_id: i64,
    pub match_type: String,
    pub match_score: f64,
    pub match_details: serde_json::Value,
}

/// Duplicate detector
pub struct DuplicateDetector;

impl DuplicateDetector {
    /// Compute SHA-256 hash of a file
    pub async fn compute_sha256<P: AsRef<Path>>(path: P) -> crate::Result<String> {
        use sha2::{Digest, Sha256};

        let data = tokio::fs::read(path).await?;
        let hash = Sha256::digest(&data);
        Ok(hex::encode(hash))
    }

    /// Compute perceptual hashes for an image
    pub async fn compute_image_hashes<P: AsRef<Path>>(
        path: P,
    ) -> crate::Result<(Option<String>, Option<String>, Option<String>)> {
        let path = path.as_ref();

        // Read image
        let data = match tokio::fs::read(path).await {
            Ok(d) => d,
            Err(_) => return Ok((None, None, None)),
        };

        let img = match image::load_from_memory(&data) {
            Ok(i) => i,
            Err(_) => return Ok((None, None, None)),
        };

        // Convert to grayscale for hashing
        let gray = img.to_luma8();

        // Compute average hash (8x8)
        let avg_hash = Self::compute_average_hash(&gray);

        // Compute difference hash (9x8 -> 8x8)
        let dhash = Self::compute_difference_hash(&gray);

        // Simple perceptual hash (downscale to 8x8)
        let perceptual = Self::compute_simple_perceptual_hash(&gray);

        Ok((avg_hash, dhash, perceptual))
    }

    /// Compute average hash (aHash)
    /// Resizes to 8x8, computes average, creates 64-bit hash
    fn compute_average_hash(img: &image::GrayImage) -> Option<String> {
        // Resize to 8x8
        let resized = image::imageops::resize(img, 8, 8, image::imageops::FilterType::Lanczos3);

        // Compute average
        let sum: u32 = resized.pixels().map(|p| p[0] as u32).sum();
        let avg = (sum / 64) as u8;

        // Create hash: 1 if pixel > avg, 0 otherwise
        let mut hash: u64 = 0;
        for (i, pixel) in resized.pixels().enumerate() {
            if pixel[0] > avg {
                hash |= 1 << (63 - i);
            }
        }

        Some(format!("{:016x}", hash))
    }

    /// Compute difference hash (dHash)
    /// Compares adjacent pixels horizontally
    fn compute_difference_hash(img: &image::GrayImage) -> Option<String> {
        // Resize to 9x8 (we need 9 columns to get 8 differences)
        let resized = image::imageops::resize(img, 9, 8, image::imageops::FilterType::Lanczos3);

        // Create hash: 1 if pixel > next pixel, 0 otherwise
        let mut hash: u64 = 0;
        let mut bit: u32 = 63;

        for y in 0..8 {
            for x in 0..8 {
                let left = resized.get_pixel(x, y)[0];
                let right = resized.get_pixel(x + 1, y)[0];
                if left > right {
                    hash |= 1 << bit;
                }
                bit = bit.saturating_sub(1);
            }
        }

        Some(format!("{:016x}", hash))
    }

    /// Simple perceptual hash (pHash-like)
    /// Uses discrete cosine transform on 8x8 image
    fn compute_simple_perceptual_hash(img: &image::GrayImage) -> Option<String> {
        // Simplified version: just use 8x8 resized and high-pass filter
        let resized = image::imageops::resize(img, 8, 8, image::imageops::FilterType::Lanczos3);

        // Get pixels as values
        let pixels: Vec<u8> = resized.pixels().map(|p| p[0]).collect();

        // Simple median-based hash
        let mut sorted = pixels.clone();
        sorted.sort_unstable();
        let median = sorted[32]; // median of 64 values

        // Create hash: 1 if pixel > median, 0 otherwise
        let mut hash: u64 = 0;
        for (i, pixel) in pixels.iter().enumerate() {
            if *pixel > median {
                hash |= 1 << (63 - i);
            }
        }

        Some(format!("{:016x}", hash))
    }

    /// Compute Hamming distance between two hex hashes
    pub fn hamming_distance(hash1: &str, hash2: &str) -> Option<u32> {
        if hash1.len() != hash2.len() {
            return None;
        }

        let num1 = u64::from_str_radix(hash1, 16).ok()?;
        let num2 = u64::from_str_radix(hash2, 16).ok()?;

        let xor = num1 ^ num2;
        Some(xor.count_ones())
    }

    /// Compute similarity score (0.0 to 1.0) from Hamming distance
    pub fn similarity_from_distance(distance: u32, hash_bits: u32) -> f64 {
        let max_dist = hash_bits as f64;
        let dist = distance.min(hash_bits) as f64;
        1.0 - (dist / max_dist)
    }

    /// Find potential duplicates in the database
    pub fn find_duplicates(
        db: &dde_db::Database,
        asset_id: i64,
        min_similarity: f64,
    ) -> crate::Result<Vec<DuplicateMatch>> {
        let conn = db.conn();

        // Get hashes for the source asset
        let source_hashes: (String, Option<String>, Option<String>) = conn.query_row(
            "SELECT sha256_hash, perceptual_hash, avg_hash FROM asset_hashes WHERE asset_id = ?1",
            [asset_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            }
        ).map_err(|e| crate::AssetForgeError::Database(e.into()))?;

        let mut duplicates = Vec::new();

        // Check for exact SHA-256 match
        let mut stmt = conn.prepare(
            "SELECT ah.asset_id, a.name 
             FROM asset_hashes ah
             JOIN assets a ON ah.asset_id = a.asset_id
             WHERE ah.sha256_hash = ?1 AND ah.asset_id != ?2",
        )?;

        let exact_matches = stmt.query_map((&source_hashes.0, asset_id), |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;

        for result in exact_matches {
            let (dup_id, name) = result?;
            duplicates.push(DuplicateMatch {
                asset_id,
                duplicate_of_asset_id: dup_id,
                match_type: "exact_hash".to_string(),
                match_score: 1.0,
                match_details: serde_json::json!({
                    "duplicate_name": name,
                    "hash_type": "sha256",
                }),
            });
        }

        // Check for perceptual hash matches if available
        if let Some(ref perceptual) = source_hashes.1 {
            let mut stmt = conn.prepare(
                "SELECT ah.asset_id, a.name, ah.perceptual_hash 
                 FROM asset_hashes ah
                 JOIN assets a ON ah.asset_id = a.asset_id
                 WHERE ah.perceptual_hash IS NOT NULL AND ah.asset_id != ?1",
            )?;

            let candidates = stmt.query_map([asset_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;

            for result in candidates {
                let (cand_id, name, cand_hash) = result?;

                if let Some(distance) = Self::hamming_distance(perceptual, &cand_hash) {
                    let similarity = Self::similarity_from_distance(distance, 64);

                    if similarity >= min_similarity {
                        duplicates.push(DuplicateMatch {
                            asset_id,
                            duplicate_of_asset_id: cand_id,
                            match_type: "perceptual_hash".to_string(),
                            match_score: similarity,
                            match_details: serde_json::json!({
                                "duplicate_name": name,
                                "hamming_distance": distance,
                                "similarity": similarity,
                            }),
                        });
                    }
                }
            }
        }

        // Check for average hash matches
        if let Some(ref avg_hash) = source_hashes.2 {
            let mut stmt = conn.prepare(
                "SELECT ah.asset_id, a.name, ah.avg_hash 
                 FROM asset_hashes ah
                 JOIN assets a ON ah.asset_id = a.asset_id
                 WHERE ah.avg_hash IS NOT NULL AND ah.asset_id != ?1",
            )?;

            let candidates = stmt.query_map([asset_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;

            for result in candidates {
                let (cand_id, name, cand_hash) = result?;

                if let Some(distance) = Self::hamming_distance(avg_hash, &cand_hash) {
                    let similarity = Self::similarity_from_distance(distance, 64);

                    if similarity >= min_similarity && similarity < 1.0 {
                        // Avoid duplicates of exact matches
                        duplicates.push(DuplicateMatch {
                            asset_id,
                            duplicate_of_asset_id: cand_id,
                            match_type: "avg_hash".to_string(),
                            match_score: similarity,
                            match_details: serde_json::json!({
                                "duplicate_name": name,
                                "hamming_distance": distance,
                                "similarity": similarity,
                            }),
                        });
                    }
                }
            }
        }

        Ok(duplicates)
    }

    /// Record duplicate detection in database
    pub fn record_duplicate(
        db: &dde_db::Database,
        asset_id: i64,
        duplicate_of: i64,
        match_type: &str,
        match_score: f64,
        details: serde_json::Value,
    ) -> crate::Result<()> {
        let now = chrono::Utc::now().timestamp_millis();

        let conn = db.conn();
        conn.execute(
            "INSERT INTO asset_duplicates 
             (asset_id, duplicate_of_asset_id, match_type, match_score, match_details_json, detected_at, resolved)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)
             ON CONFLICT DO NOTHING",
            (
                asset_id,
                duplicate_of,
                match_type,
                match_score,
                details.to_string(),
                now,
            ),
        )?;

        Ok(())
    }

    /// Compute and store all hashes for an asset
    pub async fn compute_and_store_hashes(
        db: &dde_db::Database,
        asset_id: i64,
        file_path: &Path,
    ) -> crate::Result<()> {
        let now = chrono::Utc::now().timestamp_millis();

        // Compute hashes
        let sha256 = Self::compute_sha256(file_path).await?;
        let (avg_hash, dhash, perceptual) = Self::compute_image_hashes(file_path).await?;

        // Store in database
        let conn = db.conn();
        conn.execute(
            "INSERT INTO asset_hashes 
             (asset_id, sha256_hash, perceptual_hash, avg_hash, dhash, computed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(asset_id) DO UPDATE SET
             sha256_hash = excluded.sha256_hash,
             perceptual_hash = excluded.perceptual_hash,
             avg_hash = excluded.avg_hash,
             dhash = excluded.dhash,
             computed_at = excluded.computed_at",
            (asset_id, &sha256, &perceptual, &avg_hash, &dhash, now),
        )?;

        tracing::debug!(
            "Computed hashes for asset {}: sha256={:.16}...",
            asset_id,
            sha256
        );

        Ok(())
    }
}
