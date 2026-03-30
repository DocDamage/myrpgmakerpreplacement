//! Formation Editor UI
//!
//! Visual editor for configuring battle formations with drag-and-drop positioning.
//! Supports 5x3 grid layout (5 columns, 3 rows) typical for RPGs.
//!
//! Layout:
//! - Left: Party member list with portraits
//! - Center: Formation grid (drag-drop target)
//! - Right: Selected position properties
//! - Bottom: Preset selector, Save/Load buttons

use dde_battle::formation::{
    Formation, FormationError, FormationLayout, FormationModifiers, FormationPosition,
    FormationSlot, FormationSlotAssignment, FormationSystem, SerializableFormation,
    SerializableFormationSlot,
};
use dde_core::Entity;
use dde_db::{Database, DbError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Maximum grid dimensions
pub const GRID_COLS: usize = 5;
pub const GRID_ROWS: usize = 3;

/// Grid position (column, row)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GridPosition {
    pub col: u8,
    pub row: u8,
}

impl GridPosition {
    /// Create a new grid position
    pub fn new(col: u8, row: u8) -> Self {
        Self {
            col: col.min((GRID_COLS - 1) as u8),
            row: row.min((GRID_ROWS - 1) as u8),
        }
    }

    /// Convert to formation position and slot index
    pub fn to_formation_slot(&self) -> (FormationPosition, u8) {
        let position = match self.row {
            0 => FormationPosition::FrontRow,
            1 => FormationPosition::MiddleRow, // Extended support
            _ => FormationPosition::BackRow,
        };
        (position, self.col)
    }

    /// Create from formation position and slot
    pub fn from_formation_slot(position: FormationPosition, slot_index: u8) -> Self {
        let row = match position {
            FormationPosition::FrontRow => 0,
            FormationPosition::MiddleRow => 1,
            FormationPosition::BackRow => 2,
        };
        Self::new(slot_index, row)
    }

    /// Get row name for display
    pub fn row_name(&self) -> &'static str {
        match self.row {
            0 => "Front",
            1 => "Middle",
            _ => "Back",
        }
    }
}

impl Default for GridPosition {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Party member data for display
#[derive(Debug, Clone)]
pub struct PartyMember {
    pub entity: Entity,
    pub name: String,
    pub portrait_path: Option<String>,
    pub level: u32,
    pub class: String,
    pub hp_percent: f32,
    pub mp_percent: f32,
}

impl PartyMember {
    /// Create a new party member
    pub fn new(entity: Entity, name: impl Into<String>) -> Self {
        Self {
            entity,
            name: name.into(),
            portrait_path: None,
            level: 1,
            class: "Adventurer".to_string(),
            hp_percent: 100.0,
            mp_percent: 100.0,
        }
    }

    /// Set portrait path
    pub fn with_portrait(mut self, path: impl Into<String>) -> Self {
        self.portrait_path = Some(path.into());
        self
    }

    /// Set level
    pub fn with_level(mut self, level: u32) -> Self {
        self.level = level;
        self
    }

    /// Set class
    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.class = class.into();
        self
    }
}

/// Enemy formation data
#[derive(Debug, Clone)]
pub struct EnemyFormation {
    pub name: String,
    pub positions: Vec<(GridPosition, EnemySlot)>,
}

/// Enemy slot data
#[derive(Debug, Clone)]
pub struct EnemySlot {
    pub enemy_template_id: u32,
    pub name: String,
    pub level: u32,
}

/// Formation preset with metadata
#[derive(Debug, Clone)]
pub struct FormationPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: &'static str,
    pub layout: FormationLayout,
}

impl FormationPreset {
    /// Get all built-in presets
    pub fn all_presets() -> Vec<Self> {
        vec![
            Self {
                id: "balanced".to_string(),
                name: "Balanced".to_string(),
                description: "Equal distribution across rows".to_string(),
                icon: "⚖️",
                layout: FormationLayout::Balanced,
            },
            Self {
                id: "aggressive".to_string(),
                name: "Aggressive".to_string(),
                description: "Focus on front row for maximum damage".to_string(),
                icon: "⚔️",
                layout: FormationLayout::Aggressive,
            },
            Self {
                id: "defensive".to_string(),
                name: "Defensive".to_string(),
                description: "Protect the party in back row".to_string(),
                icon: "🛡️",
                layout: FormationLayout::Defensive,
            },
            Self {
                id: "wedge".to_string(),
                name: "Wedge".to_string(),
                description: "Triangle formation, leader front".to_string(),
                icon: "🔺",
                layout: FormationLayout::Custom,
            },
            Self {
                id: "spearhead".to_string(),
                name: "Spearhead".to_string(),
                description: "Offensive V formation".to_string(),
                icon: "🔱",
                layout: FormationLayout::Custom,
            },
            Self {
                id: "shieldwall".to_string(),
                name: "Shield Wall".to_string(),
                description: "Defensive line formation".to_string(),
                icon: "🧱",
                layout: FormationLayout::Custom,
            },
        ]
    }

    /// Apply preset to formation
    pub fn apply(&self, formation: &mut Formation, party: &[Entity]) {
        match self.layout {
            FormationLayout::Custom => {
                // Apply custom preset layouts
                match self.id.as_str() {
                    "wedge" => Self::apply_wedge(formation, party),
                    "spearhead" => Self::apply_spearhead(formation, party),
                    "shieldwall" => Self::apply_shieldwall(formation, party),
                    _ => {}
                }
            }
            _ => {
                formation.apply_layout(self.layout, party);
            }
        }
    }

    /// Apply wedge formation (leader front center, others behind in triangle)
    fn apply_wedge(formation: &mut Formation, party: &[Entity]) {
        formation.clear();
        let positions = vec![
            (FormationPosition::FrontRow, 2), // Leader center front
            (FormationPosition::FrontRow, 1), // Left flank
            (FormationPosition::FrontRow, 3), // Right flank
            (FormationPosition::BackRow, 2),  // Support center back
        ];

        for (i, &entity) in party.iter().enumerate() {
            if let Some(&(pos, slot)) = positions.get(i) {
                formation.assign(entity, pos, slot);
            }
        }
        formation.default_layout = FormationLayout::Custom;
    }

    /// Apply spearhead formation (aggressive V shape)
    fn apply_spearhead(formation: &mut Formation, party: &[Entity]) {
        formation.clear();
        let positions = vec![
            (FormationPosition::FrontRow, 2), // Leader tip
            (FormationPosition::BackRow, 1),  // Left wing
            (FormationPosition::BackRow, 3),  // Right wing
            (FormationPosition::BackRow, 2),  // Support
        ];

        for (i, &entity) in party.iter().enumerate() {
            if let Some(&(pos, slot)) = positions.get(i) {
                formation.assign(entity, pos, slot);
            }
        }
        formation.default_layout = FormationLayout::Custom;
    }

    /// Apply shieldwall formation (defensive line)
    fn apply_shieldwall(formation: &mut Formation, party: &[Entity]) {
        formation.clear();
        // Everyone in back row for protection
        for (i, &entity) in party.iter().enumerate() {
            let slot = (i as u8).min(4);
            formation.assign(entity, FormationPosition::BackRow, slot);
        }
        formation.default_layout = FormationLayout::Custom;
    }
}

/// Position properties for the properties panel
#[derive(Debug, Clone)]
pub struct PositionProperties {
    pub position: GridPosition,
    pub modifiers: FormationModifiers,
    pub description: String,
    pub recommended_classes: Vec<String>,
}

impl PositionProperties {
    /// Get properties for a grid position
    pub fn for_position(position: GridPosition) -> Self {
        let (formation_pos, _) = position.to_formation_slot();
        let modifiers = formation_pos.modifiers();

        let (description, recommended) = match position.row {
            0 => (
                "Front row: Deal +10% damage, take +20% damage".to_string(),
                vec!["Warrior".to_string(), "Paladin".to_string(), "Berserker".to_string()],
            ),
            1 => (
                "Middle row: Balanced position".to_string(),
                vec!["Ranger".to_string(), "Bard".to_string(), "Rogue".to_string()],
            ),
            _ => (
                "Back row: Deal -15% physical, take -25% damage, -20% phys accuracy".to_string(),
                vec!["Mage".to_string(), "Healer".to_string(), "Summoner".to_string()],
            ),
        };

        Self {
            position,
            modifiers,
            description,
            recommended_classes: recommended,
        }
    }
}

/// Drag state for drag-and-drop
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragState {
    None,
    DraggingFromPartyList { entity: Entity },
    DraggingFromGrid { entity: Entity, from_position: GridPosition },
}

/// Formation editor UI state
pub struct FormationEditor {
    /// Whether panel is visible
    visible: bool,
    /// Current formation being edited
    formation: Formation,
    /// Party members
    party_members: Vec<PartyMember>,
    /// Enemy formation (for reference)
    enemy_formation: Option<EnemyFormation>,
    /// Currently selected grid position
    selected_position: Option<GridPosition>,
    /// Currently selected preset
    selected_preset: String,
    /// Drag state
    drag_state: DragState,
    /// Hover position
    hover_position: Option<GridPosition>,
    /// Database connection
    db: Option<Database>,
    /// Formation name
    formation_name: String,
    /// Formation ID (for saving)
    formation_id: Option<u64>,
    /// Unsaved changes flag
    has_unsaved_changes: bool,
    /// Status message
    status_message: Option<(String, f64)>,
    /// Custom presets saved by user
    custom_presets: Vec<FormationPreset>,
    /// Show delete confirmation
    show_delete_confirm: bool,
    /// Pending delete preset ID
    pending_delete_preset: Option<String>,
}

impl FormationEditor {
    /// Create a new formation editor
    pub fn new() -> Self {
        Self {
            visible: false,
            formation: Formation::new_custom(),
            party_members: Vec::new(),
            enemy_formation: None,
            selected_position: None,
            selected_preset: "balanced".to_string(),
            drag_state: DragState::None,
            hover_position: None,
            db: None,
            formation_name: "New Formation".to_string(),
            formation_id: None,
            has_unsaved_changes: false,
            status_message: None,
            custom_presets: Vec::new(),
            show_delete_confirm: false,
            pending_delete_preset: None,
        }
    }

    /// Create with database connection
    pub fn with_database(mut self, db: Database) -> Self {
        self.db = Some(db);
        // Try to load formations from database
        let _ = self.load_formations_list();
        self
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

    /// Set formation
    pub fn set_formation(&mut self, formation: Formation) {
        self.formation = formation;
        self.has_unsaved_changes = false;
    }

    /// Get current formation
    pub fn formation(&self) -> &Formation {
        &self.formation
    }

    /// Get mutable formation
    pub fn formation_mut(&mut self) -> &mut Formation {
        &mut self.formation
    }

    /// Set party members
    pub fn set_party_members(&mut self, members: Vec<PartyMember>) {
        self.party_members = members;
        // Update formation with current party
        let party: Vec<Entity> = self.party_members.iter().map(|m| m.entity).collect();
        self.formation = Formation::from_layout(FormationLayout::Balanced, &party);
        self.has_unsaved_changes = true;
    }

    /// Set enemy formation (for reference display)
    pub fn set_enemy_formation(&mut self, enemy: EnemyFormation) {
        self.enemy_formation = Some(enemy);
    }

    /// Get party members
    pub fn party_members(&self) -> &[PartyMember] {
        &self.party_members
    }

    /// Check if has unsaved changes
    pub fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }

    /// Apply a preset to the current formation
    pub fn apply_preset(&mut self, preset_id: &str) {
        let preset = FormationPreset::all_presets()
            .into_iter()
            .chain(self.custom_presets.clone().into_iter())
            .find(|p| p.id == preset_id);

        if let Some(preset) = preset {
            let party: Vec<Entity> = self.party_members.iter().map(|m| m.entity).collect();
            preset.apply(&mut self.formation, &party);
            self.selected_preset = preset_id.to_string();
            self.has_unsaved_changes = true;
            self.set_status_message(format!("Applied {} preset", preset.name));
        }
    }

    /// Move entity to a grid position
    pub fn move_entity_to_position(
        &mut self,
        entity: Entity,
        position: GridPosition,
    ) -> Result<(), FormationError> {
        let (formation_pos, slot_index) = position.to_formation_slot();

        // Check if slot is occupied by someone else
        if let Some(existing) = self.get_entity_at_position(position) {
            if existing != entity {
                // Swap positions if dragging from another slot
                if let DragState::DraggingFromGrid { from_position, .. } = self.drag_state {
                    let (from_pos, from_slot) = from_position.to_formation_slot();
                    // Move existing entity to the source position
                    self.formation.move_to_slot(existing, from_pos, from_slot)?;
                }
            }
        }

        // Check if entity is in party
        if !self.party_members.iter().any(|m| m.entity == entity) {
            return Err(FormationError::EntityNotInParty);
        }

        self.formation.assign(entity, formation_pos, slot_index);
        self.has_unsaved_changes = true;
        Ok(())
    }

    /// Remove entity from formation
    pub fn remove_entity(&mut self, entity: Entity) {
        self.formation.remove(entity);
        self.has_unsaved_changes = true;
    }

    /// Get entity at a grid position
    pub fn get_entity_at_position(&self, position: GridPosition) -> Option<Entity> {
        let (formation_pos, slot_index) = position.to_formation_slot();
        self.formation
            .slots
            .iter()
            .find(|s| s.position == formation_pos && s.slot_index == slot_index)
            .map(|s| s.entity)
    }

    /// Get position of an entity
    pub fn get_entity_position(&self, entity: Entity) -> Option<GridPosition> {
        self.formation.find_slot(entity).map(|slot| {
            GridPosition::from_formation_slot(slot.position, slot.slot_index)
        })
    }

    /// Get member info for an entity
    pub fn get_member(&self, entity: Entity) -> Option<&PartyMember> {
        self.party_members.iter().find(|m| m.entity == entity)
    }

    /// Clear the formation
    pub fn clear_formation(&mut self) {
        self.formation.clear();
        self.has_unsaved_changes = true;
    }

    /// Set status message
    fn set_status_message(&mut self, message: String) {
        self.status_message = Some((message, 3.0));
    }

    /// Update status message timer
    fn update(&mut self, dt: f32) {
        if let Some((_, ref mut time)) = self.status_message {
            *time -= dt as f64;
            if *time <= 0.0 {
                self.status_message = None;
            }
        }
    }
}

// ============================================================================
// DATABASE INTEGRATION
// ============================================================================

/// Serializable formation data for database storage
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FormationDbRecord {
    pub id: u64,
    pub name: String,
    pub slots: Vec<SerializableFormationSlot>,
    pub default_layout: FormationLayout,
    pub created_at: i64,
    pub updated_at: i64,
}

impl FormationEditor {
    /// Save current formation to database
    pub fn save_to_database(&mut self, name: Option<&str>) -> Result<(), DbError> {
        let db = self.db.as_ref().ok_or_else(|| {
            DbError::InvalidData("No database connection".to_string())
        })?;

        if let Some(name) = name {
            self.formation_name = name.to_string();
        }

        let serializable = SerializableFormation::from_formation(&self.formation);
        let slots_json = serde_json::to_string(&serializable.slots)
            .map_err(|e| DbError::InvalidData(e.to_string()))?;
        let layout_str = format!("{:?}", serializable.default_layout);
        let now = chrono::Utc::now().timestamp_millis();

        if let Some(id) = self.formation_id {
            // Update existing
            db.conn().execute(
                "UPDATE formations 
                 SET name = ?1, slots_json = ?2, default_layout = ?3, updated_at = ?4
                 WHERE formation_id = ?5",
                (&self.formation_name, &slots_json, &layout_str, &now, &(id as i64)),
            )?;
        } else {
            // Insert new
            db.conn().execute(
                "INSERT INTO formations (name, slots_json, default_layout, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                (&self.formation_name, &slots_json, &layout_str, &now, &now),
            )?;
            self.formation_id = Some(db.conn().last_insert_rowid() as u64);
        }

        self.has_unsaved_changes = false;
        self.set_status_message(format!("Saved '{}'", self.formation_name));
        tracing::info!("Saved formation '{}' to database", self.formation_name);
        Ok(())
    }

    /// Load formation from database
    pub fn load_from_database(&mut self, formation_id: u64) -> Result<(), DbError> {
        let db = self.db.as_ref().ok_or_else(|| {
            DbError::InvalidData("No database connection".to_string())
        })?;

        let record: FormationDbRecord = db.conn().query_row(
            "SELECT formation_id, name, slots_json, default_layout, created_at, updated_at
             FROM formations WHERE formation_id = ?1",
            [formation_id as i64],
            |row| {
                let slots_json: String = row.get(2)?;
                let slots: Vec<SerializableFormationSlot> = serde_json::from_str(&slots_json)
                    .unwrap_or_default();
                let layout_str: String = row.get(3)?;
                let layout = parse_formation_layout(&layout_str);

                Ok(FormationDbRecord {
                    id: row.get::<_, i64>(0)? as u64,
                    name: row.get(1)?,
                    slots,
                    default_layout: layout,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )?;

        // Convert to formation using entity mapping (entities need to be resolved)
        let serializable = SerializableFormation {
            slots: record.slots,
            default_layout: record.default_layout,
        };

        // Create a mapping from stored entity bits to current party entities
        let party: Vec<Entity> = self.party_members.iter().map(|m| m.entity).collect();
        self.formation = map_formation_to_party(&serializable, &party);

        self.formation_name = record.name;
        self.formation_id = Some(record.id);
        self.has_unsaved_changes = false;

        self.set_status_message(format!("Loaded '{}'", self.formation_name));
        Ok(())
    }

    /// List all saved formations
    pub fn list_saved_formations(&self) -> Result<Vec<(u64, String, i64)>, DbError> {
        let db = self.db.as_ref().ok_or_else(|| {
            DbError::InvalidData("No database connection".to_string())
        })?;

        let mut stmt = db.conn().prepare(
            "SELECT formation_id, name, updated_at FROM formations ORDER BY updated_at DESC"
        )?;

        let formations: Vec<(u64, String, i64)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)? as u64,
                    row.get(1)?,
                    row.get(2)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(formations)
    }

    /// Delete a formation from database
    pub fn delete_formation(&mut self, formation_id: u64) -> Result<bool, DbError> {
        let db = self.db.as_ref().ok_or_else(|| {
            DbError::InvalidData("No database connection".to_string())
        })?;

        let rows = db.conn().execute(
            "DELETE FROM formations WHERE formation_id = ?1",
            [formation_id as i64],
        )?;

        if rows > 0 && self.formation_id == Some(formation_id) {
            self.formation_id = None;
            self.has_unsaved_changes = false;
        }

        Ok(rows > 0)
    }

    /// Load formations list from database
    fn load_formations_list(&mut self) -> Result<(), DbError> {
        // This can be used to load custom presets or recent formations
        Ok(())
    }
}

/// Parse formation layout from string
fn parse_formation_layout(s: &str) -> FormationLayout {
    match s.to_lowercase().as_str() {
        "aggressive" => FormationLayout::Aggressive,
        "defensive" => FormationLayout::Defensive,
        "custom" => FormationLayout::Custom,
        _ => FormationLayout::Balanced,
    }
}

/// Map a stored formation to current party entities
fn map_formation_to_party(
    serializable: &SerializableFormation,
    party: &[Entity],
) -> Formation {
    // Try to match slots to current party by index
    let mut slots = Vec::new();

    for (i, slot) in serializable.slots.iter().enumerate() {
        if let Some(&entity) = party.get(i) {
            slots.push(FormationSlotAssignment {
                entity,
                position: slot.position,
                slot_index: slot.slot_index,
            });
        }
    }

    Formation {
        slots,
        default_layout: serializable.default_layout,
    }
}

// ============================================================================
// UI RENDERING
// ============================================================================

impl FormationEditor {
    /// Draw the formation editor window
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        self.update(ctx.input(|i| i.stable_dt));

        let mut visible = self.visible;
        egui::Window::new("🛡️ Formation Editor")
            .open(&mut visible)
            .resizable(true)
            .default_size([900.0, 650.0])
            .show(ctx, |ui| {
                self.draw_content(ui);
            });
        self.visible = visible;
    }

    /// Draw main content
    fn draw_content(&mut self, ui: &mut egui::Ui) {
        // Header
        ui.horizontal(|ui| {
            ui.heading("Battle Formation");
            ui.separator();

            // Formation name editor
            ui.label("Name:");
            ui.text_edit_singleline(&mut self.formation_name);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Status indicator
                if self.has_unsaved_changes {
                    ui.colored_label(egui::Color32::YELLOW, "● Modified");
                } else {
                    ui.colored_label(egui::Color32::GREEN, "✓ Saved");
                }

                // Status message
                if let Some((ref msg, _)) = self.status_message {
                    ui.label(egui::RichText::new(msg).color(egui::Color32::GREEN));
                }
            });
        });

        ui.separator();

        // Main 3-column layout
        egui::SidePanel::left("party_list")
            .default_width(200.0)
            .show_inside(ui, |ui| {
                self.draw_party_list(ui);
            });

        egui::SidePanel::right("properties_panel")
            .default_width(220.0)
            .show_inside(ui, |ui| {
                self.draw_properties_panel(ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.draw_formation_grid(ui);
        });

        // Bottom bar
        ui.separator();
        self.draw_bottom_bar(ui);
    }

    /// Draw party member list (left panel)
    fn draw_party_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("Party Members");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for member in &self.party_members {
                let is_in_formation = self.formation.contains(member.entity);
                let row_color = if is_in_formation {
                    ui.visuals().widgets.active.bg_fill
                } else {
                    ui.visuals().widgets.inactive.bg_fill
                };

                let response = ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.set_min_height(60.0);

                    // Portrait placeholder or image
                    ui.horizontal(|ui| {
                        // Portrait box
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(48.0, 48.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(
                            rect,
                            4.0,
                            ui.visuals().widgets.noninteractive.bg_fill,
                        );

                        // Member info
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&member.name).strong());
                            ui.label(format!("Lv.{} {}", member.level, member.class));

                            // HP/MP bars
                            ui.horizontal(|ui| {
                                ui.colored_label(egui::Color32::RED, "HP");
                                ui.add(
                                    egui::ProgressBar::new(member.hp_percent / 100.0)
                                        .desired_width(60.0),
                                );
                            });
                        });
                    });
                });

                // Drag source
                let response = response.response;
                if response.drag_started() {
                    self.drag_state = DragState::DraggingFromPartyList {
                        entity: member.entity,
                    };
                }

                // Show context menu on right-click
                response.context_menu(|ui| {
                    if is_in_formation {
                        if ui.button("Remove from formation").clicked() {
                            self.remove_entity(member.entity);
                            ui.close_menu();
                        }
                    } else {
                        ui.label("Not in formation");
                    }
                });

                ui.add_space(4.0);
            }
        });

        // Handle drag cancel
        if ui.input(|i| i.pointer.any_released()) {
            if matches!(self.drag_state, DragState::DraggingFromPartyList { .. }) {
                self.drag_state = DragState::None;
            }
        }
    }

    /// Draw formation grid (center panel)
    fn draw_formation_grid(&mut self, ui: &mut egui::Ui) {
        ui.heading("Formation Grid");
        ui.label("Drag party members to positions");
        ui.add_space(8.0);

        // Enemy formation preview (top)
        if let Some(ref enemy) = self.enemy_formation {
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new("Enemy Formation").weak());
                self.draw_enemy_formation_mini(ui, enemy);
            });
            ui.add_space(16.0);
        }

        // Player formation grid
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(250.0);

            // Draw grid rows from back to front
            for row in (0..GRID_ROWS).rev() {
                let row_name = match row {
                    0 => "Front Row",
                    1 => "Middle Row",
                    _ => "Back Row",
                };
                let row_icon = match row {
                    0 => "⚔️",
                    1 => "🎯",
                    _ => "🛡️",
                };

                ui.horizontal(|ui| {
                    ui.label(format!("{} {}", row_icon, row_name));
                    ui.separator();

                    // Grid cells
                    for col in 0..GRID_COLS {
                        let position = GridPosition::new(col as u8, row as u8);
                        self.draw_grid_cell(ui, position);
                        ui.add_space(4.0);
                    }
                });

                if row > 0 {
                    ui.add_space(8.0);
                }
            }
        });

        // Handle drag end
        if ui.input(|i| i.pointer.any_released()) {
            if let DragState::DraggingFromGrid { entity, from_position } = self.drag_state {
                // Check if dropped on a valid position
                if let Some(hover_pos) = self.hover_position {
                    if hover_pos != from_position {
                        let _ = self.move_entity_to_position(entity, hover_pos);
                    }
                }
                self.drag_state = DragState::None;
                self.hover_position = None;
            }
        }
    }

    /// Draw a single grid cell
    fn draw_grid_cell(&mut self, ui: &mut egui::Ui, position: GridPosition) {
        let cell_size = egui::vec2(70.0, 70.0);
        let (rect, response) = ui.allocate_exact_size(cell_size, egui::Sense::click_and_drag());

        let entity = self.get_entity_at_position(position);
        let is_hovered = response.hovered();
        let is_dragged = response.dragged();

        // Track hover for drop target
        if is_hovered {
            self.hover_position = Some(position);
        }

        // Determine cell color
        let bg_color = if let Some(cell_entity) = entity {
            if matches!(self.drag_state, DragState::DraggingFromGrid { entity, .. } if entity == cell_entity)
            {
                ui.visuals().widgets.inactive.bg_fill // Dim dragged entity
            } else {
                ui.visuals().widgets.active.bg_fill
            }
        } else if is_hovered && !matches!(self.drag_state, DragState::None) {
            // Highlight valid drop target
            egui::Color32::from_rgb(60, 80, 100)
        } else {
            ui.visuals().widgets.noninteractive.bg_fill
        };

        // Draw cell background
        ui.painter().rect_filled(rect, 8.0, bg_color);
        ui.painter().rect_stroke(
            rect,
            8.0,
            egui::Stroke::new(
                if self.selected_position == Some(position) {
                    2.0
                } else {
                    1.0
                },
                if self.selected_position == Some(position) {
                    egui::Color32::YELLOW
                } else {
                    ui.visuals().widgets.noninteractive.fg_stroke.color
                },
            ),
        );

        // Draw cell content
        if let Some(e) = entity {
            if let Some(member) = self.get_member(e) {
                // Draw member info in cell
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{}", member.name.chars().next().unwrap_or('?')),
                    egui::FontId::proportional(20.0),
                    ui.visuals().text_color(),
                );

                // Draw level
                ui.painter().text(
                    rect.right_bottom() - egui::vec2(8.0, 8.0),
                    egui::Align2::RIGHT_BOTTOM,
                    format!("Lv{}", member.level),
                    egui::FontId::proportional(10.0),
                    ui.visuals().weak_text_color(),
                );

                // Handle drag start from grid
                if is_dragged && matches!(self.drag_state, DragState::None) {
                    self.drag_state = DragState::DraggingFromGrid {
                        entity: e,
                        from_position: position,
                    };
                }
            }
        } else {
            // Empty cell indicator
            let indicator = if is_hovered && !matches!(self.drag_state, DragState::None) {
                "+"
            } else {
                "·"
            };
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                indicator,
                egui::FontId::proportional(24.0),
                ui.visuals().weak_text_color(),
            );
        }

        // Handle click to select
        if response.clicked() {
            self.selected_position = Some(position);
        }

        // Handle drop
        if response.dropped() {
            if let DragState::DraggingFromPartyList { entity } = self.drag_state {
                let _ = self.move_entity_to_position(entity, position);
                self.drag_state = DragState::None;
            }
        }
    }

    /// Draw enemy formation mini preview
    fn draw_enemy_formation_mini(&self, ui: &mut egui::Ui, enemy: &EnemyFormation) {
        ui.horizontal(|ui| {
            for (pos, slot) in &enemy.positions {
                let size = egui::vec2(40.0, 40.0);
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());

                // Enemy cell background (reddish)
                ui.painter().rect_filled(
                    rect,
                    4.0,
                    egui::Color32::from_rgb(80, 40, 40),
                );

                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &slot.name.chars().next().unwrap_or('?'),
                    egui::FontId::proportional(14.0),
                    egui::Color32::LIGHT_RED,
                );
            }
        });
    }

    /// Draw properties panel (right side)
    fn draw_properties_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Position Info");
        ui.separator();

        if let Some(position) = self.selected_position {
            let props = PositionProperties::for_position(position);

            ui.label(egui::RichText::new(format!("{} Row", position.row_name())).strong());
            ui.label(format!("Column {}", position.col + 1));
            ui.add_space(8.0);

            ui.label(&props.description);
            ui.add_space(8.0);

            // Modifiers
            ui.label(egui::RichText::new("Modifiers:").strong());
            let mods = &props.modifiers;
            if mods.damage_dealt_mult != 1.0 {
                let percent = ((mods.damage_dealt_mult - 1.0) * 100.0) as i32;
                let (icon, color) = if percent > 0 {
                    ("▲", egui::Color32::GREEN)
                } else {
                    ("▼", egui::Color32::RED)
                };
                ui.colored_label(color, format!("{} Damage Dealt: {:+}%", icon, percent));
            }
            if mods.damage_taken_mult != 1.0 {
                let percent = ((mods.damage_taken_mult - 1.0) * 100.0) as i32;
                let (icon, color) = if percent > 0 {
                    ("▲", egui::Color32::RED)
                } else {
                    ("▼", egui::Color32::GREEN)
                };
                ui.colored_label(color, format!("{} Damage Taken: {:+}%", icon, percent));
            }
            if mods.physical_accuracy_mult != 1.0 {
                let percent = ((mods.physical_accuracy_mult - 1.0) * 100.0) as i32;
                ui.label(format!("Physical Accuracy: {:+}%", percent));
            }

            ui.add_space(8.0);

            // Entity info if occupied
            if let Some(entity) = self.get_entity_at_position(position) {
                if let Some(member) = self.get_member(entity) {
                    ui.separator();
                    ui.label(egui::RichText::new("Occupied by:").strong());
                    ui.label(&member.name);
                    ui.label(format!("Level {} {}", member.level, member.class));

                    if ui.button("Remove from Position").clicked() {
                        self.remove_entity(entity);
                    }
                }
            }

            ui.add_space(8.0);

            // Recommended classes
            ui.label(egui::RichText::new("Recommended Classes:").weak());
            for class in &props.recommended_classes {
                ui.label(format!("• {}", class));
            }
        } else {
            ui.label("Select a position to view details");
            ui.add_space(16.0);

            // General formation info
            ui.label(egui::RichText::new("Formation Summary:").strong());
            ui.label(format!("Members: {}", self.formation.len()));
            ui.label(format!("Front Row: {}", self.formation.front_row_entities().len()));
            ui.label(format!("Back Row: {}", self.formation.back_row_entities().len()));
        }
    }

    /// Draw bottom bar with presets and save/load
    fn draw_bottom_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Preset selector
            ui.label("Preset:");

            egui::ComboBox::from_id_source("preset_selector")
                .selected_text(self.selected_preset_name())
                .show_ui(ui, |ui| {
                    for preset in FormationPreset::all_presets() {
                        if ui
                            .selectable_label(
                                self.selected_preset == preset.id,
                                format!("{} {}", preset.icon, preset.name),
                            )
                            .on_hover_text(&preset.description)
                            .clicked()
                        {
                            self.selected_preset = preset.id.clone();
                            self.apply_preset(&preset.id);
                        }
                    }
                });

            if ui.button("Apply").clicked() {
                let preset_id = self.selected_preset.clone();
                self.apply_preset(&preset_id);
            }

            ui.separator();

            // Formation actions
            if ui.button("Clear").on_hover_text("Clear all positions").clicked() {
                self.clear_formation();
            }

            if ui.button("Auto-Fill").on_hover_text("Fill empty positions with party members").clicked() {
                self.auto_fill_formation();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Database actions
                if self.db.is_some() {
                    if ui
                        .button("💾 Save")
                        .on_hover_text("Save formation to database")
                        .clicked()
                    {
                        if let Err(e) = self.save_to_database(None) {
                            self.set_status_message(format!("Save failed: {}", e));
                        }
                    }

                    if ui
                        .button("📂 Load")
                        .on_hover_text("Load formation from database")
                        .clicked()
                    {
                        // Would open a load dialog - for now just load first available
                        match self.list_saved_formations() {
                            Ok(formations) if !formations.is_empty() => {
                                if let Err(e) = self.load_from_database(formations[0].0) {
                                    self.set_status_message(format!("Load failed: {}", e));
                                }
                            }
                            _ => self.set_status_message("No saved formations".to_string()),
                        }
                    }
                }
            });
        });
    }

    /// Get selected preset display name
    fn selected_preset_name(&self) -> String {
        FormationPreset::all_presets()
            .into_iter()
            .chain(self.custom_presets.clone().into_iter())
            .find(|p| p.id == self.selected_preset)
            .map(|p| format!("{} {}", p.icon, p.name))
            .unwrap_or_else(|| "Custom".to_string())
    }

    /// Auto-fill empty positions with party members
    fn auto_fill_formation(&mut self) {
        let unassigned: Vec<Entity> = self
            .party_members
            .iter()
            .filter(|m| !self.formation.contains(m.entity))
            .map(|m| m.entity)
            .collect();

        for entity in unassigned {
            // Find first empty position
            for row in 0..GRID_ROWS {
                for col in 0..GRID_COLS {
                    let pos = GridPosition::new(col as u8, row as u8);
                    if self.get_entity_at_position(pos).is_none() {
                        if self.move_entity_to_position(entity, pos).is_ok() {
                            break;
                        }
                    }
                }
            }
        }
    }
}

impl Default for FormationEditor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// FORMATION EDITOR INTERFACE
// ============================================================================

/// Interface for the game to provide party data and receive formation updates
pub trait FormationEditorInterface {
    /// Get current party members
    fn get_party_members(&self) -> Vec<PartyMember>;

    /// Get enemy formation for current context
    fn get_enemy_formation(&self) -> Option<EnemyFormation>;

    /// Called when formation is updated
    fn on_formation_changed(&mut self, formation: &Formation);

    /// Get current formation from game state
    fn get_current_formation(&self) -> Option<Formation>;
}

// ============================================================================
// DATABASE MIGRATION
// ============================================================================

/// Add formation table migration
pub fn add_formation_table_migration(conn: &rusqlite::Connection) -> Result<(), DbError> {
    conn.execute_batch(
        r#"
        -- Formation presets and saved formations
        CREATE TABLE IF NOT EXISTS formations (
            formation_id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            slots_json TEXT NOT NULL DEFAULT '[]',
            default_layout TEXT NOT NULL DEFAULT 'Balanced',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_formations_updated ON formations(updated_at);

        -- Formation presets (built-in and custom)
        CREATE TABLE IF NOT EXISTS formation_presets (
            preset_id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            icon TEXT,
            layout_type TEXT NOT NULL,
            slots_json TEXT,
            is_custom BOOLEAN NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL
        );
        "#,
    )?;
    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use dde_core::World;

    fn create_test_party(world: &mut World) -> Vec<PartyMember> {
        vec![
            PartyMember::new(world.spawn(()), "Warrior")
                .with_class("Warrior")
                .with_level(10),
            PartyMember::new(world.spawn(()), "Mage")
                .with_class("Mage")
                .with_level(10),
            PartyMember::new(world.spawn(()), "Healer")
                .with_class("Cleric")
                .with_level(10),
            PartyMember::new(world.spawn(()), "Rogue")
                .with_class("Rogue")
                .with_level(10),
        ]
    }

    #[test]
    fn test_formation_editor_new() {
        let editor = FormationEditor::new();
        assert!(!editor.is_visible());
        assert!(editor.party_members().is_empty());
    }

    #[test]
    fn test_grid_position() {
        let pos = GridPosition::new(2, 1);
        assert_eq!(pos.col, 2);
        assert_eq!(pos.row, 1);

        // Test clamping
        let pos = GridPosition::new(10, 10);
        assert_eq!(pos.col, (GRID_COLS - 1) as u8);
        assert_eq!(pos.row, (GRID_ROWS - 1) as u8);
    }

    #[test]
    fn test_formation_presets() {
        let presets = FormationPreset::all_presets();
        assert!(!presets.is_empty());

        let balanced = presets.iter().find(|p| p.id == "balanced");
        assert!(balanced.is_some());
    }

    #[test]
    fn test_position_properties() {
        let front = PositionProperties::for_position(GridPosition::new(0, 0));
        assert!(!front.description.is_empty());
        assert!(front.modifiers.damage_dealt_mult > 1.0); // Front row deals more damage

        let back = PositionProperties::for_position(GridPosition::new(0, 2));
        assert!(back.modifiers.damage_taken_mult < 1.0); // Back row takes less damage
    }

    #[test]
    fn test_party_member_creation() {
        let mut world = World::new();
        let entity = world.spawn(());
        let member = PartyMember::new(entity, "Test")
            .with_level(5)
            .with_class("Knight")
            .with_portrait("portraits/knight.png");

        assert_eq!(member.name, "Test");
        assert_eq!(member.level, 5);
        assert_eq!(member.class, "Knight");
        assert_eq!(member.portrait_path, Some("portraits/knight.png".to_string()));
    }

    #[test]
    fn test_preset_apply() {
        let mut world = World::new();
        let party = create_test_party(&mut world);
        let party_entities: Vec<Entity> = party.iter().map(|m| m.entity).collect();

        let mut formation = Formation::new_custom();

        let preset = FormationPreset::all_presets()
            .into_iter()
            .find(|p| p.id == "balanced")
            .unwrap();
        preset.apply(&mut formation, &party_entities);

        assert!(!formation.is_empty());
    }

    #[test]
    fn test_preset_wedge() {
        let mut world = World::new();
        let party = create_test_party(&mut world);
        let party_entities: Vec<Entity> = party.iter().map(|m| m.entity).collect();

        let mut formation = Formation::new_custom();

        let preset = FormationPreset::all_presets()
            .into_iter()
            .find(|p| p.id == "wedge")
            .unwrap();
        preset.apply(&mut formation, &party_entities);

        // Wedge should have leader in front center
        let leader_slot = formation.find_slot(party_entities[0]);
        assert!(leader_slot.is_some());
    }

    #[test]
    fn test_formation_editor_set_party() {
        let mut editor = FormationEditor::new();
        let mut world = World::new();
        let party = create_test_party(&mut world);

        editor.set_party_members(party.clone());
        assert_eq!(editor.party_members().len(), 4);
        assert!(editor.has_unsaved_changes);
    }

    #[test]
    fn test_formation_editor_move_entity() {
        let mut editor = FormationEditor::new();
        let mut world = World::new();
        let party = create_test_party(&mut world);

        editor.set_party_members(party.clone());
        let entity = party[0].entity;

        // Move to front center
        let pos = GridPosition::new(2, 0);
        assert!(editor.move_entity_to_position(entity, pos).is_ok());

        // Check position
        let found_pos = editor.get_entity_position(entity);
        assert_eq!(found_pos, Some(pos));
    }

    #[test]
    fn test_drag_state_transitions() {
        let mut editor = FormationEditor::new();
        assert!(matches!(editor.drag_state, DragState::None));

        let mut world = World::new();
        let entity = world.spawn(());

        editor.drag_state = DragState::DraggingFromPartyList { entity };
        assert!(matches!(editor.drag_state, DragState::DraggingFromPartyList { .. }));
    }

    #[test]
    fn test_parse_formation_layout() {
        assert_eq!(parse_formation_layout("Balanced"), FormationLayout::Balanced);
        assert_eq!(parse_formation_layout("aggressive"), FormationLayout::Aggressive);
        assert_eq!(parse_formation_layout("DEFENSIVE"), FormationLayout::Defensive);
        assert_eq!(parse_formation_layout("custom"), FormationLayout::Custom);
        assert_eq!(parse_formation_layout("unknown"), FormationLayout::Balanced);
    }

    #[test]
    fn test_map_formation_to_party() {
        let mut world = World::new();
        let old_party: Vec<Entity> = (0..4).map(|_| world.spawn(())).collect();
        let new_party: Vec<Entity> = (0..4).map(|_| world.spawn(())).collect();

        let mut formation = Formation::from_layout(FormationLayout::Balanced, &old_party);
        let serializable = SerializableFormation::from_formation(&formation);

        let mapped = map_formation_to_party(&serializable, &new_party);
        assert_eq!(mapped.len(), 4);
        // All entities should be from new_party
        for slot in &mapped.slots {
            assert!(new_party.contains(&slot.entity));
        }
    }
}
