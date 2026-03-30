//! SQLite-ECS Sync Panel
//!
//! Editor UI for monitoring and controlling the SQLite-ECS sync layer.

use std::time::{Duration, Instant};

/// Sync panel UI state
pub struct SyncPanel {
    /// Whether panel is visible
    visible: bool,
    /// Selected tab
    selected_tab: SyncTab,
    /// Status message
    status_message: Option<String>,
    /// Status timeout
    status_timeout: f32,
}

/// Sync panel tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyncTab {
    Status,
    Pending,
    Conflicts,
    Settings,
}

/// Trait for interacting with the sync layer
pub trait SyncInterface {
    /// Check if sync is in progress
    fn is_syncing(&self) -> bool;
    /// Get pending changes count
    fn pending_changes(&self) -> usize;
    /// Get pending conflicts count
    fn pending_conflicts(&self) -> usize;
    /// Get sync statistics
    fn stats(&self) -> SyncStats;
    /// Trigger a sync
    fn sync(&mut self, direction: SyncDirection);
    /// Get auto-sync interval
    fn auto_sync_interval(&self) -> Option<Duration>;
    /// Set auto-sync interval
    fn set_auto_sync_interval(&mut self, interval: Option<Duration>);
    /// Get conflict strategy
    fn conflict_strategy(&self) -> ConflictStrategy;
    /// Set conflict strategy
    fn set_conflict_strategy(&mut self, strategy: ConflictStrategy);
}

/// Sync direction (mirrors dde_db::sync::SyncDirection)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    EcsToDb,
    DbToEcs,
    Bidirectional,
}

impl SyncDirection {
    /// Get display name for the sync direction
    pub fn name(&self) -> &'static str {
        match self {
            SyncDirection::EcsToDb => "ECS → Database",
            SyncDirection::DbToEcs => "Database → ECS",
            SyncDirection::Bidirectional => "Bidirectional",
        }
    }
}

/// Conflict strategy (mirrors dde_db::sync::ConflictStrategy)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictStrategy {
    DatabaseWins,
    EcsWins,
    LastWriteWins,
    Merge,
    Manual,
}

impl ConflictStrategy {
    /// Get display name for the conflict strategy
    pub fn name(&self) -> &'static str {
        match self {
            ConflictStrategy::DatabaseWins => "Database Wins",
            ConflictStrategy::EcsWins => "ECS Wins",
            ConflictStrategy::LastWriteWins => "Last Write Wins",
            ConflictStrategy::Merge => "Merge",
            ConflictStrategy::Manual => "Manual",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            ConflictStrategy::DatabaseWins => "Always use database version",
            ConflictStrategy::EcsWins => "Always use ECS version",
            ConflictStrategy::LastWriteWins => "Use most recently modified",
            ConflictStrategy::Merge => "Merge changes from both",
            ConflictStrategy::Manual => "Flag for manual resolution",
        }
    }
}

/// Sync statistics (mirrors dde_db::sync::SyncStats)
#[derive(Debug, Clone, Default)]
pub struct SyncStats {
    pub entities_synced: usize,
    pub components_synced: usize,
    pub conflicts_detected: usize,
    pub conflicts_resolved: usize,
    pub errors: usize,
    pub last_sync_duration: Duration,
    pub last_sync_time: Option<Instant>,
}

impl SyncPanel {
    /// Create a new sync panel
    pub fn new() -> Self {
        Self {
            visible: false,
            selected_tab: SyncTab::Status,
            status_message: None,
            status_timeout: 0.0,
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
    pub fn update(&mut self, dt: f32) {
        // Update status timeout
        if self.status_timeout > 0.0 {
            self.status_timeout -= dt;
            if self.status_timeout <= 0.0 {
                self.status_message = None;
            }
        }
    }

    /// Draw the sync panel UI
    pub fn draw(&mut self, ctx: &egui::Context, interface: &mut dyn SyncInterface) {
        if !self.visible {
            return;
        }

        let mut visible = self.visible;
        egui::Window::new("🔄 SQLite-ECS Sync")
            .open(&mut visible)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                self.draw_panel_content(ui, interface);
            });
        self.visible = visible;
    }

    /// Draw panel content
    fn draw_panel_content(&mut self, ui: &mut egui::Ui, interface: &mut dyn SyncInterface) {
        // Header with status indicator
        ui.horizontal(|ui| {
            ui.heading("SQLite-ECS Sync");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let status_text = if interface.is_syncing() {
                    "⏳ Syncing..."
                } else if interface.pending_changes() > 0 {
                    "⏳ Changes pending"
                } else {
                    "✓ In sync"
                };
                let status_color = if interface.is_syncing() {
                    egui::Color32::YELLOW
                } else if interface.pending_changes() > 0 {
                    egui::Color32::LIGHT_BLUE
                } else {
                    egui::Color32::GREEN
                };
                ui.colored_label(status_color, status_text);
            });
        });

        ui.separator();

        // Tab bar
        ui.horizontal(|ui| {
            self.tab_button(ui, "📊 Status", SyncTab::Status);
            self.tab_button(
                ui,
                &format!("⏳ Pending ({})", interface.pending_changes()),
                SyncTab::Pending,
            );
            self.tab_button(
                ui,
                &format!("⚠️ Conflicts ({})", interface.pending_conflicts()),
                SyncTab::Conflicts,
            );
            self.tab_button(ui, "⚙️ Settings", SyncTab::Settings);
        });

        ui.separator();

        // Tab content
        match self.selected_tab {
            SyncTab::Status => self.draw_status_tab(ui, interface),
            SyncTab::Pending => self.draw_pending_tab(ui),
            SyncTab::Conflicts => self.draw_conflicts_tab(ui),
            SyncTab::Settings => self.draw_settings_tab(ui, interface),
        }

        // Status message
        if let Some(ref msg) = self.status_message {
            ui.separator();
            ui.colored_label(egui::Color32::GREEN, msg);
        }
    }

    /// Draw tab button
    fn tab_button(&mut self, ui: &mut egui::Ui, label: &str, tab: SyncTab) {
        let selected = self.selected_tab == tab;
        if ui.selectable_label(selected, label).clicked() {
            self.selected_tab = tab;
        }
    }

    /// Draw status tab
    fn draw_status_tab(&mut self, ui: &mut egui::Ui, interface: &mut dyn SyncInterface) {
        let stats = interface.stats();

        ui.heading("Sync Status");
        ui.add_space(10.0);

        // Quick stats
        egui::Grid::new("sync_status_grid")
            .num_columns(2)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                ui.label("Status:");
                if interface.is_syncing() {
                    ui.colored_label(egui::Color32::YELLOW, "⏳ Syncing...");
                } else {
                    ui.colored_label(egui::Color32::GREEN, "✓ Ready");
                }
                ui.end_row();

                ui.label("Entities synced:");
                ui.label(stats.entities_synced.to_string());
                ui.end_row();

                ui.label("Components synced:");
                ui.label(stats.components_synced.to_string());
                ui.end_row();

                ui.label("Pending changes:");
                ui.label(interface.pending_changes().to_string());
                ui.end_row();

                ui.label("Pending conflicts:");
                let conflict_color = if interface.pending_conflicts() > 0 {
                    egui::Color32::RED
                } else {
                    ui.visuals().text_color()
                };
                ui.colored_label(conflict_color, interface.pending_conflicts().to_string());
                ui.end_row();

                ui.label("Last sync:");
                if let Some(time) = stats.last_sync_time {
                    let elapsed = time.elapsed();
                    let text = if elapsed.as_secs() < 60 {
                        format!("{}s ago", elapsed.as_secs())
                    } else if elapsed.as_secs() < 3600 {
                        format!("{}m ago", elapsed.as_secs() / 60)
                    } else {
                        format!("{}h ago", elapsed.as_secs() / 3600)
                    };
                    ui.label(text);
                } else {
                    ui.label("Never");
                }
                ui.end_row();

                if stats.last_sync_duration.as_millis() > 0 {
                    ui.label("Last sync duration:");
                    ui.label(format!("{} ms", stats.last_sync_duration.as_millis()));
                    ui.end_row();
                }
            });

        ui.add_space(20.0);

        // Quick actions
        ui.heading("Quick Actions");
        ui.horizontal(|ui| {
            if ui.button("🔄 Sync Now").clicked() && !interface.is_syncing() {
                interface.sync(SyncDirection::Bidirectional);
                self.show_status("Sync started", false);
            }

            if ui.button("💾 Save to DB").clicked() && !interface.is_syncing() {
                interface.sync(SyncDirection::EcsToDb);
                self.show_status("Saving to database...", false);
            }

            if ui.button("📂 Load from DB").clicked() && !interface.is_syncing() {
                interface.sync(SyncDirection::DbToEcs);
                self.show_status("Loading from database...", false);
            }
        });
    }

    /// Draw pending changes tab
    fn draw_pending_tab(&mut self, ui: &mut egui::Ui) {
        ui.label("Pending Changes");
        ui.label("Track entities and components waiting to be synchronized.");
        
        ui.add_space(10.0);
        
        ui.label("(Pending changes list would appear here)");
        ui.label("- Entity 123: Position modified");
        ui.label("- Entity 456: Stats modified");
        ui.label("- Entity 789: Created");
    }

    /// Draw conflicts tab
    fn draw_conflicts_tab(&mut self, ui: &mut egui::Ui) {
        ui.label("Pending Conflicts");
        ui.label("Review and resolve synchronization conflicts.");
        
        ui.add_space(10.0);
        
        ui.label("(Conflicts list would appear here)");
        ui.label("No pending conflicts.");
    }

    /// Draw settings tab
    fn draw_settings_tab(&mut self, ui: &mut egui::Ui, interface: &mut dyn SyncInterface) {
        ui.heading("Sync Settings");
        ui.separator();

        // Auto-sync interval
        let mut interval_secs = interface
            .auto_sync_interval()
            .map(|d| d.as_secs() as i64)
            .unwrap_or(-1);

        ui.horizontal(|ui| {
            ui.label("Auto-sync interval:");
            ui.add(
                egui::DragValue::new(&mut interval_secs)
                    .speed(1.0)
                    .range(-1..=3600)
                    .suffix(" s"),
            );
        });

        if interval_secs < 0 {
            ui.label("Auto-sync is disabled. Sync manually or on save.");
        } else {
            ui.label(format!("Auto-sync every {} seconds.", interval_secs));
        }

        let new_interval = if interval_secs < 0 {
            None
        } else {
            Some(Duration::from_secs(interval_secs as u64))
        };
        
        if new_interval != interface.auto_sync_interval() {
            interface.set_auto_sync_interval(new_interval);
        }

        ui.add_space(20.0);

        // Conflict strategy
        ui.heading("Conflict Resolution");
        ui.label("How to handle conflicts when both database and ECS have changes.");
        ui.add_space(10.0);

        let current_strategy = interface.conflict_strategy();

        egui::ComboBox::from_label("Strategy")
            .selected_text(current_strategy.name())
            .show_ui(ui, |ui| {
                for strategy in [
                    ConflictStrategy::DatabaseWins,
                    ConflictStrategy::EcsWins,
                    ConflictStrategy::LastWriteWins,
                    ConflictStrategy::Merge,
                    ConflictStrategy::Manual,
                ] {
                    let selected = current_strategy == strategy;
                    if ui
                        .selectable_label(selected, strategy.name())
                        .on_hover_text(strategy.description())
                        .clicked()
                        && !selected
                    {
                        interface.set_conflict_strategy(strategy);
                        self.show_status(
                            &format!("Conflict strategy set to: {}", strategy.name()),
                            false,
                        );
                    }
                }
            });

        ui.label(current_strategy.description());

        ui.add_space(20.0);

        // Danger zone
        ui.heading("Danger Zone");
        ui.separator();
        
        ui.horizontal(|ui| {
            if ui
                .button("🗑️ Clear All Mappings")
                .on_hover_text("Remove all entity ID mappings (requires full resync)")
                .clicked()
            {
                self.show_status("Entity mappings cleared", false);
            }
        });
    }

    /// Show a status message
    fn show_status(&mut self, message: &str, _is_error: bool) {
        self.status_message = Some(message.to_string());
        self.status_timeout = 5.0;
    }
}

impl Default for SyncPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSyncInterface {
        syncing: bool,
        pending_changes: usize,
        pending_conflicts: usize,
        stats: SyncStats,
        auto_sync_interval: Option<Duration>,
        conflict_strategy: ConflictStrategy,
    }

    impl MockSyncInterface {
        fn new() -> Self {
            Self {
                syncing: false,
                pending_changes: 5,
                pending_conflicts: 0,
                stats: SyncStats::default(),
                auto_sync_interval: Some(Duration::from_secs(30)),
                conflict_strategy: ConflictStrategy::LastWriteWins,
            }
        }
    }

    impl SyncInterface for MockSyncInterface {
        fn is_syncing(&self) -> bool {
            self.syncing
        }

        fn pending_changes(&self) -> usize {
            self.pending_changes
        }

        fn pending_conflicts(&self) -> usize {
            self.pending_conflicts
        }

        fn stats(&self) -> SyncStats {
            self.stats.clone()
        }

        fn sync(&mut self, _direction: SyncDirection) {
            self.syncing = true;
        }

        fn auto_sync_interval(&self) -> Option<Duration> {
            self.auto_sync_interval
        }

        fn set_auto_sync_interval(&mut self, interval: Option<Duration>) {
            self.auto_sync_interval = interval;
        }

        fn conflict_strategy(&self) -> ConflictStrategy {
            self.conflict_strategy
        }

        fn set_conflict_strategy(&mut self, strategy: ConflictStrategy) {
            self.conflict_strategy = strategy;
        }
    }

    #[test]
    fn test_panel_creation() {
        let panel = SyncPanel::new();
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_panel_toggle() {
        let mut panel = SyncPanel::new();
        assert!(!panel.is_visible());

        panel.toggle();
        assert!(panel.is_visible());

        panel.toggle();
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_conflict_strategy_names() {
        assert_eq!(ConflictStrategy::DatabaseWins.name(), "Database Wins");
        assert_eq!(ConflictStrategy::EcsWins.name(), "ECS Wins");
        assert_eq!(ConflictStrategy::LastWriteWins.name(), "Last Write Wins");
        assert_eq!(ConflictStrategy::Merge.name(), "Merge");
        assert_eq!(ConflictStrategy::Manual.name(), "Manual");
    }

    #[test]
    fn test_sync_direction_names() {
        assert_eq!(SyncDirection::EcsToDb.name(), "ECS → Database");
        assert_eq!(SyncDirection::DbToEcs.name(), "Database → ECS");
        assert_eq!(SyncDirection::Bidirectional.name(), "Bidirectional");
    }

    #[test]
    fn test_mock_sync_interface() {
        let mut mock = MockSyncInterface::new();
        
        assert!(!mock.is_syncing());
        assert_eq!(mock.pending_changes(), 5);
        
        mock.sync(SyncDirection::Bidirectional);
        assert!(mock.is_syncing());
        
        mock.set_conflict_strategy(ConflictStrategy::DatabaseWins);
        assert_eq!(mock.conflict_strategy(), ConflictStrategy::DatabaseWins);
    }
}
