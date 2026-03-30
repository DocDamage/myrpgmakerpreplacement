//! Visual behavior tree editor
//!
//! This module provides a node-based visual editor for creating and editing
//! behavior trees using egui.

use dde_core::ai::NodeId;
use egui::{Color32, Id, Key, PointerButton, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};

use super::debugger::{draw_node_with_status, status_color, BtDebugger};
use super::nodes::{BtNode, BtNodeType, NodeCategory};

/// Pin/socket direction for connections
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinDirection {
    Input,
    Output,
}

/// Visual behavior tree editor
#[derive(Debug, Clone)]
pub struct BehaviorTreeEditor {
    /// Root node of the tree
    root: Option<BtNode>,
    /// Currently selected node
    selected: Option<NodeId>,
    /// Node being dragged
    dragging: Option<NodeId>,
    /// Connection being drawn
    drawing_connection: Option<(NodeId, PinDirection)>,

    /// Canvas view offset
    canvas_offset: Vec2,
    /// Canvas zoom level
    canvas_zoom: f32,
    /// Node palette filter
    palette_filter: String,
    /// Show grid
    show_grid: bool,
    /// Grid size
    grid_size: f32,
    /// Show minimap
    show_minimap: bool,
    /// Node ID counter for new nodes
    _node_id_counter: u64,
    /// Clipboard for copy/paste
    clipboard: Option<BtNode>,
    /// Modified flag
    modified: bool,
    /// File path for save/load
    _file_path: Option<std::path::PathBuf>,
}

impl Default for BehaviorTreeEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl BehaviorTreeEditor {
    /// Create a new behavior tree editor
    pub fn new() -> Self {
        Self {
            root: None,
            selected: None,
            dragging: None,
            drawing_connection: None,
            canvas_offset: Vec2::new(100.0, 50.0),
            canvas_zoom: 1.0,
            palette_filter: String::new(),
            show_grid: true,
            grid_size: 20.0,
            show_minimap: false,
            _node_id_counter: 1,
            clipboard: None,
            modified: false,
            _file_path: None,
        }
    }

    /// Create a new tree with a root node
    pub fn new_tree(&mut self) {
        self.root = Some(BtNode::new(
            BtNodeType::Selector {
                children: Vec::new(),
            },
            [0.0, 0.0],
        ));
        self.selected = None;
        self.modified = true;
    }

    /// Get the root node
    pub fn root(&self) -> Option<&BtNode> {
        self.root.as_ref()
    }

    /// Get mutable root
    pub fn root_mut(&mut self) -> Option<&mut BtNode> {
        self.root.as_mut()
    }

    /// Check if the editor has unsaved changes
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Draw the editor UI
    pub fn draw_ui(&mut self, ui: &mut egui::Ui, debugger: Option<&BtDebugger>) {
        // Toolbar
        self.draw_toolbar(ui);

        ui.separator();

        // Main editor area
        let _available_size = ui.available_size();

        egui::SidePanel::right("bt_properties")
            .default_width(250.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                self.draw_properties_panel(ui, debugger);
            });

        egui::SidePanel::left("bt_palette")
            .default_width(200.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                self.draw_palette(ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.draw_canvas(ui, debugger);
        });
    }

    /// Draw the toolbar
    fn draw_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("📝 New").clicked() {
                self.new_tree();
            }

            ui.add_enabled_ui(self.root.is_some(), |ui| {
                if ui.button("💾 Save").clicked() {
                    // TODO: Implement save
                    self.modified = false;
                }
            });

            if ui.button("📂 Open").clicked() {
                // TODO: Implement open
            }

            ui.separator();

            if ui.button("▶ Export").clicked() {
                // TODO: Implement export
            }

            ui.separator();

            ui.checkbox(&mut self.show_grid, "Grid");
            ui.checkbox(&mut self.show_minimap, "Minimap");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Zoom: {:.0}%", self.canvas_zoom * 100.0));
            });
        });
    }

    /// Draw the node palette sidebar
    fn draw_palette(&mut self, ui: &mut Ui) {
        ui.heading("Node Palette");
        ui.separator();

        // Filter
        ui.text_edit_singleline(&mut self.palette_filter);
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for category in NodeCategory::all() {
                let header_text =
                    format!("{} {}", category_icon(category), category.display_name());

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
    }

    /// Draw the canvas with nodes
    fn draw_canvas(&mut self, ui: &mut Ui, debugger: Option<&BtDebugger>) {
        let available_rect = ui.available_rect_before_wrap();

        // Canvas background
        let canvas_rect = available_rect;
        let canvas_response =
            ui.interact(canvas_rect, Id::new("bt_canvas"), Sense::click_and_drag());

        // Pan canvas
        if canvas_response.dragged_by(PointerButton::Middle)
            || (canvas_response.dragged_by(PointerButton::Primary)
                && ui.input(|i| i.modifiers.shift))
        {
            self.canvas_offset += canvas_response.drag_delta();
        }

        // Zoom
        ui.input(|i| {
            let scroll_delta = i.raw_scroll_delta;
            if scroll_delta.y != 0.0 {
                let zoom_delta = if scroll_delta.y > 0.0 { 1.1 } else { 0.9 };
                self.canvas_zoom = (self.canvas_zoom * zoom_delta).clamp(0.25, 4.0);
            }
        });

        // Draw grid
        if self.show_grid {
            self.draw_grid(ui, canvas_rect);
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

        // Handle canvas click (deselect)
        if canvas_response.clicked() {
            self.selected = None;
            self.drawing_connection = None;
        }

        // Handle keyboard shortcuts
        self.handle_keyboard_shortcuts(ui);

        // Draw minimap
        if self.show_minimap {
            self.draw_minimap(ui, canvas_rect);
        }
    }

    /// Draw the grid background
    fn draw_grid(&self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter();
        let grid_color = Color32::from_gray(40);

        let offset_x = self.canvas_offset.x % self.grid_size;
        let offset_y = self.canvas_offset.y % self.grid_size;

        // Vertical lines
        let mut x = rect.left() + offset_x;
        while x < rect.right() {
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(1.0, grid_color),
            );
            x += self.grid_size * self.canvas_zoom;
        }

        // Horizontal lines
        let mut y = rect.top() + offset_y;
        while y < rect.bottom() {
            painter.line_segment(
                [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                Stroke::new(1.0, grid_color),
            );
            y += self.grid_size * self.canvas_zoom;
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    /// Draw connections between nodes
    fn draw_connections(&self, ui: &mut Ui, node: &BtNode, debugger: Option<&BtDebugger>) {
        let painter = ui.painter();
        let parent_pos = self.world_to_screen(Pos2::new(node.position[0], node.position[1]));

        let parent_output = parent_pos + Vec2::new(0.0, 30.0); // Bottom center of parent

        // Collect child positions first to avoid borrow issues
        let mut connections: Vec<(Pos2, Pos2, Color32, Option<usize>)> = Vec::new();
        let mut children_to_recurse: Vec<&BtNode> = Vec::new();

        // Draw connections to children
        if let Some(children) = node.children() {
            for (i, child) in children.iter().enumerate() {
                let child_pos =
                    self.world_to_screen(Pos2::new(child.position[0], child.position[1]));
                let child_input = child_pos - Vec2::new(0.0, 30.0); // Top center of child

                let color = if let Some(dbg) = debugger {
                    if dbg.is_in_execution_path(node.id) && dbg.is_in_execution_path(child.id) {
                        status_color(dde_core::ai::BtStatus::Running)
                    } else {
                        Color32::from_gray(100)
                    }
                } else {
                    Color32::from_gray(100)
                };

                let order = if matches!(node.node_type, BtNodeType::Sequence { .. }) {
                    Some(i + 1)
                } else {
                    None
                };

                connections.push((parent_output, child_input, color, order));
                children_to_recurse.push(child);
            }
        }

        // Draw single child connection
        if let Some(child) = node.node_type.child() {
            let child_pos = self.world_to_screen(Pos2::new(child.position[0], child.position[1]));
            let child_input = child_pos - Vec2::new(0.0, 30.0);
            let color = Color32::from_gray(100);
            connections.push((parent_output, child_input, color, None));
            children_to_recurse.push(child);
        }

        // Draw all connections
        for (start, end, color, order) in connections {
            let control_offset = ((end.y - start.y) / 2.0).max(50.0);
            let cp1 = start + Vec2::new(0.0, control_offset);
            let cp2 = end - Vec2::new(0.0, control_offset);

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
                painter.text(
                    mid_point,
                    egui::Align2::CENTER_CENTER,
                    format!("{}", idx),
                    egui::FontId::proportional(10.0),
                    Color32::WHITE,
                );
            }
        }

        // Recursively draw child connections
        for child in children_to_recurse {
            self.draw_connections(ui, child, debugger);
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    /// Draw a node and its children recursively
    fn draw_node_recursive(&mut self, ui: &mut Ui, node: BtNode, debugger: Option<&BtDebugger>) {
        let screen_pos = self.world_to_screen(Pos2::new(node.position[0], node.position[1]));
        let node_size = Vec2::new(140.0, 60.0);
        let node_rect = Rect::from_center_size(screen_pos, node_size);

        let node_id = node.id;
        let response = ui.interact(node_rect, Id::new(node_id.0), Sense::click_and_drag());

        // Handle dragging
        if response.dragged_by(PointerButton::Primary) && self.dragging.is_none() {
            self.dragging = Some(node_id);
        }

        if self.dragging == Some(node_id) {
            if response.drag_stopped() {
                self.dragging = None;
                self.modified = true;
            } else {
                // Update position
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
            self.selected = Some(node_id);
        }

        // Draw node with debug status
        draw_node_with_status(ui, node_rect, node_id, debugger, |ui| {
            let color = node.color();
            let color32 = Color32::from_rgb(color[0], color[1], color[2]);

            // Selection highlight
            if self.selected == Some(node_id) {
                ui.painter().rect_stroke(
                    node_rect.expand(3.0),
                    5.0,
                    Stroke::new(2.0, Color32::WHITE),
                );
            }

            // Node background
            let bg_color = if self.selected == Some(node_id) {
                Color32::from_gray(60)
            } else {
                Color32::from_gray(45)
            };

            ui.painter().rect_filled(node_rect, 5.0, bg_color);
            ui.painter()
                .rect_stroke(node_rect, 5.0, Stroke::new(2.0, color32));

            // Header bar
            let header_rect = Rect::from_min_max(
                node_rect.min,
                Pos2::new(node_rect.max.x, node_rect.min.y + 20.0),
            );
            ui.painter().rect_filled(header_rect, 5.0, color32);

            // Icon and title
            ui.painter().text(
                header_rect.left_center() + Vec2::new(8.0, 0.0),
                egui::Align2::LEFT_CENTER,
                format!("{}", node.icon()),
                egui::FontId::proportional(14.0),
                Color32::WHITE,
            );

            ui.painter().text(
                header_rect.center(),
                egui::Align2::CENTER_CENTER,
                node.display_name(),
                egui::FontId::proportional(11.0),
                Color32::WHITE,
            );

            // Comment preview
            if let Some(comment) = &node.comment {
                ui.painter().text(
                    node_rect.center_bottom() - Vec2::new(0.0, 5.0),
                    egui::Align2::CENTER_BOTTOM,
                    if comment.len() > 15 {
                        &comment[..15]
                    } else {
                        comment
                    },
                    egui::FontId::proportional(9.0),
                    Color32::GRAY,
                );
            }

            // Input socket (top)
            let input_pos = node_rect.center_top();
            ui.painter().circle_filled(input_pos, 5.0, Color32::WHITE);

            // Output socket (bottom)
            let output_pos = node_rect.center_bottom();
            ui.painter().circle_filled(output_pos, 5.0, Color32::WHITE);
        });

        // Draw children
        if let Some(children) = node.children() {
            for child in children.clone() {
                self.draw_node_recursive(ui, child, debugger);
            }
        }
        if let Some(child) = node.node_type.child() {
            self.draw_node_recursive(ui, child.clone(), debugger);
        }
    }

    /// Draw the properties panel
    fn draw_properties_panel(&mut self, ui: &mut Ui, debugger: Option<&BtDebugger>) {
        ui.heading("Properties");
        ui.separator();

        if let Some(selected_id) = self.selected {
            // Clone the node for editing to avoid borrow issues
            let node_clone = self
                .root
                .as_ref()
                .and_then(|r| r.find_node(selected_id))
                .cloned();

            if let Some(mut node) = node_clone {
                ui.label(format!("Node ID: {:?}", node.id));
                ui.label(format!("Type: {}", node.display_name()));
                ui.separator();

                // Position
                ui.horizontal(|ui| {
                    ui.label("Position:");
                    ui.add(egui::DragValue::new(&mut node.position[0]).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut node.position[1]).prefix("Y: "));
                });

                // Comment
                ui.label("Comment:");
                let mut comment = node.comment.clone().unwrap_or_default();
                if ui.text_edit_multiline(&mut comment).changed() {
                    node.comment = if comment.is_empty() {
                        None
                    } else {
                        Some(comment)
                    };
                    self.modified = true;
                }

                ui.separator();

                // Type-specific properties
                self.draw_node_properties(ui, &mut node);

                ui.separator();

                // Actions
                if ui.button("Delete Node").clicked() {
                    self.delete_node(selected_id);
                }

                if ui.button("Duplicate").clicked() {
                    self.duplicate_node(selected_id);
                }

                // Apply changes back to the tree
                if self.modified {
                    if let Some(root) = &mut self.root {
                        if let Some(target) = root.find_node_mut(selected_id) {
                            *target = node;
                        }
                    }
                }
            }
        } else {
            ui.label(RichText::new("Select a node to edit properties").color(Color32::GRAY));
        }

        // Debugger section
        if let Some(dbg) = debugger {
            ui.separator();
            ui.heading("Debug Info");

            if let Some(entity) = dbg.target_entity() {
                ui.label(format!("Debugging: {:?}", entity));
            }
        }
    }

    /// Draw type-specific node properties
    fn draw_node_properties(&mut self, ui: &mut Ui, node: &mut BtNode) {
        match &mut node.node_type {
            BtNodeType::IsPlayerNearby { radius } => {
                ui.label("Radius:");
                ui.add(egui::DragValue::new(radius).speed(0.1).range(0.0..=100.0));
            }
            BtNodeType::HealthBelow { percent } => {
                ui.label("Health %:");
                ui.add(egui::Slider::new(percent, 0.0..=1.0));
            }
            BtNodeType::RandomChance { percent } => {
                ui.label("Chance %:");
                ui.add(egui::Slider::new(percent, 0..=100));
            }
            BtNodeType::Wait { seconds } => {
                ui.label("Seconds:");
                ui.add(
                    egui::DragValue::new(seconds)
                        .speed(0.1)
                        .range(0.0..=f32::MAX),
                );
            }
            BtNodeType::Cooldown { seconds, .. } => {
                ui.label("Cooldown (s):");
                ui.add(
                    egui::DragValue::new(seconds)
                        .speed(0.1)
                        .range(0.01..=f32::MAX),
                );
            }
            BtNodeType::Repeater { count, .. } => {
                ui.label("Repeat count (0 = forever):");
                let mut count_val = count.unwrap_or(0);
                if ui
                    .add(egui::DragValue::new(&mut count_val).range(0..=1000))
                    .changed()
                {
                    *count = if count_val == 0 {
                        None
                    } else {
                        Some(count_val)
                    };
                }
            }
            BtNodeType::Parallel {
                success_policy,
                failure_policy,
                ..
            } => {
                ui.label("Success policy:");
                draw_parallel_policy_combo(ui, success_policy);
                ui.label("Failure policy:");
                draw_parallel_policy_combo(ui, failure_policy);
            }
            BtNodeType::MoveTo { speed, .. } => {
                ui.label("Speed:");
                draw_move_speed_combo(ui, speed);
            }
            BtNodeType::Follow { distance, .. } => {
                ui.label("Distance:");
                ui.add(egui::DragValue::new(distance).speed(0.1).range(0.0..=50.0));
            }
            BtNodeType::UseSkill { skill_id, .. } => {
                ui.label("Skill ID:");
                ui.add(egui::DragValue::new(skill_id));
            }
            BtNodeType::UseItem { item_id } => {
                ui.label("Item ID:");
                ui.add(egui::DragValue::new(item_id));
            }
            BtNodeType::PlayAnimation { anim_id } => {
                ui.label("Animation ID:");
                ui.add(egui::DragValue::new(anim_id));
            }
            BtNodeType::Speak { dialogue_id } => {
                ui.label("Dialogue ID:");
                ui.add(egui::DragValue::new(dialogue_id));
            }
            BtNodeType::CustomCondition { script } | BtNodeType::CustomAction { script } => {
                ui.label("Script:");
                ui.text_edit_multiline(script);
            }
            _ => {
                ui.label("No editable properties");
            }
        }
    }

    /// Draw minimap
    fn draw_minimap(&self, ui: &mut Ui, canvas_rect: Rect) {
        let minimap_size = Vec2::new(150.0, 100.0);
        let minimap_rect = Rect::from_min_size(
            canvas_rect.right_bottom() - minimap_size - Vec2::new(10.0, 10.0),
            minimap_size,
        );

        let painter = ui.painter();
        painter.rect_filled(minimap_rect, 5.0, Color32::from_black_alpha(200));
        painter.rect_stroke(minimap_rect, 5.0, (1.0, Color32::WHITE));

        // TODO: Draw simplified node representation
    }

    /// Handle keyboard shortcuts
    fn handle_keyboard_shortcuts(&mut self, ui: &Ui) {
        ui.input(|i| {
            // Delete
            if i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace) {
                if let Some(id) = self.selected {
                    self.delete_node(id);
                }
            }

            // Copy
            if i.modifiers.ctrl && i.key_pressed(Key::C) {
                if let Some(id) = self.selected {
                    if let Some(root) = &self.root {
                        if let Some(node) = root.find_node(id) {
                            self.clipboard = Some(node.clone());
                        }
                    }
                }
            }

            // Paste
            if i.modifiers.ctrl && i.key_pressed(Key::V) {
                if let Some(clipboard) = &self.clipboard {
                    let mut new_node = clipboard.clone();
                    new_node.id = NodeId::new();
                    new_node.position[0] += 20.0;
                    new_node.position[1] += 20.0;

                    if let Some(root) = &mut self.root {
                        // Add as child of currently selected node, or to root
                        if let Some(selected_id) = self.selected {
                            if let Some(parent) = root.find_node_mut(selected_id) {
                                let _ = parent.add_child(new_node);
                            }
                        }
                    }
                    self.modified = true;
                }
            }

            // Frame all (reset view)
            if i.key_pressed(Key::F) {
                self.canvas_offset = Vec2::new(100.0, 50.0);
                self.canvas_zoom = 1.0;
            }
        });
    }

    /// Add a node from a template
    fn add_node_from_template(&mut self, template: BtNodeType) {
        let position = [
            -self.canvas_offset.x / self.canvas_zoom + 100.0,
            -self.canvas_offset.y / self.canvas_zoom + 100.0,
        ];

        let new_node = BtNode::new(template, position);

        if let Some(root) = &mut self.root {
            if let Some(selected_id) = self.selected {
                if let Some(parent) = root.find_node_mut(selected_id) {
                    if parent.can_have_children() || parent.node_type.has_single_child() {
                        let _ = parent.add_child(new_node);
                        self.modified = true;
                        return;
                    }
                }
            }

            // If can't add as child, replace root
            self.root = Some(new_node);
        } else {
            self.root = Some(new_node);
        }

        self.modified = true;
    }

    /// Delete a node
    fn delete_node(&mut self, node_id: NodeId) {
        if let Some(root) = &mut self.root {
            if root.id == node_id {
                self.root = None;
            } else {
                root.remove_child(node_id);
            }
        }

        if self.selected == Some(node_id) {
            self.selected = None;
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
            new_node.position[0] += 20.0;
            new_node.position[1] += 20.0;

            // Add to same parent
            let parent_id = self.find_parent(node_id);
            if let Some(parent) = parent_id {
                if let Some(root) = &mut self.root {
                    if let Some(p) = root.find_node_mut(parent) {
                        let _ = p.add_child(new_node);
                        self.modified = true;
                    }
                }
            } else {
                // No parent found, replace root
                self.root = Some(new_node);
                self.modified = true;
            }
        }
    }

    /// Find parent of a node
    fn find_parent(&self, child_id: NodeId) -> Option<NodeId> {
        let root = self.root.as_ref()?;
        self.find_parent_recursive(root, child_id)
    }

    #[allow(clippy::only_used_in_recursion)]
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

    /// Convert world position to screen position
    fn world_to_screen(&self, world_pos: Pos2) -> Pos2 {
        Pos2::new(
            world_pos.x * self.canvas_zoom + self.canvas_offset.x,
            world_pos.y * self.canvas_zoom + self.canvas_offset.y,
        )
    }

    #[allow(dead_code)]
    /// Convert screen position to world position
    fn screen_to_world(&self, screen_pos: Pos2) -> Pos2 {
        Pos2::new(
            (screen_pos.x - self.canvas_offset.x) / self.canvas_zoom,
            (screen_pos.y - self.canvas_offset.y) / self.canvas_zoom,
        )
    }

    /// Save tree to JSON
    pub fn save_to_json(&self) -> Option<String> {
        self.root
            .as_ref()
            .and_then(|root| serde_json::to_string_pretty(root).ok())
    }

    /// Load tree from JSON
    pub fn load_from_json(&mut self, json: &str) -> Result<(), serde_json::Error> {
        self.root = Some(serde_json::from_str(json)?);
        self.modified = true;
        Ok(())
    }
}

/// Helper function to draw parallel policy combo box
fn draw_parallel_policy_combo(ui: &mut Ui, policy: &mut super::nodes::ParallelPolicy) {
    egui::ComboBox::from_id_source("parallel_policy")
        .selected_text(match policy {
            super::nodes::ParallelPolicy::RequireAll => "Require All",
            super::nodes::ParallelPolicy::RequireOne => "Require One",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(
                policy,
                super::nodes::ParallelPolicy::RequireAll,
                "Require All",
            );
            ui.selectable_value(
                policy,
                super::nodes::ParallelPolicy::RequireOne,
                "Require One",
            );
        });
}

/// Helper function to draw move speed combo box
fn draw_move_speed_combo(ui: &mut Ui, speed: &mut super::nodes::MoveSpeed) {
    egui::ComboBox::from_id_source("move_speed")
        .selected_text(match speed {
            super::nodes::MoveSpeed::Walk => "Walk",
            super::nodes::MoveSpeed::Run => "Run",
            super::nodes::MoveSpeed::Sprint => "Sprint",
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(speed, super::nodes::MoveSpeed::Walk, "Walk");
            ui.selectable_value(speed, super::nodes::MoveSpeed::Run, "Run");
            ui.selectable_value(speed, super::nodes::MoveSpeed::Sprint, "Sprint");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_creation() {
        let editor = BehaviorTreeEditor::new();
        assert!(editor.root().is_none());
        assert!(!editor.is_modified());
    }

    #[test]
    fn test_new_tree() {
        let mut editor = BehaviorTreeEditor::new();
        editor.new_tree();
        assert!(editor.root().is_some());
        assert!(editor.is_modified());
    }

    #[test]
    fn test_world_screen_conversion() {
        let editor = BehaviorTreeEditor::new();
        let world_pos = Pos2::new(100.0, 100.0);
        let screen_pos = editor.world_to_screen(world_pos);
        let back_to_world = editor.screen_to_world(screen_pos);

        assert!((world_pos.x - back_to_world.x).abs() < 0.001);
        assert!((world_pos.y - back_to_world.y).abs() < 0.001);
    }
}
