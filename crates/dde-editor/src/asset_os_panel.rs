//! Asset OS Pipeline Panel
//!
//! Kanban-style UI for managing the asset workflow pipeline:
//! Inbox → Staging → Review → Production → Archive
//!
//! Features:
//! - Pipeline stage columns with drag-and-drop
//! - Asset cards with thumbnails and metadata
//! - Review queue with side-by-side comparison
//! - Batch operations for multiple assets
//! - Filtering by type, status, reviewer

use dde_asset_forge::asset_os::{AssetOs, AssetPipelineStage, AssetRecord, AssetReview};
use std::collections::{HashMap, HashSet};

/// Pipeline stage column configuration
#[derive(Debug, Clone)]
pub struct PipelineColumn {
    pub stage: AssetPipelineStage,
    pub display_name: String,
    pub color: egui::Color32,
    pub icon: &'static str,
}

impl PipelineColumn {
    pub fn new(stage: AssetPipelineStage) -> Self {
        match stage {
            AssetPipelineStage::Inbox => Self {
                stage: AssetPipelineStage::Inbox,
                display_name: "📥 Inbox".to_string(),
                color: egui::Color32::from_rgb(100, 149, 237), // Cornflower blue
                icon: "📥",
            },
            AssetPipelineStage::Staging => Self {
                stage: AssetPipelineStage::Staging,
                display_name: "🔧 Staging".to_string(),
                color: egui::Color32::from_rgb(255, 165, 0), // Orange
                icon: "🔧",
            },
            AssetPipelineStage::Review => Self {
                stage: AssetPipelineStage::Review,
                display_name: "👁 Review".to_string(),
                color: egui::Color32::from_rgb(147, 112, 219), // Medium purple
                icon: "👁",
            },
            AssetPipelineStage::Approved => Self {
                stage: AssetPipelineStage::Approved,
                display_name: "✅ Production".to_string(),
                color: egui::Color32::from_rgb(50, 205, 50), // Lime green
                icon: "✅",
            },
            AssetPipelineStage::Rejected => Self {
                stage: AssetPipelineStage::Rejected,
                display_name: "❌ Rejected".to_string(),
                color: egui::Color32::from_rgb(220, 20, 60), // Crimson
                icon: "❌",
            },
        }
    }

    pub fn all_columns() -> Vec<Self> {
        vec![
            Self::new(AssetPipelineStage::Inbox),
            Self::new(AssetPipelineStage::Staging),
            Self::new(AssetPipelineStage::Review),
            Self::new(AssetPipelineStage::Approved),
            Self::new(AssetPipelineStage::Rejected),
        ]
    }
}

/// Asset type for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetTypeFilter {
    All,
    Character,
    Item,
    Tileset,
    Effect,
    Portrait,
    SpriteSheet,
    Background,
    Icon,
    Ui,
    Animation,
    BattleSprite,
    Audio,
    Other,
}

impl AssetTypeFilter {
    pub fn display_name(&self) -> &'static str {
        match self {
            AssetTypeFilter::All => "All Types",
            AssetTypeFilter::Character => "Character",
            AssetTypeFilter::Item => "Item",
            AssetTypeFilter::Tileset => "Tileset",
            AssetTypeFilter::Effect => "Effect",
            AssetTypeFilter::Portrait => "Portrait",
            AssetTypeFilter::SpriteSheet => "Sprite Sheet",
            AssetTypeFilter::Background => "Background",
            AssetTypeFilter::Icon => "Icon",
            AssetTypeFilter::Ui => "UI",
            AssetTypeFilter::Animation => "Animation",
            AssetTypeFilter::BattleSprite => "Battle Sprite",
            AssetTypeFilter::Audio => "Audio",
            AssetTypeFilter::Other => "Other",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            AssetTypeFilter::All,
            AssetTypeFilter::Character,
            AssetTypeFilter::Item,
            AssetTypeFilter::Tileset,
            AssetTypeFilter::Effect,
            AssetTypeFilter::Portrait,
            AssetTypeFilter::SpriteSheet,
            AssetTypeFilter::Background,
            AssetTypeFilter::Icon,
            AssetTypeFilter::Ui,
            AssetTypeFilter::Animation,
            AssetTypeFilter::BattleSprite,
            AssetTypeFilter::Audio,
            AssetTypeFilter::Other,
        ]
    }

    pub fn matches(&self, asset_type: &str) -> bool {
        match self {
            AssetTypeFilter::All => true,
            AssetTypeFilter::Character => asset_type == "character",
            AssetTypeFilter::Item => asset_type == "item",
            AssetTypeFilter::Tileset => asset_type == "tileset",
            AssetTypeFilter::Effect => asset_type == "effect",
            AssetTypeFilter::Portrait => asset_type == "portrait",
            AssetTypeFilter::SpriteSheet => asset_type == "sprite_sheet",
            AssetTypeFilter::Background => asset_type == "background",
            AssetTypeFilter::Icon => asset_type == "icon",
            AssetTypeFilter::Ui => asset_type == "ui",
            AssetTypeFilter::Animation => asset_type == "animation",
            AssetTypeFilter::BattleSprite => asset_type == "battle_sprite",
            AssetTypeFilter::Audio => asset_type == "audio",
            AssetTypeFilter::Other => ![
                "character", "item", "tileset", "effect", "portrait",
                "sprite_sheet", "background", "icon", "ui", "animation",
                "battle_sprite", "audio",
            ].contains(&asset_type),
        }
    }
}

/// Status filter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusFilter {
    All,
    Pending,
    InProgress,
    NeedsAttention,
}

impl StatusFilter {
    pub fn display_name(&self) -> &'static str {
        match self {
            StatusFilter::All => "All Status",
            StatusFilter::Pending => "Pending",
            StatusFilter::InProgress => "In Progress",
            StatusFilter::NeedsAttention => "Needs Attention",
        }
    }
}

/// Asset card display data
#[derive(Debug, Clone)]
pub struct AssetCard {
    pub asset: AssetRecord,
    pub thumbnail: Option<egui::TextureHandle>,
    pub tags: Vec<String>,
    pub reviewer: Option<String>,
    pub review_score: Option<i32>,
    pub is_selected: bool,
    pub is_dragging: bool,
    pub version: u32,
}

impl AssetCard {
    pub fn from_record(asset: AssetRecord) -> Self {
        // Extract tags from metadata
        let tags = asset
            .metadata
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Extract reviewer from metadata
        let reviewer = asset
            .metadata
            .get("reviewer")
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        // Extract version from metadata
        let version = asset
            .metadata
            .get("version")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(1);

        Self {
            asset,
            thumbnail: None,
            tags,
            reviewer,
            review_score: None,
            is_selected: false,
            is_dragging: false,
            version,
        }
    }

    pub fn id(&self) -> i64 {
        self.asset.asset_id
    }

    pub fn name(&self) -> &str {
        &self.asset.name
    }

    pub fn asset_type(&self) -> &str {
        &self.asset.asset_type
    }

    pub fn status(&self) -> &str {
        &self.asset.status
    }
}

/// Review queue item
#[derive(Debug, Clone)]
pub struct ReviewQueueItem {
    pub asset: AssetRecord,
    pub review: Option<AssetReview>,
    pub comparison_asset: Option<AssetRecord>,
}

/// Archive entry for old versions
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub asset_id: i64,
    pub name: String,
    pub archived_at: i64,
    pub version: u32,
    pub replaced_by: Option<i64>,
}

/// Batch operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchOperation {
    MoveToStage(AssetPipelineStage),
    AddTag(String),
    RemoveTag(String),
    AssignReviewer(String),
    Approve,
    Reject,
    Archive,
    Delete,
}

/// Asset OS Pipeline Panel
pub struct AssetOsPipelinePanel {
    /// Whether panel is visible
    visible: bool,
    /// Pipeline columns
    columns: Vec<PipelineColumn>,
    /// Assets organized by stage
    assets_by_stage: HashMap<AssetPipelineStage, Vec<AssetCard>>,
    /// Currently selected assets (for batch operations)
    selected_assets: HashSet<i64>,
    /// Dragging asset ID
    dragging_asset: Option<i64>,
    /// Drag target stage
    drag_target: Option<AssetPipelineStage>,
    /// Current filters
    type_filter: AssetTypeFilter,
    status_filter: StatusFilter,
    reviewer_filter: Option<String>,
    search_query: String,
    /// Review queue
    review_queue: Vec<ReviewQueueItem>,
    /// Currently reviewing asset
    current_review: Option<usize>,
    /// Review comment being composed
    review_comment: String,
    /// Review score
    review_score: i32,
    /// Comparison mode
    compare_mode: bool,
    /// Archive entries
    archive_entries: Vec<ArchiveEntry>,
    /// Show archive view
    show_archive: bool,
    /// Batch operation in progress
    batch_operation: Option<BatchOperation>,
    /// New tag input
    new_tag_input: String,
    /// New reviewer input
    new_reviewer_input: String,
    /// Available reviewers
    available_reviewers: Vec<String>,
    /// Available tags
    available_tags: Vec<String>,
    /// Status message
    status_message: Option<String>,
    /// Status timeout
    status_timeout: f32,
    /// Column widths
    column_width: f32,
    /// Show rejected assets
    show_rejected: bool,
    /// Preview scale
    preview_scale: f32,
}

impl Default for AssetOsPipelinePanel {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetOsPipelinePanel {
    /// Create a new pipeline panel
    pub fn new() -> Self {
        let columns = PipelineColumn::all_columns();
        let mut assets_by_stage = HashMap::new();

        for col in &columns {
            assets_by_stage.insert(col.stage.clone(), Vec::new());
        }

        Self {
            visible: false,
            columns,
            assets_by_stage,
            selected_assets: HashSet::new(),
            dragging_asset: None,
            drag_target: None,
            type_filter: AssetTypeFilter::All,
            status_filter: StatusFilter::All,
            reviewer_filter: None,
            search_query: String::new(),
            review_queue: Vec::new(),
            current_review: None,
            review_comment: String::new(),
            review_score: 3,
            compare_mode: false,
            archive_entries: Vec::new(),
            show_archive: false,
            batch_operation: None,
            new_tag_input: String::new(),
            new_reviewer_input: String::new(),
            available_reviewers: vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
                "Lead Artist".to_string(),
                "Art Director".to_string(),
            ],
            available_tags: vec![
                "character".to_string(),
                "npc".to_string(),
                "enemy".to_string(),
                "item".to_string(),
                "weapon".to_string(),
                "armor".to_string(),
                "consumable".to_string(),
                "terrain".to_string(),
                "decoration".to_string(),
                "animated".to_string(),
                "static".to_string(),
                "ui".to_string(),
                "hud".to_string(),
                "menu".to_string(),
                "portrait".to_string(),
                "face".to_string(),
                "battle".to_string(),
                "overworld".to_string(),
                "high-priority".to_string(),
                "needs-rework".to_string(),
                "approved".to_string(),
            ],
            status_message: None,
            status_timeout: 0.0,
            column_width: 250.0,
            show_rejected: false,
            preview_scale: 1.0,
        }
    }

    /// Show the panel
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the panel
    pub fn hide(&mut self) {
        self.visible = false;
        self.selected_assets.clear();
        self.current_review = None;
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Update panel state (call each frame)
    pub fn update(&mut self, dt: f32) {
        if self.status_timeout > 0.0 {
            self.status_timeout -= dt;
            if self.status_timeout <= 0.0 {
                self.status_message = None;
            }
        }
    }

    /// Refresh asset data from database (mock implementation)
    pub fn refresh_assets(&mut self) {
        // In a real implementation, this would query the database
        // For now, we'll create sample data
        self.load_sample_data();
    }

    /// Load sample data for demonstration
    fn load_sample_data(&mut self) {
        use dde_asset_forge::asset_os::AssetRecord;

        self.assets_by_stage.clear();

        // Sample Inbox assets
        let inbox_assets = vec![
            self.create_sample_asset(1, "hero_concept.png", "character", "inbox", 
                vec!["character", "concept"], None),
            self.create_sample_asset(2, "sword_variant_1.png", "item", "inbox",
                vec!["item", "weapon"], None),
            self.create_sample_asset(3, "forest_bg.png", "background", "inbox",
                vec!["background", "forest"], None),
        ];

        // Sample Staging assets
        let staging_assets = vec![
            self.create_sample_asset(4, "villain_sprite.png", "character", "staging",
                vec!["character", "enemy", "animated"], Some("Alice")),
            self.create_sample_asset(5, "potion_red.png", "item", "staging",
                vec!["item", "consumable"], Some("Bob")),
        ];

        // Sample Review assets
        let review_assets = vec![
            self.create_sample_asset(6, "battle_effect_fire.png", "effect", "review",
                vec!["effect", "battle", "animated"], Some("Lead Artist")),
            self.create_sample_asset(7, "npc_shopkeeper.png", "character", "review",
                vec!["character", "npc"], Some("Art Director")),
        ];

        // Sample Production assets
        let production_assets = vec![
            self.create_sample_asset(8, "grass_tileset.png", "tileset", "approved",
                vec!["tileset", "terrain", "approved"], Some("Lead Artist")),
            self.create_sample_asset(9, "hero_face.png", "portrait", "approved",
                vec!["portrait", "face", "approved"], Some("Art Director")),
            self.create_sample_asset(10, "sword_iron.png", "item", "approved",
                vec!["item", "weapon", "approved"], Some("Lead Artist")),
        ];

        // Sample Rejected assets
        let rejected_assets = vec![
            self.create_sample_asset(11, "old_hero_v1.png", "character", "rejected",
                vec!["character", "outdated"], Some("Art Director")),
        ];

        self.assets_by_stage.insert(
            AssetPipelineStage::Inbox,
            inbox_assets.into_iter().map(AssetCard::from_record).collect(),
        );
        self.assets_by_stage.insert(
            AssetPipelineStage::Staging,
            staging_assets.into_iter().map(AssetCard::from_record).collect(),
        );
        self.assets_by_stage.insert(
            AssetPipelineStage::Review,
            review_assets.into_iter().map(AssetCard::from_record).collect(),
        );
        self.assets_by_stage.insert(
            AssetPipelineStage::Approved,
            production_assets.into_iter().map(AssetCard::from_record).collect(),
        );
        self.assets_by_stage.insert(
            AssetPipelineStage::Rejected,
            rejected_assets.into_iter().map(AssetCard::from_record).collect(),
        );

        self.show_status(&format!(
            "Loaded {} assets",
            self.assets_by_stage.values().map(|v| v.len()).sum::<usize>()
        ));
    }

    /// Create a sample asset record
    fn create_sample_asset(
        &self,
        id: i64,
        name: &str,
        asset_type: &str,
        status: &str,
        tags: Vec<&str>,
        reviewer: Option<&str>,
    ) -> AssetRecord {
        use serde_json::json;

        let metadata = json!({
            "tags": tags,
            "reviewer": reviewer,
            "version": 1,
            "dimensions": [32, 32],
            "created_by": "Artist",
        });

        AssetRecord {
            asset_id: id,
            name: name.to_string(),
            asset_type: asset_type.to_string(),
            file_path: format!("assets/{}/{}", asset_type, name),
            file_hash: format!("hash_{}", id),
            file_size: 1024 * id,
            metadata,
            status: status.to_string(),
            created_at: chrono::Utc::now().timestamp_millis(),
            updated_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Show status message
    fn show_status(&mut self, message: &str) {
        self.status_message = Some(message.to_string());
        self.status_timeout = 5.0;
    }

    /// Get filtered assets for a stage
    fn get_filtered_assets(&self, stage: AssetPipelineStage) -> Vec<&AssetCard> {
        self.assets_by_stage
            .get(&stage)
            .map(|assets| {
                assets
                    .iter()
                    .filter(|asset| {
                        // Type filter
                        if !self.type_filter.matches(asset.asset_type()) {
                            return false;
                        }

                        // Search filter
                        if !self.search_query.is_empty() {
                            let query = self.search_query.to_lowercase();
                            if !asset.name().to_lowercase().contains(&query)
                                && !asset.asset_type().to_lowercase().contains(&query)
                                && !asset.tags.iter().any(|t| t.to_lowercase().contains(&query))
                            {
                                return false;
                            }
                        }

                        // Reviewer filter
                        if let Some(ref reviewer) = self.reviewer_filter {
                            if asset.reviewer.as_ref() != Some(reviewer) {
                                return false;
                            }
                        }

                        true
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Select an asset (for batch operations)
    fn select_asset(&mut self, asset_id: i64, add_to_selection: bool) {
        if add_to_selection {
            if self.selected_assets.contains(&asset_id) {
                self.selected_assets.remove(&asset_id);
            } else {
                self.selected_assets.insert(asset_id);
            }
        } else {
            self.selected_assets.clear();
            self.selected_assets.insert(asset_id);
        }
    }

    /// Select all assets in a stage
    fn select_all_in_stage(&mut self, stage: AssetPipelineStage) {
        if let Some(assets) = self.assets_by_stage.get(&stage) {
            for asset in assets {
                self.selected_assets.insert(asset.id());
            }
        }
    }

    /// Clear selection
    fn clear_selection(&mut self) {
        self.selected_assets.clear();
    }

    /// Get selected assets
    fn get_selected_assets(&self) -> Vec<&AssetCard> {
        self.assets_by_stage
            .values()
            .flat_map(|assets| assets.iter())
            .filter(|asset| self.selected_assets.contains(&asset.id()))
            .collect()
    }

    /// Move asset to stage
    fn move_asset_to_stage(&mut self, asset_id: i64, target_stage: AssetPipelineStage) {
        // Find and remove asset from current stage
        for (stage, assets) in self.assets_by_stage.iter_mut() {
            if let Some(pos) = assets.iter().position(|a| a.id() == asset_id) {
                let mut asset = assets.remove(pos);
                asset.asset.status = target_stage.as_str().to_string();

                // Add to target stage
                if let Some(target_assets) = self.assets_by_stage.get_mut(&target_stage) {
                    target_assets.push(asset);
                }

                self.show_status(&format!(
                    "Moved asset {} to {:?}",
                    asset_id, target_stage
                ));
                return;
            }
        }
    }

    /// Batch move selected assets to stage
    fn batch_move_to_stage(&mut self, target_stage: AssetPipelineStage) {
        let count = self.selected_assets.len();
        for asset_id in self.selected_assets.clone() {
            self.move_asset_to_stage(asset_id, target_stage.clone());
        }
        self.show_status(&format!("Moved {} assets to {:?}", count, target_stage));
        self.selected_assets.clear();
    }

    /// Add tag to selected assets
    fn batch_add_tag(&mut self, tag: String) {
        for assets in self.assets_by_stage.values_mut() {
            for asset in assets.iter_mut() {
                if self.selected_assets.contains(&asset.id()) {
                    if !asset.tags.contains(&tag) {
                        asset.tags.push(tag.clone());
                    }
                }
            }
        }
        self.show_status(&format!("Added tag '{}' to {} assets", tag, self.selected_assets.len()));
    }

    /// Remove tag from selected assets
    fn batch_remove_tag(&mut self, tag: &str) {
        for assets in self.assets_by_stage.values_mut() {
            for asset in assets.iter_mut() {
                if self.selected_assets.contains(&asset.id()) {
                    asset.tags.retain(|t| t != tag);
                }
            }
        }
        self.show_status(&format!(
            "Removed tag '{}' from {} assets",
            tag,
            self.selected_assets.len()
        ));
    }

    /// Assign reviewer to selected assets
    fn batch_assign_reviewer(&mut self, reviewer: String) {
        for assets in self.assets_by_stage.values_mut() {
            for asset in assets.iter_mut() {
                if self.selected_assets.contains(&asset.id()) {
                    asset.reviewer = Some(reviewer.clone());
                }
            }
        }
        self.show_status(&format!(
            "Assigned {} to {} assets",
            reviewer,
            self.selected_assets.len()
        ));
    }

    /// Archive old versions of selected assets
    fn batch_archive(&mut self) {
        let count = self.selected_assets.len();
        // In a real implementation, this would move old versions to archive
        self.show_status(&format!("Archived {} assets", count));
        self.selected_assets.clear();
    }

    /// Delete selected assets
    fn batch_delete(&mut self) {
        let count = self.selected_assets.len();
        for assets in self.assets_by_stage.values_mut() {
            assets.retain(|a| !self.selected_assets.contains(&a.id()));
        }
        self.show_status(&format!("Deleted {} assets", count));
        self.selected_assets.clear();
    }

    /// Start reviewing an asset
    fn start_review(&mut self, asset_id: i64) {
        // Build review queue from review stage assets
        self.review_queue.clear();
        if let Some(assets) = self.assets_by_stage.get(&AssetPipelineStage::Review) {
            for asset in assets {
                self.review_queue.push(ReviewQueueItem {
                    asset: asset.asset.clone(),
                    review: None,
                    comparison_asset: None,
                });
            }
        }

        // Find the index of the asset we want to review
        self.current_review = self
            .review_queue
            .iter()
            .position(|item| item.asset.asset_id == asset_id);

        self.review_comment.clear();
        self.review_score = 3;
    }

    /// Approve current review item
    fn approve_current_review(&mut self) {
        if let Some(idx) = self.current_review {
            if idx < self.review_queue.len() {
                let asset_id = self.review_queue[idx].asset.asset_id;
                self.move_asset_to_stage(asset_id, AssetPipelineStage::Approved);

                // Move to next item
                if idx + 1 < self.review_queue.len() {
                    self.current_review = Some(idx + 1);
                    self.review_comment.clear();
                } else {
                    self.current_review = None;
                    self.show_status("Review queue complete");
                }
            }
        }
    }

    /// Reject current review item
    fn reject_current_review(&mut self) {
        if let Some(idx) = self.current_review {
            if idx < self.review_queue.len() {
                let asset_id = self.review_queue[idx].asset.asset_id;
                self.move_asset_to_stage(asset_id, AssetPipelineStage::Rejected);

                // Move to next item
                if idx + 1 < self.review_queue.len() {
                    self.current_review = Some(idx + 1);
                    self.review_comment.clear();
                } else {
                    self.current_review = None;
                    self.show_status("Review queue complete");
                }
            }
        }
    }

    /// Draw the panel
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        let mut visible = self.visible;

        if self.show_archive {
            self.draw_archive_window(ctx, &mut visible);
        } else if self.current_review.is_some() {
            self.draw_review_window(ctx, &mut visible);
        } else {
            egui::Window::new("🔄 Asset Pipeline")
                .open(&mut visible)
                .resizable(true)
                .default_size([1400.0, 800.0])
                .show(ctx, |ui| {
                    self.draw_pipeline_view(ui);
                });
        }

        self.visible = visible;
    }

    /// Draw the main pipeline kanban view
    fn draw_pipeline_view(&mut self, ui: &mut egui::Ui) {
        // Toolbar
        self.draw_toolbar(ui);
        ui.separator();

        // Filters
        self.draw_filters(ui);
        ui.separator();

        // Batch operations bar (shown when assets are selected)
        if !self.selected_assets.is_empty() {
            self.draw_batch_operations_bar(ui);
            ui.separator();
        }

        // Kanban columns
        self.draw_kanban_columns(ui);

        // Status message
        if let Some(ref msg) = self.status_message {
            ui.separator();
            ui.colored_label(egui::Color32::GREEN, msg);
        }
    }

    /// Draw toolbar
    fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Asset Pipeline");

            ui.separator();

            // Refresh button
            if ui.button("🔄 Refresh").clicked() {
                self.refresh_assets();
            }

            // Archive button
            if ui.button("📦 Archive").clicked() {
                self.show_archive = true;
            }

            // Review queue button
            let review_count = self
                .assets_by_stage
                .get(&AssetPipelineStage::Review)
                .map(|v| v.len())
                .unwrap_or(0);
            if ui
                .button(format!("👁 Review Queue ({})", review_count))
                .clicked()
                && review_count > 0
            {
                if let Some(first) = self
                    .assets_by_stage
                    .get(&AssetPipelineStage::Review)
                    .and_then(|v| v.first())
                {
                    self.start_review(first.id());
                }
            }

            ui.separator();

            // Selection info
            if !self.selected_assets.is_empty() {
                ui.label(format!("{} selected", self.selected_assets.len()));
                if ui.button("Clear").clicked() {
                    self.clear_selection();
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Help button
                if ui.button("❓").on_hover_text(
                    "Drag and drop assets between columns to move them through the pipeline.\n\
                     Click to select, Ctrl+Click for multi-select.",
                ).clicked() {}
            });
        });
    }

    /// Draw filters
    fn draw_filters(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Type filter
            ui.label("Type:");
            egui::ComboBox::from_id_source("type_filter")
                .selected_text(self.type_filter.display_name())
                .show_ui(ui, |ui| {
                    for filter in AssetTypeFilter::all() {
                        ui.selectable_value(
                            &mut self.type_filter,
                            filter,
                            filter.display_name(),
                        );
                    }
                });

            ui.separator();

            // Search
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_query);

            if !self.search_query.is_empty() {
                if ui.button("✕").clicked() {
                    self.search_query.clear();
                }
            }

            ui.separator();

            // Show rejected toggle
            ui.checkbox(&mut self.show_rejected, "Show Rejected");
        });
    }

    /// Draw batch operations bar
    fn draw_batch_operations_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(format!("📦 {} assets selected", self.selected_assets.len()));

            ui.separator();

            // Move to stage menu
            ui.menu_button("Move to Stage", |ui| {
                for col in &self.columns {
                    if col.stage != AssetPipelineStage::Rejected
                        && ui.button(&col.display_name).clicked()
                    {
                        self.batch_move_to_stage(col.stage.clone());
                        ui.close_menu();
                    }
                }
            });

            // Tag menu
            ui.menu_button("Add Tag", |ui| {
                ui.horizontal(|ui| {
                    ui.label("New tag:");
                    ui.text_edit_singleline(&mut self.new_tag_input);
                    if ui.button("Add").clicked() && !self.new_tag_input.is_empty() {
                        self.batch_add_tag(self.new_tag_input.clone());
                        self.new_tag_input.clear();
                    }
                });

                ui.separator();

                for tag in &self.available_tags {
                    if ui.button(tag).clicked() {
                        self.batch_add_tag(tag.clone());
                        ui.close_menu();
                    }
                }
            });

            // Assign reviewer menu
            ui.menu_button("Assign Reviewer", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Reviewer:");
                    ui.text_edit_singleline(&mut self.new_reviewer_input);
                    if ui.button("Assign").clicked() && !self.new_reviewer_input.is_empty() {
                        self.batch_assign_reviewer(self.new_reviewer_input.clone());
                        self.new_reviewer_input.clear();
                    }
                });

                ui.separator();

                for reviewer in &self.available_reviewers {
                    if ui.button(reviewer).clicked() {
                        self.batch_assign_reviewer(reviewer.clone());
                        ui.close_menu();
                    }
                }
            });

            ui.separator();

            if ui.button("🗑 Delete").clicked() {
                self.batch_delete();
            }

            if ui.button("📦 Archive").clicked() {
                self.batch_archive();
            }
        });
    }

    /// Draw kanban columns
    fn draw_kanban_columns(&mut self, ui: &mut egui::Ui) {
        let columns: Vec<_> = self
            .columns
            .iter()
            .filter(|col| self.show_rejected || col.stage != AssetPipelineStage::Rejected)
            .cloned()
            .collect();

        let column_count = columns.len();
        let available_width = ui.available_width();
        let column_width = (available_width / column_count as f32).max(220.0).min(300.0);

        ui.horizontal(|ui| {
            for col in columns {
                self.draw_pipeline_column(ui, &col, column_width);
            }
        });
    }

    /// Draw a single pipeline column
    fn draw_pipeline_column(&mut self, ui: &mut egui::Ui, col: &PipelineColumn, width: f32) {
        let stage = col.stage.clone();
        let assets = self.get_filtered_assets(stage.clone());
        let count = assets.len();

        egui::Frame::group(ui.style())
            .fill(egui::Color32::from_gray(30))
            .show(ui, |ui| {
                ui.set_width(width);
                ui.set_min_height(400.0);

                // Column header
                ui.horizontal(|ui| {
                    ui.colored_label(col.color, col.icon);
                    ui.strong(&col.display_name);
                    ui.label(format!("({})", count));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("☰").clicked() {
                            self.select_all_in_stage(stage.clone());
                        }
                    });
                });

                ui.separator();

                // Drop target
                let response = ui.interact(
                    ui.available_rect_before_wrap(),
                    ui.id().with("drop_target_").with(col.stage.as_str()),
                    egui::Sense::hover(),
                );

                if response.hovered() && self.dragging_asset.is_some() {
                    ui.painter().rect_stroke(
                        response.rect,
                        4.0,
                        egui::Stroke::new(2.0, col.color),
                    );
                    self.drag_target = Some(stage.clone());
                }

                // Asset cards
                egui::ScrollArea::vertical()
                    .id_source(format!("column_{}", col.stage.as_str()))
                    .show(ui, |ui| {
                        for asset in assets {
                            self.draw_asset_card(ui, asset);
                        }
                    });

                // Handle drop
                if response.dropped() && self.dragging_asset.is_some() {
                    if let Some(asset_id) = self.dragging_asset.take() {
                        self.move_asset_to_stage(asset_id, stage.clone());
                    }
                }
            });
    }

    /// Draw an asset card
    fn draw_asset_card(&mut self, ui: &mut egui::Ui, asset: &AssetCard) {
        let is_selected = self.selected_assets.contains(&asset.id());
        let is_dragging = self.dragging_asset == Some(asset.id());

        let frame = egui::Frame::group(ui.style())
            .fill(if is_selected {
                egui::Color32::from_rgb(40, 60, 80)
            } else {
                ui.visuals().panel_fill
            })
            .stroke(if is_dragging {
                egui::Stroke::new(2.0, egui::Color32::YELLOW)
            } else {
                ui.visuals().widgets.noninteractive.bg_stroke
            });

        frame.show(ui, |ui| {
            ui.set_width(ui.available_width());

            // Drag handle and selection
            let drag_id = ui.id().with("drag_").with(asset.id());
            let drag_response = ui.interact(
                ui.available_rect_before_wrap(),
                drag_id,
                egui::Sense::drag(),
            );

            if drag_response.dragged() {
                self.dragging_asset = Some(asset.id());
            }

            if drag_response.drag_stopped() {
                self.dragging_asset = None;
            }

            // Selection click
            if drag_response.clicked() {
                let add_to_selection = ui.input(|i| i.modifiers.ctrl);
                self.select_asset(asset.id(), add_to_selection);
            }

            // Card content
            ui.horizontal(|ui| {
                // Thumbnail placeholder
                let thumbnail_size = 48.0;
                let (thumbnail_rect, _) = ui.allocate_exact_size(
                    egui::vec2(thumbnail_size, thumbnail_size),
                    egui::Sense::hover(),
                );

                // Draw thumbnail background
                ui.painter().rect_filled(
                    thumbnail_rect,
                    4.0,
                    egui::Color32::from_gray(50),
                );

                // Draw asset type icon
                let icon = match asset.asset_type() {
                    "character" => "👤",
                    "item" => "🎁",
                    "tileset" => "🗺",
                    "effect" => "✨",
                    "portrait" => "🖼",
                    "background" => "🌄",
                    "audio" => "🔊",
                    _ => "📄",
                };
                ui.painter().text(
                    thumbnail_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    icon,
                    egui::FontId::proportional(20.0),
                    egui::Color32::GRAY,
                );

                // Asset info
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(asset.name()).strong());
                    ui.horizontal(|ui| {
                        ui.label(format!("{}", asset.asset_type()));
                        if let Some(ref reviewer) = asset.reviewer {
                            ui.label("•");
                            ui.label(format!("👤 {}", reviewer));
                        }
                    });

                    // Tags
                    if !asset.tags.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            for tag in &asset.tags[..asset.tags.len().min(3)] {
                                ui.label(
                                    egui::RichText::new(format!("#{}", tag))
                                        .small()
                                        .color(egui::Color32::from_rgb(100, 149, 237)),
                                );
                            }
                            if asset.tags.len() > 3 {
                                ui.label(
                                    egui::RichText::new(format!("+{}", asset.tags.len() - 3))
                                        .small(),
                                );
                            }
                        });
                    }
                });

                // Quick actions
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Review button (for review stage)
                    if asset.status() == "review" {
                        if ui.button("👁").on_hover_text("Review").clicked() {
                            self.start_review(asset.id());
                        }
                    }
                });
            });

            // Context menu
            drag_response.context_menu(|ui| {
                ui.set_min_width(150.0);

                if ui.button("Select").clicked() {
                    self.select_asset(asset.id(), false);
                    ui.close_menu();
                }

                ui.separator();

                // Stage transitions
                match asset.status() {
                    "inbox" => {
                        if ui.button("🔧 Classify → Staging").clicked() {
                            self.move_asset_to_stage(asset.id(), AssetPipelineStage::Staging);
                            ui.close_menu();
                        }
                    }
                    "staging" => {
                        if ui.button("👁 Submit for Review").clicked() {
                            self.move_asset_to_stage(asset.id(), AssetPipelineStage::Review);
                            ui.close_menu();
                        }
                    }
                    "review" => {
                        if ui.button("👁 Review...").clicked() {
                            self.start_review(asset.id());
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("✅ Approve").clicked() {
                            self.move_asset_to_stage(asset.id(), AssetPipelineStage::Approved);
                            ui.close_menu();
                        }
                        if ui.button("❌ Reject").clicked() {
                            self.move_asset_to_stage(asset.id(), AssetPipelineStage::Rejected);
                            ui.close_menu();
                        }
                    }
                    "approved" => {
                        if ui.button("📦 Archive Old Versions").clicked() {
                            // Archive logic
                            ui.close_menu();
                        }
                    }
                    "rejected" => {
                        if ui.button("🔄 Move to Inbox").clicked() {
                            self.move_asset_to_stage(asset.id(), AssetPipelineStage::Inbox);
                            ui.close_menu();
                        }
                    }
                    _ => {}
                }

                ui.separator();

                // Tag management
                ui.menu_button("Add Tag", |ui| {
                    for tag in &self.available_tags {
                        if ui.button(tag).clicked() {
                            self.select_asset(asset.id(), false);
                            self.batch_add_tag(tag.clone());
                            ui.close_menu();
                        }
                    }
                });

                ui.separator();

                if ui.button("🗑 Delete").clicked() {
                    self.select_asset(asset.id(), false);
                    self.batch_delete();
                    ui.close_menu();
                }
            });
        });

        ui.add_space(4.0);
    }

    /// Draw review window
    fn draw_review_window(&mut self, ctx: &egui::Context, visible: &mut bool) {
        egui::Window::new("👁 Asset Review")
            .open(visible)
            .resizable(true)
            .default_size([900.0, 700.0])
            .show(ctx, |ui| {
                if let Some(idx) = self.current_review {
                    if idx >= self.review_queue.len() {
                        self.current_review = None;
                        ui.label("Review queue complete");
                        return;
                    }

                    let item = &self.review_queue[idx];
                    let remaining = self.review_queue.len() - idx;

                    // Header
                    ui.horizontal(|ui| {
                        ui.heading(&item.asset.name);
                        ui.label(format!("({} remaining)", remaining));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✕ Close").clicked() {
                                self.current_review = None;
                            }
                        });
                    });

                    ui.separator();

                    // Asset info
                    ui.horizontal(|ui| {
                        ui.label(format!("Type: {}", item.asset.asset_type));
                        ui.label("•");
                        ui.label(format!("Path: {}", item.asset.file_path));
                    });

                    ui.separator();

                    // Main content
                    egui::SidePanel::right("review_sidebar")
                        .resizable(false)
                        .default_width(300.0)
                        .show_inside(ui, |ui| {
                            self.draw_review_sidebar(ui);
                        });

                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        self.draw_review_preview(ui, item);
                    });
                } else {
                    ui.label("No assets to review");
                }
            });
    }

    /// Draw review sidebar
    fn draw_review_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading("Review");
        ui.separator();

        // Score
        ui.label("Score:");
        ui.add(egui::Slider::new(&mut self.review_score, 1..=5).text("stars"));

        ui.separator();

        // Comment
        ui.label("Comments:");
        ui.text_edit_multiline(&mut self.review_comment);

        ui.separator();

        // Actions
        ui.horizontal(|ui| {
            if ui
                .button(egui::RichText::new("✅ Approve").color(egui::Color32::GREEN))
                .clicked()
            {
                self.approve_current_review();
            }

            if ui
                .button(egui::RichText::new("❌ Reject").color(egui::Color32::RED))
                .clicked()
            {
                self.reject_current_review();
            }
        });

        ui.separator();

        // Navigation
        ui.horizontal(|ui| {
            if ui.button("◀ Previous").clicked() {
                if let Some(idx) = self.current_review {
                    if idx > 0 {
                        self.current_review = Some(idx - 1);
                    }
                }
            }

            if ui.button("Next ▶").clicked() {
                if let Some(idx) = self.current_review {
                    if idx + 1 < self.review_queue.len() {
                        self.current_review = Some(idx + 1);
                    }
                }
            }
        });

        ui.separator();

        // Comparison mode
        ui.checkbox(&mut self.compare_mode, "Compare with previous version");
    }

    /// Draw review preview
    fn draw_review_preview(&mut self, ui: &mut egui::Ui, item: &ReviewQueueItem) {
        if self.compare_mode {
            // Side-by-side comparison
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.heading("Current");
                    self.draw_preview_placeholder(ui, &item.asset);
                });

                ui.vertical(|ui| {
                    ui.heading("Previous");
                    if let Some(ref prev) = item.comparison_asset {
                        self.draw_preview_placeholder(ui, prev);
                    } else {
                        ui.label("No previous version");
                    }
                });
            });
        } else {
            // Single preview
            ui.vertical_centered(|ui| {
                ui.heading("Preview");
                self.draw_preview_placeholder(ui, &item.asset);
            });
        }
    }

    /// Draw preview placeholder
    fn draw_preview_placeholder(&self, ui: &mut egui::Ui, asset: &AssetRecord) {
        let available = ui.available_size();
        let size = available.min_elem().min(400.0);

        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());

        ui.painter().rect_filled(
            rect,
            8.0,
            egui::Color32::from_gray(40),
        );

        // Show asset info in the center
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{}", asset.asset_type),
            egui::FontId::proportional(24.0),
            egui::Color32::GRAY,
        );
    }

    /// Draw archive window
    fn draw_archive_window(&mut self, ctx: &egui::Context, visible: &mut bool) {
        egui::Window::new("📦 Archive")
            .open(visible)
            .resizable(true)
            .default_size([800.0, 600.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Archived Assets");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕ Close").clicked() {
                            self.show_archive = false;
                        }
                    });
                });

                ui.separator();

                if self.archive_entries.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.label("No archived assets");
                        ui.label("Old versions of approved assets will appear here.");
                    });
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("archive_grid")
                            .num_columns(4)
                            .spacing([20.0, 10.0])
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Name").strong());
                                ui.label(egui::RichText::new("Version").strong());
                                ui.label(egui::RichText::new("Archived").strong());
                                ui.label(egui::RichText::new("Actions").strong());
                                ui.end_row();

                                for entry in &self.archive_entries {
                                    ui.label(&entry.name);
                                    ui.label(format!("v{}", entry.version));
                                    ui.label(format!(
                                        "{}",
                                        chrono::DateTime::from_timestamp(
                                            entry.archived_at / 1000,
                                            0
                                        )
                                        .map(|dt| dt.format("%Y-%m-%d").to_string())
                                        .unwrap_or_default()
                                    ));

                                    ui.horizontal(|ui| {
                                        if ui.button("Restore").clicked() {
                                            // Restore logic
                                        }
                                        if ui.button("🗑 Delete").clicked() {
                                            // Delete logic
                                        }
                                    });

                                    ui.end_row();
                                }
                            });
                    });
                }
            });
    }

    /// Get stage counts
    pub fn get_stage_counts(&self) -> HashMap<AssetPipelineStage, usize> {
        self.assets_by_stage
            .iter()
            .map(|(stage, assets)| (stage.clone(), assets.len()))
            .collect()
    }

    /// Get total asset count
    pub fn total_asset_count(&self) -> usize {
        self.assets_by_stage.values().map(|v| v.len()).sum()
    }

    /// Get review queue length
    pub fn review_queue_length(&self) -> usize {
        self.assets_by_stage
            .get(&AssetPipelineStage::Review)
            .map(|v| v.len())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_creation() {
        let panel = AssetOsPipelinePanel::new();
        assert!(!panel.is_visible());
        assert_eq!(panel.columns.len(), 5);
    }

    #[test]
    fn test_panel_toggle() {
        let mut panel = AssetOsPipelinePanel::new();
        assert!(!panel.is_visible());

        panel.toggle();
        assert!(panel.is_visible());

        panel.toggle();
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_asset_selection() {
        let mut panel = AssetOsPipelinePanel::new();

        panel.select_asset(1, false);
        assert!(panel.selected_assets.contains(&1));

        panel.select_asset(2, true);
        assert!(panel.selected_assets.contains(&1));
        assert!(panel.selected_assets.contains(&2));

        panel.clear_selection();
        assert!(panel.selected_assets.is_empty());
    }

    #[test]
    fn test_asset_type_filter() {
        assert!(AssetTypeFilter::All.matches("character"));
        assert!(AssetTypeFilter::Character.matches("character"));
        assert!(!AssetTypeFilter::Item.matches("character"));
    }

    #[test]
    fn test_stage_counts() {
        let mut panel = AssetOsPipelinePanel::new();
        panel.load_sample_data();

        let counts = panel.get_stage_counts();
        assert!(counts.values().sum::<usize>() > 0);
    }
}
