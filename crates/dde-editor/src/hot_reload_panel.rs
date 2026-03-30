//! Enhanced Hot Reload Panel
//!
//! Comprehensive editor UI for monitoring and controlling both asset and Lua script
//! hot-reload systems. Features watched path management, module reload tracking,
//! change history, and detailed settings.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// ============================================================================
// Data Structures
// ============================================================================

/// Hot reload panel UI state
pub struct HotReloadPanel {
    visible: bool,
    selected_tab: HotReloadTab,
    asset_state: AssetReloadState,
    lua_state: LuaReloadState,
    change_log: Vec<ChangeLogEntry>,
    settings: HotReloadSettings,
    status_message: Option<(String, StatusType, Instant)>,
    last_update: Instant,
    new_path_input: String,
    selected_asset_path: Option<PathBuf>,
    selected_module: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HotReloadTab {
    Assets,
    LuaModules,
    ChangeLog,
    Settings,
}

#[derive(Debug, Clone)]
struct AssetReloadState {
    watched_paths: Vec<WatchedPath>,
    pending_changes: Vec<PendingAssetChange>,
    debounce_ms: u64,
    batch_delay_ms: u64,
    enabled: bool,
}

#[derive(Debug, Clone)]
struct WatchedPath {
    path: PathBuf,
    asset_type: AssetType,
    active: bool,
    added_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetType {
    Texture,
    Shader,
    Mesh,
    Audio,
    LuaScript,
    SpriteSheet,
    Tileset,
}

impl AssetType {
    fn name(&self) -> &'static str {
        match self {
            AssetType::Texture => "Texture",
            AssetType::Shader => "Shader",
            AssetType::Mesh => "Mesh",
            AssetType::Audio => "Audio",
            AssetType::LuaScript => "Lua Script",
            AssetType::SpriteSheet => "Sprite Sheet",
            AssetType::Tileset => "Tileset",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            AssetType::Texture => "🖼️",
            AssetType::Shader => "🎨",
            AssetType::Mesh => "🔷",
            AssetType::Audio => "🔊",
            AssetType::LuaScript => "📜",
            AssetType::SpriteSheet => "🎬",
            AssetType::Tileset => "🧱",
        }
    }
}

#[derive(Debug, Clone)]
struct PendingAssetChange {
    path: PathBuf,
    asset_type: AssetType,
    change_type: ChangeType,
    timestamp: Instant,
}

#[derive(Debug, Clone)]
struct LuaReloadState {
    modules: HashMap<String, LuaModuleInfo>,
    enabled: bool,
    module_filter: String,
}

#[derive(Debug, Clone)]
struct LuaModuleInfo {
    path: PathBuf,
    status: ModuleStatus,
    last_reload: Option<Instant>,
    last_error: Option<LuaError>,
    content_hash: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModuleStatus {
    Loaded,
    Modified,
    Reloading,
    Error,
    Unloaded,
}

impl ModuleStatus {
    fn name(&self) -> &'static str {
        match self {
            ModuleStatus::Loaded => "Loaded",
            ModuleStatus::Modified => "Modified",
            ModuleStatus::Reloading => "Reloading",
            ModuleStatus::Error => "Error",
            ModuleStatus::Unloaded => "Unloaded",
        }
    }

    fn color(&self) -> egui::Color32 {
        match self {
            ModuleStatus::Loaded => egui::Color32::GREEN,
            ModuleStatus::Modified => egui::Color32::YELLOW,
            ModuleStatus::Reloading => egui::Color32::LIGHT_BLUE,
            ModuleStatus::Error => egui::Color32::RED,
            ModuleStatus::Unloaded => egui::Color32::GRAY,
        }
    }
}

#[derive(Debug, Clone)]
struct LuaError {
    message: String,
    line: Option<usize>,
    timestamp: Instant,
}

#[derive(Debug, Clone)]
struct ChangeLogEntry {
    file_name: String,
    path: PathBuf,
    change_type: ChangeType,
    timestamp: Instant,
    reload_status: ReloadStatus,
    source_type: SourceType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChangeType {
    Modified,
    Created,
    Deleted,
}

impl ChangeType {
    fn name(&self) -> &'static str {
        match self {
            ChangeType::Modified => "Modified",
            ChangeType::Created => "Created",
            ChangeType::Deleted => "Deleted",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            ChangeType::Modified => "??",
            ChangeType::Created => "?",
            ChangeType::Deleted => "???",
        }
    }

    fn color(&self) -> egui::Color32 {
        match self {
            ChangeType::Modified => egui::Color32::YELLOW,
            ChangeType::Created => egui::Color32::GREEN,
            ChangeType::Deleted => egui::Color32::RED,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReloadStatus {
    Success,
    Failed,
    Pending,
    Skipped,
}

impl ReloadStatus {
    fn name(&self) -> &'static str {
        match self {
            ReloadStatus::Success => "Success",
            ReloadStatus::Failed => "Failed",
            ReloadStatus::Pending => "Pending",
            ReloadStatus::Skipped => "Skipped",
        }
    }

    fn color(&self) -> egui::Color32 {
        match self {
            ReloadStatus::Success => egui::Color32::GREEN,
            ReloadStatus::Failed => egui::Color32::RED,
            ReloadStatus::Pending => egui::Color32::YELLOW,
            ReloadStatus::Skipped => egui::Color32::GRAY,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceType {
    Asset,
    LuaModule,
}

#[derive(Debug, Clone)]
struct HotReloadSettings {
    enabled: bool,
    pause_on_error: bool,
    show_notifications: bool,
    max_log_entries: usize,
    auto_clear_log: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusType {
    Info,
    Success,
    Warning,
    Error,
}

impl StatusType {
    fn color(&self) -> egui::Color32 {
        match self {
            StatusType::Info => egui::Color32::LIGHT_BLUE,
            StatusType::Success => egui::Color32::GREEN,
            StatusType::Warning => egui::Color32::YELLOW,
            StatusType::Error => egui::Color32::RED,
        }
    }
}

// ============================================================================
// Trait Interfaces for Backend Integration
// ============================================================================

/// Interface for asset hot reload operations
pub trait AssetHotReloadInterface {
    fn watched_paths(&self) -> Vec<(PathBuf, AssetType, bool)>;
    fn add_watch_path(&mut self, path: &Path, asset_type: AssetType) -> Result<(), String>;
    fn remove_watch_path(&mut self, path: &Path) -> Result<(), String>;
    fn toggle_watch_path(&mut self, path: &Path, active: bool) -> Result<(), String>;
    fn pending_changes(&self) -> Vec<(PathBuf, AssetType, ChangeType)>;
    fn process_all_changes(&mut self) -> Vec<(PathBuf, ReloadStatus)>;
    fn force_reload_asset(&mut self, path: &Path) -> ReloadStatus;
    fn debounce_ms(&self) -> u64;
    fn set_debounce_ms(&mut self, ms: u64);
    fn batch_delay_ms(&self) -> u64;
    fn set_batch_delay_ms(&mut self, ms: u64);
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, enabled: bool);
}

/// Interface for Lua hot reload operations
pub trait LuaHotReloadInterface {
    fn loaded_modules(&self) -> Vec<(String, PathBuf, ModuleStatus, Option<Instant>)>;
    fn reload_module(&mut self, name: &str) -> Result<(), String>;
    fn unload_module(&mut self, name: &str) -> Result<(), String>;
    fn reload_all_modules(&mut self) -> Vec<(String, Result<(), String>)>;
    fn get_module_source(&self, name: &str) -> Option<String>;
    fn is_enabled(&self) -> bool;
    fn set_enabled(&mut self, enabled: bool);
    fn get_module_error(&self, name: &str) -> Option<(String, Option<usize>)>;
}

/// Combined interface for the panel
pub trait HotReloadManager: AssetHotReloadInterface + LuaHotReloadInterface {
    fn settings(&self) -> HotReloadSettings;
    fn update_settings(&mut self, settings: &HotReloadSettings);
}

impl Default for AssetReloadState {
    fn default() -> Self {
        Self {
            watched_paths: Vec::new(),
            pending_changes: Vec::new(),
            debounce_ms: 300,
            batch_delay_ms: 100,
            enabled: true,
        }
    }
}

impl Default for LuaReloadState {
    fn default() -> Self {
        Self {
            modules: HashMap::new(),
            enabled: true,
            module_filter: String::new(),
        }
    }
}

impl Default for HotReloadSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            pause_on_error: true,
            show_notifications: true,
            max_log_entries: 500,
            auto_clear_log: false,
        }
    }
}

// ============================================================================
// HotReloadPanel Implementation
// ============================================================================

impl HotReloadPanel {
    /// Create a new enhanced hot reload panel
    pub fn new() -> Self {
        Self {
            visible: false,
            selected_tab: HotReloadTab::Assets,
            asset_state: AssetReloadState::default(),
            lua_state: LuaReloadState::default(),
            change_log: Vec::with_capacity(500),
            settings: HotReloadSettings::default(),
            status_message: None,
            last_update: Instant::now(),
            new_path_input: String::new(),
            selected_asset_path: None,
            selected_module: None,
        }
    }

    pub fn show(&mut self) { self.visible = true; }
    pub fn hide(&mut self) { self.visible = false; }
    
    pub fn toggle(&mut self) { self.visible = !self.visible; }
    pub fn is_visible(&self) -> bool { self.visible }

    /// Update panel state from backend
    pub fn update(&mut self, _dt: f32, manager: &mut dyn HotReloadManager) {
        if let Some((_, _, timestamp)) = &self.status_message {
            if timestamp.elapsed().as_secs() > 5 {
                self.status_message = None;
            }
        }

        let now = Instant::now();
        if now.duration_since(self.last_update) >= Duration::from_millis(100) {
            self.poll_asset_changes(manager);
            self.poll_lua_modules(manager);
            self.last_update = now;
        }

        self.settings = manager.settings();
        self.asset_state.enabled = AssetHotReloadInterface::is_enabled(manager);
        self.lua_state.enabled = LuaHotReloadInterface::is_enabled(manager);
    }

    fn poll_asset_changes(&mut self, manager: &dyn AssetHotReloadInterface) {
        let paths = manager.watched_paths();
        self.asset_state.watched_paths = paths
            .into_iter()
            .map(|(path, asset_type, active)| WatchedPath {
                path,
                asset_type,
                active,
                added_at: Instant::now(),
            })
            .collect();

        let pending = manager.pending_changes();
        for (path, asset_type, change_type) in pending {
            if !self.asset_state.pending_changes.iter().any(|c| c.path == path) {
                self.asset_state.pending_changes.push(PendingAssetChange {
                    path: path.clone(),
                    asset_type,
                    change_type,
                    timestamp: Instant::now(),
                });
                self.add_to_log(&path, change_type, SourceType::Asset);
            }
        }

        self.asset_state.debounce_ms = manager.debounce_ms();
        self.asset_state.batch_delay_ms = manager.batch_delay_ms();
    }

    fn poll_lua_modules(&mut self, manager: &dyn LuaHotReloadInterface) {
        let modules = manager.loaded_modules();
        for (name, path, status, last_reload) in modules {
            let entry = self.lua_state.modules.entry(name.clone()).or_insert_with(|| LuaModuleInfo {
                path: path.clone(),
                status: ModuleStatus::Loaded,
                last_reload: None,
                last_error: None,
                content_hash: None,
            });

            if entry.status != status {
                entry.status = status;
                if status == ModuleStatus::Modified {
                    self.add_to_log(&path, ChangeType::Modified, SourceType::LuaModule);
                }
            }

            entry.last_reload = last_reload;
            entry.path = path;

            if let Some((msg, line)) = manager.get_module_error(&name) {
                entry.last_error = Some(LuaError {
                    message: msg,
                    line,
                    timestamp: Instant::now(),
                });
                entry.status = ModuleStatus::Error;
            }
        }
    }

    fn add_to_log(&mut self, path: &Path, change_type: ChangeType, source_type: SourceType) {
        let file_name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        self.change_log.push(ChangeLogEntry {
            file_name,
            path: path.to_path_buf(),
            change_type,
            timestamp: Instant::now(),
            reload_status: ReloadStatus::Pending,
            source_type,
        });

        while self.change_log.len() > self.settings.max_log_entries {
            self.change_log.remove(0);
        }
    }

    fn show_status(&mut self, message: &str, status_type: StatusType) {
        self.status_message = Some((message.to_string(), status_type, Instant::now()));
    }

    /// Draw the hot reload panel
    pub fn draw(&mut self, ctx: &egui::Context, manager: &mut dyn HotReloadManager) {
        if !self.visible { return; }

        let mut visible = self.visible;
        egui::Window::new("🔄 Hot Reload")
            .open(&mut visible)
            .resizable(true)
            .default_size([700.0, 600.0])
            .min_size([500.0, 400.0])
            .show(ctx, |ui| {
                self.draw_panel_content(ui, manager);
            });
        self.visible = visible;
    }

    fn draw_panel_content(&mut self, ui: &mut egui::Ui, manager: &mut dyn HotReloadManager) {
        self.draw_header(ui, manager);
        ui.separator();
        self.draw_tabs(ui);
        ui.separator();

        match self.selected_tab {
            HotReloadTab::Assets => self.draw_assets_tab(ui, manager),
            HotReloadTab::LuaModules => self.draw_lua_modules_tab(ui, manager),
            HotReloadTab::ChangeLog => self.draw_change_log_tab(ui),
            HotReloadTab::Settings => self.draw_settings_tab(ui, manager),
        }

        if let Some((msg, status_type, _)) = &self.status_message {
            ui.separator();
            ui.colored_label(status_type.color(), msg.as_str());
        }
    }

    fn draw_header(&self, ui: &mut egui::Ui, manager: &dyn HotReloadManager) {
        ui.horizontal(|ui| {
            ui.heading("Hot Reload Manager");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let enabled = AssetHotReloadInterface::is_enabled(manager);
                let (icon, color, text) = if enabled {
                    ("?", egui::Color32::GREEN, "Active")
                } else {
                    ("?", egui::Color32::RED, "Disabled")
                };
                ui.colored_label(color, format!("{} {}", icon, text));
            });
        });

        ui.horizontal(|ui| {
            let pending = self.asset_state.pending_changes.len();
            let modules = self.lua_state.modules.len();
            let modified = self.lua_state.modules.values()
                .filter(|m| m.status == ModuleStatus::Modified || m.status == ModuleStatus::Error)
                .count();

            ui.label(format!("? {} pending", pending));
            ui.label("�");
            ui.label(format!("?? {} modules", modules));
            if modified > 0 {
                ui.label("�");
                ui.colored_label(egui::Color32::YELLOW, format!("?? {} modified", modified));
            }
        });
    }

    fn draw_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let pending = self.asset_state.pending_changes.len();
            let modified = self.lua_state.modules.values()
                .filter(|m| m.status == ModuleStatus::Modified || m.status == ModuleStatus::Error)
                .count();

            self.tab_button(ui, &format!("🖼️ Assets ({})", self.asset_state.watched_paths.len()), HotReloadTab::Assets, pending > 0);
            self.tab_button(ui, &format!("📜 Lua Modules ({})", self.lua_state.modules.len()), HotReloadTab::LuaModules, modified > 0);
            self.tab_button(ui, &format!("📋 Change Log ({})", self.change_log.len()), HotReloadTab::ChangeLog, false);
            self.tab_button(ui, "⚙️ Settings", HotReloadTab::Settings, false);
        });
    }

    fn tab_button(&mut self, ui: &mut egui::Ui, label: &str, tab: HotReloadTab, alert: bool) {
        let selected = self.selected_tab == tab;
        let response = ui.selectable_label(selected, label);
        if alert && !selected {
            let rect = response.rect;
            let alert_pos = egui::pos2(rect.right() - 8.0, rect.top() + 4.0);
            ui.painter().circle_filled(alert_pos, 4.0, egui::Color32::YELLOW);
        }
        if response.clicked() {
            self.selected_tab = tab;
        }
    }

    // ========================================================================
    // Assets Tab
    // ========================================================================

    fn draw_assets_tab(&mut self, ui: &mut egui::Ui, manager: &mut dyn HotReloadManager) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.set_width(ui.available_width());

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Asset Hot Reload").heading().strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut enabled = self.asset_state.enabled;
                        if ui.checkbox(&mut enabled, "Enabled").changed() {
                            AssetHotReloadInterface::set_enabled(manager, enabled);
                            self.asset_state.enabled = enabled;
                        }
                    });
                });
                ui.separator();

                // Watched paths list
                ui.label("Watched Paths:");
                ui.indent("watched_paths", |ui| {
                    if self.asset_state.watched_paths.is_empty() {
                        ui.label(egui::RichText::new("No paths being watched").weak());
                    } else {
                        for i in 0..self.asset_state.watched_paths.len() {
                            self.draw_watched_path_row(ui, manager, i);
                        }
                    }
                });

                ui.add_space(8.0);

                // Add new path
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.new_path_input);
                    ui.menu_button("➕ Add Path", |ui| {
                        for asset_type in [AssetType::Texture, AssetType::Shader, AssetType::Mesh, AssetType::Audio, AssetType::LuaScript] {
                            if ui.button(format!("{} {}", asset_type.icon(), asset_type.name())).clicked() {
                                if !self.new_path_input.is_empty() {
                                    let path = PathBuf::from(&self.new_path_input);
                                    if let Err(e) = manager.add_watch_path(&path, asset_type) {
                                        self.show_status(&format!("Failed: {}", e), StatusType::Error);
                                    } else {
                                        self.show_status(&format!("Added: {}", path.display()), StatusType::Success);
                                        self.new_path_input.clear();
                                    }
                                }
                                ui.close_menu();
                            }
                        }
                    });
                });

                ui.add_space(12.0);

                // Timing controls
                ui.label("Timing Settings:");
                ui.indent("timing", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Debounce:");
                        ui.add(egui::Slider::new(&mut self.asset_state.debounce_ms, 0..=2000).suffix(" ms"));
                        if ui.button("Apply").clicked() {
                            manager.set_debounce_ms(self.asset_state.debounce_ms);
                            self.show_status("Debounce updated", StatusType::Success);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Batch Delay:");
                        ui.add(egui::Slider::new(&mut self.asset_state.batch_delay_ms, 0..=1000).suffix(" ms"));
                        if ui.button("Apply").clicked() {
                            manager.set_batch_delay_ms(self.asset_state.batch_delay_ms);
                            self.show_status("Batch delay updated", StatusType::Success);
                        }
                    });
                });

                ui.add_space(12.0);

                // Pending changes section
                if !self.asset_state.pending_changes.is_empty() {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(format!("Pending Changes ({}):", self.asset_state.pending_changes.len()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🔄 Reload All Now").clicked() {
                                let results = manager.process_all_changes();
                                let success = results.iter().filter(|(_, s)| *s == ReloadStatus::Success).count();
                                self.show_status(
                                    &format!("Reloaded {}/{}", success, results.len()),
                                    if success == results.len() { StatusType::Success } else { StatusType::Warning }
                                );
                                self.asset_state.pending_changes.clear();
                            }
                        });
                    });

                    egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                        for change in &self.asset_state.pending_changes {
                            self.draw_pending_change_row(ui, change);
                        }
                    });
                }
            });
        });
    }

    fn draw_watched_path_row(&mut self, ui: &mut egui::Ui, manager: &mut dyn HotReloadManager, index: usize) {
        if let Some(watched) = self.asset_state.watched_paths.get_mut(index) {
            ui.horizontal(|ui| {
                let mut active = watched.active;
                if ui.checkbox(&mut active, "").changed() {
                    if let Err(e) = manager.toggle_watch_path(&watched.path, active) {
                        self.show_status(&e, StatusType::Error);
                    } else {
                        watched.active = active;
                    }
                }

                ui.label(watched.asset_type.icon());

                let path_text = if watched.path.as_os_str().len() > 40 {
                    let s = watched.path.to_string_lossy();
                    format!("...{}", s.chars().rev().take(37).collect::<String>().chars().rev().collect::<String>())
                } else {
                    watched.path.to_string_lossy().to_string()
                };
                ui.label(path_text);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🗑️").on_hover_text("Remove watch path").clicked() {
                        if let Err(e) = manager.remove_watch_path(&watched.path) {
                            self.show_status(&e, StatusType::Error);
                        } else {
                            self.show_status("Watch path removed", StatusType::Success);
                        }
                    }

                    let status_text = if watched.active { "Active" } else { "Paused" };
                    let status_color = if watched.active { egui::Color32::GREEN } else { egui::Color32::GRAY };
                    ui.colored_label(status_color, status_text);
                    ui.label(format!("[{}]", watched.asset_type.name()));
                });
            });
        }
    }

    fn draw_pending_change_row(&self, ui: &mut egui::Ui, change: &PendingAssetChange) {
        ui.horizontal(|ui| {
            ui.colored_label(change.change_type.color(), change.change_type.icon());
            ui.label(change.asset_type.icon());
            ui.label(change.path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default().to_string());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let elapsed = change.timestamp.elapsed().as_secs();
                let time_str = if elapsed < 60 {
                    format!("{}s ago", elapsed)
                } else {
                    format!("{}m ago", elapsed / 60)
                };
                ui.label(egui::RichText::new(time_str).weak().size(11.0));
            });
        });
    }

    // ========================================================================
    // Lua Modules Tab
    // ========================================================================

    fn draw_lua_modules_tab(&mut self, ui: &mut egui::Ui, manager: &mut dyn HotReloadManager) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.set_width(ui.available_width());

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Lua Module Hot Reload").heading().strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut enabled = self.lua_state.enabled;
                        if ui.checkbox(&mut enabled, "Enabled").changed() {
                            LuaHotReloadInterface::set_enabled(manager, enabled);
                            self.lua_state.enabled = enabled;
                        }
                    });
                });
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("🔍");
                    ui.text_edit_singleline(&mut self.lua_state.module_filter);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let modified_count = self.lua_state.modules.values()
                            .filter(|m| m.status == ModuleStatus::Modified || m.status == ModuleStatus::Error)
                            .count();

                        if modified_count > 0 && ui.button("🔄 Reload All Modified").clicked() {
                            let results = manager.reload_all_modules();
                            let success = results.iter().filter(|(_, r)| r.is_ok()).count();
                            self.show_status(
                                &format!("Reloaded {}/{}", success, results.len()),
                                if success == results.len() { StatusType::Success } else { StatusType::Warning }
                            );
                        }
                    });
                });

                ui.add_space(8.0);

                if self.lua_state.modules.is_empty() {
                    ui.label(egui::RichText::new("No Lua modules loaded").weak());
                } else {
                    let filter = self.lua_state.module_filter.to_lowercase();
                    let modules: Vec<_> = self.lua_state.modules.iter()
                        .filter(|(name, _)| filter.is_empty() || name.to_lowercase().contains(&filter))
                        .map(|(name, info)| (name.clone(), info.clone()))
                        .collect();

                    for (name, info) in modules {
                        self.draw_lua_module_card(ui, manager, &name, &info);
                    }
                }
            });
        });
    }

    fn draw_lua_module_card(&mut self, ui: &mut egui::Ui, manager: &mut dyn HotReloadManager, name: &str, info: &LuaModuleInfo) {
        let is_selected = self.selected_module.as_ref().map(|s| s.as_str()) == Some(name);
        let has_error = info.last_error.is_some();

        egui::Frame::group(ui.style())
            .fill(if is_selected { ui.visuals().widgets.active.bg_fill } else { ui.visuals().panel_fill })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                ui.horizontal(|ui| {
                    let status_icon = match info.status {
                        ModuleStatus::Loaded => "?",
                        ModuleStatus::Modified => "~",
                        ModuleStatus::Reloading => "?",
                        ModuleStatus::Error => "?",
                        ModuleStatus::Unloaded => "?",
                    };
                    ui.colored_label(info.status.color(), status_icon);
                    ui.label(egui::RichText::new(name).strong());

                    let path_str = info.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    ui.label(egui::RichText::new(path_str).weak().size(11.0));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("???").on_hover_text("View Source").clicked() {
                            self.selected_module = Some(name.to_string());
                            if let Some(_source) = manager.get_module_source(name) {
                                self.show_status("Source loaded", StatusType::Info);
                            }
                        }

                        if ui.button("???").on_hover_text("Unload").clicked() {
                            if let Err(e) = manager.unload_module(name) {
                                self.show_status(&e, StatusType::Error);
                            } else {
                                self.show_status(&format!("Unloaded: {}", name), StatusType::Success);
                            }
                        }

                        if info.status == ModuleStatus::Modified || info.status == ModuleStatus::Error {
                            if ui.button("🔄").on_hover_text("Reload").clicked() {
                                if let Err(e) = manager.reload_module(name) {
                                    self.show_status(&e, StatusType::Error);
                                } else {
                                    self.show_status(&format!("Reloaded: {}", name), StatusType::Success);
                                }
                            }
                        }

                        if let Some(last_reload) = info.last_reload {
                            let elapsed = last_reload.elapsed().as_secs();
                            let time_str = if elapsed < 60 {
                                format!("{}s ago", elapsed)
                            } else if elapsed < 3600 {
                                format!("{}m ago", elapsed / 60)
                            } else {
                                format!("{}h ago", elapsed / 3600)
                            };
                            ui.label(egui::RichText::new(time_str).weak().size(10.0));
                        }
                    });
                });

                if has_error {
                    if let Some(error) = &info.last_error {
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::RED, "Error:");
                            if let Some(line) = error.line {
                                ui.colored_label(egui::Color32::YELLOW, format!("Line {}", line));
                            }
                        });
                        ui.colored_label(egui::Color32::RED, egui::RichText::new(&error.message).size(11.0));
                    }
                }
            });

        ui.add_space(4.0);
    }

    // ========================================================================
    // Change Log Tab
    // ========================================================================

    fn draw_change_log_tab(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Recent Changes").heading().strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("?? Clear Log").clicked() {
                        self.change_log.clear();
                        self.show_status("Change log cleared", StatusType::Success);
                    }
                });
            });
            ui.separator();

            if self.change_log.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label(egui::RichText::new("No changes recorded").weak());
                    ui.label("File changes will appear here when detected.");
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label("Filter:");
                    if ui.button("All").clicked() {}
                    if ui.button("Assets").clicked() {}
                    if ui.button("Lua").clicked() {}
                    if ui.button("Errors").clicked() {}
                });

                ui.separator();

                egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                    for entry in self.change_log.iter().rev() {
                        self.draw_log_entry(ui, entry);
                    }
                });
            }
        });
    }

    fn draw_log_entry(&self, ui: &mut egui::Ui, entry: &ChangeLogEntry) {
        ui.horizontal(|ui| {
            ui.colored_label(entry.change_type.color(), entry.change_type.icon());
            let source_icon = match entry.source_type {
                SourceType::Asset => "???",
                SourceType::LuaModule => "??",
            };
            ui.label(source_icon);
            ui.label(egui::RichText::new(&entry.file_name).strong());
            ui.colored_label(entry.reload_status.color(), entry.reload_status.name());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let elapsed = entry.timestamp.elapsed().as_secs();
                let time_str = if elapsed < 60 {
                    format!("{}s ago", elapsed)
                } else if elapsed < 3600 {
                    format!("{}m ago", elapsed / 60)
                } else {
                    format!("{}h ago", elapsed / 3600)
                };
                ui.label(egui::RichText::new(time_str).weak().size(11.0));
            });
        });
    }

    // ========================================================================
    // Settings Tab
    // ========================================================================

    fn draw_settings_tab(&mut self, ui: &mut egui::Ui, manager: &mut dyn HotReloadManager) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new("Global Settings").heading().strong());
                ui.separator();

                let mut enabled = self.settings.enabled;
                if ui.checkbox(&mut enabled, "Enable Hot Reload Globally").changed() {
                    self.settings.enabled = enabled;
                    AssetHotReloadInterface::set_enabled(manager, enabled);
                    LuaHotReloadInterface::set_enabled(manager, enabled);
                    self.show_status(if enabled { "Hot reload enabled" } else { "Hot reload disabled" }, StatusType::Success);
                }
                ui.label("When disabled, no files will be watched or reloaded.");

                ui.add_space(12.0);

                ui.checkbox(&mut self.settings.pause_on_error, "Pause on Error");
                ui.label("Automatically pause hot reloading when an error occurs.");

                ui.add_space(12.0);

                ui.checkbox(&mut self.settings.show_notifications, "Show Notifications");
                ui.label("Display desktop notifications for reload events.");

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    ui.label("Max Log Entries:");
                    ui.add(egui::DragValue::new(&mut self.settings.max_log_entries).speed(10.0).range(10..=10000));
                });

                ui.add_space(12.0);

                ui.checkbox(&mut self.settings.auto_clear_log, "Auto-clear Log on Success");
                ui.label("Clear the change log when all pending changes reload successfully.");

                ui.add_space(16.0);

                if ui.button("?? Apply Settings").clicked() {
                    manager.update_settings(&self.settings);
                    self.show_status("Settings applied", StatusType::Success);
                }
            });

            ui.add_space(16.0);

            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new("Statistics").heading().strong());
                ui.separator();

                egui::Grid::new("stats_grid").num_columns(2).spacing([20.0, 8.0]).show(ui, |ui| {
                    ui.label("Watched Paths:");
                    ui.label(self.asset_state.watched_paths.len().to_string());
                    ui.end_row();

                    ui.label("Loaded Modules:");
                    ui.label(self.lua_state.modules.len().to_string());
                    ui.end_row();

                    ui.label("Pending Changes:");
                    ui.label(self.asset_state.pending_changes.len().to_string());
                    ui.end_row();

                    ui.label("Log Entries:");
                    ui.label(self.change_log.len().to_string());
                    ui.end_row();

                    ui.label("Modified Modules:");
                    ui.label(self.lua_state.modules.values().filter(|m| m.status == ModuleStatus::Modified).count().to_string());
                    ui.end_row();

                    ui.label("Error State Modules:");
                    ui.label(self.lua_state.modules.values().filter(|m| m.status == ModuleStatus::Error).count().to_string());
                    ui.end_row();
                });
            });
        });
    }
}

impl Default for HotReloadPanel {
    fn default() -> Self { Self::new() }
}

// ============================================================================
// Mock Implementations for Testing
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    struct MockHotReloadManager {
        watched_paths: Vec<(PathBuf, AssetType, bool)>,
        modules: HashMap<String, (PathBuf, ModuleStatus)>,
        settings: HotReloadSettings,
        enabled: bool,
        debounce_ms: u64,
        batch_delay_ms: u64,
    }

    impl MockHotReloadManager {
        fn new() -> Self {
            Self {
                watched_paths: vec![
                    (PathBuf::from("assets/textures"), AssetType::Texture, true),
                    (PathBuf::from("assets/audio"), AssetType::Audio, true),
                    (PathBuf::from("assets/shaders"), AssetType::Shader, false),
                ],
                modules: [
                    ("player".to_string(), (PathBuf::from("scripts/player.lua"), ModuleStatus::Loaded)),
                    ("enemy".to_string(), (PathBuf::from("scripts/enemy.lua"), ModuleStatus::Modified)),
                ]
                .into_iter()
                .collect(),
                settings: HotReloadSettings::default(),
                enabled: true,
                debounce_ms: 300,
                batch_delay_ms: 100,
            }
        }
    }

    impl AssetHotReloadInterface for MockHotReloadManager {
        fn watched_paths(&self) -> Vec<(PathBuf, AssetType, bool)> {
            self.watched_paths.clone()
        }

        fn add_watch_path(&mut self, path: &Path, asset_type: AssetType) -> Result<(), String> {
            self.watched_paths.push((path.to_path_buf(), asset_type, true));
            Ok(())
        }

        fn remove_watch_path(&mut self, path: &Path) -> Result<(), String> {
            self.watched_paths.retain(|(p, _, _)| p != path);
            Ok(())
        }

        fn toggle_watch_path(&mut self, path: &Path, active: bool) -> Result<(), String> {
            if let Some(entry) = self.watched_paths.iter_mut().find(|(p, _, _)| p == path) {
                entry.2 = active;
            }
            Ok(())
        }

        fn pending_changes(&self) -> Vec<(PathBuf, AssetType, ChangeType)> {
            vec![
                (PathBuf::from("assets/player.png"), AssetType::Texture, ChangeType::Modified),
            ]
        }

        fn process_all_changes(&mut self) -> Vec<(PathBuf, ReloadStatus)> {
            vec![(PathBuf::from("assets/player.png"), ReloadStatus::Success)]
        }

        fn force_reload_asset(&mut self, _path: &Path) -> ReloadStatus {
            ReloadStatus::Success
        }

        fn debounce_ms(&self) -> u64 { self.debounce_ms }
        fn set_debounce_ms(&mut self, ms: u64) { self.debounce_ms = ms; }
        fn batch_delay_ms(&self) -> u64 { self.batch_delay_ms }
        fn set_batch_delay_ms(&mut self, ms: u64) { self.batch_delay_ms = ms; }
        fn is_enabled(&self) -> bool { self.enabled }
        fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }
    }

    impl LuaHotReloadInterface for MockHotReloadManager {
        fn loaded_modules(&self) -> Vec<(String, PathBuf, ModuleStatus, Option<Instant>)> {
            self.modules.iter()
                .map(|(name, (path, status))| (name.clone(), path.clone(), *status, None))
                .collect()
        }

        fn reload_module(&mut self, name: &str) -> Result<(), String> {
            if let Some((_, status)) = self.modules.get_mut(name) {
                *status = ModuleStatus::Loaded;
            }
            Ok(())
        }

        fn unload_module(&mut self, name: &str) -> Result<(), String> {
            self.modules.remove(name);
            Ok(())
        }

        fn reload_all_modules(&mut self) -> Vec<(String, Result<(), String>)> {
            self.modules.values_mut().for_each(|(_, status)| *status = ModuleStatus::Loaded);
            self.modules.keys().map(|k| (k.clone(), Ok(()))).collect()
        }

        fn get_module_source(&self, _name: &str) -> Option<String> {
            Some("-- Lua source code".to_string())
        }

        fn is_enabled(&self) -> bool { self.enabled }
        fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }

        fn get_module_error(&self, _name: &str) -> Option<(String, Option<usize>)> {
            None
        }
    }

    impl HotReloadManager for MockHotReloadManager {
        fn settings(&self) -> HotReloadSettings { self.settings.clone() }
        fn update_settings(&mut self, settings: &HotReloadSettings) { self.settings = settings.clone(); }
    }

    #[test]
    fn test_panel_creation() {
        let panel = HotReloadPanel::new();
        assert!(!panel.is_visible());
        assert!(panel.asset_state.watched_paths.is_empty());
        assert!(panel.lua_state.modules.is_empty());
    }

    #[test]
    fn test_panel_toggle() {
        let mut panel = HotReloadPanel::new();
        assert!(!panel.is_visible());
        panel.toggle();
        assert!(panel.is_visible());
        panel.hide();
        assert!(!panel.is_visible());
        panel.show();
        assert!(panel.is_visible());
    }

    #[test]
    fn test_module_status_display() {
        assert_eq!(ModuleStatus::Loaded.name(), "Loaded");
        assert_eq!(ModuleStatus::Error.name(), "Error");
    }

    #[test]
    fn test_change_type_display() {
        assert_eq!(ChangeType::Modified.name(), "Modified");
        assert_eq!(ChangeType::Created.icon(), "?");
    }

    #[test]
    fn test_reload_status_display() {
        assert_eq!(ReloadStatus::Success.name(), "Success");
        assert_eq!(ReloadStatus::Failed.name(), "Failed");
    }

    #[test]
    fn test_status_type_color() {
        assert_eq!(StatusType::Success.color(), egui::Color32::GREEN);
        assert_eq!(StatusType::Error.color(), egui::Color32::RED);
    }

    #[test]
    fn test_add_to_log() {
        let mut panel = HotReloadPanel::new();
        assert!(panel.change_log.is_empty());

        panel.add_to_log(Path::new("test.lua"), ChangeType::Modified, SourceType::LuaModule);
        assert_eq!(panel.change_log.len(), 1);
        assert_eq!(panel.change_log[0].file_name, "test.lua");

        panel.settings.max_log_entries = 5;
        for i in 0..10 {
            panel.add_to_log(Path::new(&format!("test{}.lua", i)), ChangeType::Modified, SourceType::LuaModule);
        }
        assert_eq!(panel.change_log.len(), 5);
    }

    #[test]
    fn test_mock_manager() {
        let mut manager = MockHotReloadManager::new();
        
        assert_eq!(manager.watched_paths().len(), 3);
        
        manager.add_watch_path(Path::new("new/path"), AssetType::Mesh).unwrap();
        assert_eq!(manager.watched_paths().len(), 4);
        
        manager.remove_watch_path(Path::new("new/path")).unwrap();
        assert_eq!(manager.watched_paths().len(), 3);
        
        manager.toggle_watch_path(Path::new("assets/textures"), false).unwrap();
        assert!(!manager.watched_paths()[0].2);
        
        assert_eq!(manager.loaded_modules().len(), 2);
        
        manager.reload_module("enemy").unwrap();
        let modules = manager.loaded_modules();
        let enemy = modules.iter().find(|(n, _, _, _)| n == "enemy").unwrap();
        assert_eq!(enemy.2, ModuleStatus::Loaded);
        
        manager.unload_module("enemy").unwrap();
        assert_eq!(manager.loaded_modules().len(), 1);
        
        let settings = manager.settings();
        assert!(settings.enabled);
        
        let mut new_settings = settings.clone();
        new_settings.enabled = false;
        manager.update_settings(&new_settings);
        assert!(!manager.settings().enabled);
    }
}
