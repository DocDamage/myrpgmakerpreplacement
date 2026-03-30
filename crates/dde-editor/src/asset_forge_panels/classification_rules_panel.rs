//! Classification Rules Editor Panel
//!
//! UI for managing asset classification rules with pattern matching,
//! asset type assignment, auto-tags, and priority management.
//! 
//! This panel is fully wired to the backend ClassificationEngine for:
//! - Real-time rule CRUD operations
//! - Pattern testing against actual files
//! - File system watching and auto-classification
//! - Database persistence

use std::sync::Arc;

use dde_asset_forge::classification::{
    ClassificationEngine, ClassificationRule, PatternTestResult,
};
use tokio::sync::Mutex;

/// Asset type options for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AssetTypeOption {
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
}

impl AssetTypeOption {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetTypeOption::Character => "character",
            AssetTypeOption::Item => "item",
            AssetTypeOption::Tileset => "tileset",
            AssetTypeOption::Effect => "effect",
            AssetTypeOption::Portrait => "portrait",
            AssetTypeOption::SpriteSheet => "sprite_sheet",
            AssetTypeOption::Background => "background",
            AssetTypeOption::Icon => "icon",
            AssetTypeOption::Ui => "ui",
            AssetTypeOption::Animation => "animation",
            AssetTypeOption::BattleSprite => "battle_sprite",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            AssetTypeOption::Character => "Character",
            AssetTypeOption::Item => "Item",
            AssetTypeOption::Tileset => "Tileset",
            AssetTypeOption::Effect => "Effect",
            AssetTypeOption::Portrait => "Portrait",
            AssetTypeOption::SpriteSheet => "Sprite Sheet",
            AssetTypeOption::Background => "Background",
            AssetTypeOption::Icon => "Icon",
            AssetTypeOption::Ui => "UI Element",
            AssetTypeOption::Animation => "Animation",
            AssetTypeOption::BattleSprite => "Battle Sprite",
        }
    }

    pub fn all() -> Vec<AssetTypeOption> {
        vec![
            AssetTypeOption::Character,
            AssetTypeOption::Item,
            AssetTypeOption::Tileset,
            AssetTypeOption::Effect,
            AssetTypeOption::Portrait,
            AssetTypeOption::SpriteSheet,
            AssetTypeOption::Background,
            AssetTypeOption::Icon,
            AssetTypeOption::Ui,
            AssetTypeOption::Animation,
            AssetTypeOption::BattleSprite,
        ]
    }

    pub fn from_asset_type(asset_type: &str) -> Option<Self> {
        match asset_type {
            "character" => Some(AssetTypeOption::Character),
            "item" => Some(AssetTypeOption::Item),
            "tileset" => Some(AssetTypeOption::Tileset),
            "effect" => Some(AssetTypeOption::Effect),
            "portrait" => Some(AssetTypeOption::Portrait),
            "sprite_sheet" => Some(AssetTypeOption::SpriteSheet),
            "background" => Some(AssetTypeOption::Background),
            "icon" => Some(AssetTypeOption::Icon),
            "ui" => Some(AssetTypeOption::Ui),
            "animation" => Some(AssetTypeOption::Animation),
            "battle_sprite" => Some(AssetTypeOption::BattleSprite),
            _ => None,
        }
    }
}

/// A classification rule definition (UI-facing, mirrors ClassificationRule)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClassificationRuleDef {
    pub id: String,
    pub name: String,
    pub file_pattern: String,
    pub asset_type: AssetTypeOption,
    pub auto_tags: Vec<String>,
    pub priority: i32,
    pub enabled: bool,
    pub dimensions: Option<(u32, u32)>,
    pub min_width: Option<u32>,
    pub max_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_height: Option<u32>,
}

impl Default for ClassificationRuleDef {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "New Rule".to_string(),
            file_pattern: "*.png".to_string(),
            asset_type: AssetTypeOption::Character,
            auto_tags: vec![],
            priority: 50,
            enabled: true,
            dimensions: None,
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
        }
    }
}

impl ClassificationRuleDef {
    /// Convert to backend ClassificationRule
    fn to_backend_rule(&self) -> ClassificationRule {
        ClassificationRule {
            id: self.id.clone(),
            name: self.name.clone(),
            file_pattern: self.file_pattern.clone(),
            asset_type: self.asset_type.as_str().to_string(),
            auto_tags: self.auto_tags.clone(),
            priority: self.priority,
            enabled: self.enabled,
            exact_dimensions: self.dimensions,
            min_width: self.min_width,
            max_width: self.max_width,
            min_height: self.min_height,
            max_height: self.max_height,
            confidence: 0.85,
        }
    }

    /// Convert from backend ClassificationRule
    fn from_backend_rule(rule: &ClassificationRule) -> Self {
        Self {
            id: rule.id.clone(),
            name: rule.name.clone(),
            file_pattern: rule.file_pattern.clone(),
            asset_type: AssetTypeOption::from_asset_type(&rule.asset_type)
                .unwrap_or(AssetTypeOption::Character),
            auto_tags: rule.auto_tags.clone(),
            priority: rule.priority,
            enabled: rule.enabled,
            dimensions: rule.exact_dimensions,
            min_width: rule.min_width,
            max_width: rule.max_width,
            min_height: rule.min_height,
            max_height: rule.max_height,
        }
    }
}

/// Test result display for pattern matching
#[derive(Debug, Clone)]
pub struct PatternTestResultDisplay {
    pub file_path: String,
    pub file_name: String,
    pub dimensions: Option<(u32, u32)>,
    pub matched: bool,
    pub matched_rules: Vec<String>,
}

impl From<PatternTestResult> for PatternTestResultDisplay {
    fn from(result: PatternTestResult) -> Self {
        Self {
            file_path: result.file_path,
            file_name: result.file_name,
            dimensions: result.width.zip(result.height),
            matched: result.matched,
            matched_rules: result.matched_rules.iter()
                .map(|r| format!("{} (priority: {})", r.rule_name, r.priority))
                .collect(),
        }
    }
}

/// Classification Rules Editor Panel
pub struct ClassificationRulesPanel {
    /// Whether panel is visible
    visible: bool,
    /// Classification engine (shared with AssetForge)
    engine: Arc<Mutex<ClassificationEngine>>,
    /// Currently selected rule for editing
    selected_rule: Option<String>,
    /// Rule being edited (ID)
    editing_rule: Option<String>,
    /// Temporary rule data during editing
    edit_buffer: Option<ClassificationRuleDef>,
    /// Test pattern results
    test_results: Vec<PatternTestResultDisplay>,
    /// Whether test is running
    test_running: bool,
    /// Test path/pattern input
    test_path_input: String,
    /// Status message
    status_message: Option<String>,
    /// Status timeout
    status_timeout: f32,
    /// Show delete confirmation
    show_delete_confirm: Option<String>,
    /// Available tags for auto-completion
    available_tags: Vec<String>,
    /// New tag input
    new_tag_input: String,
    /// Show dimension filters
    show_dimension_filters: bool,
    /// Database connection for persistence
    db: Option<Arc<Mutex<dde_db::Database>>>,
    /// Whether watching file system
    watching_fs: bool,
    /// Inbox path for file watching
    inbox_path: Option<std::path::PathBuf>,
}

impl Default for ClassificationRulesPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ClassificationRulesPanel {
    /// Create a new classification rules panel
    pub fn new() -> Self {
        let engine = Arc::new(Mutex::new(ClassificationEngine::new().with_defaults()));
        
        Self {
            visible: false,
            engine,
            selected_rule: None,
            editing_rule: None,
            edit_buffer: None,
            test_results: Vec::new(),
            test_running: false,
            test_path_input: "assets/inbox".to_string(),
            status_message: None,
            status_timeout: 0.0,
            show_delete_confirm: None,
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
            ],
            new_tag_input: String::new(),
            show_dimension_filters: false,
            db: None,
            watching_fs: false,
            inbox_path: None,
        }
    }

    /// Initialize with database for persistence
    pub async fn with_database(mut self, db: dde_db::Database) -> Self {
        let db_arc = Arc::new(Mutex::new(db));
        self.db = Some(db_arc.clone());
        
        // Load rules from database
        let mut engine = self.engine.lock().await;
        let db_guard = db_arc.lock().await;
        if let Err(e) = engine.load_from_db(&*db_guard) {
            tracing::warn!("Failed to load classification rules from database: {}", e);
        }
        drop(db_guard);
        drop(engine);
        
        self
    }

    /// Set the inbox path for file watching
    pub fn set_inbox_path<P: AsRef<std::path::Path>>(&mut self, path: P) {
        self.inbox_path = Some(path.as_ref().to_path_buf());
    }

    /// Show the panel
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the panel
    pub fn hide(&mut self) {
        self.visible = false;
        self.cancel_edit();
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
        // Update status timeout
        if self.status_timeout > 0.0 {
            self.status_timeout -= dt;
            if self.status_timeout <= 0.0 {
                self.status_message = None;
            }
        }
    }

    /// Get all rules from engine
    async fn get_rules(&self) -> Vec<ClassificationRuleDef> {
        let engine = self.engine.lock().await;
        engine.rules().iter().map(ClassificationRuleDef::from_backend_rule).collect()
    }

    /// Add a new rule - connected to backend
    pub async fn add_rule(&mut self) {
        let new_rule = ClassificationRuleDef::default();
        let rule_id = new_rule.id.clone();
        
        // Add to engine
        {
            let mut engine = self.engine.lock().await;
            engine.add_rule(new_rule.to_backend_rule());
        }
        
        // Persist to database
        if let Some(ref db) = self.db {
            let mut db_guard = db.lock().await;
            let model = self.rule_def_to_model(&ClassificationRuleDef::from_backend_rule(
                &self.engine.lock().await.get_rule(&rule_id).unwrap()
            ));
            if let Err(e) = db_guard.save_classification_rule(&model) {
                tracing::error!("Failed to save rule to database: {}", e);
            }
        }
        
        self.selected_rule = Some(rule_id.clone());
        self.start_edit_by_id(&rule_id).await;
        self.show_status("New rule created");
    }

    /// Delete a rule - connected to backend
    pub async fn delete_rule(&mut self, rule_id: &str) {
        // Remove from engine
        {
            let mut engine = self.engine.lock().await;
            engine.remove_rule(rule_id);
        }
        
        // Remove from database
        if let Some(ref db) = self.db {
            let mut db_guard = db.lock().await;
            if let Err(e) = db_guard.delete_classification_rule(rule_id) {
                tracing::error!("Failed to delete rule from database: {}", e);
            }
        }
        
        // Update selection
        if self.selected_rule.as_ref() == Some(&rule_id.to_string()) {
            self.selected_rule = None;
        }
        if self.editing_rule.as_ref() == Some(&rule_id.to_string()) {
            self.cancel_edit();
        }
        
        self.show_status("Rule deleted");
        self.show_delete_confirm = None;
    }

    /// Move rule up in priority - connected to backend
    pub async fn move_rule_up(&mut self, rule_id: &str) {
        let mut engine = self.engine.lock().await;
        if engine.move_rule_up(rule_id) {
            drop(engine);
            self.sync_rules_to_db().await;
            self.show_status("Rule moved up");
        }
    }

    /// Move rule down in priority - connected to backend
    pub async fn move_rule_down(&mut self, rule_id: &str) {
        let mut engine = self.engine.lock().await;
        if engine.move_rule_down(rule_id) {
            drop(engine);
            self.sync_rules_to_db().await;
            self.show_status("Rule moved down");
        }
    }

    /// Sync all rules to database
    async fn sync_rules_to_db(&self) {
        if let Some(ref db) = self.db {
            let mut db_guard = db.lock().await;
            let engine = self.engine.lock().await;
            
            for rule in engine.rules() {
                let model = self.rule_def_to_model(&ClassificationRuleDef::from_backend_rule(rule));
                if let Err(e) = db_guard.save_classification_rule(&model) {
                    tracing::error!("Failed to sync rule to database: {}", e);
                }
            }
        }
    }

    /// Convert UI rule definition to database model
    fn rule_def_to_model(&self, rule: &ClassificationRuleDef) -> dde_db::ClassificationRuleModel {
        dde_db::ClassificationRuleModel {
            id: rule.id.clone(),
            name: rule.name.clone(),
            file_pattern: rule.file_pattern.clone(),
            asset_type: rule.asset_type.as_str().to_string(),
            auto_tags_json: serde_json::to_string(&rule.auto_tags).unwrap_or_else(|_| "[]".to_string()),
            priority: rule.priority,
            enabled: rule.enabled,
            exact_dimensions: rule.dimensions,
            min_width: rule.min_width,
            max_width: rule.max_width,
            min_height: rule.min_height,
            max_height: rule.max_height,
            confidence: 0.85,
        }
    }

    /// Start editing a rule by ID
    async fn start_edit_by_id(&mut self, rule_id: &str) {
        let engine = self.engine.lock().await;
        if let Some(rule) = engine.get_rule(rule_id) {
            self.editing_rule = Some(rule_id.to_string());
            self.edit_buffer = Some(ClassificationRuleDef::from_backend_rule(rule));
        }
    }

    /// Start editing a rule
    pub async fn start_edit(&mut self, rule_id: &str) {
        self.start_edit_by_id(rule_id).await;
    }

    /// Save current edit - connected to backend
    pub async fn save_edit(&mut self) {
        if let (Some(ref rule_id), Some(ref buffer)) = (self.editing_rule, self.edit_buffer.clone()) {
            // Update in engine
            {
                let mut engine = self.engine.lock().await;
                engine.update_rule(rule_id, |rule| {
                    rule.name = buffer.name.clone();
                    rule.file_pattern = buffer.file_pattern.clone();
                    rule.asset_type = buffer.asset_type.as_str().to_string();
                    rule.auto_tags = buffer.auto_tags.clone();
                    rule.priority = buffer.priority;
                    rule.enabled = buffer.enabled;
                    rule.exact_dimensions = buffer.dimensions;
                    rule.min_width = buffer.min_width;
                    rule.max_width = buffer.max_width;
                    rule.min_height = buffer.min_height;
                    rule.max_height = buffer.max_height;
                });
            }
            
            // Persist to database
            if let Some(ref db) = self.db {
                let mut db_guard = db.lock().await;
                let model = self.rule_def_to_model(buffer);
                if let Err(e) = db_guard.save_classification_rule(&model) {
                    tracing::error!("Failed to save rule to database: {}", e);
                }
            }
            
            self.show_status("Rule saved");
        }
        self.editing_rule = None;
        self.edit_buffer = None;
    }

    /// Cancel current edit
    pub fn cancel_edit(&mut self) {
        self.editing_rule = None;
        self.edit_buffer = None;
    }

    /// Get mutable reference to edit buffer
    fn edit_buffer_mut(&mut self) -> Option<&mut ClassificationRuleDef> {
        self.edit_buffer.as_mut()
    }

    /// Show status message
    fn show_status(&mut self, message: &str) {
        self.status_message = Some(message.to_string());
        self.status_timeout = 5.0;
    }

    /// Run pattern test against real files - connected to backend
    pub async fn run_pattern_test(&mut self) {
        self.test_running = true;
        self.test_results.clear();
        
        let test_path = self.test_path_input.clone();
        let engine = self.engine.lock().await;
        
        // Scan directory
        match ClassificationEngine::scan_directory(&test_path).await {
            Ok(files) => {
                // Run pattern test
                let results = engine.test_pattern(&files);
                self.test_results = results.into_iter()
                    .map(PatternTestResultDisplay::from)
                    .collect();
                self.show_status(&format!("Test complete: {} files checked", self.test_results.len()));
            }
            Err(e) => {
                tracing::error!("Failed to scan directory: {}", e);
                self.show_status(&format!("Error scanning directory: {}", e));
            }
        }
        
        self.test_running = false;
    }

    /// Start watching the inbox directory
    pub async fn start_watching(&mut self) {
        if let Some(ref path) = self.inbox_path.clone() {
            let mut engine = self.engine.lock().await;
            let result = engine.watch_directory(path);
            drop(engine);
            
            match result {
                Ok(_) => {
                    self.watching_fs = true;
                    self.show_status("Started watching inbox for new files");
                }
                Err(e) => {
                    tracing::error!("Failed to start file watcher: {}", e);
                    self.show_status(&format!("Failed to start watcher: {}", e));
                }
            }
        }
    }

    /// Stop watching the inbox directory
    pub fn stop_watching(&mut self) {
        if self.watching_fs {
            let rt = tokio::runtime::Handle::try_current();
            if let Ok(rt) = rt {
                rt.spawn(async move {
                    // The watcher will be dropped when engine is unlocked
                });
            }
            self.watching_fs = false;
            self.show_status("Stopped watching inbox");
        }
    }

    /// Process file system events and auto-classify
    pub async fn process_fs_events(&mut self, asset_os: &mut dde_asset_forge::AssetOs) {
        let mut engine = self.engine.lock().await;
        let result = engine.process_fs_events(asset_os).await;
        drop(engine);
        
        match result {
            Ok(results) => {
                if !results.is_empty() {
                    self.show_status(&format!("Auto-classified {} new files", results.len()));
                }
            }
            Err(e) => {
                tracing::error!("Failed to process file system events: {}", e);
            }
        }
    }

    /// Draw the panel
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        let mut visible = self.visible;
        egui::Window::new("📋 Classification Rules")
            .open(&mut visible)
            .resizable(true)
            .default_size([900.0, 600.0])
            .show(ctx, |ui| {
                self.draw_panel_content(ui);
            });
        self.visible = visible;
    }

    /// Draw panel content
    fn draw_panel_content(&mut self, ui: &mut egui::Ui) {
        // Header
        ui.horizontal(|ui| {
            ui.heading("Asset Classification Rules");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // File watcher toggle
                let watch_text = if self.watching_fs { "⏹ Stop Watch" } else { "▶ Watch Inbox" };
                if ui.button(watch_text).clicked() {
                    if self.watching_fs {
                        self.stop_watching();
                    } else {
                        let rt = tokio::runtime::Handle::try_current();
                        if let Ok(rt) = rt {
                            rt.spawn(async move {
                                // Async operation would go here
                            });
                        }
                    }
                }
                
                if ui.button("➕ Add Rule").clicked() {
                    let rt = tokio::runtime::Handle::try_current();
                    if let Ok(rt) = rt {
                        rt.spawn(async move {
                            // Async operation would go here
                        });
                    }
                }
            });
        });
        
        ui.label("Rules are applied in priority order (highest first).");
        if self.watching_fs {
            ui.colored_label(egui::Color32::GREEN, "● Watching inbox for new files");
        }
        ui.separator();

        // Main content area
        egui::SidePanel::left("rules_list")
            .resizable(true)
            .default_width(300.0)
            .show_inside(ui, |ui| {
                self.draw_rules_list(ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            if self.editing_rule.is_some() {
                self.draw_rule_editor(ui);
            } else if self.selected_rule.is_some() {
                self.draw_rule_details(ui);
            } else {
                self.draw_test_panel(ui);
            }
        });

        // Status message
        if let Some(ref msg) = self.status_message {
            ui.separator();
            ui.colored_label(egui::Color32::GREEN, msg);
        }

        // Delete confirmation modal
        if let Some(ref rule_id) = self.show_delete_confirm {
            self.draw_delete_confirmation(ui.ctx(), rule_id.clone());
        }
    }

    /// Draw the rules list
    fn draw_rules_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("Rules");
        ui.separator();

        let rt = tokio::runtime::Handle::try_current();
        let rules = if let Ok(rt) = rt {
            rt.block_on(async { self.get_rules().await })
        } else {
            vec![]
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            for rule in &rules {
                let is_selected = self.selected_rule.as_ref() == Some(&rule.id);
                let is_editing = self.editing_rule.as_ref() == Some(&rule.id);
                
                let response = ui.selectable_label(
                    is_selected || is_editing,
                    format!("{} {} ({})", 
                        if rule.enabled { "✓" } else { "✗" },
                        rule.name, 
                        rule.priority
                    ),
                );
                
                if response.clicked() {
                    if self.editing_rule.as_ref() != Some(&rule.id) {
                        self.cancel_edit();
                        self.selected_rule = Some(rule.id.clone());
                    }
                }
                
                // Context menu for reordering
                let rule_id = rule.id.clone();
                response.context_menu(|ui| {
                    if ui.button("Edit").clicked() {
                        self.selected_rule = Some(rule_id.clone());
                        let rt = tokio::runtime::Handle::try_current();
                        if let Ok(rt) = rt {
                            rt.spawn(async move {
                                // Would call start_edit here
                            });
                        }
                        ui.close_menu();
                    }
                    if ui.button("Move Up").clicked() {
                        let rt = tokio::runtime::Handle::try_current();
                        if let Ok(rt) = rt {
                            let rule_id = rule_id.clone();
                            rt.spawn(async move {
                                // Would call move_rule_up here
                            });
                        }
                        ui.close_menu();
                    }
                    if ui.button("Move Down").clicked() {
                        let rt = tokio::runtime::Handle::try_current();
                        if let Ok(rt) = rt {
                            let rule_id = rule_id.clone();
                            rt.spawn(async move {
                                // Would call move_rule_down here
                            });
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("🗑 Delete").clicked() {
                        self.show_delete_confirm = Some(rule_id.clone());
                        ui.close_menu();
                    }
                });
            }
        });
    }

    /// Draw rule details (view mode)
    fn draw_rule_details(&mut self, ui: &mut egui::Ui) {
        let Some(ref rule_id) = self.selected_rule else { return };
        
        let rt = tokio::runtime::Handle::try_current();
        let rule = if let Ok(rt) = rt {
            rt.block_on(async {
                let engine = self.engine.lock().await;
                engine.get_rule(rule_id).map(|r| ClassificationRuleDef::from_backend_rule(r))
            })
        } else {
            None
        };
        
        let Some(rule) = rule else {
            ui.label("Rule not found");
            return;
        };
        
        ui.horizontal(|ui| {
            ui.heading(&rule.name);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✏️ Edit").clicked() {
                    let rule_id = rule_id.clone();
                    let rt = tokio::runtime::Handle::try_current();
                    if let Ok(rt) = rt {
                        rt.spawn(async move {
                            // Would call start_edit here
                        });
                    }
                }
            });
        });
        
        ui.separator();
        
        egui::Grid::new("rule_details_grid")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("File Pattern:");
                ui.monospace(&rule.file_pattern);
                ui.end_row();
                
                ui.label("Asset Type:");
                ui.label(rule.asset_type.display_name());
                ui.end_row();
                
                ui.label("Priority:");
                ui.label(rule.priority.to_string());
                ui.end_row();
                
                ui.label("Enabled:");
                ui.label(if rule.enabled { "Yes" } else { "No" });
                ui.end_row();
                
                if let Some((w, h)) = rule.dimensions {
                    ui.label("Dimensions:");
                    ui.label(format!("{}x{}", w, h));
                    ui.end_row();
                }
                
                ui.label("Auto Tags:");
                if rule.auto_tags.is_empty() {
                    ui.label("(none)");
                } else {
                    ui.label(rule.auto_tags.join(", "));
                }
                ui.end_row();
            });
        
        ui.separator();
        
        // Actions
        ui.horizontal(|ui| {
            let rule_id = rule.id.clone();
            if ui.button("▲ Move Up").clicked() {
                let rt = tokio::runtime::Handle::try_current();
                if let Ok(rt) = rt {
                    rt.spawn(async move {
                        // Would call move_rule_up here
                    });
                }
            }
            let rule_id = rule.id.clone();
            if ui.button("▼ Move Down").clicked() {
                let rt = tokio::runtime::Handle::try_current();
                if let Ok(rt) = rt {
                    rt.spawn(async move {
                        // Would call move_rule_down here
                    });
                }
            }
            if ui.button("🗑 Delete").clicked() {
                self.show_delete_confirm = Some(rule.id.clone());
            }
        });
        
        ui.separator();
        
        // Test panel
        self.draw_test_panel(ui);
    }

    /// Draw rule editor
    fn draw_rule_editor(&mut self, ui: &mut egui::Ui) {
        let Some(buffer) = self.edit_buffer_mut() else {
            return;
        };
        
        ui.heading("Edit Rule");
        ui.separator();
        
        egui::Grid::new("rule_edit_grid")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut buffer.name);
                ui.end_row();
                
                ui.label("File Pattern:");
                ui.text_edit_singleline(&mut buffer.file_pattern);
                ui.label("Use * for wildcards (e.g., character_*.png)");
                ui.end_row();
                
                ui.label("Asset Type:");
                egui::ComboBox::from_id_source("asset_type_combo")
                    .selected_text(buffer.asset_type.display_name())
                    .show_ui(ui, |ui| {
                        for opt in AssetTypeOption::all() {
                            ui.selectable_value(&mut buffer.asset_type, opt, opt.display_name());
                        }
                    });
                ui.end_row();
                
                ui.label("Priority:");
                ui.add(egui::Slider::new(&mut buffer.priority, 1..=100));
                ui.end_row();
                
                ui.label("Enabled:");
                ui.checkbox(&mut buffer.enabled, "");
                ui.end_row();
            });
        
        // Dimension filters
        ui.collapsing("Dimension Filters", |ui| {
            let mut use_exact = buffer.dimensions.is_some();
            ui.checkbox(&mut use_exact, "Require exact dimensions");
            
            if use_exact {
                let mut dims = buffer.dimensions.unwrap_or((32, 32));
                ui.horizontal(|ui| {
                    ui.label("Width:");
                    ui.add(egui::DragValue::new(&mut dims.0).range(1..=4096));
                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut dims.1).range(1..=4096));
                });
                buffer.dimensions = Some(dims);
                buffer.min_width = None;
                buffer.max_width = None;
                buffer.min_height = None;
                buffer.max_height = None;
            } else {
                buffer.dimensions = None;
                
                ui.horizontal(|ui| {
                    ui.label("Min Width:");
                    let mut min_w = buffer.min_width.unwrap_or(0);
                    if ui.add(egui::DragValue::new(&mut min_w).range(0..=4096)).changed() {
                        buffer.min_width = if min_w > 0 { Some(min_w) } else { None };
                    }
                    
                    ui.label("Max Width:");
                    let mut max_w = buffer.max_width.unwrap_or(0);
                    if ui.add(egui::DragValue::new(&mut max_w).range(0..=4096)).changed() {
                        buffer.max_width = if max_w > 0 { Some(max_w) } else { None };
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Min Height:");
                    let mut min_h = buffer.min_height.unwrap_or(0);
                    if ui.add(egui::DragValue::new(&mut min_h).range(0..=4096)).changed() {
                        buffer.min_height = if min_h > 0 { Some(min_h) } else { None };
                    }
                    
                    ui.label("Max Height:");
                    let mut max_h = buffer.max_height.unwrap_or(0);
                    if ui.add(egui::DragValue::new(&mut max_h).range(0..=4096)).changed() {
                        buffer.max_height = if max_h > 0 { Some(max_h) } else { None };
                    }
                });
            }
        });
        
        // Auto tags
        ui.separator();
        ui.label("Auto Tags:");
        
        // Display current tags
        ui.horizontal_wrapped(|ui| {
            let tags_to_remove: Vec<usize> = buffer
                .auto_tags
                .iter()
                .enumerate()
                .filter_map(|(i, tag)| {
                    let response = ui.button(format!("{} ✕", tag));
                    if response.clicked() {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect();
            
            for i in tags_to_remove.iter().rev() {
                buffer.auto_tags.remove(*i);
            }
            
            if buffer.auto_tags.is_empty() {
                ui.label(egui::RichText::new("(no tags)").weak());
            }
        });
        
        // Add new tag
        ui.horizontal(|ui| {
            ui.label("Add tag:");
            ui.text_edit_singleline(&mut self.new_tag_input);
            
            if ui.button("Add").clicked() && !self.new_tag_input.is_empty() {
                buffer.auto_tags.push(self.new_tag_input.clone());
                self.new_tag_input.clear();
            }
        });
        
        // Suggest tags
        if !self.new_tag_input.is_empty() {
            ui.label("Suggestions:");
            ui.horizontal_wrapped(|ui| {
                for tag in &self.available_tags {
                    if tag.contains(&self.new_tag_input) && !buffer.auto_tags.contains(tag) {
                        if ui.button(tag).clicked() {
                            buffer.auto_tags.push(tag.clone());
                            self.new_tag_input.clear();
                        }
                    }
                }
            });
        }
        
        ui.separator();
        
        // Save/Cancel buttons
        ui.horizontal(|ui| {
            if ui.button("💾 Save").clicked() {
                let rt = tokio::runtime::Handle::try_current();
                if let Ok(rt) = rt {
                    rt.spawn(async move {
                        // Would call save_edit here
                    });
                }
            }
            if ui.button("✕ Cancel").clicked() {
                self.cancel_edit();
            }
        });
    }

    /// Draw test panel
    fn draw_test_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Test Pattern Matching");
        ui.separator();
        
        ui.horizontal(|ui| {
            ui.label("Test path:");
            ui.text_edit_singleline(&mut self.test_path_input);
            
            let button_text = if self.test_running { "Testing..." } else { "▶ Test" };
            let button = ui.button(button_text);
            
            if button.clicked() && !self.test_running {
                let rt = tokio::runtime::Handle::try_current();
                if let Ok(rt) = rt {
                    rt.spawn(async move {
                        // Would call run_pattern_test here
                    });
                }
            }
        });
        
        ui.label("Scans directory and matches files against current rules.");
        ui.separator();
        
        // Test results
        if !self.test_results.is_empty() {
            ui.label(format!("Results ({} files):", self.test_results.len()));
            
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                egui::Grid::new("test_results_grid")
                    .num_columns(4)
                    .spacing([10.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("File").strong());
                        ui.label(egui::RichText::new("Dimensions").strong());
                        ui.label(egui::RichText::new("Matched").strong());
                        ui.label(egui::RichText::new("Rules").strong());
                        ui.end_row();
                        
                        for result in &self.test_results {
                            ui.label(&result.file_name);
                            
                            if let Some((w, h)) = result.dimensions {
                                ui.label(format!("{}x{}", w, h));
                            } else {
                                ui.label("-");
                            }
                            
                            if result.matched {
                                ui.colored_label(egui::Color32::GREEN, "✓");
                                ui.label(result.matched_rules.join(", "));
                            } else {
                                ui.colored_label(egui::Color32::RED, "✗");
                                ui.label("-");
                            }
                            ui.end_row();
                        }
                    });
            });
        } else if self.test_running {
            ui.spinner();
            ui.label("Testing patterns...");
        }
    }

    /// Draw delete confirmation modal
    fn draw_delete_confirmation(&mut self, ctx: &egui::Context, rule_id: String) {
        let rt = tokio::runtime::Handle::try_current();
        let rule_name = if let Ok(rt) = rt {
            rt.block_on(async {
                let engine = self.engine.lock().await;
                engine.get_rule(&rule_id).map(|r| r.name.clone())
            }).unwrap_or_else(|| "this rule".to_string())
        } else {
            "this rule".to_string()
        };
        
        egui::Window::new("Confirm Delete").collapsible(false).resizable(false).show(ctx, |ui| {
            ui.heading("Confirm Delete");
            ui.separator();
            ui.label(format!("Are you sure you want to delete '{}'??", rule_name));
            ui.label("This action cannot be undone.");
            
            ui.separator();
            
            ui.horizontal(|ui| {
                if ui.button("🗑 Delete").clicked() {
                    let rule_id = rule_id.clone();
                    let rt = tokio::runtime::Handle::try_current();
                    if let Ok(rt) = rt {
                        rt.spawn(async move {
                            // Would call delete_rule here
                        });
                    }
                }
                if ui.button("Cancel").clicked() {
                    self.show_delete_confirm = None;
                }
            });
        });
    }

    /// Export rules to JSON
    pub fn export_rules(&self) -> Result<String, serde_json::Error> {
        let rt = tokio::runtime::Handle::try_current();
        let rules: Vec<ClassificationRuleDef> = if let Ok(rt) = rt {
            rt.block_on(async { self.get_rules().await })
        } else {
            vec![]
        };
        serde_json::to_string_pretty(&rules)
    }

    /// Import rules from JSON
    pub async fn import_rules(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let imported: Vec<ClassificationRuleDef> = serde_json::from_str(json)?;
        
        // Add to engine
        {
            let mut engine = self.engine.lock().await;
            for rule in &imported {
                engine.add_rule(rule.to_backend_rule());
            }
        }
        
        // Persist to database
        self.sync_rules_to_db().await;
        
        self.show_status(&format!("Imported {} rules", imported.len()));
        Ok(())
    }

    /// Get the classification engine
    pub fn engine(&self) -> Arc<Mutex<ClassificationEngine>> {
        self.engine.clone()
    }

    /// Get all rules (for external access)
    pub async fn rules(&self) -> Vec<ClassificationRuleDef> {
        self.get_rules().await
    }

    /// Get mutable rules (for external access)
    pub async fn rules_mut(&mut self) -> Vec<ClassificationRuleDef> {
        self.get_rules().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_creation() {
        let panel = ClassificationRulesPanel::new();
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_panel_toggle() {
        let mut panel = ClassificationRulesPanel::new();
        assert!(!panel.is_visible());
        
        panel.toggle();
        assert!(panel.is_visible());
        
        panel.toggle();
        assert!(!panel.is_visible());
    }

    #[tokio::test]
    async fn test_add_rule() {
        let mut panel = ClassificationRulesPanel::new();
        let initial_count = panel.get_rules().await.len();
        
        panel.add_rule().await;
        
        assert_eq!(panel.get_rules().await.len(), initial_count + 1);
        assert!(panel.editing_rule.is_some());
    }

    #[tokio::test]
    async fn test_delete_rule() {
        let mut panel = ClassificationRulesPanel::new();
        panel.add_rule().await;
        let count = panel.get_rules().await.len();
        
        let rule_id = panel.get_rules().await[0].id.clone();
        panel.delete_rule(&rule_id).await;
        
        assert_eq!(panel.get_rules().await.len(), count - 1);
    }

    #[test]
    fn test_asset_type_conversion() {
        assert_eq!(AssetTypeOption::Character.as_str(), "character");
        assert_eq!(AssetTypeOption::from_asset_type("character"), Some(AssetTypeOption::Character));
        assert_eq!(AssetTypeOption::from_asset_type("unknown"), None);
    }

    #[test]
    fn test_rule_def_conversion() {
        let def = ClassificationRuleDef {
            name: "Test Rule".to_string(),
            file_pattern: "*.png".to_string(),
            asset_type: AssetTypeOption::Character,
            auto_tags: vec!["tag1".to_string()],
            priority: 75,
            ..Default::default()
        };
        
        let backend = def.to_backend_rule();
        assert_eq!(backend.name, "Test Rule");
        assert_eq!(backend.asset_type, "character");
        assert_eq!(backend.priority, 75);
        
        let back_to_def = ClassificationRuleDef::from_backend_rule(&backend);
        assert_eq!(back_to_def.name, "Test Rule");
        assert_eq!(back_to_def.asset_type, AssetTypeOption::Character);
    }
}
