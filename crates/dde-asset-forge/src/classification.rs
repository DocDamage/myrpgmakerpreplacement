//! Asset Classification Engine
//!
//! Determines asset types based on deterministic rules first,
//! then uses simple heuristics for ambiguous cases.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::asset_os::NewAsset;
use notify::Watcher;

/// Classification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub detected_type: String,
    pub confidence: f64,
    pub rules_matched: Vec<String>,
    pub metadata: serde_json::Value,
    pub auto_tags: Vec<String>,
}

/// Serializable classification rule for storage and UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationRule {
    pub id: String,
    pub name: String,
    pub file_pattern: String,
    pub asset_type: String,
    pub auto_tags: Vec<String>,
    pub priority: i32,
    pub enabled: bool,
    pub exact_dimensions: Option<(u32, u32)>,
    pub min_width: Option<u32>,
    pub max_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_height: Option<u32>,
    pub confidence: f64,
}

impl Default for ClassificationRule {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "New Rule".to_string(),
            file_pattern: "*.png".to_string(),
            asset_type: "character".to_string(),
            auto_tags: vec![],
            priority: 50,
            enabled: true,
            exact_dimensions: None,
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            confidence: 0.85,
        }
    }
}

impl ClassificationRule {
    /// Check if a file matches this rule
    pub fn matches(&self, file_name: &str, width: Option<u32>, height: Option<u32>) -> bool {
        if !self.enabled {
            return false;
        }

        // Check file pattern
        if !self.pattern_matches(file_name) {
            return false;
        }

        // Check dimensions
        self.dimensions_match(width, height)
    }

    /// Check if file name matches the pattern
    fn pattern_matches(&self, file_name: &str) -> bool {
        let pattern = &self.file_pattern;
        
        // Handle glob patterns
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let suffix = parts[1];
                file_name.starts_with(prefix) && file_name.ends_with(suffix)
            } else {
                // Multiple wildcards - use simple contains
                let regex_pattern = pattern.replace("*", ".*");
                if let Ok(re) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
                    re.is_match(file_name)
                } else {
                    file_name.contains(&pattern.replace('*', ""))
                }
            }
        } else if pattern.starts_with('.') || pattern.contains('.') {
            // Extension pattern like ".png" or "*.png"
            let ext = pattern.trim_start_matches("*.").trim_start_matches('.');
            file_name.ends_with(&format!(".{}", ext))
        } else {
            file_name == pattern || file_name.starts_with(pattern)
        }
    }

    /// Check if dimensions match the rule
    fn dimensions_match(&self, width: Option<u32>, height: Option<u32>) -> bool {
        // If no dimension constraints, match any
        if self.exact_dimensions.is_none() 
            && self.min_width.is_none() 
            && self.max_width.is_none()
            && self.min_height.is_none() 
            && self.max_height.is_none() {
            return true;
        }

        // If we have dimensions to check against
        if let (Some(w), Some(h)) = (width, height) {
            // Check exact dimensions
            if let Some((exact_w, exact_h)) = self.exact_dimensions {
                return w == exact_w && h == exact_h;
            }

            // Check min/max constraints
            if let Some(min_w) = self.min_width {
                if w < min_w {
                    return false;
                }
            }
            if let Some(max_w) = self.max_width {
                if w > max_w {
                    return false;
                }
            }
            if let Some(min_h) = self.min_height {
                if h < min_h {
                    return false;
                }
            }
            if let Some(max_h) = self.max_height {
                if h > max_h {
                    return false;
                }
            }
            return true;
        }

        // No dimensions provided, but rule requires them
        false
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

/// File system event for watching
#[derive(Debug, Clone)]
pub enum FileSystemEvent {
    Created { path: std::path::PathBuf },
    Modified { path: std::path::PathBuf },
    Deleted { path: std::path::PathBuf },
}

/// Classification engine that manages rules and performs classification
pub struct ClassificationEngine {
    rules: Vec<ClassificationRule>,
    file_watcher: Option<notify::RecommendedWatcher>,
    fs_event_tx: Option<mpsc::UnboundedSender<FileSystemEvent>>,
    fs_event_rx: Option<mpsc::UnboundedReceiver<FileSystemEvent>>,
    watched_paths: Vec<std::path::PathBuf>,
}

impl Default for ClassificationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ClassificationEngine {
    /// Create a new classification engine
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            file_watcher: None,
            fs_event_tx: None,
            fs_event_rx: None,
            watched_paths: Vec::new(),
        }
    }

    /// Initialize with default rules
    pub fn with_defaults(mut self) -> Self {
        self.add_default_rules();
        self
    }

    /// Load rules from database
    pub fn load_from_db(&mut self, db: &dde_db::Database) -> crate::Result<()> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, file_pattern, asset_type, auto_tags_json, priority, 
                    enabled, exact_width, exact_height, min_width, max_width, 
                    min_height, max_height, confidence
             FROM classification_rules 
             ORDER BY priority DESC"
        ).map_err(|e| crate::AssetForgeError::Database(e.into()))?;

        let rules = stmt.query_map([], |row| {
            let exact_w: Option<i64> = row.get(7)?;
            let exact_h: Option<i64> = row.get(8)?;
            let exact_dimensions = match (exact_w, exact_h) {
                (Some(w), Some(h)) => Some((w as u32, h as u32)),
                _ => None,
            };

            let auto_tags_json: String = row.get(4)?;
            let auto_tags: Vec<String> = serde_json::from_str(&auto_tags_json)
                .unwrap_or_default();

            Ok(ClassificationRule {
                id: row.get(0)?,
                name: row.get(1)?,
                file_pattern: row.get(2)?,
                asset_type: row.get(3)?,
                auto_tags,
                priority: row.get(5)?,
                enabled: row.get(6)?,
                exact_dimensions,
                min_width: row.get::<_, Option<i64>>(9)?.map(|v| v as u32),
                max_width: row.get::<_, Option<i64>>(10)?.map(|v| v as u32),
                min_height: row.get::<_, Option<i64>>(11)?.map(|v| v as u32),
                max_height: row.get::<_, Option<i64>>(12)?.map(|v| v as u32),
                confidence: row.get(13)?,
            })
        }).map_err(|e| crate::AssetForgeError::Database(e.into()))?;

        self.rules.clear();
        for rule in rules {
            self.rules.push(rule.map_err(|e| crate::AssetForgeError::Database(e.into()))?);
        }

        tracing::info!("Loaded {} classification rules from database", self.rules.len());
        Ok(())
    }

    /// Save rules to database
    pub fn save_to_db(&self, db: &mut dde_db::Database) -> crate::Result<()> {
        let tx = db.transaction()?;

        // Clear existing rules
        tx.execute("DELETE FROM classification_rules", [])
            .map_err(|e| crate::AssetForgeError::Database(dde_db::DbError::Sqlite(e)))?;

        // Insert all rules
        for rule in &self.rules {
            let auto_tags_json = serde_json::to_string(&rule.auto_tags)
                .map_err(crate::AssetForgeError::Serialization)?;

            tx.execute(
                "INSERT INTO classification_rules 
                 (id, name, file_pattern, asset_type, auto_tags_json, priority, 
                  enabled, exact_width, exact_height, min_width, max_width, 
                  min_height, max_height, confidence, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                (
                    &rule.id,
                    &rule.name,
                    &rule.file_pattern,
                    &rule.asset_type,
                    auto_tags_json,
                    rule.priority,
                    rule.enabled,
                    rule.exact_dimensions.map(|(w, _)| w as i64),
                    rule.exact_dimensions.map(|(_, h)| h as i64),
                    rule.min_width.map(|v| v as i64),
                    rule.max_width.map(|v| v as i64),
                    rule.min_height.map(|v| v as i64),
                    rule.max_height.map(|v| v as i64),
                    rule.confidence,
                    chrono::Utc::now().timestamp_millis(),
                ),
            ).map_err(|e| crate::AssetForgeError::Database(dde_db::DbError::Sqlite(e)))?;
        }

        tx.commit().map_err(|e| crate::AssetForgeError::Database(e.into()))?;
        tracing::info!("Saved {} classification rules to database", self.rules.len());
        Ok(())
    }

    /// Add a new rule
    pub fn add_rule(&mut self, rule: ClassificationRule) {
        self.rules.push(rule);
        self.sort_rules();
    }

    /// Remove a rule by ID
    pub fn remove_rule(&mut self, rule_id: &str) -> bool {
        let initial_len = self.rules.len();
        self.rules.retain(|r| r.id != rule_id);
        self.rules.len() < initial_len
    }

    /// Update an existing rule
    pub fn update_rule(&mut self, rule_id: &str, updater: impl FnOnce(&mut ClassificationRule)) -> bool {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.id == rule_id) {
            updater(rule);
            self.sort_rules();
            true
        } else {
            false
        }
    }

    /// Get all rules
    pub fn rules(&self) -> &[ClassificationRule] {
        &self.rules
    }

    /// Get mutable rules
    pub fn rules_mut(&mut self) -> &mut Vec<ClassificationRule> {
        &mut self.rules
    }

    /// Get a rule by ID
    pub fn get_rule(&self, rule_id: &str) -> Option<&ClassificationRule> {
        self.rules.iter().find(|r| r.id == rule_id)
    }

    /// Move rule up in priority (higher priority number)
    pub fn move_rule_up(&mut self, rule_id: &str) -> bool {
        if let Some(idx) = self.rules.iter().position(|r| r.id == rule_id) {
            if idx > 0 {
                self.rules.swap(idx, idx - 1);
                self.sync_priorities_to_order();
                return true;
            }
        }
        false
    }

    /// Move rule down in priority (lower priority number)
    pub fn move_rule_down(&mut self, rule_id: &str) -> bool {
        if let Some(idx) = self.rules.iter().position(|r| r.id == rule_id) {
            if idx < self.rules.len().saturating_sub(1) {
                self.rules.swap(idx, idx + 1);
                self.sync_priorities_to_order();
                return true;
            }
        }
        false
    }

    /// Sync priority values to current order (highest first)
    fn sync_priorities_to_order(&mut self) {
        let count = self.rules.len();
        for (i, rule) in self.rules.iter_mut().enumerate() {
            rule.priority = ((count - i) * 10) as i32;
        }
    }

    /// Sort rules by priority (highest first)
    fn sort_rules(&mut self) {
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Add default classification rules
    fn add_default_rules(&mut self) {
        let defaults = vec![
            ClassificationRule {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Character Sprites (32x32)".to_string(),
                file_pattern: "character_*".to_string(),
                asset_type: "character".to_string(),
                auto_tags: vec!["character".to_string(), "animated".to_string()],
                priority: 100,
                enabled: true,
                exact_dimensions: Some((32, 32)),
                min_width: None,
                max_width: None,
                min_height: None,
                max_height: None,
                confidence: 0.95,
            },
            ClassificationRule {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Character Sprites (64x64)".to_string(),
                file_pattern: "character_*".to_string(),
                asset_type: "character".to_string(),
                auto_tags: vec!["character".to_string(), "animated".to_string(), "high_res".to_string()],
                priority: 95,
                enabled: true,
                exact_dimensions: Some((64, 64)),
                min_width: None,
                max_width: None,
                min_height: None,
                max_height: None,
                confidence: 0.95,
            },
            ClassificationRule {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Item Icons".to_string(),
                file_pattern: "item_*".to_string(),
                asset_type: "item".to_string(),
                auto_tags: vec!["item".to_string(), "icon".to_string()],
                priority: 90,
                enabled: true,
                exact_dimensions: Some((32, 32)),
                min_width: None,
                max_width: None,
                min_height: None,
                max_height: None,
                confidence: 0.90,
            },
            ClassificationRule {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Portraits (64x64)".to_string(),
                file_pattern: "face_*".to_string(),
                asset_type: "portrait".to_string(),
                auto_tags: vec!["portrait".to_string(), "face".to_string()],
                priority: 85,
                enabled: true,
                exact_dimensions: Some((64, 64)),
                min_width: None,
                max_width: None,
                min_height: None,
                max_height: None,
                confidence: 0.90,
            },
            ClassificationRule {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Spell Effects".to_string(),
                file_pattern: "effect_*".to_string(),
                asset_type: "effect".to_string(),
                auto_tags: vec!["effect".to_string(), "animated".to_string()],
                priority: 80,
                enabled: true,
                exact_dimensions: Some((192, 192)),
                min_width: None,
                max_width: None,
                min_height: None,
                max_height: None,
                confidence: 0.85,
            },
            ClassificationRule {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Tilesets".to_string(),
                file_pattern: "*.tsx".to_string(),
                asset_type: "tileset".to_string(),
                auto_tags: vec!["tileset".to_string(), "terrain".to_string()],
                priority: 75,
                enabled: true,
                exact_dimensions: None,
                min_width: None,
                max_width: None,
                min_height: None,
                max_height: None,
                confidence: 0.80,
            },
            ClassificationRule {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Sprite Sheets".to_string(),
                file_pattern: "*_sheet.png".to_string(),
                asset_type: "sprite_sheet".to_string(),
                auto_tags: vec!["sprite_sheet".to_string(), "animated".to_string()],
                priority: 70,
                enabled: true,
                exact_dimensions: None,
                min_width: Some(128),
                max_width: None,
                min_height: Some(32),
                max_height: Some(128),
                confidence: 0.85,
            },
            ClassificationRule {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Backgrounds".to_string(),
                file_pattern: "bg_*".to_string(),
                asset_type: "background".to_string(),
                auto_tags: vec!["background".to_string(), "parallax".to_string()],
                priority: 65,
                enabled: true,
                exact_dimensions: None,
                min_width: Some(640),
                max_width: None,
                min_height: Some(480),
                max_height: None,
                confidence: 0.80,
            },
            ClassificationRule {
                id: uuid::Uuid::new_v4().to_string(),
                name: "UI Elements".to_string(),
                file_pattern: "ui_*".to_string(),
                asset_type: "ui".to_string(),
                auto_tags: vec!["ui".to_string(), "hud".to_string()],
                priority: 60,
                enabled: true,
                exact_dimensions: None,
                min_width: None,
                max_width: Some(512),
                min_height: None,
                max_height: Some(256),
                confidence: 0.75,
            },
            ClassificationRule {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Battle Sprites".to_string(),
                file_pattern: "battle_*".to_string(),
                asset_type: "battle_sprite".to_string(),
                auto_tags: vec!["battle".to_string(), "sv_battler".to_string()],
                priority: 55,
                enabled: true,
                exact_dimensions: None,
                min_width: Some(64),
                max_width: Some(192),
                min_height: Some(64),
                max_height: Some(64),
                confidence: 0.85,
            },
        ];

        self.rules = defaults;
        self.sort_rules();
    }

    /// Classify an asset based on its info
    pub fn classify(&self, info: &AssetInfo) -> ClassificationResult {
        let mut best_match: Option<(&ClassificationRule, f64)> = None;
        let mut all_matched_rules: Vec<String> = Vec::new();

        for rule in &self.rules {
            if rule.matches(&info.file_name, info.width, info.height) {
                all_matched_rules.push(rule.name.clone());
                
                let current_confidence = best_match.map(|(_, c)| c).unwrap_or(0.0);
                if rule.priority as f64 > current_confidence || 
                   (rule.priority as f64 == current_confidence && rule.confidence > best_match.map(|(r, _)| r.confidence).unwrap_or(0.0)) {
                    best_match = Some((rule, rule.priority as f64));
                }
            }
        }

        if let Some((rule, _)) = best_match {
            ClassificationResult {
                detected_type: rule.asset_type.clone(),
                confidence: rule.confidence,
                rules_matched: all_matched_rules,
                metadata: serde_json::json!({
                    "file_name": &info.file_name,
                    "dimensions": [info.width, info.height],
                    "file_size": info.file_size,
                    "rule_id": &rule.id,
                }),
                auto_tags: rule.auto_tags.clone(),
            }
        } else {
            // No rule matched - use heuristic classification
            self.heuristic_classify(info)
        }
    }

    /// Heuristic classification when no rules match
    fn heuristic_classify(&self, info: &AssetInfo) -> ClassificationResult {
        let (detected_type, confidence) = if let (Some(w), Some(h)) = (info.width, info.height) {
            match (w, h) {
                // Character sprites (small square)
                (32, 32) => ("character", 0.7),
                // Character sprites (large square)
                (64, 64) => ("portrait", 0.7),
                // Portrait/Face (high-res)
                (96, 96) => ("portrait", 0.7),
                // Icon (very small square)
                (16, 16) | (24, 24) => ("icon", 0.75),
                // Tileset (power of 2, square-ish)
                _ if w >= 256 && h >= 256 && w == h => ("tileset", 0.6),
                // Background (wide)
                _ if w >= 640 && w > h => ("background", 0.6),
                // Animation sheet
                _ if w >= 192 && h >= 192 && w == h => ("animation", 0.55),
                // Default
                _ => ("image", 0.5),
            }
        } else {
            ("unknown", 0.3)
        };

        ClassificationResult {
            detected_type: detected_type.to_string(),
            confidence,
            rules_matched: vec![],
            metadata: serde_json::json!({
                "file_name": &info.file_name,
                "dimensions": [info.width, info.height],
                "file_size": info.file_size,
                "heuristic": true,
            }),
            auto_tags: vec![],
        }
    }

    /// Test a pattern against a list of files
    pub fn test_pattern(&self, files: &[TestFile]) -> Vec<PatternTestResult> {
        files.iter().map(|file| {
            let mut matched_rules = Vec::new();
            let mut matched = false;

            for rule in &self.rules {
                if rule.matches(&file.file_name, file.width, file.height) {
                    matched = true;
                    matched_rules.push(MatchedRuleInfo {
                        rule_name: rule.name.clone(),
                        rule_id: rule.id.clone(),
                        priority: rule.priority,
                        asset_type: rule.asset_type.clone(),
                        auto_tags: rule.auto_tags.clone(),
                    });
                }
            }

            // Sort matched rules by priority (highest first)
            matched_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

            PatternTestResult {
                file_path: file.file_path.clone(),
                file_name: file.file_name.clone(),
                width: file.width,
                height: file.height,
                matched,
                matched_rules,
            }
        }).collect()
    }

    /// Scan a directory and return test files
    pub async fn scan_directory<P: AsRef<Path>>(path: P) -> crate::Result<Vec<TestFile>> {
        let path = path.as_ref();
        let mut files = Vec::new();

        let mut entries = tokio::fs::read_dir(path).await
            .map_err(|e| crate::AssetForgeError::Io(e))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| crate::AssetForgeError::Io(e))? {
            let file_path = entry.path();
            let file_name = file_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Skip directories for now
            if file_path.is_dir() {
                // Recursively scan subdirectories
                let sub_files = Self::scan_directory(&file_path).await?;
                files.extend(sub_files);
                continue;
            }

            // Get file metadata
            let metadata = tokio::fs::metadata(&file_path).await
                .map_err(|e| crate::AssetForgeError::Io(e))?;

            // Try to read image dimensions
            let (width, height) = if let Ok(data) = tokio::fs::read(&file_path).await {
                if let Ok(img) = image::load_from_memory(&data) {
                    (Some(img.width()), Some(img.height()))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            files.push(TestFile {
                file_path: file_path.to_string_lossy().to_string(),
                file_name,
                width,
                height,
                file_size: metadata.len(),
            });
        }

        Ok(files)
    }

    /// Start watching a directory for changes
    pub fn watch_directory<P: AsRef<Path>>(&mut self, path: P) -> crate::Result<mpsc::UnboundedReceiver<FileSystemEvent>> {
        let path = path.as_ref().to_path_buf();
        
        if self.watched_paths.contains(&path) {
            // Already watching this path, return existing receiver
            return self.fs_event_rx.take()
                .ok_or_else(|| crate::AssetForgeError::Ipc("File watcher not initialized".to_string()));
        }

        let (tx, rx) = mpsc::unbounded_channel();
        self.fs_event_tx = Some(tx.clone());
        self.fs_event_rx = Some(rx);

        let watcher_tx = tx.clone();
        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                for path in event.paths {
                    let fs_event = match event.kind {
                        notify::EventKind::Create(_) => Some(FileSystemEvent::Created { path }),
                        notify::EventKind::Modify(_) => Some(FileSystemEvent::Modified { path }),
                        notify::EventKind::Remove(_) => Some(FileSystemEvent::Deleted { path }),
                        _ => None,
                    };
                    
                    if let Some(evt) = fs_event {
                        let _ = watcher_tx.send(evt);
                    }
                }
            }
        }).map_err(crate::AssetForgeError::Notify)?;

        watcher.watch(&path, notify::RecursiveMode::Recursive)
            .map_err(crate::AssetForgeError::Notify)?;

        self.file_watcher = Some(watcher);
        self.watched_paths.push(path);

        self.fs_event_rx.take()
            .ok_or_else(|| crate::AssetForgeError::Ipc("Failed to create file watcher".to_string()))
    }

    /// Stop watching directories
    pub fn stop_watching(&mut self) {
        self.file_watcher = None;
        self.fs_event_tx = None;
        self.fs_event_rx = None;
        self.watched_paths.clear();
    }

    /// Process pending file system events
    pub async fn process_fs_events(&mut self, asset_os: &mut crate::AssetOs) -> crate::Result<Vec<ClassificationResult>> {
        use std::collections::VecDeque;
        
        // Collect events first to avoid borrow issues
        let events: VecDeque<FileSystemEvent> = if let Some(ref mut rx) = self.fs_event_rx {
            let mut collected = VecDeque::new();
            while let Ok(event) = rx.try_recv() {
                collected.push_back(event);
            }
            collected
        } else {
            return Ok(Vec::new());
        };
        
        // Now process events without borrowing self
        let mut results = Vec::new();
        for event in events {
            match event {
                FileSystemEvent::Created { path } | FileSystemEvent::Modified { path } => {
                    // Analyze and classify the file
                    if let Ok(info) = Self::analyze_file(&path).await {
                        // Clone necessary data before calling classify
                        let file_name = info.file_name.clone();
                        let file_path = info.file_path.clone();
                        let file_size = info.file_size;
                        
                        let result = self.classify(&info);
                        
                        // Auto-classify if confidence is high enough
                        if result.confidence >= 0.7 {
                            // Ingest into AssetOs
                            let new_asset = NewAsset {
                                name: file_name,
                                asset_type: result.detected_type.clone(),
                                file_path: file_path.clone(),
                                file_hash: Self::compute_file_hash(&path).await.unwrap_or_default(),
                                file_size: file_size as i64,
                                metadata: result.metadata.clone(),
                            };
                            
                            if let Ok(asset_id) = asset_os.ingest_asset(new_asset).await {
                                // Apply auto-tags
                                for tag in &result.auto_tags {
                                    let _ = asset_os.add_tag(asset_id, tag);
                                }
                                
                                tracing::info!("Auto-classified new file {} as {} (confidence: {:.2})", 
                                    path.display(), result.detected_type, result.confidence);
                            }
                        }
                        
                        results.push(result);
                    }
                }
                FileSystemEvent::Deleted { path } => {
                    tracing::debug!("File deleted: {}", path.display());
                }
            }
        }
        
        Ok(results)
    }

    /// Analyze a single file
    pub async fn analyze_file<P: AsRef<Path>>(path: P) -> crate::Result<AssetInfo> {
        let path = path.as_ref();
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file_path = path.to_string_lossy().to_string();
        let metadata = tokio::fs::metadata(path).await
            .map_err(|e| crate::AssetForgeError::Io(e))?;
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
            has_alpha: None,
            format,
        })
    }

    /// Compute SHA256 hash of a file
    async fn compute_file_hash<P: AsRef<Path>>(path: P) -> crate::Result<String> {
        use sha2::{Sha256, Digest};
        
        let data = tokio::fs::read(path).await
            .map_err(|e| crate::AssetForgeError::Io(e))?;
        
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let result = hasher.finalize();
        
        Ok(format!("{:x}", result))
    }
}

/// Test file info for pattern testing
#[derive(Debug, Clone)]
pub struct TestFile {
    pub file_path: String,
    pub file_name: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub file_size: u64,
}

/// Pattern test result for a single file
#[derive(Debug, Clone)]
pub struct PatternTestResult {
    pub file_path: String,
    pub file_name: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub matched: bool,
    pub matched_rules: Vec<MatchedRuleInfo>,
}

/// Info about a matched rule
#[derive(Debug, Clone)]
pub struct MatchedRuleInfo {
    pub rule_name: String,
    pub rule_id: String,
    pub priority: i32,
    pub asset_type: String,
    pub auto_tags: Vec<String>,
}

// Legacy AssetClassifier for backward compatibility
/// Asset classifier (legacy, use ClassificationEngine instead)
pub struct AssetClassifier {
    engine: ClassificationEngine,
}

impl Default for AssetClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetClassifier {
    /// Create a new classifier with default rules
    pub fn new() -> Self {
        Self {
            engine: ClassificationEngine::new().with_defaults(),
        }
    }

    /// Classify an asset based on its info
    pub fn classify(&self, info: &AssetInfo) -> ClassificationResult {
        self.engine.classify(info)
    }

    /// Analyze image file to extract info
    pub async fn analyze_image<P: AsRef<Path>>(path: P) -> crate::Result<AssetInfo> {
        ClassificationEngine::analyze_file(path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classification_rule_pattern_matching() {
        let rule = ClassificationRule {
            file_pattern: "character_*.png".to_string(),
            ..Default::default()
        };

        assert!(rule.matches("character_hero.png", None, None));
        assert!(rule.matches("character_villain.png", None, None));
        assert!(!rule.matches("item_sword.png", None, None));
        assert!(!rule.matches("character_hero.jpg", None, None));
    }

    #[test]
    fn test_classification_rule_dimension_matching() {
        let rule = ClassificationRule {
            file_pattern: "*.png".to_string(),
            exact_dimensions: Some((32, 32)),
            ..Default::default()
        };

        assert!(rule.matches("test.png", Some(32), Some(32)));
        assert!(!rule.matches("test.png", Some(64), Some(64)));
        assert!(!rule.matches("test.png", None, None));
    }

    #[test]
    fn test_classification_rule_min_max_dimensions() {
        let rule = ClassificationRule {
            file_pattern: "*.png".to_string(),
            min_width: Some(64),
            max_width: Some(256),
            min_height: Some(64),
            max_height: Some(256),
            ..Default::default()
        };

        assert!(rule.matches("test.png", Some(128), Some(128)));
        assert!(rule.matches("test.png", Some(64), Some(64)));
        assert!(rule.matches("test.png", Some(256), Some(256)));
        assert!(!rule.matches("test.png", Some(32), Some(32)));
        assert!(!rule.matches("test.png", Some(512), Some(512)));
    }

    #[test]
    fn test_classification_rule_disabled() {
        let rule = ClassificationRule {
            file_pattern: "*.png".to_string(),
            enabled: false,
            ..Default::default()
        };

        assert!(!rule.matches("test.png", None, None));
    }

    #[test]
    fn test_classification_engine_crud() {
        let mut engine = ClassificationEngine::new();
        
        // Add rule
        let rule = ClassificationRule {
            name: "Test Rule".to_string(),
            ..Default::default()
        };
        let rule_id = rule.id.clone();
        engine.add_rule(rule);
        
        assert_eq!(engine.rules().len(), 1);
        assert!(engine.get_rule(&rule_id).is_some());
        
        // Update rule
        engine.update_rule(&rule_id, |r| {
            r.name = "Updated Rule".to_string();
        });
        assert_eq!(engine.get_rule(&rule_id).unwrap().name, "Updated Rule");
        
        // Remove rule
        assert!(engine.remove_rule(&rule_id));
        assert_eq!(engine.rules().len(), 0);
    }

    #[test]
    fn test_classification_engine_move_rules() {
        let mut engine = ClassificationEngine::new();
        
        engine.add_rule(ClassificationRule {
            id: "rule1".to_string(),
            name: "Rule 1".to_string(),
            priority: 10,
            ..Default::default()
        });
        engine.add_rule(ClassificationRule {
            id: "rule2".to_string(),
            name: "Rule 2".to_string(),
            priority: 20,
            ..Default::default()
        });
        
        // Rule 2 should be first (higher priority)
        assert_eq!(engine.rules()[0].id, "rule2");
        
        // Move rule1 up
        engine.move_rule_up("rule1");
        assert_eq!(engine.rules()[0].id, "rule1");
        
        // Move rule1 down
        engine.move_rule_down("rule1");
        assert_eq!(engine.rules()[0].id, "rule2");
    }

    #[test]
    fn test_extension_pattern() {
        let rule = ClassificationRule {
            file_pattern: "*.png".to_string(),
            ..Default::default()
        };

        assert!(rule.matches("test.png", None, None));
        assert!(rule.matches("character_hero.png", None, None));
        assert!(!rule.matches("test.jpg", None, None));
    }
}
