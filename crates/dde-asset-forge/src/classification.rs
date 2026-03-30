//! Asset Classification Engine
//!
//! Determines asset types based on deterministic rules first,
//! then uses simple heuristics for ambiguous cases.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Classification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub detected_type: String,
    pub confidence: f64,
    pub rules_matched: Vec<String>,
    pub metadata: serde_json::Value,
}

/// Classification rule
pub struct ClassificationRule {
    pub name: String,
    pub asset_type: String,
    pub check: Box<dyn Fn(&AssetInfo) -> bool + Send + Sync>,
    pub confidence: f64,
}

impl std::fmt::Debug for ClassificationRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClassificationRule")
            .field("name", &self.name)
            .field("asset_type", &self.asset_type)
            .field("confidence", &self.confidence)
            .finish_non_exhaustive()
    }
}

impl Clone for ClassificationRule {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            asset_type: self.asset_type.clone(),
            check: Box::new(|_info| false), // Placeholder - rules are typically not cloned
            confidence: self.confidence,
        }
    }
}

/// Asset info for classification
#[derive(Debug, Clone)]
pub struct AssetInfo {
    pub file_name: String,
    pub file_path: String,
    pub file_size: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub has_alpha: Option<bool>,
    pub format: Option<String>,
}

/// Asset classifier
pub struct AssetClassifier {
    rules: Vec<ClassificationRule>,
}

impl AssetClassifier {
    /// Create a new classifier with default rules
    pub fn new() -> Self {
        let mut classifier = Self { rules: Vec::new() };
        classifier.add_default_rules();
        classifier
    }

    /// Add default classification rules
    fn add_default_rules(&mut self) {
        // Character sprite: 32x32 or 64x64, square, small file
        self.rules.push(ClassificationRule {
            name: "character_sprite_32".to_string(),
            asset_type: "character_sprite".to_string(),
            confidence: 0.95,
            check: Box::new(|info| {
                matches!(
                    (info.width, info.height),
                    (Some(32), Some(32)) | (Some(64), Some(64))
                )
            }),
        });

        // Sprite sheet: wider than tall, specific dimensions
        self.rules.push(ClassificationRule {
            name: "sprite_sheet_4dir".to_string(),
            asset_type: "sprite_sheet".to_string(),
            confidence: 0.90,
            check: Box::new(|info| {
                matches!(
                    (info.width, info.height),
                    (Some(128), Some(32)) |    // 4-dir walk
                    (Some(192), Some(32)) |    // 4-dir with extra frames
                    (Some(256), Some(32)) |    // 8-dir or animated
                    (Some(128), Some(64)) |    // Larger character
                    (Some(384), Some(64)) |    // 12-frame sheet
                    (Some(512), Some(64))
                ) // 16-frame sheet
            }),
        });

        // Portrait: 1:1.2 to 1:1.5 ratio, smaller size
        self.rules.push(ClassificationRule {
            name: "portrait".to_string(),
            asset_type: "portrait".to_string(),
            confidence: 0.85,
            check: Box::new(|info| {
                matches!(
                    (info.width, info.height),
                    (Some(64), Some(64)) |     // Standard face
                    (Some(96), Some(96)) |     // High-res face
                    (Some(128), Some(128))
                ) // Portrait large
            }),
        });

        // Tileset: specific dimensions, power-of-2 aligned
        self.rules.push(ClassificationRule {
            name: "tileset_rpg".to_string(),
            asset_type: "tileset".to_string(),
            confidence: 0.90,
            check: Box::new(|info| {
                matches!(
                    (info.width, info.height),
                    (Some(256), Some(256))
                        | (Some(512), Some(512))
                        | (Some(768), Some(768))
                        | (Some(1024), Some(1024))
                        | (Some(512), Some(256))
                        | (Some(768), Some(384))
                        | (Some(1024), Some(512))
                )
            }),
        });

        // Battle sprite: side view, wider than tall
        self.rules.push(ClassificationRule {
            name: "battle_sprite".to_string(),
            asset_type: "battle_sprite".to_string(),
            confidence: 0.80,
            check: Box::new(|info| {
                matches!(
                    (info.width, info.height),
                    (Some(64), Some(64)) |     // SV battler base
                    (Some(128), Some(64)) |    // SV battler sheet
                    (Some(192), Some(64))
                ) // SV battler extended
            }),
        });

        // Background/Parallax: large, wide aspect ratio
        self.rules.push(ClassificationRule {
            name: "background".to_string(),
            asset_type: "background".to_string(),
            confidence: 0.85,
            check: Box::new(|info| {
                if let (Some(w), Some(h)) = (info.width, info.height) {
                    let aspect = w as f32 / h as f32;
                    // Wide backgrounds
                    (w >= 640 && aspect >= 1.5) ||
                    // Square backgrounds for battles
                    (w >= 640 && h >= 480 && (1.0..=1.5).contains(&aspect))
                } else {
                    false
                }
            }),
        });

        // Icon: very small, square
        self.rules.push(ClassificationRule {
            name: "icon".to_string(),
            asset_type: "icon".to_string(),
            confidence: 0.95,
            check: Box::new(|info| {
                matches!(
                    (info.width, info.height),
                    (Some(16), Some(16)) | (Some(24), Some(24)) | (Some(32), Some(32))
                )
            }),
        });

        // UI element: specific UI dimensions
        self.rules.push(ClassificationRule {
            name: "ui_element".to_string(),
            asset_type: "ui".to_string(),
            confidence: 0.75,
            check: Box::new(|info| {
                matches!(
                    (info.width, info.height),
                    // Common UI sizes
                    (Some(192), Some(64)) |    // Button strip
                    (Some(256), Some(64)) |    // Wide button
                    (Some(128), Some(128))
                ) // Panel element
            }),
        });

        // Animation sheet: tall and narrow or specific pattern
        self.rules.push(ClassificationRule {
            name: "animation_sheet".to_string(),
            asset_type: "animation".to_string(),
            confidence: 0.80,
            check: Box::new(|info| {
                matches!(
                    (info.width, info.height),
                    (Some(192), Some(192)) |   // 5x5 animation
                    (Some(256), Some(256)) |   // 5x5 or 8x8
                    (Some(384), Some(384)) |   // RPG Maker style
                    (Some(512), Some(512))
                ) // Large animation
            }),
        });
    }

    /// Classify an asset based on its info
    pub fn classify(&self, info: &AssetInfo) -> ClassificationResult {
        let mut best_match: Option<(String, f64, Vec<String>)> = None;

        for rule in &self.rules {
            if (rule.check)(info) {
                let current_confidence = best_match.as_ref().map(|(_, c, _)| *c).unwrap_or(0.0);
                if rule.confidence > current_confidence {
                    best_match = Some((
                        rule.asset_type.clone(),
                        rule.confidence,
                        vec![rule.name.clone()],
                    ));
                }
            }
        }

        if let Some((asset_type, confidence, rules_matched)) = best_match {
            ClassificationResult {
                detected_type: asset_type,
                confidence,
                rules_matched,
                metadata: serde_json::json!({
                    "file_name": &info.file_name,
                    "dimensions": [info.width, info.height],
                    "file_size": info.file_size,
                }),
            }
        } else {
            // No rule matched - generic image
            ClassificationResult {
                detected_type: "image".to_string(),
                confidence: 0.5,
                rules_matched: vec![],
                metadata: serde_json::json!({
                    "file_name": &info.file_name,
                    "dimensions": [info.width, info.height],
                    "file_size": info.file_size,
                }),
            }
        }
    }

    /// Analyze image file to extract info
    pub async fn analyze_image<P: AsRef<Path>>(path: P) -> crate::Result<AssetInfo> {
        let path = path.as_ref();
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file_path = path.to_string_lossy().to_string();
        let metadata = tokio::fs::metadata(path).await?;
        let file_size = metadata.len();

        // Try to read image dimensions
        let (width, height, format) = if let Ok(data) = tokio::fs::read(path).await {
            if let Ok(img) = image::load_from_memory(&data) {
                let width = img.width();
                let height = img.height();
                let format = match image::ImageFormat::from_path(path) {
                    Ok(image::ImageFormat::Png) => Some("png".to_string()),
                    Ok(image::ImageFormat::Jpeg) => Some("jpg".to_string()),
                    Ok(image::ImageFormat::WebP) => Some("webp".to_string()),
                    _ => Some("unknown".to_string()),
                };
                (Some(width), Some(height), format)
            } else {
                (None, None, None)
            }
        } else {
            (None, None, None)
        };

        Ok(AssetInfo {
            file_name,
            file_path,
            file_size,
            width,
            height,
            has_alpha: None, // Would need format-specific detection
            format,
        })
    }
}

impl Default for AssetClassifier {
    fn default() -> Self {
        Self::new()
    }
}
