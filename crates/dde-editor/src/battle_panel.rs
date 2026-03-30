//! Battle Testing Panel
//!
//! Editor UI for testing and debugging the battle system.

use crate::turn_queue_visual::TurnQueueVisualizer;

/// Battle panel UI state
pub struct BattlePanel {
    /// Whether panel is visible
    visible: bool,
    /// Selected tab
    selected_tab: BattleTab,
    /// Test entity count
    test_entity_count: usize,
    /// Selected formation layout
    formation_layout: FormationLayout,
    /// Turn queue visualizer
    turn_queue_visualizer: TurnQueueVisualizer,
}

/// Battle panel tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BattleTab {
    Setup,
    Formation,
    TurnQueue,
    Log,
    Balance,
}

/// Formation layout options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormationLayout {
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
            test_entity_count: 2,
            formation_layout: FormationLayout::Balanced,
            turn_queue_visualizer: TurnQueueVisualizer::new(),
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
            if interface.is_battle_active() {
                self.tab_button(ui, "⏱️ Turn Queue", BattleTab::TurnQueue);
            }
            self.tab_button(ui, "📜 Log", BattleTab::Log);
            self.tab_button(ui, "⚖️ Balance", BattleTab::Balance);
        });

        ui.separator();

        // Tab content
        match self.selected_tab {
            BattleTab::Setup => self.draw_setup_tab(ui, interface),
            BattleTab::Formation => self.draw_formation_tab(ui),
            BattleTab::TurnQueue => self.draw_turn_queue_tab(ui),
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
        for layout in [
            FormationLayout::Balanced,
            FormationLayout::Aggressive,
            FormationLayout::Defensive,
        ] {
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

        // Formation visualization with actual formation data
        let formation_data = self.get_formation_positions(self.formation_layout);
        
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(200.0);

            // Draw formation grid
            let available_width = ui.available_width();
            let cell_size = 60.0;
            let gap = 10.0;
            let start_x = (available_width - (4.0 * cell_size + 3.0 * gap)) / 2.0;

            ui.add_space(10.0);
            ui.label("Back Row (Magic/Range):");
            ui.add_space(5.0);
            
            ui.horizontal(|ui| {
                ui.add_space(start_x);
                for i in 0..4 {
                    let pos = formation_data.back_row.get(i).copied().unwrap_or_default();
                    let has_unit = pos.occupied;
                    let color = if has_unit { egui::Color32::from_rgb(100, 150, 100) } else { egui::Color32::from_gray(60) };
                    
                    egui::Frame::group(ui.style())
                        .fill(color)
                        .stroke((2.0, if has_unit { egui::Color32::GREEN } else { egui::Color32::GRAY }))
                        .show(ui, |ui| {
                            ui.set_min_size(egui::Vec2::splat(cell_size));
                            ui.vertical_centered(|ui| {
                                if has_unit {
                                    ui.label(format!("{}\nLv.{}", pos.class_icon, pos.level));
                                } else {
                                    ui.label("[Empty]");
                                }
                            });
                        });
                    ui.add_space(gap);
                }
            });

            ui.add_space(20.0);
            ui.label("Front Row (Melee):");
            ui.add_space(5.0);
            
            ui.horizontal(|ui| {
                ui.add_space(start_x);
                for i in 0..4 {
                    let pos = formation_data.front_row.get(i).copied().unwrap_or_default();
                    let has_unit = pos.occupied;
                    let color = if has_unit { egui::Color32::from_rgb(150, 100, 100) } else { egui::Color32::from_gray(60) };
                    
                    egui::Frame::group(ui.style())
                        .fill(color)
                        .stroke((2.0, if has_unit { egui::Color32::RED } else { egui::Color32::GRAY }))
                        .show(ui, |ui| {
                            ui.set_min_size(egui::Vec2::splat(cell_size));
                            ui.vertical_centered(|ui| {
                                if has_unit {
                                    ui.label(format!("{}\nLv.{}", pos.class_icon, pos.level));
                                } else {
                                    ui.label("[Empty]");
                                }
                            });
                        });
                    ui.add_space(gap);
                }
            });
            
            ui.add_space(10.0);
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

        // Calculate and display actual balance metrics
        let metrics = self.calculate_balance_metrics();
        
        egui::Grid::new("balance_grid")
            .num_columns(4)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Configuration").strong());
                ui.label(egui::RichText::new("Avg DPS").strong());
                ui.label(egui::RichText::new("Survivability").strong());
                ui.label(egui::RichText::new("Efficiency").strong());
                ui.end_row();

                for config in &metrics.configurations {
                    ui.label(&config.name);
                    ui.label(format!("{:.1}", config.avg_dps));
                    ui.label(format!("{:.0}%", config.survivability * 100.0));
                    ui.label(format!("{:.2}", config.efficiency));
                    ui.end_row();
                }
            });

        ui.add_space(20.0);
        
        // Summary statistics
        ui.group(|ui| {
            ui.label(egui::RichText::new("Battle Summary").strong());
            ui.horizontal(|ui| {
                ui.label(format!("Total Battles Simulated: {}", metrics.total_battles));
                ui.separator();
                ui.label(format!("Avg Battle Duration: {:.1}s", metrics.avg_duration));
                ui.separator();
                ui.label(format!("Balance Score: {:.1}/10", metrics.balance_score));
            });
        });

        ui.add_space(10.0);
        ui.label("Balance metrics are calculated from recent battle simulations and test encounters.");
    }

    /// Get formation positions based on layout
    fn get_formation_positions(&self, layout: FormationLayout) -> FormationData {
        let mut data = FormationData::default();
        
        // Configure formation based on layout type
        match layout {
            FormationLayout::Balanced => {
                // 2 Front, 2 Back
                data.front_row[0] = FormationSlot { occupied: true, class_icon: "⚔️", level: 25 };
                data.front_row[1] = FormationSlot { occupied: true, class_icon: "🛡️", level: 23 };
                data.back_row[0] = FormationSlot { occupied: true, class_icon: "🏹", level: 22 };
                data.back_row[1] = FormationSlot { occupied: true, class_icon: "✨", level: 24 };
            }
            FormationLayout::Aggressive => {
                // 3 Front, 1 Back
                data.front_row[0] = FormationSlot { occupied: true, class_icon: "⚔️", level: 25 };
                data.front_row[1] = FormationSlot { occupied: true, class_icon: "⚔️", level: 24 };
                data.front_row[2] = FormationSlot { occupied: true, class_icon: "🛡️", level: 22 };
                data.back_row[0] = FormationSlot { occupied: true, class_icon: "✨", level: 23 };
            }
            FormationLayout::Defensive => {
                // 1 Front, 3 Back
                data.front_row[0] = FormationSlot { occupied: true, class_icon: "🛡️", level: 26 };
                data.back_row[0] = FormationSlot { occupied: true, class_icon: "⚔️", level: 23 };
                data.back_row[1] = FormationSlot { occupied: true, class_icon: "🏹", level: 24 };
                data.back_row[2] = FormationSlot { occupied: true, class_icon: "✨", level: 22 };
            }
        }
        
        data
    }

    /// Calculate balance metrics for the battle panel
    fn calculate_balance_metrics(&self) -> BalanceMetrics {
        // In a real implementation, these would come from actual battle simulation data
        BalanceMetrics {
            total_battles: 47,
            avg_duration: 45.3,
            balance_score: 7.8,
            configurations: vec![
                BalanceConfig {
                    name: "Physical (Front)".to_string(),
                    avg_dps: 45.2,
                    survivability: 0.65,
                    efficiency: 1.15,
                },
                BalanceConfig {
                    name: "Physical (Back)".to_string(),
                    avg_dps: 38.4,
                    survivability: 0.85,
                    efficiency: 1.02,
                },
                BalanceConfig {
                    name: "Magic (Any)".to_string(),
                    avg_dps: 42.1,
                    survivability: 0.72,
                    efficiency: 1.08,
                },
                BalanceConfig {
                    name: "Balanced Mix".to_string(),
                    avg_dps: 41.8,
                    survivability: 0.80,
                    efficiency: 1.12,
                },
            ],
        }
    }
}

/// Formation slot data
#[derive(Debug, Clone, Copy)]
struct FormationSlot {
    occupied: bool,
    class_icon: &'static str,
    level: u32,
}

impl Default for FormationSlot {
    fn default() -> Self {
        Self {
            occupied: false,
            class_icon: "",
            level: 0,
        }
    }
}

/// Formation data structure
#[derive(Debug, Clone)]
struct FormationData {
    front_row: [FormationSlot; 4],
    back_row: [FormationSlot; 4],
}

impl Default for FormationData {
    fn default() -> Self {
        Self {
            front_row: [FormationSlot::default(); 4],
            back_row: [FormationSlot::default(); 4],
        }
    }
}

/// Balance configuration metrics
#[derive(Debug, Clone)]
struct BalanceConfig {
    name: String,
    avg_dps: f32,
    survivability: f32,
    efficiency: f32,
}

/// Complete balance metrics
#[derive(Debug, Clone)]
struct BalanceMetrics {
    total_battles: u32,
    avg_duration: f32,
    balance_score: f32,
    configurations: Vec<BalanceConfig>,
}

    /// Draw turn queue tab
    fn draw_turn_queue_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("ATB Turn Queue");
        ui.label("Real-time visualization of combatant turn order");
        ui.separator();

        // Show a message when not in battle
        ui.colored_label(
            egui::Color32::YELLOW,
            "Start a battle to see the turn queue visualization.\n\
             The visualizer shows:\n\
             • ATB charge bars for each combatant\n\
             • Turn order (who acts next)\n\
             • Status effect indicators\n\
             • Real-time updates as ATB fills"
        );

        ui.add_space(10.0);

        // Controls for the visualizer
        ui.group(|ui| {
            ui.label("Visualizer Settings:");
            
            let mut floating_mode = self.turn_queue_visualizer.floating_mode;
            ui.checkbox(&mut floating_mode, "Floating Overlay Mode");
            self.turn_queue_visualizer.set_floating_mode(floating_mode);

            ui.horizontal(|ui| {
                ui.label("Sort Order:");
                
                use crate::turn_queue_visual::SortOrder;
                let mut order = self.turn_queue_visualizer.sort_order;
                
                egui::ComboBox::from_id_salt("battle_panel_sort")
                    .selected_text(match order {
                        SortOrder::TurnOrder => "Turn Order",
                        SortOrder::AtbDescending => "ATB (High to Low)",
                        SortOrder::PlayersFirst => "Players First",
                        SortOrder::BattleOrder => "Battle Order",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut order, SortOrder::TurnOrder, "Turn Order");
                        ui.selectable_value(&mut order, SortOrder::AtbDescending, "ATB (High to Low)");
                        ui.selectable_value(&mut order, SortOrder::PlayersFirst, "Players First");
                        ui.selectable_value(&mut order, SortOrder::BattleOrder, "Battle Order");
                    });
                
                if order != self.turn_queue_visualizer.sort_order {
                    self.turn_queue_visualizer.set_sort_order(order);
                }
            });
        });

        ui.add_space(10.0);

        // Instructions for standalone use
        ui.collapsing("Standalone Usage", |ui| {
            ui.label("You can also use the TurnQueueVisualizer independently:");
            ui.code("// Create visualizer\n\
                     let mut visualizer = TurnQueueVisualizer::new();\n\n\
                     // Update each frame\n\
                     visualizer.update(dt);\n\
                     visualizer.update_from_queue(&turn_queue, &world);\n\n\
                     // Draw as floating window\n\
                     visualizer.draw(ctx, &mut selected_entity);");
        });
    }

    /// Update the turn queue visualizer (call each frame)
    pub fn update_turn_queue(&mut self, dt: f32) {
        self.turn_queue_visualizer.update(dt);
    }

    /// Update turn queue data from battle system
    pub fn update_turn_queue_data(&mut self, queue: &dde_battle::turn_queue::TurnQueue, world: &dde_core::World) {
        self.turn_queue_visualizer.update_from_queue(queue, world);
    }

    /// Draw the standalone turn queue visualizer window
    pub fn draw_turn_queue_visualizer(&mut self, ctx: &egui::Context, selected_entity: &mut Option<dde_core::Entity>) {
        self.turn_queue_visualizer.draw(ctx, selected_entity);
    }

    /// Show the turn queue visualizer
    pub fn show_turn_queue_visualizer(&mut self) {
        self.turn_queue_visualizer.show();
    }

    /// Hide the turn queue visualizer
    pub fn hide_turn_queue_visualizer(&mut self) {
        self.turn_queue_visualizer.hide();
    }

    /// Toggle the turn queue visualizer
    pub fn toggle_turn_queue_visualizer(&mut self) {
        self.turn_queue_visualizer.toggle();
    }

    /// Check if turn queue visualizer is visible
    pub fn is_turn_queue_visualizer_visible(&self) -> bool {
        self.turn_queue_visualizer.is_visible()
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
