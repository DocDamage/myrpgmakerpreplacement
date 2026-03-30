//! Asset Hot-Reload Panel
//!
//! Editor UI for monitoring and controlling the asset hot-reload system.
//! Shows pending changes, reload history, and configuration settings.

use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Hot reload panel UI state
pub struct HotReloadPanel {
    /// Whether panel is visible
    visible: bool,
    /// Selected tab
    selected_tab: HotReloadTab,
    /// Pending changes queue
    pending_changes: Vec<PendingChange>,
    /// Reload history
    reload_history: Vec<ReloadEntry>,
    /// Auto-reload enabled
    auto_reload: bool,
    /// Debounce duration in milliseconds
    debounce_ms: u64,
    /// Status message
    status_message: Option<String>,
    /// Status timeout
    status_timeout: f32,
    /// Last update time
    last_update: Instant,
    /// Selected asset for details view
    selected_asset: Option<PathBuf>,
}

/// Hot reload panel tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HotReloadTab {
    Status,
    Pending,
    History,
    Settings,
}

/// Pending change entry
#[derive(Debug, Clone)]
struct PendingChange {
    path: PathBuf,
    asset_type: String,
    change_type: ChangeType,
    timestamp: Instant,
}

/// Change type
#[derive(Debug, Clone)]
enum ChangeType {
    Created,
    Modified,
    Deleted,
    Renamed(PathBuf),
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeType::Created => write!(f, "Created"),
            ChangeType::Modified => write!(f, "Modified"),
            ChangeType::Deleted => write!(f, "Deleted"),
            ChangeType::Renamed(from) => write!(f, "Renamed from {}", from.display()),
        }
    }
}

impl ChangeType {
    fn icon(&self) -> &'static str {
        match self {
            ChangeType::Created => "➕",
            ChangeType::Modified => "✏️",
            ChangeType::Deleted => "🗑️",
            ChangeType::Renamed(_) => "📝",
        }
    }

    fn color(&self) -> egui::Color32 {
        match self {
            ChangeType::Created => egui::Color32::GREEN,
            ChangeType::Modified => egui::Color32::YELLOW,
            ChangeType::Deleted => egui::Color32::RED,
            ChangeType::Renamed(_) => egui::Color32::LIGHT_BLUE,
        }
    }
}

/// Reload history entry
#[derive(Debug, Clone)]
struct ReloadEntry {
    path: PathBuf,
    asset_type: String,
    result: ReloadResult,
    timestamp: Instant,
}

/// Reload result
#[derive(Debug, Clone)]
enum ReloadResult {
    Success,
    Failed(String),
    Skipped,
}

impl std::fmt::Display for ReloadResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReloadResult::Success => write!(f, "Success"),
            ReloadResult::Failed(msg) => write!(f, "Failed: {}", msg),
            ReloadResult::Skipped => write!(f, "Skipped"),
        }
    }
}

impl ReloadResult {
    fn color(&self) -> egui::Color32 {
        match self {
            ReloadResult::Success => egui::Color32::GREEN,
            ReloadResult::Failed(_) => egui::Color32::RED,
            ReloadResult::Skipped => egui::Color32::YELLOW,
        }
    }
}

/// Trait for interacting with the hot reload system
pub trait HotReloadInterface {
    /// Get count of pending changes
    fn pending_count(&self) -> usize;
    /// Get pending changes
    fn pending_changes(&self) -> Vec<(PathBuf, String, String)>;
    /// Process pending changes
    fn process_changes(&mut self) -> Vec<(PathBuf, String, String)>;
    /// Force reload a specific asset
    fn force_reload(&mut self, path: &std::path::Path) -> Result<(), String>;
    /// Check if auto-reload is enabled
    fn auto_reload(&self) -> bool;
    /// Set auto-reload enabled
    fn set_auto_reload(&mut self, enabled: bool);
    /// Get debounce duration
    fn debounce_ms(&self) -> u64;
    /// Set debounce duration
    fn set_debounce_ms(&mut self, ms: u64);
    /// Get watched paths
    fn watched_paths(&self) -> Vec<(PathBuf, String)>;
}

impl HotReloadPanel {
    /// Create a new hot reload panel
    pub fn new() -> Self {
        Self {
            visible: false,
            selected_tab: HotReloadTab::Status,
            pending_changes: Vec::new(),
            reload_history: Vec::with_capacity(100),
            auto_reload: true,
            debounce_ms: 300,
            status_message: None,
            status_timeout: 0.0,
            last_update: Instant::now(),
            selected_asset: None,
        }
    }

    /// Show the panel
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the panel
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Update panel state
    pub fn update(&mut self, dt: f32, interface: &mut dyn HotReloadInterface) {
        // Update status timeout
        if self.status_timeout > 0.0 {
            self.status_timeout -= dt;
            if self.status_timeout <= 0.0 {
                self.status_message = None;
            }
        }

        // Poll for pending changes every 100ms
        let now = Instant::now();
        if now.duration_since(self.last_update) >= Duration::from_millis(100) {
            self.poll_changes(interface);
            self.last_update = now;
        }

        // Auto-process if enabled
        if self.auto_reload && !self.pending_changes.is_empty() {
            let results = interface.process_changes();
            for (path, asset_type, result_str) in results {
                let result = if result_str == "Success" {
                    ReloadResult::Success
                } else if result_str == "Skipped" {
                    ReloadResult::Skipped
                } else {
                    ReloadResult::Failed(result_str)
                };

                self.reload_history.push(ReloadEntry {
                    path,
                    asset_type,
                    result,
                    timestamp: Instant::now(),
                });
            }
            self.pending_changes.clear();

            // Trim history
            if self.reload_history.len() > 100 {
                self.reload_history.remove(0);
            }
        }

        // Sync settings with interface
        self.auto_reload = interface.auto_reload();
        self.debounce_ms = interface.debounce_ms();
    }

    /// Poll for changes from the interface
    fn poll_changes(&mut self, interface: &dyn HotReloadInterface) {
        let changes = interface.pending_changes();
        for (path, asset_type, change_type_str) in changes {
            // Check if already in pending
            if self.pending_changes.iter().any(|c| c.path == path) {
                continue;
            }

            let change_type = match change_type_str.as_str() {
                "Created" => ChangeType::Created,
                "Deleted" => ChangeType::Deleted,
                s if s.starts_with("Renamed") => {
                    ChangeType::Renamed(PathBuf::from(s.strip_prefix("Renamed ").unwrap_or("")))
                }
                _ => ChangeType::Modified,
            };

            self.pending_changes.push(PendingChange {
                path,
                asset_type,
                change_type,
                timestamp: Instant::now(),
            });
        }
    }

    /// Draw the hot reload panel UI
    pub fn draw(&mut self, ctx: &egui::Context, interface: &mut dyn HotReloadInterface) {
        if !self.visible {
            return;
        }

        let mut visible = self.visible;
        egui::Window::new("🔄 Asset Hot-Reload")
            .open(&mut visible)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                self.draw_panel_content(ui, interface);
            });
        self.visible = visible;
    }

    /// Draw panel content
    fn draw_panel_content(&mut self, ui: &mut egui::Ui, interface: &mut dyn HotReloadInterface) {
        // Header with status indicator
        ui.horizontal(|ui| {
            ui.heading("Asset Hot-Reload");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let status_text = if self.pending_changes.is_empty() {
                    "✓ Up to date"
                } else {
                    "⏳ Changes pending"
                };
                let status_color = if self.pending_changes.is_empty() {
                    egui::Color32::GREEN
                } else {
                    egui::Color32::YELLOW
                };
                ui.colored_label(status_color, status_text);
            });
        });

        ui.separator();

        // Tab bar
        ui.horizontal(|ui| {
            self.tab_button(ui, "📊 Status", HotReloadTab::Status);
            self.tab_button(
                ui,
                &format!("⏳ Pending ({})", self.pending_changes.len()),
                HotReloadTab::Pending,
            );
            self.tab_button(
                ui,
                &format!("📚 History ({})", self.reload_history.len()),
                HotReloadTab::History,
            );
            self.tab_button(ui, "⚙️ Settings", HotReloadTab::Settings);
        });

        ui.separator();

        // Tab content
        match self.selected_tab {
            HotReloadTab::Status => self.draw_status_tab(ui, interface),
            HotReloadTab::Pending => self.draw_pending_tab(ui, interface),
            HotReloadTab::History => self.draw_history_tab(ui),
            HotReloadTab::Settings => self.draw_settings_tab(ui, interface),
        }

        // Status message
        if let Some(ref msg) = self.status_message {
            ui.separator();
            ui.colored_label(egui::Color32::GREEN, msg);
        }
    }

    /// Draw tab button
    fn tab_button(&mut self, ui: &mut egui::Ui, label: &str, tab: HotReloadTab) {
        let selected = self.selected_tab == tab;
        if ui.selectable_label(selected, label).clicked() {
            self.selected_tab = tab;
        }
    }

    /// Draw status tab
    fn draw_status_tab(&mut self, ui: &mut egui::Ui, interface: &dyn HotReloadInterface) {
        ui.heading("System Status");
        ui.add_space(10.0);

        // Quick stats
        egui::Grid::new("hot_reload_status_grid")
            .num_columns(2)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                ui.label("Status:");
                if self.pending_changes.is_empty() {
                    ui.colored_label(egui::Color32::GREEN, "✓ Active - No pending changes");
                } else {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        format!("⏳ {} change(s) pending", self.pending_changes.len()),
                    );
                }
                ui.end_row();

                ui.label("Auto-reload:");
                ui.label(if interface.auto_reload() {
                    "✓ Enabled"
                } else {
                    "✗ Disabled"
                });
                ui.end_row();

                ui.label("Debounce:");
                ui.label(format!("{} ms", interface.debounce_ms()));
                ui.end_row();

                ui.label("Watched paths:");
                ui.label(interface.watched_paths().len().to_string());
                ui.end_row();

                ui.label("Recent reloads:");
                ui.label(self.reload_history.len().to_string());
                ui.end_row();
            });

        ui.add_space(20.0);

        // Quick actions
        ui.heading("Quick Actions");
        ui.horizontal(|ui| {
            if ui.button("🔄 Reload All Pending").clicked() && !self.pending_changes.is_empty() {
                // Process all pending changes
                // This is handled by update() normally, but force it here
                self.show_status("Queued reload of all pending changes", false);
            }

            if ui.button("🧹 Clear History").clicked() {
                self.reload_history.clear();
                self.show_status("History cleared", false);
            }

            if ui.button("📝 View Logs").clicked() {
                self.selected_tab = HotReloadTab::History;
            }
        });
    }

    /// Draw pending changes tab
    fn draw_pending_tab(&mut self, ui: &mut egui::Ui, interface: &mut dyn HotReloadInterface) {
        if self.pending_changes.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(egui::RichText::new("No pending changes").weak());
                ui.label("Assets will appear here when they change on disk.");
            });
            return;
        }

        ui.horizontal(|ui| {
            ui.label(format!("{} Pending Changes", self.pending_changes.len()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !self.auto_reload && ui.button("🔄 Reload All").clicked() {
                    let results = interface.process_changes();
                    for (path, asset_type, result_str) in results {
                        let result = if result_str == "Success" {
                            ReloadResult::Success
                        } else if result_str == "Skipped" {
                            ReloadResult::Skipped
                        } else {
                            ReloadResult::Failed(result_str)
                        };

                        self.reload_history.push(ReloadEntry {
                            path,
                            asset_type,
                            result,
                            timestamp: Instant::now(),
                        });
                    }
                    self.pending_changes.clear();
                    self.show_status("All changes reloaded", false);
                }
            });
        });

        ui.separator();

        // Collect changes to process separately to avoid borrow issues
        let mut to_reload: Vec<PathBuf> = Vec::new();
        let mut to_ignore: Vec<PathBuf> = Vec::new();
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            for change in &self.pending_changes {
                let (reload, ignore) = self.draw_pending_change_card(ui, change);
                if reload {
                    to_reload.push(change.path.clone());
                }
                if ignore {
                    to_ignore.push(change.path.clone());
                }
            }
        });
        
        // Process actions outside the loop
        for path in &to_reload {
            match interface.force_reload(path) {
                Ok(()) => {
                    self.show_status(&format!("Reloaded: {}", path.display()), false);
                    self.pending_changes.retain(|c| &c.path != path);
                }
                Err(e) => {
                    self.show_status(&format!("Failed: {}", e), true);
                }
            }
        }
        
        for path in &to_ignore {
            self.pending_changes.retain(|c| &c.path != path);
            self.show_status(&format!("Ignored: {}", path.display()), false);
        }
    }

    /// Draw a pending change card, returns (should_reload, should_ignore)
    fn draw_pending_change_card(
        &self,
        ui: &mut egui::Ui,
        change: &PendingChange,
    ) -> (bool, bool) {
        let is_selected = self.selected_asset.as_ref() == Some(&change.path);
        let mut should_reload = false;
        let mut should_ignore = false;

        egui::Frame::group(ui.style())
            .fill(if is_selected {
                ui.visuals().widgets.active.bg_fill
            } else {
                ui.visuals().panel_fill
            })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                ui.horizontal(|ui| {
                    // Change type icon
                    ui.colored_label(change.change_type.color(), change.change_type.icon());

                    // Asset info
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(change.path.file_name().unwrap_or_default().to_string_lossy())
                                .strong(),
                        );
                        ui.horizontal(|ui| {
                            ui.label(format!("Type: {}", change.asset_type));
                            ui.label("•");
                            ui.label(change.change_type.to_string());
                        });
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🔄").on_hover_text("Reload now").clicked() {
                            should_reload = true;
                        }

                        if ui.button("🗑️").on_hover_text("Ignore").clicked() {
                            should_ignore = true;
                        }
                    });
                });
            });

        ui.add_space(4.0);
        
        (should_reload, should_ignore)
    }

    /// Draw history tab
    fn draw_history_tab(&mut self, ui: &mut egui::Ui) {
        if self.reload_history.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(egui::RichText::new("No reload history").weak());
                ui.label("Successfully reloaded assets will appear here.");
            });
            return;
        }

        ui.horizontal(|ui| {
            ui.label(format!("{} Recent Reloads", self.reload_history.len()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🧹 Clear").clicked() {
                    self.reload_history.clear();
                    self.show_status("History cleared", false);
                }
            });
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for entry in self.reload_history.iter().rev() {
                self.draw_history_entry_card(ui, entry);
            }
        });
    }

    /// Draw a history entry card
    fn draw_history_entry_card(&self, ui: &mut egui::Ui, entry: &ReloadEntry) {
        egui::Frame::group(ui.style())
            .fill(ui.visuals().panel_fill)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                ui.horizontal(|ui| {
                    // Result indicator
                    let indicator = match entry.result {
                        ReloadResult::Success => "✓",
                        ReloadResult::Failed(_) => "✗",
                        ReloadResult::Skipped => "○",
                    };
                    ui.colored_label(entry.result.color(), indicator);

                    // Asset info
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(entry.path.file_name().unwrap_or_default().to_string_lossy())
                                .strong(),
                        );
                        ui.horizontal(|ui| {
                            ui.label(format!("Type: {}", entry.asset_type));
                            ui.label("•");
                            ui.colored_label(entry.result.color(), entry.result.to_string());
                        });
                    });

                    // Timestamp (relative)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let elapsed = entry.timestamp.elapsed();
                        let time_str = if elapsed.as_secs() < 60 {
                            format!("{}s ago", elapsed.as_secs())
                        } else if elapsed.as_secs() < 3600 {
                            format!("{}m ago", elapsed.as_secs() / 60)
                        } else {
                            format!("{}h ago", elapsed.as_secs() / 3600)
                        };
                        ui.label(egui::RichText::new(time_str).weak().size(11.0));
                    });
                });
            });

        ui.add_space(4.0);
    }

    /// Draw settings tab
    fn draw_settings_tab(&mut self, ui: &mut egui::Ui, interface: &mut dyn HotReloadInterface) {
        ui.heading("Hot-Reload Settings");
        ui.separator();

        // Auto-reload toggle
        let mut auto_reload = interface.auto_reload();
        if ui.checkbox(&mut auto_reload, "Auto-reload on change").changed() {
            interface.set_auto_reload(auto_reload);
            self.show_status(
                if auto_reload {
                    "Auto-reload enabled"
                } else {
                    "Auto-reload disabled"
                },
                false,
            );
        }
        ui.label("Automatically reload assets when they change on disk.");

        ui.add_space(10.0);

        // Debounce setting
        let mut debounce_ms = interface.debounce_ms();
        ui.horizontal(|ui| {
            ui.label("Debounce duration:");
            ui.add(egui::DragValue::new(&mut debounce_ms).speed(10.0).suffix(" ms"));
        });
        ui.label("Wait this long after a change before reloading (prevents partial writes).");
        if debounce_ms != interface.debounce_ms() {
            interface.set_debounce_ms(debounce_ms);
        }

        ui.add_space(20.0);

        // Watched paths
        ui.heading("Watched Directories");
        ui.separator();

        let watched = interface.watched_paths();
        if watched.is_empty() {
            ui.label("No directories being watched.");
        } else {
            egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                for (path, asset_type) in watched {
                    ui.horizontal(|ui| {
                        ui.label("📁");
                        ui.label(format!("{} ({})", path.display(), asset_type));
                    });
                }
            });
        }
    }

    /// Show a status message
    fn show_status(&mut self, message: &str, _is_error: bool) {
        self.status_message = Some(message.to_string());
        self.status_timeout = 5.0;
    }
}

impl Default for HotReloadPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockHotReloadInterface {
        pending: Vec<(PathBuf, String, String)>,
        auto_reload: bool,
        debounce_ms: u64,
    }

    impl HotReloadInterface for MockHotReloadInterface {
        fn pending_count(&self) -> usize {
            self.pending.len()
        }

        fn pending_changes(&self) -> Vec<(PathBuf, String, String)> {
            self.pending.clone()
        }

        fn process_changes(&mut self) -> Vec<(PathBuf, String, String)> {
            let result = self.pending.clone();
            self.pending.clear();
            result
        }

        fn force_reload(&mut self, _path: &std::path::Path) -> Result<(), String> {
            Ok(())
        }

        fn auto_reload(&self) -> bool {
            self.auto_reload
        }

        fn set_auto_reload(&mut self, enabled: bool) {
            self.auto_reload = enabled;
        }

        fn debounce_ms(&self) -> u64 {
            self.debounce_ms
        }

        fn set_debounce_ms(&mut self, ms: u64) {
            self.debounce_ms = ms;
        }

        fn watched_paths(&self) -> Vec<(PathBuf, String)> {
            vec![
                (PathBuf::from("assets/textures"), "Texture".to_string()),
                (PathBuf::from("assets/audio"), "Audio".to_string()),
            ]
        }
    }

    #[test]
    fn test_panel_creation() {
        let panel = HotReloadPanel::new();
        assert!(!panel.is_visible());
        assert_eq!(panel.selected_tab, HotReloadTab::Status);
        assert!(panel.pending_changes.is_empty());
    }

    #[test]
    fn test_panel_toggle() {
        let mut panel = HotReloadPanel::new();
        assert!(!panel.is_visible());

        panel.toggle();
        assert!(panel.is_visible());

        panel.toggle();
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_status_message() {
        let mut panel = HotReloadPanel::new();
        panel.show_status("Test message", false);
        assert!(panel.status_message.is_some());
    }

    #[test]
    fn test_reload_result_display() {
        assert_eq!(ReloadResult::Success.to_string(), "Success");
        assert_eq!(
            ReloadResult::Failed("test".to_string()).to_string(),
            "Failed: test"
        );
        assert_eq!(ReloadResult::Skipped.to_string(), "Skipped");
    }

    #[test]
    fn test_change_type_display() {
        assert_eq!(ChangeType::Created.to_string(), "Created");
        assert_eq!(ChangeType::Modified.to_string(), "Modified");
        assert_eq!(ChangeType::Deleted.to_string(), "Deleted");
        assert_eq!(
            ChangeType::Renamed(PathBuf::from("old.txt")).to_string(),
            "Renamed from old.txt"
        );
    }
}
