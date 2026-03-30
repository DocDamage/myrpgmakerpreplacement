//! Tilemap editor UI using egui

use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

use super::layer::LayerType;
use super::map::TileMap;
use super::tools::{BrushSize, ToolState, ToolType};

/// Tilemap editor panel
pub struct TileMapEditorPanel {
    /// Current zoom level
    pub zoom: f32,
    /// Minimum zoom
    pub min_zoom: f32,
    /// Maximum zoom
    pub max_zoom: f32,
    /// Show grid
    pub show_grid: bool,
    /// Grid color
    pub grid_color: Color32,
    /// Selected layer
    pub selected_layer: LayerType,
    /// Show all layers
    pub show_all_layers: bool,
}

impl Default for TileMapEditorPanel {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            min_zoom: 0.25,
            max_zoom: 4.0,
            show_grid: true,
            grid_color: Color32::from_rgba_premultiplied(255, 255, 255, 30),
            selected_layer: LayerType::Terrain,
            show_all_layers: true,
        }
    }
}

impl TileMapEditorPanel {
    /// Create new editor panel
    pub fn new() -> Self {
        Self::default()
    }

    /// Zoom in
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.25).min(self.max_zoom);
    }

    /// Zoom out
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.25).max(self.min_zoom);
    }

    /// Reset zoom
    pub fn reset_zoom(&mut self) {
        self.zoom = 1.0;
    }

    /// Draw the editor toolbar
    pub fn draw_toolbar(&mut self, ui: &mut Ui, tools: &mut ToolState) {
        ui.horizontal(|ui| {
            ui.label("Tool:");

            // Tool buttons
            for tool in [
                ToolType::Brush,
                ToolType::Eraser,
                ToolType::Fill,
                ToolType::Eyedropper,
                ToolType::Select,
            ] {
                let selected = tools.current_tool == tool;
                if ui
                    .selectable_label(selected, format!("{} ({})", tool.name(), tool.shortcut()))
                    .clicked()
                {
                    tools.set_tool(tool);
                }
            }

            ui.separator();

            // Brush size (only for brush and eraser)
            if tools.current_tool == ToolType::Brush || tools.current_tool == ToolType::Eraser {
                ui.label("Size:");
                for size in BrushSize::all() {
                    let selected = tools.brush_size == size;
                    if ui.selectable_label(selected, size.name()).clicked() {
                        tools.set_brush_size(size);
                    }
                }
            }

            ui.separator();

            // Zoom controls
            ui.label("Zoom:");
            if ui.button("-").clicked() {
                self.zoom_out();
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if ui.button("+").clicked() {
                self.zoom_in();
            }
            if ui.button("Reset").clicked() {
                self.reset_zoom();
            }

            ui.separator();

            // Grid toggle
            ui.checkbox(&mut self.show_grid, "Grid");
        });
    }

    /// Draw the layer panel
    pub fn draw_layer_panel(&mut self, ui: &mut Ui, map: &mut TileMap) {
        ui.heading("Layers");
        ui.separator();

        for layer_type in LayerType::all() {
            if let Some(layer) = map.get_layer_mut(layer_type) {
                ui.horizontal(|ui| {
                    // Visibility toggle
                    let mut visible = layer.visible;
                    if ui.checkbox(&mut visible, "").changed() {
                        layer.visible = visible;
                    }

                    // Layer name (selectable)
                    let selected = self.selected_layer == layer_type;
                    let name = format!("{} {}", if selected { "▶" } else { " " }, layer.name);
                    if ui.selectable_label(selected, name).clicked() {
                        self.selected_layer = layer_type;
                    }

                    // Lock toggle
                    let locked = layer.locked;
                    if ui.selectable_label(locked, "🔒").clicked() {
                        layer.locked = !locked;
                    }

                    // Opacity slider
                    ui.add(
                        egui::Slider::new(&mut layer.opacity, 0.0..=1.0)
                            .show_value(false)
                            .fixed_decimals(1),
                    );
                });
            }
        }

        ui.separator();

        // Show all layers toggle
        ui.checkbox(&mut self.show_all_layers, "Show All Layers");
    }

    /// Draw the tile palette
    pub fn draw_tile_palette(&mut self, ui: &mut Ui, tools: &mut ToolState) {
        ui.heading("Tile Palette");
        ui.separator();

        // TODO: Load actual tileset and display tiles
        // For now, show a simple color grid as placeholder

        let tile_size = 32.0;
        let _tiles_per_row = 4;

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for i in 0..16 {
                    let tile_index = i as u32;
                    let selected = tools.selected_tile.tile_index == tile_index;

                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::splat(tile_size), Sense::click());

                    // Draw tile background
                    let color = match i % 4 {
                        0 => Color32::from_rgb(100, 150, 100),
                        1 => Color32::from_rgb(150, 100, 100),
                        2 => Color32::from_rgb(100, 100, 150),
                        _ => Color32::from_rgb(150, 150, 100),
                    };

                    ui.painter().rect_filled(rect, 2.0, color);

                    // Draw selection border
                    if selected {
                        ui.painter()
                            .rect_stroke(rect, 2.0, Stroke::new(3.0, Color32::WHITE));
                    }

                    // Draw hover border
                    if response.hovered() {
                        ui.painter()
                            .rect_stroke(rect, 2.0, Stroke::new(2.0, Color32::YELLOW));
                    }

                    if response.clicked() {
                        tools.select_tile(0, tile_index);
                    }
                }
            });
        });
    }

    /// Draw the map viewport
    pub fn draw_viewport(
        &mut self,
        ui: &mut Ui,
        map: &mut TileMap,
        tools: &mut ToolState,
        camera_offset: &mut Vec2,
    ) -> Response {
        let available_size = ui.available_size();

        // Allocate space for the viewport
        let (response, painter) = ui.allocate_painter(available_size, Sense::drag());

        let rect = response.rect;
        let tile_size = map.tile_size as f32 * self.zoom;

        // Handle panning
        if response.dragged() {
            let delta = response.drag_delta();
            camera_offset.x -= delta.x / self.zoom;
            camera_offset.y -= delta.y / self.zoom;
        }

        // Calculate visible range
        let start_x = (camera_offset.x / tile_size).floor() as i32;
        let start_y = (camera_offset.y / tile_size).floor() as i32;
        let end_x = start_x + (rect.width() / tile_size).ceil() as i32 + 1;
        let end_y = start_y + (rect.height() / tile_size).ceil() as i32 + 1;

        // Clamp to map bounds
        let start_x = start_x.max(0) as u32;
        let start_y = start_y.max(0) as u32;
        let end_x = (end_x as u32).min(map.width);
        let end_y = (end_y as u32).min(map.height);

        // Draw tiles
        for y in start_y..end_y {
            for x in start_x..end_x {
                let screen_x = rect.min.x + (x as f32 * tile_size) - camera_offset.x * self.zoom;
                let screen_y = rect.min.y + (y as f32 * tile_size) - camera_offset.y * self.zoom;

                let tile_rect =
                    Rect::from_min_size(Pos2::new(screen_x, screen_y), Vec2::splat(tile_size));

                // Draw tile from visible layers
                if self.show_all_layers {
                    for layer_type in LayerType::all() {
                        if let Some(layer) = map.get_layer(layer_type) {
                            if !layer.visible {
                                continue;
                            }

                            if let Some(tile) = layer.get_tile(x, y) {
                                if !tile.empty {
                                    // Draw tile (placeholder colors)
                                    let color = Self::tile_color(tile.tile_index);
                                    painter.rect_filled(tile_rect, 0.0, color);
                                }
                            }
                        }
                    }
                } else {
                    // Show only selected layer
                    if let Some(layer) = map.get_layer(self.selected_layer) {
                        if let Some(tile) = layer.get_tile(x, y) {
                            if !tile.empty {
                                let color = Self::tile_color(tile.tile_index);
                                painter.rect_filled(tile_rect, 0.0, color);
                            }
                        }
                    }
                }

                // Draw grid
                if self.show_grid {
                    painter.rect_stroke(tile_rect, 0.0, Stroke::new(0.5, self.grid_color));
                }
            }
        }

        // Handle mouse input for tools
        if let Some(mouse_pos) = response.hover_pos() {
            // Convert screen to tile coordinates
            let tile_x =
                ((mouse_pos.x - rect.min.x + camera_offset.x * self.zoom) / tile_size) as u32;
            let tile_y =
                ((mouse_pos.y - rect.min.y + camera_offset.y * self.zoom) / tile_size) as u32;

            if tile_x < map.width && tile_y < map.height {
                tools.set_current_pos(Some((tile_x, tile_y)));

                // Highlight hovered tile
                let highlight_x =
                    rect.min.x + (tile_x as f32 * tile_size) - camera_offset.x * self.zoom;
                let highlight_y =
                    rect.min.y + (tile_y as f32 * tile_size) - camera_offset.y * self.zoom;
                let highlight_rect = Rect::from_min_size(
                    Pos2::new(highlight_x, highlight_y),
                    Vec2::splat(tile_size),
                );

                let highlight_color = match tools.current_tool {
                    ToolType::Brush => Color32::from_rgba_premultiplied(0, 255, 0, 50),
                    ToolType::Eraser => Color32::from_rgba_premultiplied(255, 0, 0, 50),
                    ToolType::Fill => Color32::from_rgba_premultiplied(0, 100, 255, 50),
                    _ => Color32::from_rgba_premultiplied(255, 255, 0, 50),
                };

                painter.rect_filled(highlight_rect, 0.0, highlight_color);
                painter.rect_stroke(highlight_rect, 0.0, Stroke::new(2.0, Color32::WHITE));

                // Handle tool actions
                if response.clicked() || (response.dragged() && !tools.is_dragging) {
                    tools.start_drag(tile_x, tile_y);

                    use super::tools::ToolOps;

                    let action = match tools.current_tool {
                        ToolType::Brush => {
                            ToolOps::brush(map, tools, tile_x, tile_y, self.selected_layer)
                        }
                        ToolType::Eraser => {
                            ToolOps::erase(map, tools, tile_x, tile_y, self.selected_layer)
                        }
                        ToolType::Fill => {
                            ToolOps::fill(map, tools, tile_x, tile_y, self.selected_layer)
                        }
                        ToolType::Eyedropper => {
                            ToolOps::eyedropper(map, tools, tile_x, tile_y, self.selected_layer);
                            None
                        }
                        _ => None,
                    };

                    if let Some(edit_action) = action {
                        tools.push_undo(edit_action);
                    }
                } else if response.dragged() && tools.is_dragging {
                    // Continue dragging for brush/eraser
                    use super::tools::ToolOps;

                    let action = match tools.current_tool {
                        ToolType::Brush => {
                            ToolOps::brush(map, tools, tile_x, tile_y, self.selected_layer)
                        }
                        ToolType::Eraser => {
                            ToolOps::erase(map, tools, tile_x, tile_y, self.selected_layer)
                        }
                        _ => None,
                    };

                    if let Some(edit_action) = action {
                        tools.push_undo(edit_action);
                    }
                }
            }
        }

        if !response.dragged() && tools.is_dragging {
            tools.end_drag();
        }

        response
    }

    /// Get color for a tile index (placeholder)
    fn tile_color(index: u32) -> Color32 {
        let colors = [
            Color32::from_rgb(34, 139, 34),   // Grass
            Color32::from_rgb(139, 69, 19),   // Dirt
            Color32::from_rgb(70, 130, 180),  // Water
            Color32::from_rgb(128, 128, 128), // Stone
            Color32::from_rgb(210, 180, 140), // Sand
            Color32::from_rgb(0, 100, 0),     // Forest
            Color32::from_rgb(255, 255, 255), // Snow
            Color32::from_rgb(255, 0, 0),     // Lava
        ];

        colors[index as usize % colors.len()]
    }

    /// Draw the status bar
    pub fn draw_status_bar(&self, ui: &mut Ui, map: &TileMap, tools: &ToolState) {
        ui.horizontal(|ui| {
            // Map info
            ui.label(format!(
                "Map: {}x{} | Tile Size: {}px",
                map.width, map.height, map.tile_size
            ));

            ui.separator();

            // Current tool info
            ui.label(format!("Tool: {}", tools.current_tool.name()));

            if tools.current_tool == ToolType::Brush || tools.current_tool == ToolType::Eraser {
                ui.label(format!("Size: {}", tools.brush_size.name()));
            }

            ui.separator();

            // Mouse position
            if let Some((x, y)) = tools.current_tile_pos {
                ui.label(format!("Pos: ({}, {})", x, y));
            }

            ui.separator();

            // Selected layer
            ui.label(format!("Layer: {}", self.selected_layer.name()));

            ui.separator();

            // Zoom level
            ui.label(format!("Zoom: {:.0}%", self.zoom * 100.0));
        });
    }
}

/// Info panel for map properties
pub struct MapInfoPanel;

impl MapInfoPanel {
    /// Draw map info panel
    pub fn draw(ui: &mut Ui, map: &mut TileMap) {
        ui.heading("Map Properties");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut map.name);
        });

        ui.horizontal(|ui| {
            ui.label("ID:");
            ui.label(&map.id);
        });

        ui.separator();

        let mut width = map.width;
        let mut height = map.height;
        let mut tile_size = map.tile_size;

        ui.horizontal(|ui| {
            ui.label("Width:");
            if ui
                .add(egui::DragValue::new(&mut width).range(1..=256))
                .changed()
            {
                map.resize(width, map.height);
            }
        });

        ui.horizontal(|ui| {
            ui.label("Height:");
            if ui
                .add(egui::DragValue::new(&mut height).range(1..=256))
                .changed()
            {
                map.resize(map.width, height);
            }
        });

        ui.horizontal(|ui| {
            ui.label("Tile Size:");
            ui.add(egui::DragValue::new(&mut tile_size).range(8..=128));
        });

        ui.separator();

        // Properties
        ui.collapsing("Properties", |ui| {
            // TODO: Add custom property editing
            ui.label("No properties");
        });
    }
}
