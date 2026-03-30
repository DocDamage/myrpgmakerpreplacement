//! Behavior Tree Visual Editor
//!
//! A comprehensive visual node editor for creating and editing behavior trees.
//! Provides drag-and-drop node editing, connection management, property editing,
//! and integration with the runtime behavior tree system.
//!
//! ## Features
//!
//! - Visual node canvas with pan/zoom
//! - Drag-and-drop node creation from palette
//! - Connection management with bezier curves
//! - Property panel for selected nodes
//! - Save/load to JSON format
//! - Runtime debugging integration
//!
//! ## Usage
//!
//! ```rust,ignore
//! use dde_editor::behavior_tree_editor::BehaviorTreeVisualEditor;
//!
//! let mut editor = BehaviorTreeVisualEditor::new();
//! editor.new_tree();
//!
//! // In your egui UI loop:
//! // editor.draw_ui(ui, Some(&debugger));
//! ```

use dde_core::ai::{BehaviorTreeRunner, BtStatus, NodeId};
use dde_core::Entity;
use egui::{
    Align2, Color32, DragValue, Id, Key, Painter, PointerButton, Pos2, Rect, RichText, Sense,
    Stroke, Ui, Vec2,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::behavior_tree::{
    compile, CompileError, BtDebugger, BtNode, BtNodeError, BtNodeType, MoveSpeed, MoveTarget,
    NodeCategory, ParallelPolicy, Target, VariableValue,
};

/// The main behavior tree visual editor
#[derive(Debug, Clone)]
pub struct BehaviorTreeVisualEditor {
    /// The root node of the tree
    root: Option<BtNode>,
    /// Currently selected node ID
    selected_node: Option<NodeId>,
    /// Node being dragged
    dragging_node: Option<NodeId>,
    /// Connection being drawn (source node, source is output)
    drawing_connection: Option<(NodeId, ConnectionSource)>,
    /// Canvas view offset for panning
    canvas_offset: Vec2,
    /// Canvas zoom level
    canvas_zoom: f32,
    /// Whether to show the grid
    show_grid: bool,
    /// Grid size in pixels
    grid_size: f32,
    /// Show minimap
    show_minimap: bool,
    /// Node palette filter
    palette_filter: String,
    /// Clipboard for copy/paste
    clipboard: Option<BtNode>,
    /// Modified flag for unsaved changes
    modified: bool,
    /// Current file path
    file_path: Option<std::path::PathBuf>,
    /// Execution mode for debugging
    execution_mode: ExecutionMode,
    /// Undo/redo history
    history: Vec<HistoryEntry>,
    /// Current history position
    history_pos: usize,
    /// Max history size
    max_history: usize,
    /// Node ID counter for new nodes
    _node_id_counter: u64,
    /// Show debug overlays
    show_debug_info: bool,
    /// Auto-layout flag
    auto_layout: bool,
}

/// Source of a connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionSource {
    /// Output socket (bottom of node)
    Output,
    /// Child index for composites
    Child(usize),
}

/// Execution mode for the editor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutionMode {
    #[default]
    Edit,
    Debug,
    Simulate,
}

/// History entry for undo/redo
#[derive(Debug, Clone)]
struct HistoryEntry {
    root: Option<BtNode>,
    description: String,
}

/// Visual theme for nodes
#[derive(Debug, Clone, Copy)]
pub struct NodeTheme {
    pub background: Color32,
    pub header: Color32,
    pub border: Color32,
    pub text: Color32,
    pub socket: Color32,
    pub selected_border: Color32,
}

impl Default for BehaviorTreeVisualEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl BehaviorTreeVisualEditor {
    /// Create a new behavior tree visual editor
    pub fn new() -> Self {
        Self {
            root: None,
            selected_node: None,
            dragging_node: None,
            drawing_connection: None,
            canvas_offset: Vec2::new(100.0, 50.0),
            canvas_zoom: 1.0,
            show_grid: true,
            grid_size: 20.0,
            show_minimap: false,
            palette_filter: String::new(),
            clipboard: None,
            modified: false,
            file_path: None,
            execution_mode: ExecutionMode::Edit,
            history: Vec::new(),
            history_pos: 0,
            max_history: 50,
            _node_id_counter: 1,
            show_debug_info: false,
            auto_layout: false,
        }
    }

    /// Create a new empty behavior tree with a root node
    pub fn new_tree(&mut self) {
        self.save_to_history("New tree");
        self.root = Some(BtNode::new(
            BtNodeType::Selector {
                children: Vec::new(),
            },
            [0.0, 0.0],
        ));
        self.selected_node = None;
        self.modified = true;
        self.file_path = None;
    }

    /// Get the root node
    pub fn root(&self) -> Option<&BtNode> {
        self.root.as_ref()
    }

    /// Get mutable root
    pub fn root_mut(&mut self) -> Option<&mut BtNode> {
        self.root.as_mut()
    }

    /// Check if there are unsaved changes
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Get the current file path
    pub fn file_path(&self) -> Option<&std::path::Path> {
        self.file_path.as_deref()
    }

    /// Set the execution mode
    pub fn set_execution_mode(&mut self, mode: ExecutionMode) {
        self.execution_mode = mode;
    }

    /// Draw the complete editor UI
    pub fn draw_ui(&mut self, ui: &mut Ui, debugger: Option<&BtDebugger>) {
        // Top toolbar
        self.draw_toolbar(ui);

        ui.separator();

        // Main editor area with side panels
        egui::SidePanel::left("bt_palette_panel")
            .default_width(220.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                self.draw_palette(ui);
            });

        egui::SidePanel::right("bt_properties_panel")
            .default_width(280.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                self.draw_properties_panel(ui, debugger);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.draw_canvas(ui, debugger);
        });
    }

    /// Draw the toolbar
    fn draw_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // File operations
            ui.menu_button("File", |ui| {
                if ui.button("📝 New Tree").clicked() {
                    self.new_tree();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("💾 Save").clicked() {
                    self.save_dialog();
                    ui.close_menu();
                }
                if ui.button("💾 Save As...").clicked() {
                    self.save_as_dialog();
                    ui.close_menu();
                }
                if ui.button("📂 Open...").clicked() {
                    self.open_dialog();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("📤 Export JSON").clicked() {
                    self.export_json();
                    ui.close_menu();
                }
                if ui.button("📥 Import JSON").clicked() {
                    self.import_json();
                    ui.close_menu();
                }
            });

            ui.separator();

            // Edit operations
            ui.menu_button("Edit", |ui| {
                let can_undo = self.history_pos > 0;
                let can_redo = self.history_pos < self.history.len();

                if ui.add_enabled(can_undo, egui::Button::new("↩ Undo")).clicked() {
                    self.undo();
                    ui.close_menu();
                }
                if ui.add_enabled(can_redo, egui::Button::new("↪ Redo")).clicked() {
                    self.redo();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("✂ Cut").clicked() {
                    self.cut_selected();
                    ui.close_menu();
                }
                if ui.button("📋 Copy").clicked() {
                    self.copy_selected();
                    ui.close_menu();
                }
                if ui
                    .add_enabled(self.clipboard.is_some(), egui::Button::new("📋 Paste"))
                    .clicked()
                {
                    self.paste();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("🗑 Delete Selected").clicked() {
                    if let Some(id) = self.selected_node {
                        self.delete_node(id);
                    }
                    ui.close_menu();
                }
            });

            ui.separator();

            // View options
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.show_grid, "Show Grid");
                ui.checkbox(&mut self.show_minimap, "Show Minimap");
                ui.checkbox(&mut self.show_debug_info, "Debug Info");
                ui.separator();
                if ui.button("Frame All (F)").clicked() {
                    self.frame_all();
                    ui.close_menu();
                }
                if ui.button("Reset View").clicked() {
                    self.canvas_offset = Vec2::new(100.0, 50.0);
                    self.canvas_zoom = 1.0;
                    ui.close_menu();
                }
            });

            ui.separator();

            // Layout
            if ui.button("🔄 Auto Layout").clicked() {
                self.auto_layout_tree();
            }

            ui.separator();

            // Execution mode
            ui.horizontal(|ui| {
                ui.label("Mode:");
                ui.selectable_value(&mut self.execution_mode, ExecutionMode::Edit, "Edit");
                ui.selectable_value(&mut self.execution_mode, ExecutionMode::Debug, "Debug");
                ui.selectable_value(&mut self.execution_mode, ExecutionMode::Simulate, "Simulate");
            });

            // Right-aligned info
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.modified {
                    ui.label(RichText::new("●").color(Color32::YELLOW));
                }
                ui.label(format!("{:.0}%", self.canvas_zoom * 100.0));
            });
        });
    }

    /// Draw the node palette
    fn draw_palette(&mut self, ui: &mut Ui) {
        ui.heading("Node Palette");
        ui.separator();

        // Search filter
        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.text_edit_singleline(&mut self.palette_filter);
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for category in NodeCategory::all() {
                let icon = category_icon(category);
                let header_text = format!("{} {}", icon, category.display_name());

                egui::CollapsingHeader::new(header_text)
                    .default_open(true)
                    .show(ui, |ui| {
                        for template in category.templates() {
                            let name = template.display_name();

                            // Apply filter
                            if !self.palette_filter.is_empty()
                                && !name
                                    .to_lowercase()
                                    .contains(&self.palette_filter.to_lowercase())
                            {
                                continue;
                            }

                            let button_text = format!("{} {}", template.icon(), name);

                            if ui.button(button_text).clicked() {
                                self.add_node_from_template(template);
                            }
                        }
                    });
            }
        });

        // Drag hint
        ui.separator();
        ui.label(
            RichText::new("Tip: Drag nodes from here to canvas")
                .small()
                .color(Color32::GRAY),
        );
    }

    /// Draw the main canvas area
    fn draw_canvas(&mut self, ui: &mut Ui, debugger: Option<&BtDebugger>) {
        let available_rect = ui.available_rect_before_wrap();
        let canvas_id = Id::new("bt_canvas");

        // Canvas background interaction
        let canvas_response = ui.interact(available_rect, canvas_id, Sense::click_and_drag());

        // Pan canvas with middle mouse or shift+drag
        if canvas_response.dragged_by(PointerButton::Middle)
            || (canvas_response.dragged_by(PointerButton::Primary)
                && ui.input(|i| i.modifiers.shift))
        {
            self.canvas_offset += canvas_response.drag_delta();
        }

        // Zoom with scroll
        ui.input(|i| {
            let scroll = i.raw_scroll_delta.y;
            if scroll != 0.0 {
                let zoom_delta = if scroll > 0.0 { 1.1 } else { 0.9 };
                let new_zoom = (self.canvas_zoom * zoom_delta).clamp(0.25, 4.0);

                // Zoom towards mouse pointer
                if let Some(pointer_pos) = i.pointer.hover_pos() {
                    let zoom_center = self.screen_to_world(pointer_pos);
                    self.canvas_zoom = new_zoom;
                    let new_screen_pos = self.world_to_screen(zoom_center);
                    self.canvas_offset += pointer_pos - new_screen_pos;
                } else {
                    self.canvas_zoom = new_zoom;
                }
            }
        });

        let painter = ui.painter();

        // Draw grid
        if self.show_grid {
            self.draw_grid(painter, available_rect);
        }

        // Draw connections first (behind nodes)
        if let Some(root) = &self.root {
            let root_clone = root.clone();
            self.draw_connections(ui, &root_clone, debugger);
        }

        // Draw nodes
        if let Some(root) = self.root.clone() {
            self.draw_node_recursive(ui, root, debugger);
        }

        // Draw connection being created
        if let Some((source_id, source_type)) = self.drawing_connection {
            if let Some(root) = &self.root {
                if let Some(source_node) = root.find_node(source_id) {
                    let source_pos =
                        self.world_to_screen(Pos2::new(source_node.position[0], source_node.position[1]));
                    let output_pos = source_pos + Vec2::new(0.0, 30.0);

                    let end_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(output_pos);

                    self.draw_bezier_connection(
                        painter,
                        output_pos,
                        end_pos,
                        Color32::from_rgb(255, 200, 50),
                        None,
                    );
                }
            }
        }

        // Canvas click (deselect)
        if canvas_response.clicked() {
            self.selected_node = None;
            self.drawing_connection = None;
        }

        // Context menu
        canvas_response.context_menu(|ui| {
            if ui.button("Add Selector").clicked() {
                self.add_node_at_mouse(BtNodeType::Selector { children: Vec::new() }, ui);
                ui.close_menu();
            }
            if ui.button("Add Sequence").clicked() {
                self.add_node_at_mouse(BtNodeType::Sequence { children: Vec::new() }, ui);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Paste").clicked() {
                self.paste();
                ui.close_menu();
            }
        });

        // Handle keyboard shortcuts
        self.handle_keyboard_shortcuts(ui);

        // Draw minimap
        if self.show_minimap {
            self.draw_minimap(ui, available_rect);
        }

        // Draw debug info
        if self.show_debug_info {
            self.draw_debug_info(ui, available_rect);
        }
    }

    /// Draw the grid background
    fn draw_grid(&self, painter: &Painter, rect: Rect) {
        let grid_color = Color32::from_gray(40);
        let major_grid_color = Color32::from_gray(55);

        let offset_x = self.canvas_offset.x.rem_euclid(self.grid_size * self.canvas_zoom);
        let offset_y = self.canvas_offset.y.rem_euclid(self.grid_size * self.canvas_zoom);

        // Minor grid lines
        let mut x = rect.left() + offset_x;
        let grid_step = self.grid_size * self.canvas_zoom;
        while x < rect.right() {
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(1.0, grid_color),
            );
            x += grid_step;
        }

        let mut y = rect.top() + offset_y;
        while y < rect.bottom() {
            painter.line_segment(
                [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                Stroke::new(1.0, grid_color),
            );
            y += grid_step;
        }

        // Major grid lines (every 5 cells)
        let major_step = grid_step * 5.0;
        let major_offset_x = self.canvas_offset.x.rem_euclid(major_step);
        let major_offset_y = self.canvas_offset.y.rem_euclid(major_step);

        x = rect.left() + major_offset_x;
        while x < rect.right() {
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(1.0, major_grid_color),
            );
            x += major_step;
        }

        y = rect.top() + major_offset_y;
        while y < rect.bottom() {
            painter.line_segment(
                [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                Stroke::new(1.0, major_grid_color),
            );
            y += major_step;
        }
    }

    /// Draw connections between nodes recursively
    fn draw_connections(&self, ui: &mut Ui, node: &BtNode, debugger: Option<&BtDebugger>) {
        let painter = ui.painter();
        let parent_pos = self.world_to_screen(Pos2::new(node.position[0], node.position[1]));
        let parent_output = parent_pos + Vec2::new(0.0, 30.0);

        // Collect connections to draw
        let mut connections: Vec<(Pos2, Pos2, Color32, Option<usize>)> = Vec::new();
        let mut children_to_process: Vec<&BtNode> = Vec::new();

        // Multiple children (composites)
        if let Some(children) = node.children() {
            for (i, child) in children.iter().enumerate() {
                let child_pos =
                    self.world_to_screen(Pos2::new(child.position[0], child.position[1]));
                let child_input = child_pos - Vec2::new(0.0, 30.0);

                let color = self.get_connection_color(node.id, child.id, debugger);
                let order = if matches!(node.node_type, BtNodeType::Sequence { .. }) {
                    Some(i + 1)
                } else {
                    None
                };

                connections.push((parent_output, child_input, color, order));
                children_to_process.push(child);
            }
        }

        // Single child (decorators)
        if let Some(child) = node.node_type.child() {
            let child_pos = self.world_to_screen(Pos2::new(child.position[0], child.position[1]));
            let child_input = child_pos - Vec2::new(0.0, 30.0);
            let color = self.get_connection_color(node.id, child.id, debugger);
            connections.push((parent_output, child_input, color, None));
            children_to_process.push(child);
        }

        // Draw all connections
        for (start, end, color, order) in connections {
            self.draw_bezier_connection(painter, start, end, color, order);
        }

        // Recurse into children
        for child in children_to_process {
            self.draw_connections(ui, child, debugger);
        }
    }

    /// Draw a bezier curve connection
    fn draw_bezier_connection(
        &self,
        painter: &Painter,
        start: Pos2,
        end: Pos2,
        color: Color32,
        order: Option<usize>,
    ) {
        let control_offset = ((end.y - start.y) / 2.0).max(30.0 * self.canvas_zoom);
        let cp1 = start + Vec2::new(0.0, control_offset);
        let cp2 = end - Vec2::new(0.0, control_offset);

        // Draw shadow
        painter.add(egui::Shape::CubicBezier(
            egui::epaint::CubicBezierShape::from_points_stroke(
                [start + Vec2::new(2.0, 2.0), cp1, cp2, end + Vec2::new(2.0, 2.0)],
                false,
                Color32::TRANSPARENT,
                Stroke::new(3.0, Color32::from_black_alpha(100)),
            ),
        ));

        // Draw main line
        painter.add(egui::Shape::CubicBezier(
            egui::epaint::CubicBezierShape::from_points_stroke(
                [start, cp1, cp2, end],
                false,
                Color32::TRANSPARENT,
                Stroke::new(2.0, color),
            ),
        ));

        // Draw order indicator for sequences
        if let Some(idx) = order {
            let mid_point = start + (end - start) * 0.5;
            let circle_radius = 8.0;
            painter.circle_filled(mid_point, circle_radius, Color32::from_gray(60));
            painter.circle_stroke(mid_point, circle_radius, Stroke::new(1.0, color));
            painter.text(
                mid_point,
                Align2::CENTER_CENTER,
                format!("{}", idx),
                egui::FontId::proportional(10.0),
                Color32::WHITE,
            );
        }
    }

    /// Get the color for a connection
    fn get_connection_color(
        &self,
        parent_id: NodeId,
        child_id: NodeId,
        debugger: Option<&BtDebugger>,
    ) -> Color32 {
        if let Some(dbg) = debugger {
            if dbg.is_in_execution_path(parent_id) && dbg.is_in_execution_path(child_id) {
                return Color32::from_rgb(255, 200, 50); // Yellow for active
            }
        }
        Color32::from_gray(100)
    }

    /// Draw a node recursively
    fn draw_node_recursive(&mut self, ui: &mut Ui, node: BtNode, debugger: Option<&BtDebugger>) {
        let screen_pos = self.world_to_screen(Pos2::new(node.position[0], node.position[1]));
        let node_size = Vec2::new(160.0, 70.0);
        let node_rect = Rect::from_center_size(screen_pos, node_size);
        let node_id = node.id;

        // Node interaction
        let response = ui.interact(node_rect, Id::new(node_id.0), Sense::click_and_drag());

        // Handle dragging
        if response.dragged_by(PointerButton::Primary) && self.dragging_node.is_none() {
            self.dragging_node = Some(node_id);
        }

        if self.dragging_node == Some(node_id) {
            if response.drag_stopped() {
                self.dragging_node = None;
                self.save_to_history("Move node");
            } else {
                let delta = response.drag_delta() / self.canvas_zoom;
                if let Some(root) = &mut self.root {
                    if let Some(n) = root.find_node_mut(node_id) {
                        n.position[0] += delta.x;
                        n.position[1] += delta.y;
                        // Snap to grid
                        if self.show_grid {
                            n.position[0] =
                                (n.position[0] / self.grid_size).round() * self.grid_size;
                            n.position[1] =
                                (n.position[1] / self.grid_size).round() * self.grid_size;
                        }
                    }
                }
            }
        }

        // Handle selection
        if response.clicked() {
            self.selected_node = Some(node_id);
        }

        // Handle double-click for connection
        if response.double_clicked() {
            self.drawing_connection = Some((node_id, ConnectionSource::Output));
        }

        // Draw the node
        self.draw_node_visual(ui, node_rect, &node, response.hovered(), debugger);

        // Context menu
        response.context_menu(|ui| {
            if ui.button("Duplicate").clicked() {
                self.duplicate_node(node_id);
                ui.close_menu();
            }
            if ui.button("Delete").clicked() {
                self.delete_node(node_id);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Copy").clicked() {
                self.copy_node(node_id);
                ui.close_menu();
            }
        });

        // Draw children
        if let Some(children) = node.children() {
            for child in children.clone() {
                self.draw_node_recursive(ui, child, debugger);
            }
        }
        if let Some(child) = node.node_type.child() {
            self.draw_node_recursive(ui, (*child).clone(), debugger);
        }
    }

    /// Draw the visual representation of a node
    fn draw_node_visual(
        &self,
        ui: &mut Ui,
        rect: Rect,
        node: &BtNode,
        hovered: bool,
        debugger: Option<&BtDebugger>,
    ) {
        let painter = ui.painter();
        let color = node.color();
        let color32 = Color32::from_rgb(color[0], color[1], color[2]);

        // Shadow
        painter.rect_filled(
            rect.translate(Vec2::new(3.0, 3.0)),
            6.0,
            Color32::from_black_alpha(100),
        );

        // Selection highlight
        let is_selected = self.selected_node == Some(node.id);
        if is_selected {
            painter.rect_stroke(
                rect.expand(4.0),
                6.0,
                Stroke::new(2.0, Color32::WHITE),
            );
        }

        // Hover effect
        if hovered && !is_selected {
            painter.rect_stroke(
                rect.expand(2.0),
                6.0,
                Stroke::new(1.0, Color32::from_gray(200)),
            );
        }

        // Background
        let bg_color = if debugger.map(|d| d.has_breakpoint(node.id)).unwrap_or(false) {
            Color32::from_rgb(80, 40, 40)
        } else if is_selected {
            Color32::from_gray(55)
        } else {
            Color32::from_gray(45)
        };
        painter.rect_filled(rect, 6.0, bg_color);

        // Header bar
        let header_rect = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.min.y + 24.0));
        painter.rect_filled(header_rect, 6.0, color32);

        // Debug status overlay
        if let Some(dbg) = debugger {
            // Note: node_status is private - using placeholder
            let status: Option<BtStatus> = None;
            if let Some(status) = status {
                let status_col = match status {
                    BtStatus::Success => Color32::from_rgb(100, 200, 100),
                    BtStatus::Failure => Color32::from_rgb(200, 100, 100),
                    BtStatus::Running => Color32::from_rgb(255, 200, 50),
                };
                painter.rect_stroke(rect, 6.0, Stroke::new(3.0, status_col));

                // Status dot
                let dot_pos = rect.left_top() + Vec2::new(8.0, 8.0);
                painter.circle_filled(dot_pos, 5.0, status_col);
            }
        }

        // Icon
        painter.text(
            header_rect.left_center() + Vec2::new(12.0, 0.0),
            Align2::LEFT_CENTER,
            format!("{}", node.icon()),
            egui::FontId::proportional(16.0),
            Color32::WHITE,
        );

        // Title
        painter.text(
            header_rect.center(),
            Align2::CENTER_CENTER,
            node.display_name(),
            egui::FontId::proportional(12.0),
            Color32::WHITE,
        );

        // Node ID (debug)
        if self.show_debug_info {
            painter.text(
                header_rect.right_center() - Vec2::new(4.0, 0.0),
                Align2::RIGHT_CENTER,
                format!("{:?}", node.id.0 % 1000),
                egui::FontId::proportional(9.0),
                Color32::from_gray(200),
            );
        }

        // Comment preview
        if let Some(comment) = &node.comment {
            painter.text(
                rect.center_bottom() - Vec2::new(0.0, 8.0),
                Align2::CENTER_BOTTOM,
                if comment.len() > 20 {
                    format!("{}...", &comment[..20])
                } else {
                    comment.clone()
                },
                egui::FontId::proportional(9.0),
                Color32::GRAY,
            );
        }

        // Input socket (top center)
        let input_pos = rect.center_top();
        painter.circle_filled(input_pos, 6.0, Color32::from_gray(80));
        painter.circle_stroke(input_pos, 6.0, Stroke::new(1.0, Color32::WHITE));

        // Output socket (bottom center)
        let output_pos = rect.center_bottom();
        painter.circle_filled(output_pos, 6.0, Color32::from_gray(80));
        painter.circle_stroke(output_pos, 6.0, Stroke::new(1.0, Color32::WHITE));

        // Connection interaction zones
        let input_rect = Rect::from_center_size(input_pos, Vec2::new(12.0, 12.0));
        let output_rect = Rect::from_center_size(output_pos, Vec2::new(12.0, 12.0));

        let _input_response = ui.interact(input_rect, Id::new((node.id.0, "input")), Sense::click());
        let output_response = ui.interact(output_rect, Id::new((node.id.0, "output")), Sense::click());

        if output_response.clicked() {
            // Start drawing connection
            // This would need to be handled at editor level
        }
    }

    /// Draw the properties panel
    fn draw_properties_panel(&mut self, ui: &mut Ui, debugger: Option<&BtDebugger>) {
        ui.heading("Properties");
        ui.separator();

        if let Some(selected_id) = self.selected_node {
            let node_clone = self
                .root
                .as_ref()
                .and_then(|r| r.find_node(selected_id))
                .cloned();

            if let Some(mut node) = node_clone {
                // Node info header
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{}", node.icon()))
                            .size(20.0)
                            .color(Color32::from_rgb(
                                node.color()[0],
                                node.color()[1],
                                node.color()[2],
                            )),
                    );
                    ui.vertical(|ui| {
                        ui.label(RichText::new(node.display_name()).strong());
                        ui.label(
                            RichText::new(format!("ID: {:?}", node.id)).small().monospace(),
                        );
                    });
                });

                ui.separator();

                // Position
                ui.group(|ui| {
                    ui.label("Position");
                    ui.horizontal(|ui| {
                        ui.add(DragValue::new(&mut node.position[0]).prefix("X: ").speed(1.0));
                        ui.add(DragValue::new(&mut node.position[1]).prefix("Y: ").speed(1.0));
                    });
                });

                ui.separator();

                // Comment
                ui.label("Comment:");
                let mut comment = node.comment.clone().unwrap_or_default();
                if ui.text_edit_singleline(&mut comment).changed() {
                    node.comment = if comment.is_empty() { None } else { Some(comment) };
                    self.modified = true;
                }

                ui.separator();

                // Type-specific properties
                ui.collapsing("Node Parameters", |ui| {
                    self.draw_node_properties(ui, &mut node);
                });

                ui.separator();

                // Actions
                ui.horizontal(|ui| {
                    if ui.button("📋 Copy").clicked() {
                        self.clipboard = Some(node.clone());
                    }
                    if ui.button("🗑 Delete").clicked() {
                        self.delete_node(selected_id);
                    }
                });

                // Apply changes back
                if let Some(root) = &mut self.root {
                    if let Some(target) = root.find_node_mut(selected_id) {
                        *target = node;
                    }
                }
            } else {
                ui.label(RichText::new("Node not found").color(Color32::RED));
                self.selected_node = None;
            }
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(RichText::new("Select a node").size(16.0).color(Color32::GRAY));
                ui.label(RichText::new("Click on a node in the canvas to edit its properties").small().color(Color32::GRAY));
            });
        }

        // Debugger section
        if let Some(dbg) = debugger {
            ui.separator();
            ui.heading("Debug Info");

            if let Some(entity) = dbg.target_entity() {
                ui.label(format!("Entity: {:?}", entity));
            }

            if let Some(selected_id) = self.selected_node {
                // Note: node_status is private - using placeholder
                let status: Option<BtStatus> = None;
                if let Some(status) = status {
                    let status_text = format!("{:?}", status);
                    let status_color = match status {
                        BtStatus::Success => Color32::GREEN,
                        BtStatus::Failure => Color32::RED,
                        BtStatus::Running => Color32::YELLOW,
                    };
                    ui.horizontal(|ui| {
                        ui.label("Status:");
                        ui.label(RichText::new(status_text).color(status_color));
                    });
                }
            }
        }
    }

    /// Draw type-specific node properties
    fn draw_node_properties(&mut self, ui: &mut Ui, node: &mut BtNode) {
        match &mut node.node_type {
            BtNodeType::IsPlayerNearby { radius } => {
                ui.label("Detection Radius:");
                ui.add(DragValue::new(radius).speed(0.1).range(0.0..=100.0).suffix(" units"));
            }
            BtNodeType::HealthBelow { percent } => {
                ui.label("Health Threshold:");
                ui.add(egui::Slider::new(percent, 0.0..=1.0).text("%"));
            }
            BtNodeType::RandomChance { percent } => {
                ui.label("Success Chance:");
                ui.add(egui::Slider::new(percent, 0..=100).text("%"));
            }
            BtNodeType::Wait { seconds } => {
                ui.label("Wait Duration:");
                ui.add(DragValue::new(seconds).speed(0.1).range(0.0..=f32::MAX).suffix(" s"));
            }
            BtNodeType::Cooldown { seconds, .. } => {
                ui.label("Cooldown Duration:");
                ui.add(DragValue::new(seconds).speed(0.1).range(0.01..=f32::MAX).suffix(" s"));
            }
            BtNodeType::Repeater { count, .. } => {
                ui.label("Repeat Count:");
                let mut count_val = count.unwrap_or(0);
                ui.add(DragValue::new(&mut count_val).range(0..=10000));
                ui.label(RichText::new("(0 = forever)").small().color(Color32::GRAY));
                *count = if count_val == 0 { None } else { Some(count_val) };
            }
            BtNodeType::Parallel {
                success_policy,
                failure_policy,
                ..
            } => {
                ui.label("Success Policy:");
                draw_parallel_policy_combo(ui, success_policy);
                ui.label("Failure Policy:");
                draw_parallel_policy_combo(ui, failure_policy);
            }
            BtNodeType::MoveTo { target, speed } => {
                ui.label("Speed:");
                draw_move_speed_combo(ui, speed);
                ui.label("Target:");
                // Simplified target display
                let target_str = match target {
                    MoveTarget::Player => "Player",
                    MoveTarget::Entity(_) => "Entity",
                    MoveTarget::Position(_) => "Position",
                    MoveTarget::PatrolPoint(_) => "Patrol Point",
                };
                ui.label(target_str);
            }
            BtNodeType::Follow { distance, .. } => {
                ui.label("Follow Distance:");
                ui.add(DragValue::new(distance).speed(0.1).range(0.0..=50.0).suffix(" units"));
            }
            BtNodeType::UseSkill { skill_id, .. } => {
                ui.label("Skill ID:");
                ui.add(DragValue::new(skill_id));
            }
            BtNodeType::UseItem { item_id } => {
                ui.label("Item ID:");
                ui.add(DragValue::new(item_id));
            }
            BtNodeType::PlayAnimation { anim_id } => {
                ui.label("Animation ID:");
                ui.add(DragValue::new(anim_id));
            }
            BtNodeType::Speak { dialogue_id } => {
                ui.label("Dialogue ID:");
                ui.add(DragValue::new(dialogue_id));
            }
            BtNodeType::SetVariable { name, value } => {
                ui.label("Variable Name:");
                ui.text_edit_singleline(name);
                ui.label("Value:");
                match value {
                    VariableValue::Bool(b) => {
                        ui.checkbox(b, "True");
                    }
                    VariableValue::Int(i) => {
                        ui.add(DragValue::new(i));
                    }
                    VariableValue::Float(f) => {
                        ui.add(DragValue::new(f));
                    }
                    VariableValue::String(s) => {
                        ui.text_edit_singleline(s);
                    }
                    VariableValue::Entity(_) => {
                        ui.label("Entity (read-only)");
                    }
                }
            }
            BtNodeType::CustomCondition { script } | BtNodeType::CustomAction { script } => {
                ui.label("Script:");
                ui.text_edit_multiline(script);
            }
            _ => {
                ui.label("No editable properties for this node type.");
            }
        }
    }

    /// Draw the minimap
    fn draw_minimap(&self, ui: &mut Ui, canvas_rect: Rect) {
        let minimap_size = Vec2::new(180.0, 120.0);
        let padding = Vec2::new(10.0, 10.0);
        let minimap_rect = Rect::from_min_size(
            canvas_rect.right_bottom() - minimap_size - padding,
            minimap_size,
        );

        let painter = ui.painter();

        // Background
        painter.rect_filled(minimap_rect, 5.0, Color32::from_black_alpha(220));
        painter.rect_stroke(minimap_rect, 5.0, Stroke::new(1.0, Color32::GRAY));

        // Draw simplified node representations
        if let Some(root) = &self.root {
            self.draw_minimap_nodes(painter, minimap_rect, root);
        }

        // Viewport indicator
        let viewport_rect = self.calculate_viewport_rect(minimap_rect);
        painter.rect_stroke(viewport_rect, 2.0, Stroke::new(1.0, Color32::WHITE));
    }

    /// Draw simplified nodes on minimap
    fn draw_minimap_nodes(&self, painter: &Painter, minimap_rect: Rect, node: &BtNode) {
        // Calculate bounds
        let (min_x, max_x, min_y, max_y) = self.calculate_tree_bounds();
        let tree_width = (max_x - min_x).max(400.0);
        let tree_height = (max_y - min_y).max(300.0);

        let scale_x = minimap_rect.width() / tree_width;
        let scale_y = minimap_rect.height() / tree_height;
        let scale = scale_x.min(scale_y).min(0.1);

        let offset_x = minimap_rect.left() - min_x * scale + 10.0;
        let offset_y = minimap_rect.top() - min_y * scale + 10.0;

        fn draw_node_recursive(
            painter: &Painter,
            node: &BtNode,
            scale: f32,
            offset_x: f32,
            offset_y: f32,
            color: Color32,
        ) {
            let x = node.position[0] * scale + offset_x;
            let y = node.position[1] * scale + offset_y;
            let size = Vec2::new(8.0, 6.0);
            let rect = Rect::from_center_size(Pos2::new(x, y), size);

            painter.rect_filled(rect, 2.0, color);

            // Draw connections
            if let Some(children) = node.children() {
                for child in children {
                    let cx = child.position[0] * scale + offset_x;
                    let cy = child.position[1] * scale + offset_y;
                    painter.line_segment(
                        [Pos2::new(x, y + 3.0), Pos2::new(cx, cy - 3.0)],
                        Stroke::new(1.0, Color32::from_gray(80)),
                    );
                    draw_node_recursive(painter, child, scale, offset_x, offset_y, color);
                }
            }

            if let Some(child) = node.node_type.child() {
                let cx = child.position[0] * scale + offset_x;
                let cy = child.position[1] * scale + offset_y;
                painter.line_segment(
                    [Pos2::new(x, y + 3.0), Pos2::new(cx, cy - 3.0)],
                    Stroke::new(1.0, Color32::from_gray(80)),
                );
                draw_node_recursive(painter, child, scale, offset_x, offset_y, color);
            }
        }

        let color = Color32::from_rgb(node.color()[0], node.color()[1], node.color()[2]);
        draw_node_recursive(painter, node, scale, offset_x, offset_y, color);
    }

    /// Calculate the bounds of the tree
    fn calculate_tree_bounds(&self) -> (f32, f32, f32, f32) {
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;

        if let Some(root) = &self.root {
            fn collect_bounds(node: &BtNode, min_x: &mut f32, max_x: &mut f32, min_y: &mut f32, max_y: &mut f32) {
                *min_x = (*min_x).min(node.position[0]);
                *max_x = (*max_x).max(node.position[0]);
                *min_y = (*min_y).min(node.position[1]);
                *max_y = (*max_y).max(node.position[1]);

                if let Some(children) = node.children() {
                    for child in children {
                        collect_bounds(child, min_x, max_x, min_y, max_y);
                    }
                }
                if let Some(child) = node.node_type.child() {
                    collect_bounds(child, min_x, max_x, min_y, max_y);
                }
            }
            collect_bounds(root, &mut min_x, &mut max_x, &mut min_y, &mut max_y);
        }

        if min_x == f32::MAX {
            (0.0, 400.0, 0.0, 300.0)
        } else {
            (min_x - 100.0, max_x + 100.0, min_y - 100.0, max_y + 100.0)
        }
    }

    /// Calculate viewport rectangle for minimap
    fn calculate_viewport_rect(&self, minimap_rect: Rect) -> Rect {
        // Simplified viewport indicator
        let viewport_size = Vec2::new(40.0, 30.0);
        Rect::from_center_size(minimap_rect.center(), viewport_size)
    }

    /// Draw debug info overlay
    fn draw_debug_info(&self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter();
        let text = format!(
            "Zoom: {:.1}x | Offset: ({:.0}, {:.0}) | Nodes: {} | Mode: {:?}",
            self.canvas_zoom,
            self.canvas_offset.x,
            self.canvas_offset.y,
            self.count_nodes(),
            self.execution_mode
        );

        painter.text(
            rect.left_top() + Vec2::new(10.0, 10.0),
            Align2::LEFT_TOP,
            text,
            egui::FontId::monospace(10.0),
            Color32::GRAY,
        );
    }

    /// Count total nodes in tree
    fn count_nodes(&self) -> usize {
        if let Some(root) = &self.root {
            let mut count = 0;
            fn count_recursive(node: &BtNode, count: &mut usize) {
                *count += 1;
                if let Some(children) = node.children() {
                    for child in children {
                        count_recursive(child, count);
                    }
                }
                if let Some(child) = node.node_type.child() {
                    count_recursive(child, count);
                }
            }
            count_recursive(root, &mut count);
            count
        } else {
            0
        }
    }

    /// Handle keyboard shortcuts
    fn handle_keyboard_shortcuts(&mut self, ui: &Ui) {
        ui.input(|i| {
            // Delete
            if i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace) {
                if let Some(id) = self.selected_node {
                    self.delete_node(id);
                }
            }

            // Copy
            if i.modifiers.ctrl && i.key_pressed(Key::C) {
                if let Some(id) = self.selected_node {
                    self.copy_node(id);
                }
            }

            // Paste
            if i.modifiers.ctrl && i.key_pressed(Key::V) {
                self.paste();
            }

            // Cut
            if i.modifiers.ctrl && i.key_pressed(Key::X) {
                self.cut_selected();
            }

            // Undo/Redo
            if i.modifiers.ctrl && i.key_pressed(Key::Z) {
                if i.modifiers.shift {
                    self.redo();
                } else {
                    self.undo();
                }
            }

            // Frame all
            if i.key_pressed(Key::F) {
                self.frame_all();
            }

            // Save
            if i.modifiers.ctrl && i.key_pressed(Key::S) {
                if self.file_path.is_some() {
                    self.save_to_file();
                } else {
                    self.save_as_dialog();
                }
            }
        });
    }

    /// Save current state to history
    fn save_to_history(&mut self, description: &str) {
        // Remove redo entries
        self.history.truncate(self.history_pos);

        self.history.push(HistoryEntry {
            root: self.root.clone(),
            description: description.to_string(),
        });

        // Limit history size
        if self.history.len() > self.max_history {
            self.history.remove(0);
        } else {
            self.history_pos += 1;
        }
    }

    /// Undo last action
    fn undo(&mut self) {
        if self.history_pos > 0 {
            self.history_pos -= 1;
            if let Some(entry) = self.history.get(self.history_pos) {
                self.root = entry.root.clone();
                self.modified = true;
            }
        }
    }

    /// Redo last undone action
    fn redo(&mut self) {
        if self.history_pos < self.history.len() {
            if let Some(entry) = self.history.get(self.history_pos) {
                self.root = entry.root.clone();
                self.history_pos += 1;
                self.modified = true;
            }
        }
    }

    /// Add a node from a template
    fn add_node_from_template(&mut self, template: BtNodeType) {
        let position = [
            -self.canvas_offset.x / self.canvas_zoom + 200.0,
            -self.canvas_offset.y / self.canvas_zoom + 150.0,
        ];

        let new_node = BtNode::new(template, position);
        self.save_to_history("Add node");

        if let Some(root) = &mut self.root {
            if let Some(selected_id) = self.selected_node {
                if let Some(parent) = root.find_node_mut(selected_id) {
                    if parent.can_have_children() || parent.node_type.has_single_child() {
                        let _ = parent.add_child(new_node);
                        self.modified = true;
                        return;
                    }
                }
            }
        } else {
            self.root = Some(new_node);
            self.modified = true;
        }
    }

    /// Add a node at mouse position
    fn add_node_at_mouse(&mut self, node_type: BtNodeType, ui: &Ui) {
        if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
            let world_pos = self.screen_to_world(pos);
            let new_node = BtNode::new(node_type, [world_pos.x, world_pos.y]);
            self.save_to_history("Add node at mouse");

            if let Some(root) = &mut self.root {
                if let Some(selected_id) = self.selected_node {
                    if let Some(parent) = root.find_node_mut(selected_id) {
                        if parent.can_have_children() || parent.node_type.has_single_child() {
                            let _ = parent.add_child(new_node);
                            self.modified = true;
                            return;
                        }
                    }
                }
            } else {
                self.root = Some(new_node);
                self.modified = true;
            }
        }
    }

    /// Copy a node to clipboard
    fn copy_node(&mut self, node_id: NodeId) {
        if let Some(root) = &self.root {
            if let Some(node) = root.find_node(node_id) {
                self.clipboard = Some(node.clone());
            }
        }
    }

    /// Copy selected node
    fn copy_selected(&mut self) {
        if let Some(id) = self.selected_node {
            self.copy_node(id);
        }
    }

    /// Cut selected node
    fn cut_selected(&mut self) {
        if let Some(id) = self.selected_node {
            self.copy_node(id);
            self.delete_node(id);
        }
    }

    /// Paste from clipboard
    fn paste(&mut self) {
        if let Some(clipboard) = &self.clipboard {
            let mut new_node = clipboard.clone();
            new_node.id = NodeId::new();
            new_node.position[0] += 30.0;
            new_node.position[1] += 30.0;

            self.save_to_history("Paste node");

            if let Some(root) = &mut self.root {
                if let Some(selected_id) = self.selected_node {
                    if let Some(parent) = root.find_node_mut(selected_id) {
                        if parent.can_have_children() || parent.node_type.has_single_child() {
                            let _ = parent.add_child(new_node);
                            self.modified = true;
                            return;
                        }
                    }
                }
            }
        }
    }

    /// Delete a node
    fn delete_node(&mut self, node_id: NodeId) {
        self.save_to_history("Delete node");

        if let Some(root) = &mut self.root {
            if root.id == node_id {
                self.root = None;
            } else {
                root.remove_child(node_id);
            }
        }

        if self.selected_node == Some(node_id) {
            self.selected_node = None;
        }

        self.modified = true;
    }

    /// Duplicate a node
    fn duplicate_node(&mut self, node_id: NodeId) {
        let node_to_clone = self
            .root
            .as_ref()
            .and_then(|r| r.find_node(node_id))
            .cloned();

        if let Some(node) = node_to_clone {
            let mut new_node = node.clone();
            new_node.id = NodeId::new();
            new_node.position[0] += 40.0;
            new_node.position[1] += 40.0;

            self.save_to_history("Duplicate node");

            // Add to same parent
            let parent_id = self.find_parent(node_id);
            if let Some(parent) = parent_id {
                if let Some(root) = &mut self.root {
                    if let Some(p) = root.find_node_mut(parent) {
                        let _ = p.add_child(new_node);
                        self.modified = true;
                    }
                }
            }
        }
    }

    /// Find parent of a node
    fn find_parent(&self, child_id: NodeId) -> Option<NodeId> {
        let root = self.root.as_ref()?;
        self.find_parent_recursive(root, child_id)
    }

    fn find_parent_recursive(&self, node: &BtNode, target_id: NodeId) -> Option<NodeId> {
        if let Some(children) = node.children() {
            for child in children {
                if child.id == target_id {
                    return Some(node.id);
                }
                if let Some(parent) = self.find_parent_recursive(child, target_id) {
                    return Some(parent);
                }
            }
        }
        if let Some(child) = node.node_type.child() {
            if child.id == target_id {
                return Some(node.id);
            }
            if let Some(parent) = self.find_parent_recursive(child, target_id) {
                return Some(parent);
            }
        }
        None
    }

    /// Auto-layout the tree
    fn auto_layout_tree(&mut self) {
        if let Some(root) = &mut self.root {
            self.save_to_history("Auto layout");
            layout_node_recursive(root, 0.0, 0.0, 0);
            self.frame_all();
            self.modified = true;
        }
    }

    /// Frame all nodes in view
    fn frame_all(&mut self) {
        let (min_x, max_x, min_y, max_y) = self.calculate_tree_bounds();
        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        self.canvas_offset = Vec2::new(400.0 - center_x * self.canvas_zoom, 300.0 - center_y * self.canvas_zoom);
    }

    /// Convert world position to screen position
    fn world_to_screen(&self, world_pos: Pos2) -> Pos2 {
        Pos2::new(
            world_pos.x * self.canvas_zoom + self.canvas_offset.x,
            world_pos.y * self.canvas_zoom + self.canvas_offset.y,
        )
    }

    /// Convert screen position to world position
    fn screen_to_world(&self, screen_pos: Pos2) -> Pos2 {
        Pos2::new(
            (screen_pos.x - self.canvas_offset.x) / self.canvas_zoom,
            (screen_pos.y - self.canvas_offset.y) / self.canvas_zoom,
        )
    }

    /// Save to file
    fn save_to_file(&self) {
        if let Some(path) = &self.file_path {
            if let Some(json) = self.save_to_json() {
                let _ = std::fs::write(path, json);
            }
        }
    }

    /// Save dialog
    fn save_dialog(&mut self) {
        if self.file_path.is_some() {
            self.save_to_file();
        } else {
            self.save_as_dialog();
        }
    }

    /// Save as dialog
    fn save_as_dialog(&mut self) {
        // In a real implementation, this would open a file dialog
        // For now, we'll just mark as saved
        self.modified = false;
    }

    /// Open dialog
    fn open_dialog(&mut self) {
        // In a real implementation, this would open a file dialog
    }

    /// Export to JSON
    fn export_json(&mut self) {
        // Export functionality
        if let Some(_json) = self.save_to_json() {
            // Copy to clipboard or save to file
        }
    }

    /// Import from JSON
    fn import_json(&mut self) {
        // Import functionality
    }

    /// Save tree to JSON string
    pub fn save_to_json(&self) -> Option<String> {
        self.root.as_ref().and_then(|root| serde_json::to_string_pretty(root).ok())
    }

    /// Load tree from JSON string
    pub fn load_from_json(&mut self, json: &str) -> Result<(), serde_json::Error> {
        self.save_to_history("Load from JSON");
        self.root = Some(serde_json::from_str(json)?);
        self.modified = true;
        Ok(())
    }

    /// Save tree to file
    pub fn save_to_file_path(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = self
            .save_to_json()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Serialization failed"))?;
        std::fs::write(path, json)
    }

    /// Load tree from file
    pub fn load_from_file_path(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        let json = std::fs::read_to_string(path)?;
        self.load_from_json(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.file_path = Some(path.to_path_buf());
        self.modified = false;
        Ok(())
    }

    /// Compile the tree to runtime format
    pub fn compile(&self) -> Result<dde_core::ai::CompiledBehaviorTree, CompileError> {
        match &self.root {
            Some(root) => compile(root),
            None => Err(CompileError::Validation("No root node".to_string())),
        }
    }

    /// Set the root node
    pub fn set_root(&mut self, root: BtNode) {
        self.save_to_history("Set root");
        self.root = Some(root);
        self.modified = true;
    }

    /// Clear the editor
    pub fn clear(&mut self) {
        self.save_to_history("Clear");
        self.root = None;
        self.selected_node = None;
        self.modified = false;
        self.file_path = None;
    }
}

/// Recursive layout function
fn layout_node_recursive(node: &mut BtNode, x: f32, y: f32, depth: usize) {
    node.position[0] = x;
    node.position[1] = y;

    let vertical_spacing = 120.0;
    let horizontal_spacing = 180.0;

    if let Some(children) = node.children_mut() {
        let child_count = children.len();
        let total_width = (child_count.saturating_sub(1)) as f32 * horizontal_spacing;
        let start_x = x - total_width / 2.0;

        for (i, child) in children.iter_mut().enumerate() {
            let child_x = start_x + i as f32 * horizontal_spacing;
            let child_y = y + vertical_spacing;
            layout_node_recursive(child, child_x, child_y, depth + 1);
        }
    }

    if node.node_type.has_single_child() {
        if let Some(child) = node.node_type.child_mut() {
            layout_node_recursive(child, x, y + vertical_spacing, depth + 1);
        }
    }
}

/// Helper function to draw parallel policy combo box
fn draw_parallel_policy_combo(ui: &mut Ui, policy: &mut ParallelPolicy) {
    egui::ComboBox::from_id_source("parallel_policy")
        .selected_text(match policy {
            ParallelPolicy::RequireAll => "Require All",
            ParallelPolicy::RequireOne => "Require One",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(policy, ParallelPolicy::RequireAll, "Require All");
            ui.selectable_value(policy, ParallelPolicy::RequireOne, "Require One");
        });
}

/// Helper function to draw move speed combo box
fn draw_move_speed_combo(ui: &mut Ui, speed: &mut MoveSpeed) {
    egui::ComboBox::from_id_source("move_speed")
        .selected_text(match speed {
            MoveSpeed::Walk => "Walk",
            MoveSpeed::Run => "Run",
            MoveSpeed::Sprint => "Sprint",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(speed, MoveSpeed::Walk, "Walk");
            ui.selectable_value(speed, MoveSpeed::Run, "Run");
            ui.selectable_value(speed, MoveSpeed::Sprint, "Sprint");
        });
}

/// Get icon for a node category
fn category_icon(category: &NodeCategory) -> &'static str {
    match category {
        NodeCategory::Composite => "🔀",
        NodeCategory::Decorator => "🔧",
        NodeCategory::Condition => "❓",
        NodeCategory::Action => "⚡",
    }
}

/// Extension trait for egui input state
#[derive(Debug, Clone, Copy)]
pub struct EditorConfig {
    pub grid_size: f32,
    pub snap_to_grid: bool,
    pub show_node_ids: bool,
    pub theme: EditorTheme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTheme {
    Dark,
    Light,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            grid_size: 20.0,
            snap_to_grid: true,
            show_node_ids: false,
            theme: EditorTheme::Dark,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_creation() {
        let editor = BehaviorTreeVisualEditor::new();
        assert!(editor.root().is_none());
        assert!(!editor.is_modified());
    }

    #[test]
    fn test_new_tree() {
        let mut editor = BehaviorTreeVisualEditor::new();
        editor.new_tree();
        assert!(editor.root().is_some());
        assert!(editor.is_modified());
    }

    #[test]
    fn test_world_screen_conversion() {
        let editor = BehaviorTreeVisualEditor::new();
        let world_pos = Pos2::new(100.0, 100.0);
        let screen_pos = editor.world_to_screen(world_pos);
        let back_to_world = editor.screen_to_world(screen_pos);

        assert!((world_pos.x - back_to_world.x).abs() < 0.001);
        assert!((world_pos.y - back_to_world.y).abs() < 0.001);
    }

    #[test]
    fn test_node_counting() {
        let mut editor = BehaviorTreeVisualEditor::new();
        assert_eq!(editor.count_nodes(), 0);

        editor.new_tree();
        assert_eq!(editor.count_nodes(), 1);
    }

    #[test]
    fn test_json_save_load() {
        let mut editor = BehaviorTreeVisualEditor::new();
        editor.new_tree();

        let json = editor.save_to_json();
        assert!(json.is_some());

        let mut editor2 = BehaviorTreeVisualEditor::new();
        assert!(editor2.load_from_json(&json.unwrap()).is_ok());
        assert_eq!(editor2.count_nodes(), 1);
    }

    #[test]
    fn test_undo_redo() {
        let mut editor = BehaviorTreeVisualEditor::new();
        editor.new_tree();
        editor.save_to_history("test");

        let initial_pos = editor.root().unwrap().position;

        // Modify
        if let Some(root) = editor.root_mut() {
            root.position[0] += 100.0;
        }
        editor.save_to_history("move");

        // Undo
        editor.undo();
        let after_undo = editor.root().unwrap().position;
        assert_eq!(after_undo[0], initial_pos[0]);

        // Redo
        editor.redo();
        let after_redo = editor.root().unwrap().position;
        assert_eq!(after_redo[0], initial_pos[0] + 100.0);
    }
}
