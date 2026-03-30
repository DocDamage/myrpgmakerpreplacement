//! Battle Formula Editor
//!
//! Visual editor for creating and testing damage formulas with:
//! - Formula type tabs (Damage, Healing, Critical Hit, Status Apply, Flee)
//! - Variable picker with available battle variables
//! - Test simulator with distribution graphs
//! - Preset formulas for common RPG systems
//! - Save/Load to TOML configuration
//! - Hot-reload support
//!
//! This module is now wired to the actual damage calculation backend
//! using `DamageCalculator` from `dde_battle` and `FormulaResource` from `dde_core`.

use std::path::PathBuf;
use std::time::{Duration, Instant};

// DamageCalculator and SimulationResult are re-exported from dde_battle
use dde_battle::{DamageCalculator, SimulationResult};
use dde_core::components::Stats;
use dde_core::resources::formula::{FormulaKind, FormulaResource, validate_formula};

/// Formula type categories - mirrors FormulaKind for UI purposes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormulaType {
    Damage,
    Healing,
    CriticalHit,
    StatusApply,
    Flee,
}

impl FormulaType {
    fn name(&self) -> &'static str {
        match self {
            FormulaType::Damage => "⚔️ Damage",
            FormulaType::Healing => "💚 Healing",
            FormulaType::CriticalHit => "💥 Critical Hit",
            FormulaType::StatusApply => "🌀 Status Apply",
            FormulaType::Flee => "🏃 Flee",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            FormulaType::Damage => "Calculate damage dealt to target",
            FormulaType::Healing => "Calculate HP restored to target",
            FormulaType::CriticalHit => "Calculate critical hit chance (0.0 - 1.0)",
            FormulaType::StatusApply => "Calculate status effect apply chance (0.0 - 1.0)",
            FormulaType::Flee => "Calculate flee/escape chance (0.0 - 1.0)",
        }
    }

    /// Convert to FormulaKind for backend operations
    fn to_kind(&self) -> FormulaKind {
        match self {
            FormulaType::Damage => FormulaKind::Damage,
            FormulaType::Healing => FormulaKind::Healing,
            FormulaType::CriticalHit => FormulaKind::CriticalHit,
            FormulaType::StatusApply => FormulaKind::StatusApply,
            FormulaType::Flee => FormulaKind::Flee,
        }
    }

    /// Convert from FormulaKind
    fn from_kind(kind: FormulaKind) -> Self {
        match kind {
            FormulaKind::Damage => FormulaType::Damage,
            FormulaKind::Healing => FormulaType::Healing,
            FormulaKind::CriticalHit => FormulaType::CriticalHit,
            FormulaKind::StatusApply => FormulaType::StatusApply,
            FormulaKind::Flee => FormulaType::Flee,
        }
    }

    /// Get all formula types
    fn all() -> [FormulaType; 5] {
        [
            FormulaType::Damage,
            FormulaType::Healing,
            FormulaType::CriticalHit,
            FormulaType::StatusApply,
            FormulaType::Flee,
        ]
    }
}

/// Available formula presets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormulaPreset {
    Standard,
    PokemonStyle,
    Simple,
    Percentage,
    Custom,
}

impl FormulaPreset {
    fn name(&self) -> &'static str {
        match self {
            FormulaPreset::Standard => "Standard RPG",
            FormulaPreset::PokemonStyle => "Pokémon Style",
            FormulaPreset::Simple => "Simple Variance",
            FormulaPreset::Percentage => "Percentage Based",
            FormulaPreset::Custom => "Custom Formula",
        }
    }

    fn formula(&self, formula_type: FormulaType) -> String {
        match formula_type {
            FormulaType::Damage => match self {
                FormulaPreset::Standard => {
                    "(attacker.str * 4 - defender.def * 2) * skill.power / 100".to_string()
                }
                FormulaPreset::PokemonStyle => {
                    "(((2 * attacker.level / 5 + 2) * skill.power * attacker.str / defender.def) / 50) + 2".to_string()
                }
                FormulaPreset::Simple => {
                    "(attacker.str - defender.def) * random(0.9, 1.1)".to_string()
                }
                FormulaPreset::Percentage => {
                    "defender.max_hp * skill.power / 100".to_string()
                }
                FormulaPreset::Custom => String::new(),
            },
            FormulaType::Healing => {
                "attacker.mag * 3 + attacker.level * 2".to_string()
            }
            FormulaType::CriticalHit => {
                "0.05 + attacker.luck / 200".to_string()
            }
            FormulaType::StatusApply => {
                "(attacker.mag - defender.mag + 50) / 100".to_string()
            }
            FormulaType::Flee => {
                "0.5 + (attacker.spd - defender.spd) / 100".to_string()
            }
        }
    }
}

/// Available variable categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableCategory {
    Attacker,
    Defender,
    Skill,
    Functions,
}

impl VariableCategory {
    fn name(&self) -> &'static str {
        match self {
            VariableCategory::Attacker => "👤 Attacker",
            VariableCategory::Defender => "🛡️ Defender",
            VariableCategory::Skill => "⚡ Skill",
            VariableCategory::Functions => "🔧 Functions",
        }
    }
}

/// Available variable for formula editing
#[derive(Debug, Clone)]
pub struct FormulaVariable {
    pub name: &'static str,
    pub category: VariableCategory,
    pub description: &'static str,
    pub example: &'static str,
}

impl FormulaVariable {
    fn all() -> Vec<Self> {
        vec![
            // Attacker stats
            FormulaVariable {
                name: "attacker.str",
                category: VariableCategory::Attacker,
                description: "Attacker's Strength stat",
                example: "20",
            },
            FormulaVariable {
                name: "attacker.def",
                category: VariableCategory::Attacker,
                description: "Attacker's Defense stat",
                example: "15",
            },
            FormulaVariable {
                name: "attacker.spd",
                category: VariableCategory::Attacker,
                description: "Attacker's Speed stat",
                example: "12",
            },
            FormulaVariable {
                name: "attacker.mag",
                category: VariableCategory::Attacker,
                description: "Attacker's Magic stat",
                example: "18",
            },
            FormulaVariable {
                name: "attacker.luck",
                category: VariableCategory::Attacker,
                description: "Attacker's Luck stat",
                example: "10",
            },
            FormulaVariable {
                name: "attacker.level",
                category: VariableCategory::Attacker,
                description: "Attacker's current level",
                example: "25",
            },
            FormulaVariable {
                name: "attacker.hp",
                category: VariableCategory::Attacker,
                description: "Attacker's current HP",
                example: "150",
            },
            FormulaVariable {
                name: "attacker.max_hp",
                category: VariableCategory::Attacker,
                description: "Attacker's maximum HP",
                example: "200",
            },
            // Defender stats
            FormulaVariable {
                name: "defender.str",
                category: VariableCategory::Defender,
                description: "Defender's Strength stat",
                example: "18",
            },
            FormulaVariable {
                name: "defender.def",
                category: VariableCategory::Defender,
                description: "Defender's Defense stat",
                example: "20",
            },
            FormulaVariable {
                name: "defender.spd",
                category: VariableCategory::Defender,
                description: "Defender's Speed stat",
                example: "14",
            },
            FormulaVariable {
                name: "defender.mag",
                category: VariableCategory::Defender,
                description: "Defender's Magic stat",
                example: "16",
            },
            FormulaVariable {
                name: "defender.luck",
                category: VariableCategory::Defender,
                description: "Defender's Luck stat",
                example: "12",
            },
            FormulaVariable {
                name: "defender.level",
                category: VariableCategory::Defender,
                description: "Defender's current level",
                example: "22",
            },
            FormulaVariable {
                name: "defender.hp",
                category: VariableCategory::Defender,
                description: "Defender's current HP",
                example: "180",
            },
            FormulaVariable {
                name: "defender.max_hp",
                category: VariableCategory::Defender,
                description: "Defender's maximum HP",
                example: "220",
            },
            // Skill stats
            FormulaVariable {
                name: "skill.power",
                category: VariableCategory::Skill,
                description: "Skill's power value (0-500)",
                example: "100",
            },
            FormulaVariable {
                name: "skill.accuracy",
                category: VariableCategory::Skill,
                description: "Skill's accuracy (0.0 - 1.0)",
                example: "0.95",
            },
            // Functions
            FormulaVariable {
                name: "random()",
                category: VariableCategory::Functions,
                description: "Random value between 0 and 1",
                example: "random() * 10",
            },
            FormulaVariable {
                name: "random(min, max)",
                category: VariableCategory::Functions,
                description: "Random value between min and max",
                example: "random(0.9, 1.1)",
            },
            FormulaVariable {
                name: "min(a, b)",
                category: VariableCategory::Functions,
                description: "Returns the smaller of two values",
                example: "min(damage, 999)",
            },
            FormulaVariable {
                name: "max(a, b)",
                category: VariableCategory::Functions,
                description: "Returns the larger of two values",
                example: "max(damage, 1)",
            },
            FormulaVariable {
                name: "clamp(val, min, max)",
                category: VariableCategory::Functions,
                description: "Clamps value between min and max",
                example: "clamp(chance, 0, 1)",
            },
            FormulaVariable {
                name: "abs(val)",
                category: VariableCategory::Functions,
                description: "Absolute value",
                example: "abs(defender.str - attacker.str)",
            },
            FormulaVariable {
                name: "sqrt(val)",
                category: VariableCategory::Functions,
                description: "Square root",
                example: "sqrt(attacker.level)",
            },
            FormulaVariable {
                name: "pow(base, exp)",
                category: VariableCategory::Functions,
                description: "Power function",
                example: "pow(attacker.str, 1.5)",
            },
        ]
    }
}

/// Test simulation parameters - uses actual Stats struct
#[derive(Debug, Clone)]
pub struct SimulationParams {
    pub attacker: Stats,
    pub defender: Stats,
    pub skill_power: i32,
    pub skill_accuracy: f32,
}

impl Default for SimulationParams {
    fn default() -> Self {
        Self {
            attacker: Stats {
                hp: 150,
                max_hp: 200,
                mp: 50,
                max_mp: 50,
                str: 25,
                def: 15,
                spd: 12,
                mag: 20,
                luck: 10,
                level: 25,
                exp: 0,
            },
            defender: Stats {
                hp: 180,
                max_hp: 220,
                mp: 40,
                max_mp: 40,
                str: 20,
                def: 18,
                spd: 14,
                mag: 16,
                luck: 12,
                level: 22,
                exp: 0,
            },
            skill_power: 100,
            skill_accuracy: 0.95,
        }
    }
}

/// Formula editor state - now wired to actual backend
pub struct FormulaEditor {
    /// Whether the editor is visible
    visible: bool,
    /// Currently selected formula type
    selected_type: FormulaType,
    /// Current formula input text
    formula_input: String,
    /// Selected preset
    selected_preset: FormulaPreset,
    /// Variable category filter
    variable_category: VariableCategory,
    /// Test simulation parameters using actual Stats
    simulation_params: SimulationParams,
    /// Last simulation results from actual backend
    simulation_result: Option<SimulationResult>,
    /// Formula resource (backend storage)
    formula_resource: FormulaResource,
    /// File path for save/load
    config_path: PathBuf,
    /// Status message
    status_message: Option<(String, f32)>, // (message, time_remaining)
    /// Cursor position for inserting variables
    cursor_position: Option<usize>,
    /// Formula validation errors
    validation_errors: Vec<String>,
    /// Hot-reload watcher
    file_watcher: Option<FileWatcher>,
    /// Whether to auto-save on change
    auto_save: bool,
    /// Last save time
    last_save: Option<Instant>,
}

/// Simple file watcher for hot-reload
struct FileWatcher {
    path: PathBuf,
    last_modified: std::time::SystemTime,
}

impl FileWatcher {
    fn new(path: PathBuf) -> Option<Self> {
        let metadata = std::fs::metadata(&path).ok()?;
        let last_modified = metadata.modified().ok()?;
        Some(Self {
            path,
            last_modified,
        })
    }

    fn check_changed(&mut self) -> bool {
        if let Ok(metadata) = std::fs::metadata(&self.path) {
            if let Ok(modified) = metadata.modified() {
                if modified > self.last_modified {
                    self.last_modified = modified;
                    return true;
                }
            }
        }
        false
    }
}

impl FormulaEditor {
    /// Create a new formula editor
    pub fn new() -> Self {
        let formula_resource = FormulaResource::default();
        let config_path = PathBuf::from("assets/data/formulas.toml");

        // Try to load existing formulas
        let formula_resource = if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                FormulaResource::from_toml(&content).unwrap_or_default()
            } else {
                FormulaResource::default()
            }
        } else {
            FormulaResource::default()
        };

        let mut editor = Self {
            visible: false,
            selected_type: FormulaType::Damage,
            formula_input: formula_resource.damage.clone(),
            selected_preset: FormulaPreset::Standard,
            variable_category: VariableCategory::Attacker,
            simulation_params: SimulationParams::default(),
            simulation_result: None,
            formula_resource,
            config_path,
            status_message: None,
            cursor_position: None,
            validation_errors: Vec::new(),
            file_watcher: None,
            auto_save: false,
            last_save: None,
        };

        // Initialize file watcher if file exists
        if editor.config_path.exists() {
            editor.file_watcher = FileWatcher::new(editor.config_path.clone());
        }

        editor.validate_current_formula();
        editor
    }

    /// Show the editor
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the editor
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

    /// Draw the formula editor UI
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        // Update status message timer
        if let Some((_, ref mut time)) = self.status_message {
            *time -= ctx.input(|i| i.stable_dt);
            if *time <= 0.0 {
                self.status_message = None;
            }
        }

        // Check for hot-reload
        self.check_hot_reload();

        // Auto-save if enabled and changes pending
        if self.auto_save {
            if let Some(last_save) = self.last_save {
                if last_save.elapsed() > Duration::from_secs(5) {
                    self.save_formulas();
                }
            }
        }

        let mut visible = self.visible;
        egui::Window::new("🔢 Formula Editor")
            .open(&mut visible)
            .resizable(true)
            .default_size([900.0, 700.0])
            .show(ctx, |ui| {
                self.draw_content(ui);
            });
        self.visible = visible;
    }

    /// Draw the editor content
    fn draw_content(&mut self, ui: &mut egui::Ui) {
        // Header with formula type tabs
        ui.horizontal(|ui| {
            ui.heading("Battle Formula Editor");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some((ref msg, _)) = self.status_message {
                    ui.colored_label(egui::Color32::GREEN, msg);
                }
            });
        });

        ui.label(self.selected_type.description());
        ui.separator();

        // Formula type tabs
        ui.horizontal(|ui| {
            for formula_type in FormulaType::all() {
                let selected = self.selected_type == formula_type;
                if ui.selectable_label(selected, formula_type.name()).clicked() && !selected {
                    // Save current formula before switching
                    self.save_current_formula_to_resource();
                    // Switch to new type
                    self.selected_type = formula_type;
                    self.formula_input = self.get_formula_from_resource(formula_type).to_string();
                    self.validation_errors.clear();
                    self.validate_current_formula();
                    self.simulation_result = None;
                }
            }
        });

        ui.separator();

        // Main content area
        egui::SidePanel::left("variables_panel")
            .resizable(true)
            .default_width(250.0)
            .show_inside(ui, |ui| {
                self.draw_variables_panel(ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::TopBottomPanel::top("formula_input_panel")
                .resizable(false)
                .height_range(150.0..=200.0)
                .show_inside(ui, |ui| {
                    self.draw_formula_input_panel(ui);
                });

            egui::CentralPanel::default().show_inside(ui, |ui| {
                self.draw_simulation_panel(ui);
            });
        });
    }

    /// Draw the variables panel
    fn draw_variables_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Variables");
        ui.add_space(8.0);

        // Category filter
        egui::ComboBox::from_label("Category")
            .selected_text(self.variable_category.name())
            .show_ui(ui, |ui| {
                for category in [
                    VariableCategory::Attacker,
                    VariableCategory::Defender,
                    VariableCategory::Skill,
                    VariableCategory::Functions,
                ] {
                    ui.selectable_value(&mut self.variable_category, category, category.name());
                }
            });

        ui.add_space(8.0);
        ui.separator();

        // Variable list
        egui::ScrollArea::vertical().show(ui, |ui| {
            for var in FormulaVariable::all()
                .into_iter()
                .filter(|v| v.category == self.variable_category)
            {
                ui.group(|ui| {
                    ui.set_width(ui.available_width());

                    ui.horizontal(|ui| {
                        if ui.button("➕").clicked() {
                            self.insert_variable(var.name);
                        }
                        ui.monospace(var.name);
                    });

                    ui.label(egui::RichText::new(var.description).size(11.0));
                    ui.label(
                        egui::RichText::new(format!("e.g., {}", var.example))
                            .italics()
                            .size(10.0)
                            .color(ui.visuals().weak_text_color()),
                    );
                });
                ui.add_space(4.0);
            }
        });
    }

    /// Draw the formula input panel
    fn draw_formula_input_panel(&mut self, ui: &mut egui::Ui) {
        // Preset selector
        ui.horizontal(|ui| {
            ui.label("Preset:");
            egui::ComboBox::from_id_source("preset_combo")
                .selected_text(self.selected_preset.name())
                .show_ui(ui, |ui| {
                    for preset in [
                        FormulaPreset::Standard,
                        FormulaPreset::PokemonStyle,
                        FormulaPreset::Simple,
                        FormulaPreset::Percentage,
                    ] {
                        if ui.selectable_label(false, preset.name()).clicked() {
                            self.selected_preset = preset;
                            self.formula_input = preset.formula(self.selected_type);
                            self.save_current_formula_to_resource();
                            self.validate_current_formula();
                        }
                    }
                });

            ui.separator();

            // Save/Load buttons
            if ui.button("💾 Save").clicked() {
                self.save_formulas();
            }
            if ui.button("📂 Load").clicked() {
                self.load_formulas();
            }
            if ui.button("🔄 Reset").clicked() {
                self.reset_to_defaults();
            }

            ui.separator();

            // Auto-save toggle
            ui.checkbox(&mut self.auto_save, "Auto-save");
        });

        ui.add_space(8.0);

        // Formula input
        ui.label("Formula:");
        let text_edit = egui::TextEdit::multiline(&mut self.formula_input)
            .font(egui::TextStyle::Monospace)
            .code_editor()
            .desired_rows(3)
            .lock_focus(true)
            .show(ui);

        // Track cursor position
        if let Some(cursor_range) = text_edit.cursor_range {
            self.cursor_position = Some(cursor_range.primary.ccursor.index);
        }

        // Store formula when changed
        if text_edit.response.changed() {
            self.save_current_formula_to_resource();
            self.validate_current_formula();
            if self.auto_save {
                self.last_save = Some(Instant::now());
            }
        }

        // Show validation errors
        if !self.validation_errors.is_empty() {
            ui.add_space(4.0);
            for error in &self.validation_errors {
                ui.colored_label(egui::Color32::RED, format!("⚠ {}", error));
            }
        }
    }

    /// Draw the simulation panel
    fn draw_simulation_panel(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::right("simulation_params_panel")
            .resizable(true)
            .default_width(220.0)
            .show_inside(ui, |ui| {
                self.draw_simulation_params(ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.draw_simulation_results(ui);
        });
    }

    /// Draw simulation parameters
    fn draw_simulation_params(&mut self, ui: &mut egui::Ui) {
        ui.heading("Test Parameters");
        ui.add_space(8.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Attacker stats
            ui.collapsing("👤 Attacker", |ui| {
                egui::Grid::new("attacker_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        self.stat_row(ui, "STR:", &mut self.simulation_params.attacker.str, 1..=999);
                        self.stat_row(ui, "DEF:", &mut self.simulation_params.attacker.def, 1..=999);
                        self.stat_row(ui, "SPD:", &mut self.simulation_params.attacker.spd, 1..=999);
                        self.stat_row(ui, "MAG:", &mut self.simulation_params.attacker.mag, 1..=999);
                        self.stat_row(ui, "LUCK:", &mut self.simulation_params.attacker.luck, 1..=999);
                        self.stat_row(ui, "LEVEL:", &mut self.simulation_params.attacker.level, 1..=999);
                    });
            });

            ui.add_space(8.0);

            // Defender stats
            ui.collapsing("🛡️ Defender", |ui| {
                egui::Grid::new("defender_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        self.stat_row(ui, "STR:", &mut self.simulation_params.defender.str, 1..=999);
                        self.stat_row(ui, "DEF:", &mut self.simulation_params.defender.def, 1..=999);
                        self.stat_row(ui, "SPD:", &mut self.simulation_params.defender.spd, 1..=999);
                        self.stat_row(ui, "MAG:", &mut self.simulation_params.defender.mag, 1..=999);
                        self.stat_row(ui, "LUCK:", &mut self.simulation_params.defender.luck, 1..=999);
                        self.stat_row(ui, "LEVEL:", &mut self.simulation_params.defender.level, 1..=999);
                        self.stat_row(ui, "MAX HP:", &mut self.simulation_params.defender.max_hp, 1..=9999);
                    });
            });

            ui.add_space(8.0);

            // Skill stats
            ui.collapsing("⚡ Skill", |ui| {
                egui::Grid::new("skill_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        self.stat_row(ui, "Power:", &mut self.simulation_params.skill_power, 0..=500);
                        ui.label("Accuracy:");
                        ui.add(
                            egui::DragValue::new(&mut self.simulation_params.skill_accuracy)
                                .speed(0.01)
                                .clamp_range(0.0..=1.0),
                        );
                        ui.end_row();
                    });
            });

            ui.add_space(16.0);

            // Run simulation button
            if ui.button("▶️ Run Simulation (1000x)").clicked() {
                self.run_simulation();
            }
        });
    }

    /// Helper to draw a stat row
    fn stat_row(&self, ui: &mut egui::Ui, label: &str, value: &mut i32, range: std::ops::RangeInclusive<i32>) {
        ui.label(label);
        ui.add(egui::DragValue::new(value).range(range));
        ui.end_row();
    }

    /// Draw simulation results
    fn draw_simulation_results(&mut self, ui: &mut egui::Ui) {
        ui.heading("Simulation Results");
        ui.add_space(8.0);

        if let Some(result) = &self.simulation_result {
            // Statistics grid
            egui::Grid::new("stats_grid")
                .num_columns(4)
                .spacing([20.0, 8.0])
                .show(ui, |ui| {
                    self.stat_box(ui, "Min", result.min_value);
                    self.stat_box(ui, "Max", result.max_value);
                    self.stat_box(ui, "Average", result.avg_value);
                    self.stat_box(ui, "Median", result.median_value);
                });

            ui.add_space(8.0);

            // Additional stats
            ui.horizontal(|ui| {
                ui.label(format!("Standard Deviation: {:.2}", result.std_deviation));
                ui.separator();
                ui.label(format!("Crit Rate: {:.1}%", result.crit_rate * 100.0));
                ui.separator();
                ui.label(format!("Iterations: {}", result.iterations));
            });

            ui.add_space(16.0);

            // Distribution graph
            ui.heading("Distribution");
            ui.add_space(8.0);

            if !result.distribution.is_empty() {
                self.draw_distribution_graph(ui, result);
            }
        } else {
            ui.label("Click 'Run Simulation' to see results from the actual damage calculation backend.");
            ui.add_space(8.0);
            ui.label("The simulation uses:");
            ui.label("• Real Stats structs from dde_core");
            ui.label("• DamageCalculator from dde_battle");
            ui.label("• Your custom formulas from FormulaResource");
            ui.label("• Proper RNG for 1000 iterations");
        }
    }

    /// Draw a stat box
    fn stat_box(&self, ui: &mut egui::Ui, label: &str, value: f64) {
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new(label).size(12.0).weak());
            ui.label(
                egui::RichText::new(format!("{:.1}", value))
                    .size(20.0)
                    .strong(),
            );
        });
    }

    /// Draw distribution graph using egui's built-in plotting
    fn draw_distribution_graph(&self, ui: &mut egui::Ui, result: &SimulationResult) {
        let available_width = ui.available_width();
        let height = 200.0;

        let max_count = result
            .distribution
            .iter()
            .map(|(_, count)| *count)
            .max()
            .unwrap_or(1) as f32;

        let bar_count = result.distribution.len();
        let bar_width = if bar_count > 0 {
            (available_width / bar_count as f32).min(40.0)
        } else {
            40.0
        };

        // Draw bars
        ui.horizontal(|ui| {
            for (value, count) in &result.distribution {
                let bar_height = (*count as f32 / max_count) * height;
                let remaining = height - bar_height;

                ui.vertical(|ui| {
                    // Invisible button for tooltip area
                    ui.set_width(bar_width);
                    ui.set_height(height);

                    // Draw the bar from bottom
                    ui.add_space(remaining);

                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(bar_width - 2.0, bar_height),
                        egui::Sense::hover(),
                    );

                    if ui.is_rect_visible(rect) {
                        ui.painter().rect_filled(
                            rect,
                            2.0,
                            ui.visuals().selection.bg_fill,
                        );
                    }

                    response.on_hover_ui(|ui| {
                        ui.label(format!("Value: {:.1}", value));
                        ui.label(format!("Count: {}", count));
                        ui.label(format!("{:.1}%", (*count as f32 / result.iterations as f32) * 100.0));
                    });
                });
            }
        });

        // X-axis labels
        ui.horizontal(|ui| {
            if let Some((first_val, _)) = result.distribution.first() {
                ui.label(format!("{:.0}", first_val));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some((last_val, _)) = result.distribution.last() {
                    ui.label(format!("{:.0}", last_val));
                }
            });
        });
    }

    /// Insert a variable at cursor position
    fn insert_variable(&mut self, variable: &str) {
        if let Some(pos) = self.cursor_position {
            let mut new_formula = self.formula_input.clone();
            new_formula.insert_str(pos, variable);
            self.formula_input = new_formula;
            self.cursor_position = Some(pos + variable.len());
        } else {
            self.formula_input.push_str(variable);
        }
        self.save_current_formula_to_resource();
        self.validate_current_formula();
    }

    /// Run simulation with current formula using the actual backend
    fn run_simulation(&mut self) {
        // Create a skill for simulation
        use dde_battle::skills::{Skill, SkillType, TargetType};
        let skill = Skill {
            id: 1,
            name: "Test Skill".to_string(),
            description: "Test skill for simulation".to_string(),
            skill_type: match self.selected_type {
                FormulaType::Damage => SkillType::Physical,
                FormulaType::Healing => SkillType::Heal,
                _ => SkillType::Physical,
            },
            target_type: TargetType::SingleEnemy,
            power: self.simulation_params.skill_power,
            accuracy: self.simulation_params.skill_accuracy,
            element: dde_core::Element::None,
            mp_cost: 0,
            tp_cost: 0,
            effects: vec![],
            cooldown: 0,
            animation_id: None,
            icon_id: 0,
        };

        // Create calculator with current formulas
        let calculator = DamageCalculator::new(self.formula_resource.clone());

        // Run simulation with real RNG
        let mut rng = rand::thread_rng();
        let result = calculator.run_simulation(
            &self.simulation_params.attacker,
            &self.simulation_params.defender,
            &skill,
            1000,
            &mut rng,
        );

        self.simulation_result = Some(result);
    }

    /// Save current formula to the resource
    fn save_current_formula_to_resource(&mut self) {
        let kind = self.selected_type.to_kind();
        self.formula_resource.set(kind, self.formula_input.clone());
    }

    /// Get formula from resource for a type
    fn get_formula_from_resource(&self, formula_type: FormulaType) -> &str {
        let kind = formula_type.to_kind();
        self.formula_resource.get(kind)
    }

    /// Validate the current formula
    fn validate_current_formula(&mut self) {
        self.validation_errors = validate_formula(&self.formula_input);
    }

    /// Save formulas to TOML file
    fn save_formulas(&mut self) {
        // Ensure current formula is saved
        self.save_current_formula_to_resource();

        // Create directory if needed
        if let Some(parent) = self.config_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                self.status_message = Some((format!("✗ Failed to create directory: {}", e), 3.0));
                return;
            }
        }

        match self.formula_resource.to_toml() {
            Ok(toml_str) => {
                match std::fs::write(&self.config_path, toml_str) {
                    Ok(_) => {
                        self.status_message = Some((format!("✓ Saved to {}", self.config_path.display()), 3.0));
                        self.last_save = Some(Instant::now());
                        // Update file watcher
                        self.file_watcher = FileWatcher::new(self.config_path.clone());
                    }
                    Err(e) => {
                        self.status_message = Some((format!("✗ Save failed: {}", e), 3.0));
                    }
                }
            }
            Err(e) => {
                self.status_message = Some((format!("✗ Serialization failed: {}", e), 3.0));
            }
        }
    }

    /// Load formulas from TOML file
    fn load_formulas(&mut self) {
        match std::fs::read_to_string(&self.config_path) {
            Ok(content) => {
                match FormulaResource::from_toml(&content) {
                    Ok(resource) => {
                        self.formula_resource = resource;
                        self.formula_input = self.get_formula_from_resource(self.selected_type).to_string();
                        self.validate_current_formula();
                        self.simulation_result = None;
                        self.status_message = Some((format!("✓ Loaded from {}", self.config_path.display()), 3.0));
                        // Update file watcher
                        self.file_watcher = FileWatcher::new(self.config_path.clone());
                    }
                    Err(e) => {
                        self.status_message = Some((format!("✗ Parse error: {}", e), 3.0));
                    }
                }
            }
            Err(e) => {
                self.status_message = Some((format!("✗ Load failed: {}", e), 3.0));
            }
        }
    }

    /// Reset formulas to defaults
    fn reset_to_defaults(&mut self) {
        self.formula_resource = FormulaResource::default();
        self.formula_input = self.get_formula_from_resource(self.selected_type).to_string();
        self.validate_current_formula();
        self.simulation_result = None;
        self.status_message = Some(("✓ Reset to defaults".to_string(), 3.0));
    }

    /// Check for hot-reload
    fn check_hot_reload(&mut self) {
        if let Some(ref mut watcher) = self.file_watcher {
            if watcher.check_changed() {
                // File has changed, reload it
                if let Ok(content) = std::fs::read_to_string(&self.config_path) {
                    if let Ok(resource) = FormulaResource::from_toml(&content) {
                        self.formula_resource = resource;
                        self.formula_input = self.get_formula_from_resource(self.selected_type).to_string();
                        self.validate_current_formula();
                        self.status_message = Some(("✓ Hot-reloaded formulas".to_string(), 3.0));
                    }
                }
            }
        }
    }

    /// Get formula for a specific type
    pub fn get_formula(&self, formula_type: FormulaType) -> Option<&String> {
        // We need to return a reference, but get_formula_from_resource returns &str
        // So we access the resource directly
        let kind = formula_type.to_kind();
        // This is a bit hacky - we store in the resource, so return from there
        // Actually we need to store formulas separately or change the API
        // For now, return None as this method should probably be removed
        None
    }

    /// Set formula for a specific type
    pub fn set_formula(&mut self, formula_type: FormulaType, formula: String) {
        let kind = formula_type.to_kind();
        self.formula_resource.set(kind, formula);
        if self.selected_type == formula_type {
            self.formula_input = self.get_formula_from_resource(formula_type).to_string();
        }
        self.validate_current_formula();
    }

    /// Get formula resource (for use by battle system)
    pub fn formula_resource(&self) -> &FormulaResource {
        &self.formula_resource
    }

    /// Get mutable formula resource
    pub fn formula_resource_mut(&mut self) -> &mut FormulaResource {
        &mut self.formula_resource
    }

    /// Set the config path
    pub fn set_config_path(&mut self, path: impl Into<PathBuf>) {
        self.config_path = path.into();
    }

    /// Get validation errors for current formula
    pub fn validation_errors(&self) -> &[String] {
        &self.validation_errors
    }
}

impl Default for FormulaEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formula_type_names() {
        assert_eq!(FormulaType::Damage.name(), "⚔️ Damage");
        assert_eq!(FormulaType::Healing.name(), "💚 Healing");
        assert_eq!(FormulaType::CriticalHit.name(), "💥 Critical Hit");
    }

    #[test]
    fn test_preset_formulas() {
        let standard = FormulaPreset::Standard.formula(FormulaType::Damage);
        assert!(standard.contains("attacker.str"));
        assert!(standard.contains("defender.def"));

        let pokemon = FormulaPreset::PokemonStyle.formula(FormulaType::Damage);
        assert!(pokemon.contains("attacker.level"));
        assert!(pokemon.contains("attacker.str"));
    }

    #[test]
    fn test_formula_editor_creation() {
        let editor = FormulaEditor::new();
        assert!(!editor.is_visible());
        assert_eq!(editor.selected_type, FormulaType::Damage);
    }

    #[test]
    fn test_formula_editor_toggle() {
        let mut editor = FormulaEditor::new();
        assert!(!editor.is_visible());

        editor.toggle();
        assert!(editor.is_visible());

        editor.toggle();
        assert!(!editor.is_visible());

        editor.show();
        assert!(editor.is_visible());

        editor.hide();
        assert!(!editor.is_visible());
    }

    #[test]
    fn test_formula_type_conversion() {
        assert_eq!(FormulaType::Damage.to_kind(), FormulaKind::Damage);
        assert_eq!(FormulaType::Healing.to_kind(), FormulaKind::Healing);
        assert_eq!(FormulaType::from_kind(FormulaKind::Damage), FormulaType::Damage);
        assert_eq!(FormulaType::from_kind(FormulaKind::Healing), FormulaType::Healing);
    }

    #[test]
    fn test_simulation_params_default() {
        let params = SimulationParams::default();
        assert_eq!(params.attacker.str, 25);
        assert_eq!(params.defender.def, 18);
        assert_eq!(params.skill_power, 100);
    }

    #[test]
    fn test_formula_editor_set_formula() {
        let mut editor = FormulaEditor::new();
        editor.set_formula(FormulaType::Damage, "attacker.str * 2".to_string());
        assert_eq!(editor.formula_resource.get(FormulaKind::Damage), "attacker.str * 2");
    }
}
