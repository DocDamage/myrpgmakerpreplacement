//! Battle Log Viewer
//!
//! Comprehensive debugging interface for battle logs with filtering,
//! statistics, and replay capabilities.

use dde_battle::{BattleLog, LogEntry, LogEntryType, LogSeverity};
use dde_battle::{CombatantStatistics, CombatantStats, LogStyle};
use dde_core::Entity;
use std::collections::HashMap;

/// Battle Log Viewer panel
pub struct BattleLogViewer {
    /// Whether the panel is visible
    visible: bool,
    /// Currently selected tab
    selected_tab: LogViewerTab,
    /// Filter settings
    filters: LogFilters,
    /// Replay state
    replay_state: ReplayState,
    /// Combatant name cache (entity -> name)
    combatant_names: HashMap<Entity, String>,
    /// Currently selected entry index for detailed view
    selected_entry: Option<usize>,
    /// Auto-scroll to bottom in log view
    auto_scroll: bool,
    /// Search query
    search_query: String,
}

/// Available tabs in the log viewer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogViewerTab {
    /// Main log display with filters
    Log,
    /// Statistics overview
    Statistics,
    /// Replay controls
    Replay,
    /// Settings
    Settings,
}

/// Filter configuration for log entries
#[derive(Debug, Clone)]
pub struct LogFilters {
    /// Filter by combatant (None = all)
    combatant_filter: Option<Entity>,
    /// Filter by action types (empty = all)
    action_type_filter: Vec<LogEntryType>,
    /// Filter by turn range
    turn_range: Option<(u32, u32)>,
    /// Filter by severity (empty = all)
    severity_filter: Vec<LogSeverity>,
    /// Show only entries with damage
    show_only_damage: bool,
    /// Show only entries with healing
    show_only_healing: bool,
}

impl Default for LogFilters {
    fn default() -> Self {
        Self {
            combatant_filter: None,
            action_type_filter: Vec::new(),
            turn_range: None,
            severity_filter: Vec::new(),
            show_only_damage: false,
            show_only_healing: false,
        }
    }
}

/// Replay state for stepping through battle
#[derive(Debug, Clone)]
pub struct ReplayState {
    /// Current replay position (entry index)
    current_index: usize,
    /// Whether replay is playing automatically
    is_playing: bool,
    /// Playback speed multiplier
    playback_speed: f32,
    /// Current turn being viewed
    current_turn: u32,
    /// Auto-advance to next entry on timer
    auto_advance: bool,
    /// Time until next auto-advance
    next_advance_timer: f32,
}

impl Default for ReplayState {
    fn default() -> Self {
        Self {
            current_index: 0,
            is_playing: false,
            playback_speed: 1.0,
            current_turn: 0,
            auto_advance: false,
            next_advance_timer: 0.0,
        }
    }
}

/// Interface for battle log data access
pub trait BattleLogInterface {
    /// Get the current battle log
    fn get_battle_log(&self) -> Option<&BattleLog>;
    /// Get combatant name for an entity
    fn get_combatant_name(&self, entity: Entity) -> String;
    /// Check if battle is currently active
    fn is_battle_active(&self) -> bool;
    /// Get current turn number
    fn current_turn(&self) -> u32;
}

impl BattleLogViewer {
    /// Create a new battle log viewer
    pub fn new() -> Self {
        Self {
            visible: false,
            selected_tab: LogViewerTab::Log,
            filters: LogFilters::default(),
            replay_state: ReplayState::default(),
            combatant_names: HashMap::new(),
            selected_entry: None,
            auto_scroll: true,
            search_query: String::new(),
        }
    }

    /// Show the viewer
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the viewer
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

    /// Set combatant name for display
    pub fn set_combatant_name(&mut self, entity: Entity, name: impl Into<String>) {
        self.combatant_names.insert(entity, name.into());
    }

    /// Get combatant name (cached or default)
    fn get_combatant_name(&self, entity: Option<Entity>, interface: &dyn BattleLogInterface) -> String {
        match entity {
            Some(e) => self.combatant_names.get(&e).cloned()
                .unwrap_or_else(|| interface.get_combatant_name(e)),
            None => "-".to_string(),
        }
    }

    /// Draw the battle log viewer
    pub fn draw(&mut self, ctx: &egui::Context, interface: &dyn BattleLogInterface) {
        if !self.visible {
            return;
        }

        let mut visible = self.visible;
        egui::Window::new("📜 Battle Log Viewer")
            .open(&mut visible)
            .resizable(true)
            .default_size([800.0, 600.0])
            .show(ctx, |ui| {
                self.draw_content(ui, interface);
            });
        self.visible = visible;
    }

    /// Draw the main content area
    fn draw_content(&mut self, ui: &mut egui::Ui, interface: &dyn BattleLogInterface) {
        // Header with status
        ui.horizontal(|ui| {
            ui.heading("Battle Log Viewer");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let status_text = if interface.is_battle_active() {
                    format!("⏳ Battle Active - Turn {}", interface.current_turn())
                } else {
                    "✓ Battle Inactive".to_string()
                };
                let status_color = if interface.is_battle_active() {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::GREEN
                };
                ui.colored_label(status_color, status_text);
            });
        });

        ui.separator();

        // Tab bar
        ui.horizontal(|ui| {
            self.tab_button(ui, "📜 Log", LogViewerTab::Log);
            self.tab_button(ui, "📊 Statistics", LogViewerTab::Statistics);
            self.tab_button(ui, "▶️ Replay", LogViewerTab::Replay);
            self.tab_button(ui, "⚙️ Settings", LogViewerTab::Settings);
        });

        ui.separator();

        // Tab content
        match self.selected_tab {
            LogViewerTab::Log => self.draw_log_tab(ui, interface),
            LogViewerTab::Statistics => self.draw_statistics_tab(ui, interface),
            LogViewerTab::Replay => self.draw_replay_tab(ui, interface),
            LogViewerTab::Settings => self.draw_settings_tab(ui, interface),
        }
    }

    /// Draw tab button
    fn tab_button(&mut self, ui: &mut egui::Ui, label: &str, tab: LogViewerTab) {
        let selected = self.selected_tab == tab;
        if ui.selectable_label(selected, label).clicked() {
            self.selected_tab = tab;
        }
    }

    /// Draw the log tab with filters and entry list
    fn draw_log_tab(&mut self, ui: &mut egui::Ui, interface: &dyn BattleLogInterface) {
        let Some(log) = interface.get_battle_log() else {
            ui.label("No battle log available.");
            return;
        };

        // Filter controls
        ui.collapsing("🔍 Filters", |ui| {
            self.draw_filter_controls(ui, log, interface);
        });

        ui.separator();

        // Search bar
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_query);
            if ui.button("Clear").clicked() {
                self.search_query.clear();
            }
            ui.checkbox(&mut self.auto_scroll, "Auto-scroll");
        });

        ui.separator();

        // Log entries table
        let filtered_entries: Vec<(usize, &LogEntry)> = log
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, e)| self.passes_filters(e))
            .filter(|(_, e)| {
                if self.search_query.is_empty() {
                    true
                } else {
                    e.message.to_lowercase().contains(&self.search_query.to_lowercase())
                }
            })
            .collect();

        if filtered_entries.is_empty() {
            ui.label("No log entries match the current filters.");
        } else {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    egui::Grid::new("log_entries_grid")
                        .num_columns(6)
                        .spacing([10.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            // Header
                            ui.strong("Turn");
                            ui.strong("Tick");
                            ui.strong("Time");
                            ui.strong("Actor");
                            ui.strong("Type");
                            ui.strong("Message");
                            ui.end_row();

                            for (index, entry) in filtered_entries {
                                let is_selected = self.selected_entry == Some(index);
                                let row_response = ui.selectable_label(
                                    is_selected,
                                    format!("{}", entry.turn)
                                );
                                if row_response.clicked() {
                                    self.selected_entry = Some(index);
                                }

                                ui.label(format!("{}", entry.tick));
                                
                                // Format timestamp
                                let time_str = format_timestamp(&entry.timestamp);
                                ui.label(time_str);

                                let actor_name = self.get_combatant_name(entry.actor, interface);
                                ui.label(&actor_name);

                                let type_str = format!("{:?}", entry.entry_type);
                                let type_color = entry.entry_type.style().color;
                                ui.colored_label(
                                    egui::Color32::from_rgb(type_color.0, type_color.1, type_color.2),
                                    type_str
                                );

                                let style = entry.entry_type.style();
                                let msg_color = egui::Color32::from_rgb(
                                    style.color.0, style.color.1, style.color.2
                                );
                                let icon = style.icon.unwrap_or("");
                                ui.colored_label(msg_color, format!("{} {}", icon, entry.message));

                                ui.end_row();
                            }
                        });

                    if self.auto_scroll {
                        ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                    }
                });
        }

        // Entry detail view
        if let Some(index) = self.selected_entry {
            if let Some(entry) = log.get(index) {
                ui.separator();
                self.draw_entry_detail(ui, entry, interface);
            }
        }
    }

    /// Draw filter controls
    fn draw_filter_controls(&mut self, ui: &mut egui::Ui, log: &BattleLog, interface: &dyn BattleLogInterface) {
        ui.horizontal(|ui| {
            // Combatant filter
            ui.vertical(|ui| {
                ui.label("Combatant:");
                egui::ComboBox::from_id_source("combatant_filter")
                    .selected_text(
                        self.filters.combatant_filter
                            .map(|e| self.get_combatant_name(Some(e), interface))
                            .unwrap_or_else(|| "All".to_string())
                    )
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(self.filters.combatant_filter.is_none(), "All").clicked() {
                            self.filters.combatant_filter = None;
                        }
                        for entity in log.get_combatants() {
                            let name = self.get_combatant_name(Some(entity), interface);
                            let selected = self.filters.combatant_filter == Some(entity);
                            if ui.selectable_label(selected, &name).clicked() {
                                self.filters.combatant_filter = Some(entity);
                            }
                        }
                    });
            });

            // Action type filter
            ui.vertical(|ui| {
                ui.label("Action Types:");
                ui.horizontal_wrapped(|ui| {
                    for entry_type in [
                        LogEntryType::Damage,
                        LogEntryType::Crit,
                        LogEntryType::Heal,
                        LogEntryType::Miss,
                        LogEntryType::StatusApplied,
                        LogEntryType::SkillUse,
                        LogEntryType::ItemUse,
                    ] {
                        let type_name = format!("{:?}", entry_type);
                        let is_selected = self.filters.action_type_filter.contains(&entry_type);
                        if ui.selectable_label(is_selected, type_name).clicked() {
                            if is_selected {
                                self.filters.action_type_filter.retain(|&t| t != entry_type);
                            } else {
                                self.filters.action_type_filter.push(entry_type);
                            }
                        }
                    }
                });
            });
        });

        ui.horizontal(|ui| {
            // Turn range filter
            ui.vertical(|ui| {
                ui.label("Turn Range:");
                ui.horizontal(|ui| {
                    let mut start = self.filters.turn_range.map(|(s, _)| s).unwrap_or(0);
                    let mut end = self.filters.turn_range.map(|(_, e)| e).unwrap_or(999);
                    ui.add(egui::DragValue::new(&mut start).range(0..=999).prefix("From: "));
                    ui.add(egui::DragValue::new(&mut end).range(0..=999).prefix("To: "));
                    if start > 0 || end < 999 {
                        self.filters.turn_range = Some((start, end));
                    } else {
                        self.filters.turn_range = None;
                    }
                });
            });

            ui.checkbox(&mut self.filters.show_only_damage, "Only Damage");
            ui.checkbox(&mut self.filters.show_only_healing, "Only Healing");
        });

        if ui.button("Clear All Filters").clicked() {
            self.filters = LogFilters::default();
            self.search_query.clear();
        }
    }

    /// Check if an entry passes all current filters
    fn passes_filters(&self, entry: &LogEntry) -> bool {
        // Combatant filter
        if let Some(combatant) = self.filters.combatant_filter {
            if entry.actor != Some(combatant) && entry.target != Some(combatant) {
                return false;
            }
        }

        // Action type filter
        if !self.filters.action_type_filter.is_empty() {
            if !self.filters.action_type_filter.contains(&entry.entry_type) {
                return false;
            }
        }

        // Turn range filter
        if let Some((start, end)) = self.filters.turn_range {
            if entry.turn < start || entry.turn > end {
                return false;
            }
        }

        // Damage filter
        if self.filters.show_only_damage {
            if !matches!(entry.entry_type, LogEntryType::Damage | LogEntryType::Crit) {
                return false;
            }
        }

        // Healing filter
        if self.filters.show_only_healing {
            if entry.entry_type != LogEntryType::Heal {
                return false;
            }
        }

        true
    }

    /// Draw detailed view of a single entry
    fn draw_entry_detail(&mut self, ui: &mut egui::Ui, entry: &LogEntry, interface: &dyn BattleLogInterface) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.strong("Entry Details");
            
            egui::Grid::new("entry_detail_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Turn:");
                    ui.label(format!("{}", entry.turn));
                    ui.end_row();

                    ui.label("Tick:");
                    ui.label(format!("{}", entry.tick));
                    ui.end_row();

                    ui.label("Type:");
                    ui.label(format!("{:?}", entry.entry_type));
                    ui.end_row();

                    ui.label("Actor:");
                    ui.label(self.get_combatant_name(entry.actor, interface));
                    ui.end_row();

                    ui.label("Target:");
                    ui.label(self.get_combatant_name(entry.target, interface));
                    ui.end_row();

                    if let Some(damage) = entry.damage_dealt {
                        ui.label("Damage:");
                        ui.label(format!("{}", damage));
                        ui.end_row();
                    }

                    if let Some(healing) = entry.healing_done {
                        ui.label("Healing:");
                        ui.label(format!("{}", healing));
                        ui.end_row();
                    }

                    ui.label("Message:");
                    ui.label(&entry.message);
                    ui.end_row();
                });
        });
    }

    /// Draw the statistics tab
    fn draw_statistics_tab(&mut self, ui: &mut egui::Ui, interface: &dyn BattleLogInterface) {
        let Some(log) = interface.get_battle_log() else {
            ui.label("No battle log available.");
            return;
        };

        let stats = log.get_statistics_summary();

        ui.heading("📊 Battle Statistics");
        ui.add_space(10.0);

        // Overall summary
        ui.group(|ui| {
            ui.strong("Overall Summary");
            ui.horizontal(|ui| {
                ui.label(format!("Total Damage Dealt: {}", stats.total_damage_dealt()));
                ui.separator();
                ui.label(format!("Total Healing Done: {}", stats.total_healing_done()));
                ui.separator();
                ui.label(format!("Total Combatants: {}", stats.combatants().len()));
            });
        });

        ui.add_space(10.0);

        // Per-combatant statistics
        ui.strong("Per-Combatant Statistics");
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("stats_grid")
                .num_columns(8)
                .spacing([15.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    // Header
                    ui.strong("Combatant");
                    ui.strong("Dmg Dealt");
                    ui.strong("Dmg Taken");
                    ui.strong("Heal Done");
                    ui.strong("Heal Recv");
                    ui.strong("Attacks");
                    ui.strong("Crits");
                    ui.strong("Misses");
                    ui.end_row();

                    for entity in stats.combatants() {
                        let name = self.get_combatant_name(Some(entity), interface);
                        let combatant_stats = stats.get(entity).unwrap();

                        ui.label(name);
                        ui.label(format!("{}", combatant_stats.damage_dealt));
                        ui.label(format!("{}", combatant_stats.damage_taken));
                        ui.label(format!("{}", combatant_stats.healing_done));
                        ui.label(format!("{}", combatant_stats.healing_received));
                        ui.label(format!("{}", combatant_stats.attacks_made));
                        ui.label(format!("{}", combatant_stats.crits_made));
                        ui.label(format!("{}", combatant_stats.misses));
                        ui.end_row();
                    }
                });
        });

        // Additional metrics
        ui.add_space(10.0);
        ui.collapsing("Detailed Metrics", |ui| {
            for entity in stats.combatants() {
                let name = self.get_combatant_name(Some(entity), interface);
                let s = stats.get(entity).unwrap();
                
                ui.group(|ui| {
                    ui.strong(format!("{} - Detailed Stats", name));
                    
                    let accuracy = if s.attacks_made > 0 {
                        ((s.attacks_made - s.misses) as f32 / s.attacks_made as f32) * 100.0
                    } else {
                        0.0
                    };
                    
                    let crit_rate = if s.attacks_made > 0 {
                        (s.crits_made as f32 / s.attacks_made as f32) * 100.0
                    } else {
                        0.0
                    };

                    ui.label(format!("  Accuracy: {:.1}%", accuracy));
                    ui.label(format!("  Critical Rate: {:.1}%", crit_rate));
                    ui.label(format!("  Status Effects Applied: {}", s.statuses_applied));
                    ui.label(format!("  Status Effects Received: {}", s.statuses_received));
                });
            }
        });
    }

    /// Draw the replay tab
    fn draw_replay_tab(&mut self, ui: &mut egui::Ui, interface: &dyn BattleLogInterface) {
        let Some(log) = interface.get_battle_log() else {
            ui.label("No battle log available.");
            return;
        };

        if log.is_empty() {
            ui.label("Battle log is empty.");
            return;
        }

        ui.heading("▶️ Battle Replay");
        ui.add_space(10.0);

        // Playback controls
        ui.group(|ui| {
            ui.horizontal(|ui| {
                if ui.button("⏮️ First").clicked() {
                    self.replay_state.current_index = 0;
                }
                if ui.button("⏪ Prev").clicked() {
                    if self.replay_state.current_index > 0 {
                        self.replay_state.current_index -= 1;
                    }
                }
                
                let play_text = if self.replay_state.is_playing { "⏸️ Pause" } else { "▶️ Play" };
                if ui.button(play_text).clicked() {
                    self.replay_state.is_playing = !self.replay_state.is_playing;
                }
                
                if ui.button("⏩ Next").clicked() {
                    if self.replay_state.current_index < log.len() - 1 {
                        self.replay_state.current_index += 1;
                    }
                }
                if ui.button("⏭️ Last").clicked() {
                    self.replay_state.current_index = log.len().saturating_sub(1);
                }
            });

            ui.horizontal(|ui| {
                ui.label("Speed:");
                ui.add(egui::Slider::new(&mut self.replay_state.playback_speed, 0.5..=5.0)
                    .text("x"));
                
                ui.checkbox(&mut self.replay_state.auto_advance, "Auto-advance");
            });
        });

        ui.add_space(10.0);

        // Progress bar
        let total_entries = log.len();
        let current = self.replay_state.current_index.min(total_entries.saturating_sub(1));
        let progress = if total_entries > 0 {
            current as f32 / total_entries as f32
        } else {
            0.0
        };

        ui.add(egui::ProgressBar::new(progress)
            .text(format!("Entry {} / {}", current + 1, total_entries)));

        // Jump to turn
        ui.horizontal(|ui| {
            ui.label("Jump to Turn:");
            let max_turn = log.max_turn();
            let mut target_turn = self.replay_state.current_turn;
            if ui.add(egui::DragValue::new(&mut target_turn).range(0..=max_turn)).changed() {
                self.replay_state.current_turn = target_turn;
                // Find first entry of this turn
                for (i, entry) in log.entries().iter().enumerate() {
                    if entry.turn == target_turn {
                        self.replay_state.current_index = i;
                        break;
                    }
                }
            }
        });

        ui.add_space(10.0);

        // Current entry display
        if let Some(entry) = log.get(current) {
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.strong("Current Entry");
                
                let style = entry.entry_type.style();
                let color = egui::Color32::from_rgb(style.color.0, style.color.1, style.color.2);
                let icon = style.icon.unwrap_or("");
                
                ui.horizontal(|ui| {
                    ui.colored_label(color, format!("Turn {} - {} {}", 
                        entry.turn, icon, entry.entry_type));
                });
                
                ui.label(format!("Actor: {}", self.get_combatant_name(entry.actor, interface)));
                ui.label(format!("Target: {}", self.get_combatant_name(entry.target, interface)));
                ui.colored_label(color, &entry.message);
                
                if let Some(dmg) = entry.damage_dealt {
                    ui.colored_label(egui::Color32::RED, format!("Damage: {}", dmg));
                }
                if let Some(heal) = entry.healing_done {
                    ui.colored_label(egui::Color32::GREEN, format!("Healing: {}", heal));
                }
            });
        }

        // Turn summary
        ui.add_space(10.0);
        ui.collapsing("Turn Summary", |ui| {
            let turns = log.get_turns_with_entries();
            for turn in turns.iter().take(20) {
                let entries = log.entries_for_turn(*turn);
                ui.label(format!("Turn {}: {} entries", turn, entries.len()));
            }
        });
    }

    /// Draw the settings tab
    fn draw_settings_tab(&mut self, ui: &mut egui::Ui, _interface: &dyn BattleLogInterface) {
        ui.heading("⚙️ Viewer Settings");
        ui.add_space(10.0);

        ui.checkbox(&mut self.auto_scroll, "Auto-scroll to new entries");
        
        ui.add_space(10.0);
        
        ui.label("Export Options:");
        ui.horizontal(|ui| {
            if ui.button("Export as Text").clicked() {
                // Would trigger export callback
            }
            if ui.button("Export as JSON").clicked() {
                // Would trigger export callback
            }
        });

        ui.add_space(10.0);

        ui.label("Debug:");
        if ui.button("Clear Cache").clicked() {
            self.combatant_names.clear();
        }
    }

    /// Update replay state (call each frame)
    pub fn update(&mut self, dt: f32, interface: &dyn BattleLogInterface) {
        if !self.replay_state.is_playing || !self.replay_state.auto_advance {
            return;
        }

        let Some(log) = interface.get_battle_log() else {
            return;
        };

        self.replay_state.next_advance_timer -= dt * self.replay_state.playback_speed;
        
        if self.replay_state.next_advance_timer <= 0.0 {
            self.replay_state.next_advance_timer = 1.0; // Reset timer
            
            if self.replay_state.current_index < log.len().saturating_sub(1) {
                self.replay_state.current_index += 1;
            } else {
                self.replay_state.is_playing = false; // Stop at end
            }
        }
    }

    /// Get the current replay entry index
    pub fn current_replay_index(&self) -> usize {
        self.replay_state.current_index
    }

    /// Set the current replay position
    pub fn set_replay_index(&mut self, index: usize) {
        self.replay_state.current_index = index;
    }

    /// Jump to a specific turn
    pub fn jump_to_turn(&mut self, turn: u32, interface: &dyn BattleLogInterface) {
        if let Some(log) = interface.get_battle_log() {
            for (i, entry) in log.entries().iter().enumerate() {
                if entry.turn == turn {
                    self.replay_state.current_index = i;
                    self.replay_state.current_turn = turn;
                    break;
                }
            }
        }
    }
}

impl Default for BattleLogViewer {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a system time to a readable string
fn format_timestamp(time: &std::time::SystemTime) -> String {
    use std::time::UNIX_EPOCH;
    let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    let mins = secs / 60;
    let hours = mins / 60;
    format!("{:02}:{:02}:{:02}", hours % 24, mins % 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dde_battle::log::{BattleLog, LogEntryType};
    use dde_core::World;

    struct MockBattleLogInterface {
        log: BattleLog,
    }

    impl MockBattleLogInterface {
        fn new() -> Self {
            let mut log = BattleLog::new(100);
            
            // Add some test entries
            log.add(LogEntry::new(0, 1, LogEntryType::BattleStart, "Battle started!"));
            log.add(LogEntry::new(10, 1, LogEntryType::TurnStart, "Hero's turn")
                .with_actor(Entity::from_id(1)));
            log.add(LogEntry::new(15, 1, LogEntryType::Damage, "Hero deals 50 damage to Goblin")
                .with_actor(Entity::from_id(1))
                .with_target(Entity::from_id(2))
                .with_damage(50));
            log.add(LogEntry::new(20, 1, LogEntryType::TurnStart, "Goblin's turn")
                .with_actor(Entity::from_id(2)));
            log.add(LogEntry::new(25, 1, LogEntryType::Miss, "Goblin misses Hero")
                .with_actor(Entity::from_id(2))
                .with_target(Entity::from_id(1)));
            
            Self { log }
        }
    }

    impl BattleLogInterface for MockBattleLogInterface {
        fn get_battle_log(&self) -> Option<&BattleLog> {
            Some(&self.log)
        }

        fn get_combatant_name(&self, entity: Entity) -> String {
            match entity.id() {
                1 => "Hero".to_string(),
                2 => "Goblin".to_string(),
                _ => format!("Entity{}", entity.id()),
            }
        }

        fn is_battle_active(&self) -> bool {
            true
        }

        fn current_turn(&self) -> u32 {
            1
        }
    }

    #[test]
    fn test_viewer_creation() {
        let viewer = BattleLogViewer::new();
        assert!(!viewer.is_visible());
    }

    #[test]
    fn test_viewer_toggle() {
        let mut viewer = BattleLogViewer::new();
        assert!(!viewer.is_visible());
        
        viewer.toggle();
        assert!(viewer.is_visible());
        
        viewer.toggle();
        assert!(!viewer.is_visible());
    }

    #[test]
    fn test_combatant_names() {
        let mut viewer = BattleLogViewer::new();
        let entity = Entity::from_id(1);
        
        viewer.set_combatant_name(entity, "Test Hero");
        
        let mock = MockBattleLogInterface::new();
        let name = viewer.get_combatant_name(Some(entity), &mock);
        assert_eq!(name, "Test Hero");
    }

    #[test]
    fn test_filter_passes() {
        let viewer = BattleLogViewer::new();
        let entry = LogEntry::new(0, 1, LogEntryType::Damage, "Test")
            .with_damage(50);
        
        assert!(viewer.passes_filters(&entry));
    }

    #[test]
    fn test_replay_navigation() {
        let mut viewer = BattleLogViewer::new();
        let mock = MockBattleLogInterface::new();
        
        assert_eq!(viewer.current_replay_index(), 0);
        
        viewer.set_replay_index(2);
        assert_eq!(viewer.current_replay_index(), 2);
        
        viewer.jump_to_turn(1, &mock);
        // Should find first entry of turn 1
        assert!(viewer.current_replay_index() < mock.log.len());
    }
}
