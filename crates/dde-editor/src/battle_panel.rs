//! Battle Testing Panel
//!
//! Editor UI for testing and debugging the battle system.

/// Battle panel UI state
pub struct BattlePanel {
    /// Whether panel is visible
    visible: bool,
    /// Selected tab
    selected_tab: BattleTab,
    /// Battle state
    battle_active: bool,
    /// Test entity count
    test_entity_count: usize,
    /// Selected formation layout
    formation_layout: FormationLayout,
}

/// Battle panel tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BattleTab {
    Setup,
    Formation,
    Log,
    Balance,
}

/// Formation layout options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormationLayout {
    Balanced,
    Aggressive,
    Defensive,
}

impl FormationLayout {
    fn name(&self) -> &'static str {
        match self {
            FormationLayout::Balanced => "Balanced",
            FormationLayout::Aggressive => "Aggressive",
            FormationLayout::Defensive => "Defensive",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            FormationLayout::Balanced => "2 Front / 2 Back - Standard formation",
            FormationLayout::Aggressive => "3 Front / 1 Back - More damage dealt/taken",
            FormationLayout::Defensive => "1 Front / 3 Back - Less damage dealt/taken",
        }
    }
}

/// Interface for battle system
pub trait BattleInterface {
    /// Check if battle is active
    fn is_battle_active(&self) -> bool;
    /// Start test battle
    fn start_test_battle(&mut self, enemy_count: usize, formation: FormationLayout);
    /// End current battle
    fn end_battle(&mut self);
    /// Get battle log entries
    fn get_battle_log(&self) -> Vec<BattleLogEntry>;
    /// Get turn number
    fn turn_number(&self) -> u32;
    /// Get alive player count
    fn alive_players(&self) -> usize;
    /// Get alive enemy count
    fn alive_enemies(&self) -> usize;
}

/// Battle log entry
#[derive(Debug, Clone)]
pub struct BattleLogEntry {
    pub turn: u32,
    pub actor: String,
    pub action: String,
    pub result: String,
}

impl BattlePanel {
    /// Create a new battle panel
    pub fn new() -> Self {
        Self {
            visible: false,
            selected_tab: BattleTab::Setup,
            battle_active: false,
            test_entity_count: 2,
            formation_layout: FormationLayout::Balanced,
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

    /// Draw the battle panel UI
    pub fn draw(&mut self, ctx: &egui::Context, interface: &mut dyn BattleInterface) {
        if !self.visible {
            return;
        }

        let mut visible = self.visible;
        egui::Window::new("⚔️ Battle Testing")
            .open(&mut visible)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                self.draw_panel_content(ui, interface);
            });
        self.visible = visible;
    }

    /// Draw panel content
    fn draw_panel_content(&mut self, ui: &mut egui::Ui, interface: &mut dyn BattleInterface) {
        // Header with battle status
        ui.horizontal(|ui| {
            ui.heading("Battle Testing");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let status_text = if interface.is_battle_active() {
                    "⏳ Battle Active"
                } else {
                    "✓ Idle"
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
            self.tab_button(ui, "🎮 Setup", BattleTab::Setup);
            self.tab_button(ui, "🛡️ Formation", BattleTab::Formation);
            self.tab_button(ui, "📜 Log", BattleTab::Log);
            self.tab_button(ui, "⚖️ Balance", BattleTab::Balance);
        });

        ui.separator();

        // Tab content
        match self.selected_tab {
            BattleTab::Setup => self.draw_setup_tab(ui, interface),
            BattleTab::Formation => self.draw_formation_tab(ui),
            BattleTab::Log => self.draw_log_tab(ui, interface),
            BattleTab::Balance => self.draw_balance_tab(ui),
        }
    }

    /// Draw tab button
    fn tab_button(&mut self, ui: &mut egui::Ui, label: &str, tab: BattleTab) {
        let selected = self.selected_tab == tab;
        if ui.selectable_label(selected, label).clicked() {
            self.selected_tab = tab;
        }
    }

    /// Draw setup tab
    fn draw_setup_tab(&mut self, ui: &mut egui::Ui, interface: &mut dyn BattleInterface) {
        ui.heading("Test Battle Setup");
        ui.add_space(10.0);

        // Enemy count
        ui.horizontal(|ui| {
            ui.label("Enemy Count:");
            ui.add(egui::DragValue::new(&mut self.test_entity_count).range(1..=8));
        });

        ui.add_space(10.0);

        // Formation layout
        ui.label("Formation Layout:");
        for layout in [FormationLayout::Balanced, FormationLayout::Aggressive, FormationLayout::Defensive] {
            ui.horizontal(|ui| {
                let selected = self.formation_layout == layout;
                if ui.radio(selected, layout.name()).clicked() && !selected {
                    self.formation_layout = layout;
                }
            });
            ui.label(format!("  {}", layout.description()));
        }

        ui.add_space(20.0);

        // Battle controls
        ui.horizontal(|ui| {
            if interface.is_battle_active() {
                if ui.button("⏹️ End Battle").clicked() {
                    interface.end_battle();
                }
            } else if ui.button("▶️ Start Test Battle").clicked() {
                interface.start_test_battle(self.test_entity_count, self.formation_layout);
            }
        });

        ui.add_space(20.0);

        // Quick stats
        if interface.is_battle_active() {
            ui.heading("Current Battle");
            egui::Grid::new("battle_stats_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Turn:");
                    ui.label(interface.turn_number().to_string());
                    ui.end_row();

                    ui.label("Players Alive:");
                    ui.label(interface.alive_players().to_string());
                    ui.end_row();

                    ui.label("Enemies Alive:");
                    ui.label(interface.alive_enemies().to_string());
                    ui.end_row();
                });
        }
    }

    /// Draw formation tab
    fn draw_formation_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Formation Editor");
        ui.label("Configure party formation for battle positioning.");
        
        ui.add_space(10.0);

        // Formation visualization placeholder
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(150.0);
            
            ui.label("Back Row:");
            ui.horizontal(|ui| {
                for i in 0..4 {
                    if ui.button(format!("[{}]", i)).clicked() {
                        // Would select slot
                    }
                }
            });

            ui.add_space(20.0);

            ui.label("Front Row:");
            ui.horizontal(|ui| {
                for i in 0..4 {
                    if ui.button(format!("[{}]", i)).clicked() {
                        // Would select slot
                    }
                }
            });
        });

        ui.add_space(10.0);

        ui.label("Front Row: Deal/take more damage");
        ui.label("Back Row: Take less damage, reduced physical accuracy");
    }

    /// Draw log tab
    fn draw_log_tab(&mut self, ui: &mut egui::Ui, interface: &dyn BattleInterface) {
        ui.heading("Battle Log");
        ui.add_space(10.0);

        let entries = interface.get_battle_log();
        
        if entries.is_empty() {
            ui.label("No battle actions recorded yet.");
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for entry in entries.iter().rev().take(50) {
                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.label(format!("Turn {}: {}", entry.turn, entry.actor));
                        ui.label(format!("  Action: {}", entry.action));
                        ui.label(format!("  Result: {}", entry.result));
                    });
                    ui.add_space(4.0);
                }
            });
        }
    }

    /// Draw balance tab
    fn draw_balance_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Damage Balance Analysis");
        ui.label("Compare damage output across different configurations.");
        
        ui.add_space(10.0);

        // Balance metrics placeholder
        egui::Grid::new("balance_grid")
            .num_columns(3)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                ui.label("Configuration");
                ui.label("Avg DPS");
                ui.label("Survivability");
                ui.end_row();

                ui.label("Physical (Front)");
                ui.label("45.2");
                ui.label("Low");
                ui.end_row();

                ui.label("Physical (Back)");
                ui.label("38.4");
                ui.label("Medium");
                ui.end_row();

                ui.label("Magic (Any)");
                ui.label("42.1");
                ui.label("Medium");
                ui.end_row();
            });

        ui.add_space(20.0);

        ui.label("(Detailed balance analysis would require battle simulation data)");
    }
}

impl Default for BattlePanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockBattleInterface {
        active: bool,
        turn: u32,
        players: usize,
        enemies: usize,
        log: Vec<BattleLogEntry>,
    }

    impl MockBattleInterface {
        fn new() -> Self {
            Self {
                active: false,
                turn: 0,
                players: 4,
                enemies: 0,
                log: vec![
                    BattleLogEntry {
                        turn: 1,
                        actor: "Player 1".to_string(),
                        action: "Attack".to_string(),
                        result: "Dealt 25 damage".to_string(),
                    },
                    BattleLogEntry {
                        turn: 1,
                        actor: "Enemy 1".to_string(),
                        action: "Slash".to_string(),
                        result: "Dealt 15 damage".to_string(),
                    },
                ],
            }
        }
    }

    impl BattleInterface for MockBattleInterface {
        fn is_battle_active(&self) -> bool {
            self.active
        }

        fn start_test_battle(&mut self, enemy_count: usize, _formation: FormationLayout) {
            self.active = true;
            self.enemies = enemy_count;
            self.turn = 1;
        }

        fn end_battle(&mut self) {
            self.active = false;
        }

        fn get_battle_log(&self) -> Vec<BattleLogEntry> {
            self.log.clone()
        }

        fn turn_number(&self) -> u32 {
            self.turn
        }

        fn alive_players(&self) -> usize {
            self.players
        }

        fn alive_enemies(&self) -> usize {
            self.enemies
        }
    }

    #[test]
    fn test_panel_creation() {
        let panel = BattlePanel::new();
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_panel_toggle() {
        let mut panel = BattlePanel::new();
        assert!(!panel.is_visible());

        panel.toggle();
        assert!(panel.is_visible());

        panel.toggle();
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_mock_battle_interface() {
        let mut mock = MockBattleInterface::new();
        
        assert!(!mock.is_battle_active());
        assert_eq!(mock.turn_number(), 0);
        
        mock.start_test_battle(3, FormationLayout::Balanced);
        assert!(mock.is_battle_active());
        assert_eq!(mock.turn_number(), 1);
        assert_eq!(mock.alive_enemies(), 3);
        
        mock.end_battle();
        assert!(!mock.is_battle_active());
    }

    #[test]
    fn test_formation_layout() {
        assert_eq!(FormationLayout::Balanced.name(), "Balanced");
        assert_eq!(FormationLayout::Aggressive.name(), "Aggressive");
        assert_eq!(FormationLayout::Defensive.name(), "Defensive");
    }
}
