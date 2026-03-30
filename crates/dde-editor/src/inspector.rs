//! Component Inspector Panel
//!
//! Provides UI for inspecting and editing entity components during live play mode.

use dde_core::{components::*, Entity, World};
use serde_json::Value;
use std::collections::HashMap;

/// Inspector panel for viewing and editing entity components
pub struct ComponentInspector {
    /// Currently inspected entity
    selected_entity: Option<Entity>,
    /// Expanded state for component sections
    expanded_components: HashMap<String, bool>,
    /// Component values being edited
    editing_values: HashMap<String, serde_json::Value>,
    /// Whether in edit mode
    edit_mode: bool,
    /// Filter for component list
    component_filter: String,
    /// Show only editable components
    show_editable_only: bool,
    /// Component modification callback
    on_modify: Option<Box<dyn Fn(Entity, String, serde_json::Value) + Send + Sync>>,
}

impl Default for ComponentInspector {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentInspector {
    /// Create a new component inspector
    pub fn new() -> Self {
        Self {
            selected_entity: None,
            expanded_components: HashMap::new(),
            editing_values: HashMap::new(),
            edit_mode: false,
            component_filter: String::new(),
            show_editable_only: false,
            on_modify: None,
        }
    }

    /// Set the entity to inspect
    pub fn inspect(&mut self, entity: Entity, world: &World) {
        self.selected_entity = Some(entity);
        self.refresh_values(world);
    }

    /// Clear the current selection
    pub fn clear(&mut self) {
        self.selected_entity = None;
        self.editing_values.clear();
    }

    /// Get the selected entity
    pub fn selected_entity(&self) -> Option<Entity> {
        self.selected_entity
    }

    /// Check if an entity is selected
    pub fn has_selection(&self) -> bool {
        self.selected_entity.is_some()
    }

    /// Toggle edit mode
    pub fn toggle_edit_mode(&mut self) {
        self.edit_mode = !self.edit_mode;
    }

    /// Set edit mode
    pub fn set_edit_mode(&mut self, enabled: bool) {
        self.edit_mode = enabled;
    }

    /// Check if in edit mode
    pub fn is_edit_mode(&self) -> bool {
        self.edit_mode
    }

    /// Set component filter
    pub fn set_filter(&mut self, filter: impl Into<String>) {
        self.component_filter = filter.into();
    }

    /// Get component filter
    pub fn filter(&self) -> &str {
        &self.component_filter
    }

    /// Toggle showing only editable components
    pub fn toggle_editable_only(&mut self) {
        self.show_editable_only = !self.show_editable_only;
    }

    /// Set modification callback
    pub fn on_modify<F>(&mut self, callback: F)
    where
        F: Fn(Entity, String, serde_json::Value) + Send + Sync + 'static,
    {
        self.on_modify = Some(Box::new(callback));
    }

    /// Refresh editing values from world
    pub fn refresh_values(&mut self, world: &World) {
        self.editing_values.clear();

        let Some(entity) = self.selected_entity else {
            return;
        };

        // Read component values from world
        let components = self.read_components(entity, world);
        self.editing_values = components;
    }

    /// Read components from world
    fn read_components(&self, entity: Entity, world: &World) -> HashMap<String, Value> {
        let mut components = HashMap::new();

        // Position
        if let Ok(pos) = world.query::<&Position>().get(entity) {
            if let Ok(json) = serde_json::to_value(*pos) {
                components.insert("Position".to_string(), json);
            }
        }

        // SubPosition
        if let Ok(sub_pos) = world.query::<&SubPosition>().get(entity) {
            if let Ok(json) = serde_json::to_value(*sub_pos) {
                components.insert("SubPosition".to_string(), json);
            }
        }

        // Name
        if let Ok(name) = world.query::<&Name>().get(entity) {
            if let Ok(json) = serde_json::to_value(name.clone()) {
                components.insert("Name".to_string(), json);
            }
        }

        // Stats
        if let Ok(stats) = world.query::<&Stats>().get(entity) {
            if let Ok(json) = serde_json::to_value(*stats) {
                components.insert("Stats".to_string(), json);
            }
        }

        // EntityKind
        if let Ok(kind) = world.query::<&EntityKindComp>().get(entity) {
            if let Ok(json) = serde_json::to_value(*kind) {
                components.insert("EntityKind".to_string(), json);
            }
        }

        // Inventory
        if let Ok(inventory) = world.query::<&Inventory>().get(entity) {
            if let Ok(json) = serde_json::to_value(inventory.clone()) {
                components.insert("Inventory".to_string(), json);
            }
        }

        // Equipment
        if let Ok(equipment) = world.query::<&Equipment>().get(entity) {
            if let Ok(json) = serde_json::to_value(*equipment) {
                components.insert("Equipment".to_string(), json);
            }
        }

        // Biome
        if let Ok(biome) = world.query::<&Biome>().get(entity) {
            if let Ok(json) = serde_json::to_value(*biome) {
                components.insert("Biome".to_string(), json);
            }
        }

        // Passability
        if let Ok(passability) = world.query::<&Passability>().get(entity) {
            if let Ok(json) = serde_json::to_value(*passability) {
                components.insert("Passability".to_string(), json);
            }
        }

        // Interactable
        if let Ok(interactable) = world.query::<&Interactable>().get(entity) {
            if let Ok(json) = serde_json::to_value(*interactable) {
                components.insert("Interactable".to_string(), json);
            }
        }

        // StatusEffects
        if let Ok(status_effects) = world.query::<&StatusEffects>().get(entity) {
            if let Ok(json) = serde_json::to_value(status_effects.clone()) {
                components.insert("StatusEffects".to_string(), json);
            }
        }

        // Respawn
        if let Ok(respawn) = world.query::<&Respawn>().get(entity) {
            if let Ok(json) = serde_json::to_value(*respawn) {
                components.insert("Respawn".to_string(), json);
            }
        }

        // CameraTarget
        if let Ok(camera_target) = world.query::<&CameraTarget>().get(entity) {
            if let Ok(json) = serde_json::to_value(*camera_target) {
                components.insert("CameraTarget".to_string(), json);
            }
        }

        // MapId
        if let Ok(map_id) = world.query::<&MapId>().get(entity) {
            if let Ok(json) = serde_json::to_value(*map_id) {
                components.insert("MapId".to_string(), json);
            }
        }

        // FactionId
        if let Ok(faction_id) = world.query::<&FactionId>().get(entity) {
            if let Ok(json) = serde_json::to_value(*faction_id) {
                components.insert("FactionId".to_string(), json);
            }
        }

        components
    }

    /// Draw the inspector UI
    pub fn draw(&mut self, ctx: &egui::Context, world: &mut World) {
        let Some(entity) = self.selected_entity else {
            egui::Window::new("Inspector").show(ctx, |ui| {
                ui.label("No entity selected");
                ui.label("Click an entity to inspect");
            });
            return;
        };

        egui::Window::new(format!("Inspector - {:?}", entity)).show(ctx, |ui| {
            self.draw_inspector_contents(ui, entity, world);
        });
    }

    /// Draw inspector contents
    fn draw_inspector_contents(&mut self, ui: &mut egui::Ui, entity: Entity, world: &mut World) {
        // Header with entity info
        ui.horizontal(|ui| {
            ui.heading("Entity");
            ui.monospace(format!("{:?}", entity));

            if ui.button("Deselect").clicked() {
                self.clear();
                return;
            }

            if ui.button("Refresh").clicked() {
                self.refresh_values(world);
            }
        });

        ui.separator();

        // Edit mode toggle
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.edit_mode, "Edit Mode");

            ui.checkbox(&mut self.show_editable_only, "Editable Only");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label("Filter:");
                ui.text_edit_singleline(&mut self.component_filter);
            });
        });

        ui.separator();

        // Component list
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let components: Vec<_> = self.editing_values.keys().cloned().collect();

                for component_name in components {
                    // Apply filter
                    if !self.component_filter.is_empty()
                        && !component_name
                            .to_lowercase()
                            .contains(&self.component_filter.to_lowercase())
                    {
                        continue;
                    }

                    self.draw_component_section(ui, entity, &component_name);
                }
            });

        // Add component button (editor mode only)
        if self.edit_mode {
            ui.separator();
            ui.menu_button("➕ Add Component", |ui| {
                ui.label("Select component to add:");
                ui.separator();
                for component_name in Self::available_components() {
                    // Only show components the entity doesn't already have
                    if !self.has_component(component_name) {
                        if ui.button(component_name).clicked() {
                            if let Err(e) = self.add_component(entity, component_name, world) {
                                tracing::warn!("Failed to add component: {}", e);
                            }
                            ui.close_menu();
                        }
                    }
                }
            });
        }
    }

    /// Draw a single component section
    fn draw_component_section(&mut self, ui: &mut egui::Ui, entity: Entity, component_name: &str) {
        let is_expanded = self
            .expanded_components
            .entry(component_name.to_string())
            .or_insert(true);

        let header_response = ui.collapsing(component_name, |ui| {
            *is_expanded = true;

            if let Some(value) = self.editing_values.get(component_name) {
                self.draw_component_editor(ui, entity, component_name, value.clone());
            }

            // Remove button in edit mode
            if self.edit_mode {
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() - 60.0);
                    if ui.button("🗑 Remove").clicked() {
                        if let Err(e) = self.remove_component(entity, component_name, world) {
                            tracing::warn!("Failed to remove component: {}", e);
                        }
                    }
                });
            }
        });

        // Track expanded state from header response
        if header_response.finished {
            *is_expanded = header_response.body_response.is_some();
        }
    }

    /// Draw editor for a specific component
    fn draw_component_editor(
        &mut self,
        ui: &mut egui::Ui,
        entity: Entity,
        component_name: &str,
        value: serde_json::Value,
    ) {
        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    ui.horizontal(|ui| {
                        ui.label(&key);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            self.draw_value_editor(ui, entity, component_name, &key, val);
                        });
                    });
                }
            }
            Value::Array(arr) => {
                ui.label(format!("[{} items]", arr.len()));
                for (i, item) in arr.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("[{}]", i));
                        self.draw_value_editor(
                            ui,
                            entity,
                            component_name,
                            &i.to_string(),
                            item.clone(),
                        );
                    });
                }
            }
            _ => {
                self.draw_value_editor(ui, entity, component_name, "value", value);
            }
        }
    }

    /// Draw editor for a single value
    fn draw_value_editor(
        &mut self,
        ui: &mut egui::Ui,
        entity: Entity,
        component_name: &str,
        field_name: &str,
        value: serde_json::Value,
    ) {
        if !self.edit_mode {
            // Display only
            match &value {
                Value::Bool(b) => {
                    ui.label(if *b { "✓ true" } else { "✗ false" });
                }
                Value::Number(n) => {
                    ui.monospace(n.to_string());
                }
                Value::String(s) => {
                    ui.label(s.as_str());
                }
                _ => {
                    ui.monospace(format!("{:?}", value));
                }
            }
            return;
        }

        // Editable fields
        let mut modified = false;
        let mut new_value = value.clone();

        match value {
            Value::Bool(mut b) => {
                if ui.checkbox(&mut b, "").changed() {
                    new_value = Value::Bool(b);
                    modified = true;
                }
            }
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    let mut val = i as i32;
                    if ui.add(egui::DragValue::new(&mut val)).changed() {
                        new_value = Value::Number(val.into());
                        modified = true;
                    }
                } else if let Some(f) = n.as_f64() {
                    let mut val = f as f32;
                    if ui.add(egui::DragValue::new(&mut val).speed(0.1)).changed() {
                        new_value = Value::Number(
                            serde_json::Number::from_f64(val as f64).unwrap_or_else(|| 0.into()),
                        );
                        modified = true;
                    }
                }
            }
            Value::String(mut s) => {
                if ui.text_edit_singleline(&mut s).changed() {
                    new_value = Value::String(s);
                    modified = true;
                }
            }
            _ => {
                ui.monospace(format!("{:?}", value));
            }
        }

        if modified {
            self.on_value_modified(entity, component_name, field_name, new_value);
        }
    }

    /// Handle value modification
    fn on_value_modified(
        &mut self,
        entity: Entity,
        component_name: &str,
        field_name: &str,
        new_value: serde_json::Value,
    ) {
        tracing::debug!(
            "Modified {}.{} on {:?}: {:?}",
            component_name,
            field_name,
            entity,
            new_value
        );

        // Update local value
        if let Some(component) = self.editing_values.get_mut(component_name) {
            if let Some(obj) = component.as_object_mut() {
                obj.insert(field_name.to_string(), new_value.clone());
            }
        }

        // Notify callback
        if let Some(ref callback) = self.on_modify {
            callback(
                entity,
                format!("{}.{}", component_name, field_name),
                new_value,
            );
        }
    }

    /// Add a component to the entity
    pub fn add_component(
        &mut self,
        entity: Entity,
        component_name: &str,
        world: &mut World,
    ) -> Result<(), InspectorError> {
        use dde_core::components::*;
        
        // Insert the appropriate component based on name
        match component_name {
            "Position" => {
                world.insert_one(entity, Position::default()).map_err(|_| {
                    InspectorError::EntityNotFound(entity)
                })?;
            }
            "SubPosition" => {
                world.insert_one(entity, SubPosition::default()).map_err(|_| {
                    InspectorError::EntityNotFound(entity)
                })?;
            }
            "Name" => {
                world.insert_one(entity, Name::default()).map_err(|_| {
                    InspectorError::EntityNotFound(entity)
                })?;
            }
            "Stats" => {
                world.insert_one(entity, Stats::default()).map_err(|_| {
                    InspectorError::EntityNotFound(entity)
                })?;
            }
            "Inventory" => {
                world.insert_one(entity, Inventory::default()).map_err(|_| {
                    InspectorError::EntityNotFound(entity)
                })?;
            }
            "Equipment" => {
                world.insert_one(entity, Equipment::default()).map_err(|_| {
                    InspectorError::EntityNotFound(entity)
                })?;
            }
            "EntityKind" => {
                world.insert_one(entity, EntityKindComp::default()).map_err(|_| {
                    InspectorError::EntityNotFound(entity)
                })?;
            }
            "Biome" => {
                world.insert_one(entity, Biome::default()).map_err(|_| {
                    InspectorError::EntityNotFound(entity)
                })?;
            }
            "Passability" => {
                world.insert_one(entity, Passability::default()).map_err(|_| {
                    InspectorError::EntityNotFound(entity)
                })?;
            }
            "Interactable" => {
                world.insert_one(entity, Interactable::default()).map_err(|_| {
                    InspectorError::EntityNotFound(entity)
                })?;
            }
            "StatusEffects" => {
                world.insert_one(entity, StatusEffects::default()).map_err(|_| {
                    InspectorError::EntityNotFound(entity)
                })?;
            }
            "Respawn" => {
                world.insert_one(entity, Respawn::default()).map_err(|_| {
                    InspectorError::EntityNotFound(entity)
                })?;
            }
            "CameraTarget" => {
                world.insert_one(entity, CameraTarget::default()).map_err(|_| {
                    InspectorError::EntityNotFound(entity)
                })?;
            }
            "MapId" => {
                world.insert_one(entity, MapId::default()).map_err(|_| {
                    InspectorError::EntityNotFound(entity)
                })?;
            }
            "FactionId" => {
                world.insert_one(entity, FactionId::default()).map_err(|_| {
                    InspectorError::EntityNotFound(entity)
                })?;
            }
            "WorldStateComp" => {
                world.insert_one(entity, WorldStateComp::default()).map_err(|_| {
                    InspectorError::EntityNotFound(entity)
                })?;
            }
            "TilesetRef" => {
                world.insert_one(entity, TilesetRef::default()).map_err(|_| {
                    InspectorError::EntityNotFound(entity)
                })?;
            }
            _ => return Err(InspectorError::InvalidComponentType(component_name.to_string())),
        }
        
        tracing::info!("Added {} to {:?}", component_name, entity);
        
        // Refresh values after modification
        self.refresh_values(world);
        
        Ok(())
    }

    /// Remove a component from the entity
    pub fn remove_component(
        &mut self,
        entity: Entity,
        component_name: &str,
        world: &mut World,
    ) -> Result<(), InspectorError> {
        use dde_core::components::*;
        
        // Remove the appropriate component based on name
        match component_name {
            "Position" => {
                world.remove_one::<Position>(entity).map_err(|_| {
                    InspectorError::ComponentNotFound(component_name.to_string())
                })?;
            }
            "SubPosition" => {
                world.remove_one::<SubPosition>(entity).map_err(|_| {
                    InspectorError::ComponentNotFound(component_name.to_string())
                })?;
            }
            "Name" => {
                world.remove_one::<Name>(entity).map_err(|_| {
                    InspectorError::ComponentNotFound(component_name.to_string())
                })?;
            }
            "Stats" => {
                world.remove_one::<Stats>(entity).map_err(|_| {
                    InspectorError::ComponentNotFound(component_name.to_string())
                })?;
            }
            "Inventory" => {
                world.remove_one::<Inventory>(entity).map_err(|_| {
                    InspectorError::ComponentNotFound(component_name.to_string())
                })?;
            }
            "Equipment" => {
                world.remove_one::<Equipment>(entity).map_err(|_| {
                    InspectorError::ComponentNotFound(component_name.to_string())
                })?;
            }
            "EntityKind" => {
                world.remove_one::<EntityKindComp>(entity).map_err(|_| {
                    InspectorError::ComponentNotFound(component_name.to_string())
                })?;
            }
            "Biome" => {
                world.remove_one::<Biome>(entity).map_err(|_| {
                    InspectorError::ComponentNotFound(component_name.to_string())
                })?;
            }
            "Passability" => {
                world.remove_one::<Passability>(entity).map_err(|_| {
                    InspectorError::ComponentNotFound(component_name.to_string())
                })?;
            }
            "Interactable" => {
                world.remove_one::<Interactable>(entity).map_err(|_| {
                    InspectorError::ComponentNotFound(component_name.to_string())
                })?;
            }
            "StatusEffects" => {
                world.remove_one::<StatusEffects>(entity).map_err(|_| {
                    InspectorError::ComponentNotFound(component_name.to_string())
                })?;
            }
            "Respawn" => {
                world.remove_one::<Respawn>(entity).map_err(|_| {
                    InspectorError::ComponentNotFound(component_name.to_string())
                })?;
            }
            "CameraTarget" => {
                world.remove_one::<CameraTarget>(entity).map_err(|_| {
                    InspectorError::ComponentNotFound(component_name.to_string())
                })?;
            }
            "MapId" => {
                world.remove_one::<MapId>(entity).map_err(|_| {
                    InspectorError::ComponentNotFound(component_name.to_string())
                })?;
            }
            "FactionId" => {
                world.remove_one::<FactionId>(entity).map_err(|_| {
                    InspectorError::ComponentNotFound(component_name.to_string())
                })?;
            }
            "WorldStateComp" => {
                world.remove_one::<WorldStateComp>(entity).map_err(|_| {
                    InspectorError::ComponentNotFound(component_name.to_string())
                })?;
            }
            "TilesetRef" => {
                world.remove_one::<TilesetRef>(entity).map_err(|_| {
                    InspectorError::ComponentNotFound(component_name.to_string())
                })?;
            }
            _ => return Err(InspectorError::InvalidComponentType(component_name.to_string())),
        }
        
        tracing::info!("Removed {} from {:?}", component_name, entity);
        
        // Remove from editing values
        self.editing_values.remove(component_name);
        
        // Refresh values after modification
        self.refresh_values(world);
        
        Ok(())
    }

    /// Get component count
    pub fn component_count(&self) -> usize {
        self.editing_values.len()
    }

    /// Check if entity has a specific component
    pub fn has_component(&self, component_name: &str) -> bool {
        self.editing_values.contains_key(component_name)
    }

    /// Get available component types
    pub fn available_components() -> Vec<&'static str> {
        vec![
            "Position",
            "SubPosition",
            "Name",
            "Stats",
            "Inventory",
            "Equipment",
            "EntityKind",
            "Biome",
            "Passability",
            "Interactable",
            "StatusEffects",
            "Respawn",
            "CameraTarget",
            "MapId",
            "FactionId",
            "WorldStateComp",
            "TilesetRef",
        ]
    }
}

/// Inspector error types
#[derive(thiserror::Error, Debug)]
pub enum InspectorError {
    #[error("Entity not found: {0:?}")]
    EntityNotFound(Entity),

    #[error("Component not found: {0}")]
    ComponentNotFound(String),

    #[error("Cannot modify component: {0}")]
    ModificationFailed(String),

    #[error("Invalid component type: {0}")]
    InvalidComponentType(String),
}

/// Inspector panel for egui integration
pub struct InspectorPanel {
    inspector: ComponentInspector,
    position: InspectorPosition,
    size: dde_core::glam::Vec2,
}

/// Inspector panel position
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InspectorPosition {
    Right,
    Left,
    Floating,
}

impl Default for InspectorPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl InspectorPanel {
    /// Create a new inspector panel
    pub fn new() -> Self {
        Self {
            inspector: ComponentInspector::new(),
            position: InspectorPosition::Right,
            size: dde_core::glam::Vec2::new(350.0, 600.0),
        }
    }

    /// Set panel position
    pub fn set_position(&mut self, position: InspectorPosition) {
        self.position = position;
    }

    /// Set panel size
    pub fn set_size(&mut self, size: dde_core::glam::Vec2) {
        self.size = size;
    }

    /// Get mutable reference to inspector
    pub fn inspector_mut(&mut self) -> &mut ComponentInspector {
        &mut self.inspector
    }

    /// Get reference to inspector
    pub fn inspector(&self) -> &ComponentInspector {
        &self.inspector
    }

    /// Inspect an entity
    pub fn inspect(&mut self, entity: Entity, world: &World) {
        self.inspector.inspect(entity, world);
    }

    /// Draw the panel
    pub fn draw(&mut self, ctx: &egui::Context, world: &mut World) {
        match self.position {
            InspectorPosition::Right => {
                egui::SidePanel::right("inspector_panel")
                    .default_width(self.size.x)
                    .show(ctx, |_ui| {
                        self.inspector.draw(ctx, world);
                    });
            }
            InspectorPosition::Left => {
                egui::SidePanel::left("inspector_panel")
                    .default_width(self.size.x)
                    .show(ctx, |_ui| {
                        self.inspector.draw(ctx, world);
                    });
            }
            InspectorPosition::Floating => {
                self.inspector.draw(ctx, world);
            }
        }
    }
}

/// Entity list panel for selecting entities to inspect
pub struct EntityListPanel {
    filter: String,
    selected: Option<Entity>,
    show_all: bool,
    on_select: Option<Box<dyn Fn(Entity) + Send + Sync>>,
}

impl Default for EntityListPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityListPanel {
    /// Create a new entity list panel
    pub fn new() -> Self {
        Self {
            filter: String::new(),
            selected: None,
            show_all: true,
            on_select: None,
        }
    }

    /// Set selection callback
    pub fn on_select<F>(&mut self, callback: F)
    where
        F: Fn(Entity) + Send + Sync + 'static,
    {
        self.on_select = Some(Box::new(callback));
    }

    /// Draw the entity list
    pub fn draw(&mut self, ctx: &egui::Context, world: &World) {
        egui::Window::new("Entities").show(ctx, |ui| {
            // Filter input
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.text_edit_singleline(&mut self.filter);
            });

            ui.separator();

            // Entity list
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    for (entity, ()) in world.query::<()>().iter() {
                        // Apply filter
                        let name = world
                            .query::<&Name>()
                            .get(entity)
                            .map(|n| n.display.clone())
                            .unwrap_or_else(|_| format!("{:?}", entity));

                        if !self.filter.is_empty()
                            && !name.to_lowercase().contains(&self.filter.to_lowercase())
                        {
                            continue;
                        }

                        let is_selected = self.selected == Some(entity);
                        let response =
                            ui.selectable_label(is_selected, format!("{} ({:?})", name, entity));

                        if response.clicked() {
                            self.selected = Some(entity);
                            if let Some(ref callback) = self.on_select {
                                callback(entity);
                            }
                        }
                    }
                });
        });
    }

    /// Get selected entity
    pub fn selected(&self) -> Option<Entity> {
        self.selected
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selected = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspector_new() {
        let inspector = ComponentInspector::new();
        assert!(!inspector.has_selection());
        assert_eq!(inspector.component_count(), 0);
    }

    #[test]
    fn test_inspector_edit_mode() {
        let mut inspector = ComponentInspector::new();
        assert!(!inspector.is_edit_mode());

        inspector.toggle_edit_mode();
        assert!(inspector.is_edit_mode());

        inspector.set_edit_mode(false);
        assert!(!inspector.is_edit_mode());
    }

    #[test]
    fn test_inspector_filter() {
        let mut inspector = ComponentInspector::new();
        assert!(inspector.filter().is_empty());

        inspector.set_filter("Position");
        assert_eq!(inspector.filter(), "Position");
    }

    #[test]
    fn test_available_components() {
        let components = ComponentInspector::available_components();
        assert!(components.contains(&"Position"));
        assert!(components.contains(&"Stats"));
        assert!(components.contains(&"Name"));
    }

    #[test]
    fn test_inspector_panel() {
        let mut panel = InspectorPanel::new();
        assert_eq!(panel.inspector().component_count(), 0);

        panel.set_position(InspectorPosition::Left);
        panel.set_size(dde_core::glam::Vec2::new(400.0, 700.0));
    }

    #[test]
    fn test_entity_list_panel() {
        let mut panel = EntityListPanel::new();
        assert!(panel.selected().is_none());

        panel.clear_selection();
        assert!(panel.selected().is_none());
    }
}
