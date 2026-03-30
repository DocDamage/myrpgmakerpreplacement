//! Tilemap editor module
//!
//! Provides a complete tilemap editing experience with:
//! - 5-layer system (terrain, objects, collision, regions, events)
//! - Brush tools (1x1, 3x3, 5x5)
//! - Flood fill, eraser, eyedropper
//! - Zoom/pan navigation
//! - JSON export for Tiled compatibility

pub mod layer;
pub mod map;
pub mod tools;
pub mod ui;

pub use layer::{LayerType, Tile, TileLayer};
pub use map::{TileMap, TilesetRef};
pub use tools::{BrushSize, EditAction, TileChange, ToolOps, ToolState, ToolType};
pub use ui::{MapInfoPanel, TileMapEditorPanel};

use std::path::Path;

/// Tilemap editor state
pub struct TileMapEditor {
    /// Current map being edited
    pub map: TileMap,
    /// Tool state
    pub tools: ToolState,
    /// UI state
    pub panel: TileMapEditorPanel,
    /// Camera offset for panning
    pub camera_offset: egui::Vec2,
    /// Whether the editor is active
    pub active: bool,
    /// Whether the map has unsaved changes
    pub dirty: bool,
}

impl Default for TileMapEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl TileMapEditor {
    /// Create a new tilemap editor
    pub fn new() -> Self {
        Self {
            map: TileMap::default(),
            tools: ToolState::new(),
            panel: TileMapEditorPanel::new(),
            camera_offset: egui::Vec2::ZERO,
            active: false,
            dirty: false,
        }
    }

    /// Create editor with a specific map
    pub fn with_map(map: TileMap) -> Self {
        Self {
            map,
            tools: ToolState::new(),
            panel: TileMapEditorPanel::new(),
            camera_offset: egui::Vec2::ZERO,
            active: false,
            dirty: false,
        }
    }

    /// Create a new map
    pub fn new_map(&mut self, width: u32, height: u32, tile_size: u32) {
        self.map = TileMap::new(
            format!("map_{}", uuid::Uuid::new_v4()),
            "Untitled Map",
            width,
            height,
            tile_size,
        );
        self.tools = ToolState::new();
        self.dirty = false;
    }

    /// Load a map from file
    pub fn load_map<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let json_str = std::fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&json_str)?;

        if let Some(map) = TileMap::from_tiled_json(&json) {
            self.map = map;
            self.tools = ToolState::new();
            self.dirty = false;
            Ok(())
        } else {
            Err("Failed to parse map".into())
        }
    }

    /// Save map to file
    pub fn save_map<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let json = self.map.to_tiled_json();
        let json_str = serde_json::to_string_pretty(&json)?;
        std::fs::write(path, json_str)?;
        self.dirty = false;
        Ok(())
    }

    /// Export to Tiled JSON format
    pub fn export_tiled<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let json = self.map.to_tiled_json();
        let json_str = serde_json::to_string_pretty(&json)?;
        std::fs::write(path, json_str)?;
        Ok(())
    }

    /// Toggle editor active state
    pub fn toggle(&mut self) {
        self.active = !self.active;
    }

    /// Check if editor is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Set editor active state
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Handle keyboard shortcuts
    pub fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        // Tool shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::B)) {
            self.tools.set_tool(ToolType::Brush);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::E)) {
            self.tools.set_tool(ToolType::Eraser);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F)) {
            self.tools.set_tool(ToolType::Fill);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::I)) {
            self.tools.set_tool(ToolType::Eyedropper);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::S)) {
            self.tools.set_tool(ToolType::Select);
        }

        // Undo/Redo
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
            self.undo();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Y)) {
            self.redo();
        }

        // Zoom shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::Plus)) {
            self.panel.zoom_in();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Minus)) {
            self.panel.zoom_out();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Num0)) {
            self.panel.reset_zoom();
        }
    }

    /// Undo last action
    pub fn undo(&mut self) -> bool {
        if self.tools.undo(&mut self.map) {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Redo last undone action
    pub fn redo(&mut self) -> bool {
        if self.tools.redo(&mut self.map) {
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Draw the editor UI
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.active {
            return;
        }

        self.handle_shortcuts(ctx);

        // Main editor window
        egui::Window::new("Tilemap Editor")
            .default_size([1200.0, 800.0])
            .show(ctx, |ui| {
                self.draw_editor_ui(ui);
            });
    }

    /// Draw the editor UI components
    fn draw_editor_ui(&mut self, ui: &mut egui::Ui) {
        // Toolbar
        self.panel.draw_toolbar(ui, &mut self.tools);

        ui.separator();

        // Main editing area
        egui::SidePanel::left("layer_panel")
            .default_width(200.0)
            .show_inside(ui, |ui| {
                self.panel.draw_layer_panel(ui, &mut self.map);

                ui.separator();

                self.panel.draw_tile_palette(ui, &mut self.tools);
            });

        egui::SidePanel::right("properties_panel")
            .default_width(200.0)
            .show_inside(ui, |ui| {
                MapInfoPanel::draw(ui, &mut self.map);
            });

        egui::TopBottomPanel::bottom("status_bar").show_inside(ui, |ui| {
            self.panel.draw_status_bar(ui, &self.map, &self.tools);
        });

        // Viewport
        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.panel
                .draw_viewport(ui, &mut self.map, &mut self.tools, &mut self.camera_offset);
        });
    }

    /// Get the current layer type
    pub fn current_layer(&self) -> LayerType {
        self.panel.selected_layer
    }

    /// Set the current layer
    pub fn set_layer(&mut self, layer: LayerType) {
        self.panel.selected_layer = layer;
    }

    /// Check if map has unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark map as dirty
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_creation() {
        let editor = TileMapEditor::new();
        assert!(!editor.active);
        assert!(!editor.dirty);
        assert_eq!(editor.map.width, 64); // Default size
    }

    #[test]
    fn test_new_map() {
        let mut editor = TileMapEditor::new();
        editor.new_map(32, 32, 16);

        assert_eq!(editor.map.width, 32);
        assert_eq!(editor.map.height, 32);
        assert_eq!(editor.map.tile_size, 16);
    }

    #[test]
    fn test_undo_redo() {
        let mut editor = TileMapEditor::new();

        // Paint a tile
        use tools::ToolOps;
        editor.tools.select_tile(1, 5);

        if let Some(action) =
            ToolOps::brush(&mut editor.map, &mut editor.tools, 5, 5, LayerType::Terrain)
        {
            editor.tools.push_undo(action);
        }

        assert!(
            editor
                .map
                .get_tile_at(5, 5, LayerType::Terrain)
                .unwrap()
                .tile_index
                == 5
        );

        // Undo
        editor.undo();
        assert!(editor
            .map
            .get_tile_at(5, 5, LayerType::Terrain)
            .unwrap()
            .is_empty());

        // Redo
        editor.redo();
        assert!(
            editor
                .map
                .get_tile_at(5, 5, LayerType::Terrain)
                .unwrap()
                .tile_index
                == 5
        );
    }
}
