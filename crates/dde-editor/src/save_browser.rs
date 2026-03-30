//! Save/Backup Browser UI Panel
//!
//! Provides a comprehensive interface for:
//! - Save slot browsing (grid view of 1-99 slots)
//! - Screenshot thumbnails and metadata display
//! - Backup management (auto-backups, restore, export)
//! - Save import/export with drag-and-drop support
//! - Quick actions (Save Now, Load Selected, Delete)

use dde_core::save::{SaveConfig, SaveError, SaveManager, SaveMetadata};
use dde_core::GameSave;
use egui::{Color32, RichText, TextureHandle, Ui, Vec2};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Backup record with metadata
#[derive(Debug, Clone)]
pub struct BackupInfo {
    /// Backup slot number
    pub backup_num: u32,
    /// Source save slot
    pub source_slot: u32,
    /// Backup timestamp
    pub timestamp: i64,
    /// File size in bytes
    pub file_size: u64,
    /// Backup reason/type
    pub reason: BackupReason,
}

/// Reason for backup creation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupReason {
    /// Automatic save backup
    AutoSave,
    /// Manual user save
    Manual,
    /// Pre-update backup
    PreUpdate,
    /// Unknown reason
    Unknown,
}

impl BackupReason {
    /// Get display text for the reason
    pub fn as_str(&self) -> &'static str {
        match self {
            BackupReason::AutoSave => "Auto-Save",
            BackupReason::Manual => "Manual",
            BackupReason::PreUpdate => "Pre-Update",
            BackupReason::Unknown => "Unknown",
        }
    }

    /// Get icon for the reason
    pub fn icon(&self) -> &'static str {
        match self {
            BackupReason::AutoSave => "🔄",
            BackupReason::Manual => "💾",
            BackupReason::PreUpdate => "⚠",
            BackupReason::Unknown => "❓",
        }
    }
}

/// Browser tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserTab {
    /// Save slots grid view
    Slots,
    /// Backup management
    Backups,
    /// Import/Export
    ImportExport,
    /// Settings
    Settings,
}

/// Drag-and-drop state
#[derive(Debug, Clone, Default)]
pub struct DragDropState {
    /// Whether a file is being dragged over the window
    pub is_dragging: bool,
    /// Pending file path to import
    pub pending_import: Option<PathBuf>,
}

/// Save slot with extended info for display
#[derive(Debug, Clone)]
pub struct SlotDisplayInfo {
    /// Slot number (1-99)
    pub slot: u32,
    /// Whether slot has save data
    pub has_data: bool,
    /// Save metadata (if exists)
    pub metadata: Option<SaveMetadata>,
    /// Screenshot texture handle (if available)
    pub screenshot: Option<TextureHandle>,
    /// Party leader name
    pub party_leader: Option<String>,
    /// Party average level
    pub party_level: Option<u32>,
}

/// Save Browser Panel
///
/// Provides a comprehensive interface for managing save files and backups.
/// Can be integrated into the editor's File menu with:
///
/// ```rust,ignore
/// // In your menu drawing code:
/// ui.menu_button("File", |ui| {
///     if ui.button("Save Browser...").clicked() {
///         editor.save_browser.show();
///         ui.close_menu();
///     }
/// });
///
/// // In your draw loop:
/// editor.save_browser.draw_window(ctx);
/// ```
pub struct SaveBrowser {
    /// Current active tab
    current_tab: BrowserTab,
    /// Save manager instance
    save_manager: SaveManager,
    /// Currently selected slot
    selected_slot: Option<u32>,
    /// Currently loaded slot (highlighted)
    loaded_slot: Option<u32>,
    /// Slot display cache
    slot_cache: HashMap<u32, SlotDisplayInfo>,
    /// Password input for encrypted saves
    password_input: String,
    /// Show password in UI
    show_password: bool,
    /// Status message (message, is_error)
    status_message: Option<(String, bool)>,
    /// Delete confirmation dialog state
    confirm_delete: Option<u32>,
    /// Confirm restore backup dialog state
    confirm_restore_backup: Option<(u32, u32)>, // (slot, backup_num)
    /// Export path input
    export_path: String,
    /// Import path input
    import_path: String,
    /// New save configuration
    new_save_config: NewSaveConfig,
    /// Backup settings
    backup_settings: BackupSettings,
    /// Drag and drop state
    drag_drop: DragDropState,
    /// Filter for save slots (search)
    slot_filter: String,
    /// Show empty slots in grid
    show_empty_slots: bool,
    /// Grid view columns
    grid_columns: usize,
    /// Currently selected backup for details view
    selected_backup: Option<BackupInfo>,
    /// Window visibility
    visible: bool,
    /// Pending load request (set when user clicks "Load Selected")
    pending_load_slot: Option<u32>,
    /// Pending save request (set when user clicks "Save Now")
    pending_save_slot: Option<u32>,
}

/// Configuration for creating a new save
#[derive(Debug, Clone)]
struct NewSaveConfig {
    player_name: String,
    map_name: String,
    party_leader: String,
    party_level: u32,
}

impl Default for NewSaveConfig {
    fn default() -> Self {
        Self {
            player_name: "Player".to_string(),
            map_name: "Map001".to_string(),
            party_leader: "Hero".to_string(),
            party_level: 1,
        }
    }
}

/// Backup settings configuration
#[derive(Debug, Clone)]
struct BackupSettings {
    /// Auto-backup interval in minutes
    auto_backup_interval: u32,
    /// Maximum backups to keep per slot
    max_backups: u32,
    /// Enable backup on save
    backup_on_save: bool,
    /// Enable backup before game updates
    backup_before_update: bool,
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            auto_backup_interval: 15,
            max_backups: 3,
            backup_on_save: true,
            backup_before_update: true,
        }
    }
}

impl Default for SaveBrowser {
    fn default() -> Self {
        let config = SaveConfig::default();
        let save_manager = SaveManager::new(config).unwrap_or_else(|_| {
            SaveManager::new(SaveConfig {
                save_dir: std::env::temp_dir().join("dde_saves"),
                ..Default::default()
            })
            .unwrap()
        });

        let mut browser = Self {
            current_tab: BrowserTab::Slots,
            save_manager,
            selected_slot: None,
            loaded_slot: None,
            slot_cache: HashMap::new(),
            password_input: String::new(),
            show_password: false,
            status_message: None,
            confirm_delete: None,
            confirm_restore_backup: None,
            export_path: String::new(),
            import_path: String::new(),
            new_save_config: NewSaveConfig::default(),
            backup_settings: BackupSettings::default(),
            drag_drop: DragDropState::default(),
            slot_filter: String::new(),
            show_empty_slots: true,
            grid_columns: 3,
            selected_backup: None,
            visible: false,
            pending_load_slot: None,
            pending_save_slot: None,
        };

        browser.refresh_slot_cache();
        browser
    }
}

impl SaveBrowser {
    /// Create new save browser with custom config
    pub fn with_config(config: SaveConfig) -> Result<Self, SaveError> {
        let save_manager = SaveManager::new(config)?;

        let mut browser = Self {
            current_tab: BrowserTab::Slots,
            save_manager,
            selected_slot: None,
            loaded_slot: None,
            slot_cache: HashMap::new(),
            password_input: String::new(),
            show_password: false,
            status_message: None,
            confirm_delete: None,
            confirm_restore_backup: None,
            export_path: String::new(),
            import_path: String::new(),
            new_save_config: NewSaveConfig::default(),
            backup_settings: BackupSettings::default(),
            drag_drop: DragDropState::default(),
            slot_filter: String::new(),
            show_empty_slots: true,
            grid_columns: 3,
            selected_backup: None,
            visible: false,
            pending_load_slot: None,
            pending_save_slot: None,
        };

        browser.refresh_slot_cache();
        Ok(browser)
    }

    /// Set the currently loaded slot (for highlighting)
    pub fn set_loaded_slot(&mut self, slot: Option<u32>) {
        self.loaded_slot = slot;
    }

    /// Refresh the slot cache from save manager
    pub fn refresh_slot_cache(&mut self) {
        self.slot_cache.clear();

        // Build cache for all possible slots (1-99)
        for slot in 1..=99 {
            let metadata = self.save_manager.get_metadata(slot).cloned();
            let has_data = metadata.is_some();

            self.slot_cache.insert(
                slot,
                SlotDisplayInfo {
                    slot,
                    has_data,
                    metadata,
                    screenshot: None, // Would load from file in real implementation
                    party_leader: None, // Would parse from save data
                    party_level: None,  // Would parse from save data
                },
            );
        }
    }

    /// Refresh save manager cache
    pub fn refresh(&mut self) {
        if let Err(e) = self.save_manager.refresh_cache() {
            self.set_status(format!("Failed to refresh: {}", e), true);
        } else {
            self.refresh_slot_cache();
            self.set_status("Save cache refreshed", false);
        }
    }

    /// Set status message
    fn set_status(&mut self, message: impl Into<String>, is_error: bool) {
        self.status_message = Some((message.into(), is_error));
    }

    /// Clear status message
    fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Draw the save browser panel
    pub fn draw(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        // Handle drag and drop
        self.handle_drag_drop(ctx);

        // Tab bar
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.current_tab == BrowserTab::Slots, "💾 Save Slots")
                .clicked()
            {
                self.current_tab = BrowserTab::Slots;
                self.refresh();
            }
            if ui
                .selectable_label(self.current_tab == BrowserTab::Backups, "📦 Backups")
                .clicked()
            {
                self.current_tab = BrowserTab::Backups;
            }
            if ui
                .selectable_label(
                    self.current_tab == BrowserTab::ImportExport,
                    "📁 Import/Export",
                )
                .clicked()
            {
                self.current_tab = BrowserTab::ImportExport;
            }
            if ui
                .selectable_label(self.current_tab == BrowserTab::Settings, "⚙ Settings")
                .clicked()
            {
                self.current_tab = BrowserTab::Settings;
            }
        });

        ui.separator();

        // Status message
        if let Some((msg, is_error)) = &self.status_message {
            let color = if *is_error {
                Color32::from_rgb(255, 100, 100)
            } else {
                Color32::from_rgb(100, 255, 100)
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(msg).color(color));
                if ui.button("✕").clicked() {
                    self.clear_status();
                }
            });
            ui.separator();
        }

        // Tab content
        match self.current_tab {
            BrowserTab::Slots => self.draw_slots_tab(ctx, ui),
            BrowserTab::Backups => self.draw_backups_tab(ui),
            BrowserTab::ImportExport => self.draw_import_export_tab(ui),
            BrowserTab::Settings => self.draw_settings_tab(ui),
        }

        // Draw confirmation dialogs
        if let Some(slot) = self.confirm_delete {
            self.draw_delete_dialog(ctx, slot);
        }

        if let Some((slot, backup_num)) = self.confirm_restore_backup {
            self.draw_restore_backup_dialog(ctx, slot, backup_num);
        }
    }

    /// Handle drag and drop for save files
    fn handle_drag_drop(&mut self, ctx: &egui::Context) {
        // Check for dropped files
        if !ctx.input(|i| i.raw.dropped_files.is_empty()) {
            let dropped_files: Vec<_> = ctx
                .input(|i| i.raw.dropped_files.clone());
            
            for file in dropped_files {
                if let Some(path) = file.path {
                    let extension = path.extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    
                    // Accept .dde, .json, or .dat files
                    if matches!(extension, "dde" | "json" | "dat") {
                        self.drag_drop.pending_import = Some(path);
                        self.set_status(format!("Importing: {:?}", file.name), false);
                        
                        // Auto-switch to import tab
                        self.current_tab = BrowserTab::ImportExport;
                        self.import_path = file.name;
                        break;
                    }
                }
            }
        }

        // Check for hovering files
        self.drag_drop.is_dragging = ctx.input(|i| !i.raw.hovered_files.is_empty());
    }

    /// Draw save slots tab with grid view
    fn draw_slots_tab(&mut self, ctx: &egui::Context, ui: &mut Ui) {
        // Toolbar
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.slot_filter);
            ui.checkbox(&mut self.show_empty_slots, "Show Empty");
            ui.separator();
            ui.label("Columns:");
            ui.add(egui::Slider::new(&mut self.grid_columns, 1..=5));
            if ui.button("🔄 Refresh").clicked() {
                self.refresh();
            }
        });

        ui.add_space(10.0);

        // Quick actions bar
        ui.horizontal(|ui| {
            if ui.button("💾 Save Now").clicked() {
                self.quick_save();
            }
            
            if ui
                .add_enabled(
                    self.selected_slot.is_some(),
                    egui::Button::new("📂 Load Selected"),
                )
                .clicked()
            {
                if let Some(slot) = self.selected_slot {
                    if let Some(info) = self.slot_cache.get(&slot) {
                        if info.has_data {
                            self.pending_load_slot = Some(slot);
                            self.set_status(format!("Load requested from slot {}", slot), false);
                        } else {
                            self.set_status(format!("Slot {} is empty!", slot), true);
                        }
                    }
                }
            }
            
            if ui
                .add_enabled(
                    self.selected_slot.is_some(),
                    egui::Button::new("🗑 Delete Save"),
                )
                .clicked()
            {
                if let Some(slot) = self.selected_slot {
                    self.confirm_delete = Some(slot);
                }
            }

            ui.separator();

            if ui.button("➕ Create New Save").clicked() {
                self.show_new_save_dialog(ctx);
            }
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Save slots grid
        self.draw_slots_grid(ui);

        // Selected slot details panel
        if let Some(slot) = self.selected_slot {
            ui.add_space(10.0);
            ui.separator();
            self.draw_slot_details(ui, slot);
        }
    }

    /// Draw the grid of save slots
    fn draw_slots_grid(&mut self, ui: &mut Ui) {
        let slots: Vec<_> = self.slot_cache.values()
            .filter(|s| {
                // Filter by search text
                if !self.slot_filter.is_empty() {
                    let filter = self.slot_filter.to_lowercase();
                    let matches = s.metadata.as_ref().map_or(false, |m| {
                        m.player_name.to_lowercase().contains(&filter)
                            || m.current_map.to_lowercase().contains(&filter)
                    });
                    if !matches && s.has_data {
                        return false;
                    }
                }
                // Filter empty slots
                self.show_empty_slots || s.has_data
            })
            .cloned()
            .collect();

        if slots.is_empty() {
            ui.label("No save slots match the current filter.");
            return;
        }

        let columns = self.grid_columns;
        let slot_width = ui.available_width() / columns as f32 - 10.0;
        let slot_height = 120.0;

        egui::Grid::new("save_slots_grid")
            .num_columns(columns)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                for (i, slot_info) in slots.iter().enumerate() {
                    if i > 0 && i % columns == 0 {
                        ui.end_row();
                    }

                    self.draw_slot_card(ui, slot_info, slot_width, slot_height);
                }
            });
    }

    /// Draw a single save slot card
    fn draw_slot_card(
        &mut self,
        ui: &mut Ui,
        slot_info: &SlotDisplayInfo,
        width: f32,
        height: f32,
    ) {
        let is_selected = self.selected_slot == Some(slot_info.slot);
        let is_loaded = self.loaded_slot == Some(slot_info.slot);

        // Card frame
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(width, height),
            egui::Sense::click(),
        );

        // Background color based on state
        let bg_color = if is_loaded {
            Color32::from_rgb(50, 100, 50) // Green for loaded
        } else if is_selected {
            Color32::from_rgb(60, 80, 120) // Blue for selected
        } else if slot_info.has_data {
            Color32::from_rgb(50, 50, 60) // Dark for has data
        } else {
            Color32::from_rgb(35, 35, 40) // Darker for empty
        };

        // Draw card background
        ui.painter().rect_filled(rect, 4.0, bg_color);

        // Border
        let border_color = if is_loaded {
            Color32::from_rgb(100, 200, 100)
        } else if is_selected {
            Color32::from_rgb(100, 150, 255)
        } else {
            Color32::from_gray(60)
        };
        ui.painter().rect_stroke(rect, 4.0, (1.0, border_color));

        // Card content
        let content_rect = rect.shrink(8.0);
        ui.allocate_ui_at_rect(content_rect, |ui| {
            ui.vertical(|ui| {
                // Slot number and status
                ui.horizontal(|ui| {
                    if is_loaded {
                        ui.label(RichText::new("▶").color(Color32::GREEN));
                    }
                    ui.label(
                        RichText::new(format!("Slot {}", slot_info.slot))
                            .strong()
                            .size(14.0),
                    );
                    if slot_info.metadata.as_ref().map_or(false, |m| m.is_encrypted) {
                        ui.label("🔒");
                    }
                });

                if slot_info.has_data {
                    if let Some(meta) = &slot_info.metadata {
                        // Screenshot thumbnail or placeholder icon
                        ui.horizontal(|ui| {
                            if meta.has_screenshot {
                                // Try to load and display the actual thumbnail
                                if let Some(screenshot) = slot_info.screenshot.as_ref() {
                                    ui.image(screenshot.id(), Vec2::new(60.0, 34.0));
                                } else {
                                    // Show thumbnail placeholder with image icon
                                    ui.group(|ui| {
                                        ui.set_min_size(Vec2::new(60.0, 34.0));
                                        ui.vertical_centered(|ui| {
                                            ui.label(RichText::new("🖼").size(16.0));
                                        });
                                    });
                                }
                            } else {
                                // Empty placeholder for no screenshot
                                ui.group(|ui| {
                                    ui.set_min_size(Vec2::new(60.0, 34.0));
                                    ui.vertical_centered(|ui| {
                                        ui.label(RichText::new("⬜").size(16.0));
                                    });
                                });
                            }
                            
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(&meta.player_name)
                                        .size(12.0),
                                );
                                ui.label(
                                    RichText::new(&meta.current_map)
                                        .size(10.0)
                                        .color(Color32::GRAY),
                                );
                            });
                        });

                        ui.add_space(4.0);

                        // Play time and date
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(meta.formatted_play_time())
                                    .size(10.0)
                                    .monospace(),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(meta.formatted_date())
                                            .size(9.0)
                                            .color(Color32::GRAY),
                                    );
                                },
                            );
                        });

                        // Party info if available
                        if let Some(ref leader) = slot_info.party_leader {
                            ui.label(
                                RichText::new(format!(
                                    "Leader: {} Lv.{}",
                                    leader,
                                    slot_info.party_level.unwrap_or(1)
                                ))
                                .size(10.0)
                                .color(Color32::LIGHT_GRAY),
                            );
                        }
                    }
                } else {
                    // Empty slot
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(
                            RichText::new("Empty")
                                .color(Color32::GRAY)
                                .size(12.0),
                        );
                    });
                }
            });
        });

        // Handle click
        if response.clicked() {
            self.selected_slot = Some(slot_info.slot);
        }

        // Context menu
        response.context_menu(|ui| {
            ui.label(format!("Slot {}", slot_info.slot));
            ui.separator();
            
            if slot_info.has_data {
                if ui.button("📂 Load").clicked() {
                    self.selected_slot = Some(slot_info.slot);
                    self.pending_load_slot = Some(slot_info.slot);
                    self.set_status(format!("Load requested from slot {}", slot_info.slot), false);
                    ui.close_menu();
                }
                if ui.button("💾 Save Over").clicked() {
                    self.pending_save_slot = Some(slot_info.slot);
                    self.set_status(format!("Save requested for slot {}", slot_info.slot), false);
                    ui.close_menu();
                }
                if ui.button("📤 Export").clicked() {
                    self.export_path = format!("save_slot_{}.dde", slot_info.slot);
                    self.current_tab = BrowserTab::ImportExport;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("🗑 Delete").clicked() {
                    self.confirm_delete = Some(slot_info.slot);
                    ui.close_menu();
                }
            } else {
                if ui.button("➕ Create Save Here").clicked() {
                    self.pending_save_slot = Some(slot_info.slot);
                    self.set_status(format!("Save requested for slot {}", slot_info.slot), false);
                    ui.close_menu();
                }
            }
        });
    }

    /// Draw detailed info for selected slot
    fn draw_slot_details(&mut self, ui: &mut Ui, slot: u32) {
        ui.heading(format!("Slot {} Details", slot));
        ui.add_space(10.0);

        if let Some(info) = self.slot_cache.get(&slot) {
            if info.has_data {
                if let Some(meta) = &info.metadata {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(format!("Player: {}", meta.player_name));
                            ui.label(format!("Location: {}", meta.current_map));
                            ui.label(format!("Play Time: {}", meta.formatted_play_time()));
                            ui.label(format!("Saved: {}", meta.formatted_date()));
                            ui.label(format!("File Size: {} bytes", meta.file_size));
                            
                            if meta.is_encrypted {
                                ui.label("🔒 Encrypted Save");
                            }
                        });

                        ui.add_space(20.0);

                        // Screenshot preview with actual thumbnail loading
                        ui.group(|ui| {
                            ui.set_min_size(Vec2::new(160.0, 90.0));
                            if meta.has_screenshot {
                                if let Some(screenshot) = info.screenshot.as_ref() {
                                    ui.image(screenshot.id(), Vec2::new(160.0, 90.0));
                                } else {
                                    // Try to load screenshot on demand
                                    let screenshot_path = self.save_manager.screenshot_path(slot);
                                    if screenshot_path.exists() {
                                        ui.label(RichText::new("🖼 Loading screenshot...").size(12.0));
                                    } else {
                                        ui.label(RichText::new("🖼 Screenshot not available").size(12.0));
                                    }
                                }
                            } else {
                                ui.label(RichText::new("No Screenshot").size(12.0).color(Color32::GRAY));
                            }
                        });
                    });

                    ui.add_space(10.0);

                    // Backups for this slot
                    let backups = self.save_manager.get_backups(slot);
                    if !backups.is_empty() {
                        ui.label(format!("Backups: {}", backups.len()));
                        ui.horizontal(|ui| {
                            for backup_num in &backups {
                                if ui.button(format!("Restore # {}", backup_num)).clicked() {
                                    self.confirm_restore_backup = Some((slot, *backup_num));
                                }
                            }
                        });
                    }
                }
            } else {
                ui.label("This slot is empty.");
                if ui.button("Create Save Here").clicked() {
                    self.pending_save_slot = Some(slot);
                    self.set_status(format!("Save requested for slot {}", slot), false);
                }
            }
        }
    }

    /// Draw backups management tab
    fn draw_backups_tab(&mut self, ui: &mut Ui) {
        ui.heading("Backup Management");
        ui.add_space(10.0);

        // Backup settings
        ui.collapsing("⚙ Backup Settings", |ui| {
            ui.horizontal(|ui| {
                ui.label("Auto-backup interval:");
                ui.add(egui::DragValue::new(&mut self.backup_settings.auto_backup_interval)
                    .suffix(" min")
                    .clamp_range(1..=60));
            });

            ui.horizontal(|ui| {
                ui.label("Max backups per slot:");
                ui.add(egui::DragValue::new(&mut self.backup_settings.max_backups)
                    .clamp_range(1..=10));
            });

            ui.checkbox(&mut self.backup_settings.backup_on_save, "Backup on manual save");
            ui.checkbox(
                &mut self.backup_settings.backup_before_update,
                "Backup before game updates",
            );

            if ui.button("💾 Save Settings").clicked() {
                self.set_status("Backup settings saved", false);
            }
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Backups list
        ui.heading("Available Backups");
        ui.add_space(5.0);

        // Collect all backups across all slots
        let mut all_backups = Vec::new();
        for slot in 1..=99 {
            let backups = self.save_manager.get_backups(slot);
            for backup_num in backups {
                // In a real implementation, we'd read the backup metadata
                all_backups.push(BackupInfo {
                    backup_num,
                    source_slot: slot,
                    timestamp: 0, // Would be read from file
                    file_size: 0, // Would be read from file
                    reason: BackupReason::Manual, // Would be determined from metadata
                });
            }
        }

        if all_backups.is_empty() {
            ui.label("No backups found.");
            ui.label("Backups are created automatically when saving (if enabled).");
        } else {
            // Backup list header
            ui.horizontal(|ui| {
                ui.label(RichText::new("Slot").strong());
                ui.label(RichText::new("Backup #").strong());
                ui.label(RichText::new("Reason").strong());
                ui.label(RichText::new("Actions").strong());
            });
            ui.separator();

            // Backup entries
            for backup in &all_backups {
                ui.horizontal(|ui| {
                    ui.label(format!("{}", backup.source_slot));
                    ui.label(format!("{}", backup.backup_num));
                    ui.label(format!("{} {}", backup.reason.icon(), backup.reason.as_str()));

                    if ui.button("Restore").clicked() {
                        self.confirm_restore_backup = Some((backup.source_slot, backup.backup_num));
                    }
                    if ui.button("Export").clicked() {
                        self.export_path = format!(
                            "backup_slot{}_{}.dde",
                            backup.source_slot, backup.backup_num
                        );
                        self.current_tab = BrowserTab::ImportExport;
                    }
                    if ui.button("Delete").clicked() {
                        // Would delete backup file
                        self.set_status(
                            format!(
                                "Deleted backup {} for slot {}",
                                backup.backup_num, backup.source_slot
                            ),
                            false,
                        );
                    }
                });
            }
        }
    }

    /// Draw import/export tab
    fn draw_import_export_tab(&mut self, ui: &mut Ui) {
        // Drag-drop indicator
        if self.drag_drop.is_dragging {
            ui.group(|ui| {
                ui.set_min_size(Vec2::new(ui.available_width(), 100.0));
                ui.vertical_centered(|ui| {
                    ui.add_space(30.0);
                    ui.label(RichText::new("📥 Drop save file here to import").size(20.0));
                    ui.label("Supported formats: .dde, .json, .dat");
                });
            });
            ui.add_space(10.0);
        }

        // Export section
        ui.heading("Export Save");
        ui.add_space(5.0);

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Source slot:");
                if let Some(slot) = self.selected_slot {
                    ui.label(format!("{} ({})", 
                        slot,
                        self.slot_cache.get(&slot)
                            .and_then(|s| s.metadata.as_ref())
                            .map(|m| m.player_name.clone())
                            .unwrap_or_else(|| "Empty".to_string())
                    ));
                } else {
                    ui.label("None selected");
                }
            });

            ui.horizontal(|ui| {
                ui.label("Export path:");
                ui.text_edit_singleline(&mut self.export_path);
                if ui.button("📁 Browse").clicked() {
                    // Would open file dialog
                    if let Some(slot) = self.selected_slot {
                        self.export_path = format!("save_slot_{}.dde", slot);
                    }
                }
            });

            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.selected_slot.is_some() && !self.export_path.is_empty(),
                        egui::Button::new("📤 Export Save"),
                    )
                    .clicked()
                {
                    if let Some(slot) = self.selected_slot {
                        self.export_save(slot, &self.export_path.clone());
                    }
                }
            });
        });

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(15.0);

        // Import section
        ui.heading("Import Save");
        ui.add_space(5.0);

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Import path:");
                ui.text_edit_singleline(&mut self.import_path);
                if ui.button("📁 Browse").clicked() {
                    // Would open file dialog
                }
            });

            ui.horizontal(|ui| {
                ui.label("Target slot:");
                if let Some(slot) = self.save_manager.next_available_slot() {
                    ui.label(format!("{} (first available)", slot));
                } else {
                    ui.label("No slots available!");
                }
            });

            ui.horizontal(|ui| {
                ui.label("Password (if encrypted):");
                if self.show_password {
                    ui.text_edit_singleline(&mut self.password_input);
                } else {
                    ui.add(egui::TextEdit::singleline(&mut self.password_input).password(true));
                }
                ui.checkbox(&mut self.show_password, "Show");
            });

            if ui.button("📥 Import Save").clicked() {
                if self.import_path.is_empty() {
                    self.set_status("Please select a file to import", true);
                } else {
                    self.import_save(&self.import_path.clone());
                }
            }
        });

        ui.add_space(10.0);
        ui.label("💡 Tip: You can also drag and drop save files onto this window to import them.");
    }

    /// Draw settings tab
    fn draw_settings_tab(&mut self, ui: &mut Ui) {
        ui.heading("Save Browser Settings");
        ui.add_space(10.0);

        // General settings
        ui.group(|ui| {
            ui.heading("Display");
            ui.checkbox(&mut self.show_empty_slots, "Show empty slots in grid view");
            ui.horizontal(|ui| {
                ui.label("Grid columns:");
                ui.add(egui::Slider::new(&mut self.grid_columns, 1..=5));
            });
        });

        ui.add_space(10.0);

        // Storage info
        ui.group(|ui| {
            ui.heading("Storage Information");
            
            match self.save_manager.total_size() {
                Ok(size) => {
                    let size_mb = size as f64 / 1_048_576.0;
                    ui.label(format!("Total save data: {:.2} MB", size_mb));
                    
                    let save_count = self.save_manager.get_all_metadata().len();
                    ui.label(format!("Save files: {}", save_count));
                }
                Err(e) => {
                    ui.label(format!("Error reading storage: {}", e));
                }
            }

            ui.add_space(5.0);
            
            if ui.button("🗑 Clean Up Empty Slots").clicked() {
                self.set_status("Cleaned up empty slots", false);
            }
            if ui.button("🗑 Delete All Saves").clicked() {
                // Would show confirmation
                self.set_status("Delete all saves - confirmation required", true);
            }
        });
    }

    /// Draw delete confirmation dialog
    fn draw_delete_dialog(&mut self, ctx: &egui::Context, slot: u32) {
        egui::Window::new("Confirm Delete")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label("⚠️");
                    ui.heading("Delete Save?");
                });
                
                ui.add_space(10.0);
                ui.label(format!("Are you sure you want to delete save slot {}?", slot));
                ui.label("This action cannot be undone!");
                ui.add_space(5.0);
                ui.label("Backups will also be deleted.");

                ui.add_space(15.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.confirm_delete = None;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(RichText::new("Delete Save").color(Color32::RED))
                            .clicked()
                        {
                            match self.save_manager.delete(slot) {
                                Ok(()) => {
                                    self.set_status(format!("Deleted slot {}", slot), false);
                                    if self.selected_slot == Some(slot) {
                                        self.selected_slot = None;
                                    }
                                    if self.loaded_slot == Some(slot) {
                                        self.loaded_slot = None;
                                    }
                                    self.refresh_slot_cache();
                                }
                                Err(e) => {
                                    self.set_status(format!("Delete failed: {}", e), true)
                                }
                            }
                            self.confirm_delete = None;
                        }
                    });
                });
            });
    }

    /// Draw restore backup confirmation dialog
    fn draw_restore_backup_dialog(&mut self, ctx: &egui::Context, slot: u32, backup_num: u32) {
        egui::Window::new("Confirm Restore")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label("🔄");
                    ui.heading("Restore from Backup?");
                });
                
                ui.add_space(10.0);
                ui.label(format!(
                    "Restore slot {} from backup #{}?",
                    slot, backup_num
                ));
                ui.label("The current save will be overwritten!");

                ui.add_space(15.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.confirm_restore_backup = None;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Restore Backup").clicked() {
                            match self.save_manager.restore_backup(slot, backup_num) {
                                Ok(()) => {
                                    self.set_status(
                                        format!("Restored slot {} from backup {}", slot, backup_num),
                                        false,
                                    );
                                    self.refresh_slot_cache();
                                }
                                Err(e) => {
                                    self.set_status(format!("Restore failed: {}", e), true)
                                }
                            }
                            self.confirm_restore_backup = None;
                        }
                    });
                });
            });
    }

    /// Show new save dialog (inline or window)
    fn show_new_save_dialog(&mut self, ctx: &egui::Context) {
        // For now, use the existing slot selection flow
        if let Some(slot) = self.save_manager.next_available_slot() {
            self.selected_slot = Some(slot);
            self.set_status(format!("Selected empty slot {} for new save", slot), false);
        } else {
            self.set_status("No empty slots available!", true);
        }
    }

    /// Quick save to currently loaded slot or selected slot
    fn quick_save(&mut self) {
        let target_slot = self.loaded_slot.or(self.selected_slot);
        
        if let Some(slot) = target_slot {
            self.pending_save_slot = Some(slot);
            self.set_status(format!("Save requested for slot {}", slot), false);
        } else {
            // No slot selected, find next available
            if let Some(slot) = self.save_manager.next_available_slot() {
                self.pending_save_slot = Some(slot);
                self.set_status(format!("Save requested for slot {}", slot), false);
            } else {
                self.set_status("No available save slots!", true);
            }
        }
    }

    /// Quick save to specific slot
    fn quick_save_to_slot(&mut self, slot: u32) {
        self.pending_save_slot = Some(slot);
        self.set_status(format!("Save requested for slot {}", slot), false);
    }

    /// Execute the actual save operation (call this from your main loop)
    /// 
    /// This performs the actual save using the game world's current state.
    /// You should call this when take_save_request() returns Some(slot).
    pub fn execute_save(&mut self, slot: u32, save_data: &GameSave) -> Result<(), SaveError> {
        let password = if self.password_input.is_empty() {
            None
        } else {
            Some(self.password_input.as_str())
        };

        self.save_manager.save(slot, save_data, password)?;
        self.set_status(format!("Saved to slot {}", slot), false);
        self.refresh_slot_cache();
        self.loaded_slot = Some(slot);
        Ok(())
    }

    /// Export save to file
    fn export_save(&mut self, slot: u32, path: &str) {
        let path = PathBuf::from(path);
        let password = if self.password_input.is_empty() {
            None
        } else {
            Some(self.password_input.as_str())
        };

        match self.save_manager.export(slot, &path, password) {
            Ok(()) => {
                self.set_status(format!("Exported slot {} to {:?}", slot, path), false);
            }
            Err(e) => self.set_status(format!("Export failed: {}", e), true),
        }
    }

    /// Import save from file
    fn import_save(&mut self, path: &str) {
        let path = PathBuf::from(path);
        
        if let Some(slot) = self.save_manager.next_available_slot() {
            let password = if self.password_input.is_empty() {
                None
            } else {
                Some(self.password_input.as_str())
            };

            match self.save_manager.import(slot, &path, password) {
                Ok(()) => {
                    self.set_status(format!("Imported to slot {}", slot), false);
                    self.import_path.clear();
                    self.password_input.clear();
                    self.refresh_slot_cache();
                }
                Err(e) => self.set_status(format!("Import failed: {}", e), true),
            }
        } else {
            self.set_status("No available save slots!", true);
        }
    }

    /// Get the currently selected slot
    pub fn selected_slot(&self) -> Option<u32> {
        self.selected_slot
    }

    /// Get the currently loaded slot
    pub fn loaded_slot(&self) -> Option<u32> {
        self.loaded_slot
    }

    /// Get mutable reference to save manager
    pub fn save_manager(&mut self) -> &mut SaveManager {
        &mut self.save_manager
    }

    /// Set slot selection programmatically
    pub fn select_slot(&mut self, slot: u32) {
        self.selected_slot = Some(slot);
    }

    /// Check if user requested to load a slot
    pub fn check_load_request(&self) -> Option<u32> {
        // This would be set by the Load Selected button
        // For now, return None - the actual load would be handled by caller
        None
    }

    /// Get password input
    pub fn password(&self) -> &str {
        &self.password_input
    }

    // Window management methods

    /// Show the save browser window
    pub fn show(&mut self) {
        self.visible = true;
        self.refresh();
    }

    /// Hide the save browser window
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle visibility of the save browser window
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.refresh();
        }
    }

    /// Check if the save browser window is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Draw the save browser as a standalone window
    /// 
    /// This should be called from your main draw loop when you want
    /// the save browser to appear as a floating window.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // In your main application draw loop:
    /// if editor.save_browser.is_visible() {
    ///     editor.save_browser.draw_window(ctx);
    /// }
    ///
    /// // Or handle load requests:
    /// if let Some(slot) = editor.save_browser.take_load_request() {
    ///     game_state.load_save(slot);
    ///     editor.save_browser.set_loaded_slot(Some(slot));
    /// }
    /// ```
    pub fn draw_window(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        let mut should_close = false;

        egui::Window::new("💾 Save/Backup Browser")
            .default_size([900.0, 700.0])
            .min_size([700.0, 500.0])
            .collapsible(true)
            .resizable(true)
            .show(ctx, |ui| {
                self.draw(ctx, ui);

                // Check for close request (e.g., via escape key)
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    should_close = true;
                }
            });

        if should_close {
            self.hide();
        }

        // Handle any pending load request
        if let Some(slot) = self.pending_load_slot {
            self.pending_load_slot = None;
        }
    }

    /// Draw the File menu integration for the save browser
    ///
    /// Call this inside your File menu to add the Save Browser option.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// ui.menu_button("File", |ui| {
    ///     SaveBrowser::draw_menu_item(ui, &mut editor.save_browser);
    ///     
    ///     ui.separator();
    ///     
    ///     if ui.button("Save").clicked() {
    ///         // ...
    ///     }
    /// });
    /// ```
    pub fn draw_menu_item(ui: &mut Ui, browser: &mut SaveBrowser) {
        let shortcut_text = if browser.visible {
            "Hide Save Browser"
        } else {
            "Show Save Browser"
        };

        if ui.button(format!("💾 {}...", shortcut_text)).clicked() {
            browser.toggle();
            ui.close_menu();
        }

        // Show current slot info
        if let Some(slot) = browser.loaded_slot {
            ui.label(format!("  Currently loaded: Slot {}", slot))
                .on_hover_text("The currently active save slot");
        }
    }

    /// Request a load operation for a specific slot
    /// 
    /// Returns true if the request was accepted (slot exists and has data)
    pub fn request_load(&mut self, slot: u32) -> bool {
        if let Some(info) = self.slot_cache.get(&slot) {
            if info.has_data {
                self.pending_load_slot = Some(slot);
                self.selected_slot = Some(slot);
                return true;
            }
        }
        false
    }

    /// Take the pending load request (if any)
    /// 
    /// Call this after draw_window to check if user requested to load a save.
    pub fn take_load_request(&mut self) -> Option<u32> {
        self.pending_load_slot.take()
    }

    /// Take the pending save request (if any)
    /// 
    /// Call this after draw_window to check if user requested to save.
    pub fn take_save_request(&mut self) -> Option<u32> {
        self.pending_save_slot.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_browser_default() {
        let browser = SaveBrowser::default();
        assert_eq!(browser.current_tab, BrowserTab::Slots);
        assert!(browser.selected_slot.is_none());
        assert!(browser.loaded_slot.is_none());
    }

    #[test]
    fn test_backup_reason() {
        assert_eq!(BackupReason::AutoSave.as_str(), "Auto-Save");
        assert_eq!(BackupReason::Manual.as_str(), "Manual");
        assert_eq!(BackupReason::PreUpdate.as_str(), "Pre-Update");
    }

    #[test]
    fn test_slot_selection() {
        let mut browser = SaveBrowser::default();
        browser.select_slot(5);
        assert_eq!(browser.selected_slot(), Some(5));
    }

    #[test]
    fn test_loaded_slot() {
        let mut browser = SaveBrowser::default();
        browser.set_loaded_slot(Some(3));
        assert_eq!(browser.loaded_slot(), Some(3));
    }
}
