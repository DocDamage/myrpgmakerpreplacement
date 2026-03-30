//! Visual Scripting Canvas
//!
//! Provides a node-based visual editor using egui with:
//! - Canvas with pan/zoom
//! - Node rendering with pins
//! - Connection dragging between pins
//! - Node selection and movement
//! - Right-click context menu for adding nodes
//! - Mini-map overview

use super::nodes::{
    get_node_categories, Node, NodeCategory, NodeId, Pin, PinId, PinType,
};
use egui::{pos2, vec2, Color32, Pos2, Rect, Response, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A connection between two pins
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Connection {
    pub source_node: NodeId,
    pub source_pin: PinId,
    pub target_node: NodeId,
    pub target_pin: PinId,
}

impl Connection {
    /// Create a new connection
    pub fn new(
        source_node: NodeId,
        source_pin: PinId,
        target_node: NodeId,
        target_pin: PinId,
    ) -> Self {
        Self {
            source_node,
            source_pin,
            target_node,
            target_pin,
        }
    }
}

/// The node graph containing all nodes and connections
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeGraph {
    pub nodes: HashMap<NodeId, Node>,
    pub connections: Vec<Connection>,
}

impl NodeGraph {
    /// Create a new empty graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: Node) -> NodeId {
        let id = node.id;
        self.nodes.insert(id, node);
        id
    }

    /// Remove a node and all its connections
    pub fn remove_node(&mut self, node_id: NodeId) {
        self.nodes.remove(&node_id);
        self.connections
            .retain(|c| c.source_node != node_id && c.target_node != node_id);
    }

    /// Add a connection between pins
    pub fn add_connection(&mut self, connection: Connection) -> bool {
        // Check if connection already exists
        if self.connections.contains(&connection) {
            return false;
        }

        // Validate the connection
        if let (Some(source_node), Some(target_node)) =
            (self.nodes.get(&connection.source_node), self.nodes.get(&connection.target_node))
        {
            if let (Some(source_pin), Some(target_pin)) = (
                source_node.get_pin(connection.source_pin),
                target_node.get_pin(connection.target_pin),
            ) {
                // Check type compatibility
                if !source_pin.pin_type.can_connect_to(&target_pin.pin_type) {
                    return false;
                }

                // For data pins, only allow one connection to an input
                if target_pin.pin_type != PinType::Execution {
                    self.connections.retain(|c| {
                        !(c.target_node == connection.target_node
                            && c.target_pin == connection.target_pin)
                    });
                }

                self.connections.push(connection);
                return true;
            }
        }

        false
    }

    /// Remove a connection
    pub fn remove_connection(&mut self, connection: &Connection) {
        self.connections.retain(|c| c != connection);
    }

    /// Get connections from a specific pin
    pub fn get_connections_from(&self, node_id: NodeId, pin_id: PinId) -> Vec<&Connection> {
        self.connections
            .iter()
            .filter(|c| c.source_node == node_id && c.source_pin == pin_id)
            .collect()
    }

    /// Get connections to a specific pin
    pub fn get_connections_to(&self, node_id: NodeId, pin_id: PinId) -> Vec<&Connection> {
        self.connections
            .iter()
            .filter(|c| c.target_node == node_id && c.target_pin == pin_id)
            .collect()
    }

    /// Get all connections for a node
    pub fn get_node_connections(&self, node_id: NodeId) -> Vec<&Connection> {
        self.connections
            .iter()
            .filter(|c| c.source_node == node_id || c.target_node == node_id)
            .collect()
    }

    /// Clear all nodes and connections
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.connections.clear();
    }

    /// Get event nodes (nodes without execution inputs)
    pub fn get_event_nodes(&self) -> Vec<&Node> {
        self.nodes.values().filter(|n| n.is_event_node()).collect()
    }
}

/// Current interaction state for the canvas
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CanvasInteraction {
    None,
    Panning,
    Selecting,
    DraggingNode,
    DraggingConnection { source_node: NodeId, source_pin: PinId },
}

/// Selection state
#[derive(Debug, Clone, Default)]
pub struct Selection {
    pub selected_nodes: HashSet<NodeId>,
    pub hovered_node: Option<NodeId>,
    pub hovered_pin: Option<(NodeId, PinId)>,
}

impl Selection {
    /// Check if a node is selected
    pub fn is_selected(&self, node_id: NodeId) -> bool {
        self.selected_nodes.contains(&node_id)
    }

    /// Select a single node
    pub fn select_single(&mut self, node_id: NodeId) {
        self.selected_nodes.clear();
        self.selected_nodes.insert(node_id);
    }

    /// Toggle node selection
    pub fn toggle_selection(&mut self, node_id: NodeId) {
        if self.selected_nodes.contains(&node_id) {
            self.selected_nodes.remove(&node_id);
        } else {
            self.selected_nodes.insert(node_id);
        }
    }

    /// Add to selection
    pub fn add_to_selection(&mut self, node_id: NodeId) {
        self.selected_nodes.insert(node_id);
    }

    /// Clear selection
    pub fn clear(&mut self) {
        self.selected_nodes.clear();
        self.hovered_node = None;
        self.hovered_pin = None;
    }
}

/// Visual styling for the node canvas
#[derive(Debug, Clone)]
pub struct CanvasStyle {
    pub node_corner_radius: f32,
    pub node_header_height: f32,
    pub node_pin_radius: f32,
    pub node_pin_spacing: f32,
    pub node_min_width: f32,
    pub node_padding: f32,
    pub connection_thickness: f32,
    pub grid_size: f32,
    pub grid_color: Color32,
    pub background_color: Color32,
    pub selection_color: Color32,
    pub node_shadow: bool,
}

impl Default for CanvasStyle {
    fn default() -> Self {
        Self {
            node_corner_radius: 6.0,
            node_header_height: 28.0,
            node_pin_radius: 6.0,
            node_pin_spacing: 24.0,
            node_min_width: 140.0,
            node_padding: 10.0,
            connection_thickness: 2.0,
            grid_size: 20.0,
            grid_color: Color32::from_gray(40),
            background_color: Color32::from_gray(25),
            selection_color: Color32::from_rgb(100, 150, 255),
            node_shadow: true,
        }
    }
}

/// The node canvas editor
#[derive(Debug)]
pub struct NodeCanvas {
    pub graph: NodeGraph,
    pub camera_offset: Vec2,
    pub zoom: f32,
    pub selection: Selection,
    pub style: CanvasStyle,
    pub show_grid: bool,
    pub show_minimap: bool,
    interaction: CanvasInteraction,
    drag_start: Option<Pos2>,
    last_mouse_pos: Option<Pos2>,
    node_categories: Vec<NodeCategory>,
    context_menu_pos: Option<Pos2>,
}

impl Default for NodeCanvas {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeCanvas {
    /// Create a new canvas
    pub fn new() -> Self {
        Self {
            graph: NodeGraph::new(),
            camera_offset: Vec2::ZERO,
            zoom: 1.0,
            selection: Selection::default(),
            style: CanvasStyle::default(),
            show_grid: true,
            show_minimap: true,
            interaction: CanvasInteraction::None,
            drag_start: None,
            last_mouse_pos: None,
            node_categories: get_node_categories(),
            context_menu_pos: None,
        }
    }

    /// Load a graph into the canvas
    pub fn load_graph(&mut self, graph: NodeGraph) {
        self.graph = graph;
        self.selection.clear();
    }

    /// Get the graph reference
    pub fn graph(&self) -> &NodeGraph {
        &self.graph
    }

    /// Get the mutable graph reference
    pub fn graph_mut(&mut self) -> &mut NodeGraph {
        &mut self.graph
    }

    /// Convert screen position to canvas position
    pub fn screen_to_canvas(&self, screen_pos: Pos2, rect: &Rect) -> Pos2 {
        let center = rect.center();
        pos2(
            (screen_pos.x - center.x) / self.zoom - self.camera_offset.x + center.x,
            (screen_pos.y - center.y) / self.zoom - self.camera_offset.y + center.y,
        )
    }

    /// Convert canvas position to screen position
    pub fn canvas_to_screen(&self, canvas_pos: Pos2, rect: &Rect) -> Pos2 {
        let center = rect.center();
        pos2(
            (canvas_pos.x - center.x + self.camera_offset.x) * self.zoom + center.x,
            (canvas_pos.y - center.y + self.camera_offset.y) * self.zoom + center.y,
        )
    }

    /// Draw the canvas
    pub fn draw(&mut self, ui: &mut egui::Ui) -> Response {
        let available_size = ui.available_size();
        let (id, rect) = ui.allocate_space(available_size);

        // Handle input
        let response = ui.interact(rect, id, egui::Sense::click_and_drag());

        // Context menu
        self.handle_context_menu(ui, &response, &rect);

        // Handle interactions
        self.handle_input(ui, &response, &rect);

        // Draw background
        self.draw_background(ui, &rect);

        // Draw grid
        if self.show_grid {
            self.draw_grid(ui, &rect);
        }

        // Draw connections first (behind nodes)
        self.draw_connections(ui, &rect);

        // Draw active connection being dragged
        if let CanvasInteraction::DraggingConnection { source_node, source_pin } = self.interaction {
            if let (Some(mouse_pos), Some(node)) =
                (self.last_mouse_pos, self.graph.nodes.get(&source_node))
            {
                if let Some(pin) = node.get_pin(source_pin) {
                    let pin_pos = self.get_pin_screen_position(node, pin, &rect);
                    self.draw_connection_line(ui, pin_pos, mouse_pos, pin.pin_type.color());
                }
            }
        }

        // Draw nodes
        self.draw_nodes(ui, &rect);

        // Draw selection box
        if let CanvasInteraction::Selecting = self.interaction {
            if let (Some(start), Some(current)) = (self.drag_start, self.last_mouse_pos) {
                let selection_rect = Rect::from_two_pos(start, current);
                ui.painter().rect_stroke(
                    selection_rect,
                    0.0,
                    Stroke::new(1.0, self.style.selection_color),
                );
                ui.painter().rect_filled(
                    selection_rect,
                    0.0,
                    self.style.selection_color.gamma_multiply(0.2),
                );
            }
        }

        // Draw minimap
        if self.show_minimap {
            self.draw_minimap(ui, &rect);
        }

        response
    }

    /// Handle context menu
    fn handle_context_menu(&mut self, ui: &mut egui::Ui, response: &Response, rect: &Rect) {
        // Check for right-click to open context menu
        if response.secondary_clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                self.context_menu_pos = Some(pos);
            }
        }

        // Show context menu
        if let Some(menu_pos) = self.context_menu_pos {
            let canvas_pos = self.screen_to_canvas(menu_pos, rect);

            egui::Window::new("Add Node")
                .fixed_pos(menu_pos)
                .title_bar(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                        for category in &self.node_categories {
                            ui.collapsing(
                                egui::RichText::new(category.name).color(category.color),
                                |ui| {
                                    for template in &category.node_types {
                                        if ui.button(template.name).on_hover_text(template.description).clicked() {
                                            let node = template.create_node([canvas_pos.x, canvas_pos.y]);
                                            self.graph.add_node(node);
                                            self.context_menu_pos = None;
                                        }
                                    }
                                },
                            );
                        }
                    });

                    ui.separator();
                    if ui.button("Cancel").clicked() {
                        self.context_menu_pos = None;
                    }
                });

            // Close menu if clicked elsewhere
            if response.clicked() || response.drag_started() {
                self.context_menu_pos = None;
            }
        }
    }

    /// Handle input events
    fn handle_input(&mut self, ui: &mut egui::Ui, response: &Response, rect: &Rect) {
        // Track mouse position
        self.last_mouse_pos = ui.input(|i| i.pointer.hover_pos());

        // Handle scroll for zoom
        if response.hovered() {
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
            if scroll_delta.y != 0.0 {
                let zoom_delta = if scroll_delta.y > 0.0 { 1.1 } else { 0.9 };
                let new_zoom = (self.zoom * zoom_delta).clamp(0.25, 3.0);

                // Zoom towards mouse position
                if let Some(mouse_pos) = self.last_mouse_pos {
                    let canvas_mouse_before = self.screen_to_canvas(mouse_pos, rect);
                    self.zoom = new_zoom;
                    let canvas_mouse_after = self.screen_to_canvas(mouse_pos, rect);
                    self.camera_offset += canvas_mouse_after - canvas_mouse_before;
                } else {
                    self.zoom = new_zoom;
                }
            }
        }

        // Handle dragging
        if response.drag_started() {
            let button = ui.input(|i| i.pointer.primary_down());
            let modifiers = ui.input(|i| i.modifiers);

            if button {
                if let Some(pos) = response.interact_pointer_pos() {
                    // Check if clicking on a pin
                    if let Some((node_id, pin_id)) = self.find_pin_at(pos, rect) {
                        self.interaction = CanvasInteraction::DraggingConnection {
                            source_node: node_id,
                            source_pin: pin_id,
                        };
                    } else if let Some(node_id) = self.find_node_at(pos, rect) {
                        // Clicking on a node
                        if !modifiers.shift && !self.selection.is_selected(node_id) {
                            self.selection.select_single(node_id);
                        } else if modifiers.shift {
                            self.selection.toggle_selection(node_id);
                        }
                        self.interaction = CanvasInteraction::DraggingNode;
                        self.drag_start = Some(pos);
                    } else {
                        // Clicking on empty canvas
                        if !modifiers.shift {
                            self.selection.clear();
                        }
                        self.interaction = CanvasInteraction::Selecting;
                        self.drag_start = Some(pos);
                    }
                }
            } else if ui.input(|i| i.pointer.secondary_down()) {
                // Middle/right mouse for panning
                self.interaction = CanvasInteraction::Panning;
            }
        }

        // Handle drag
        if response.dragged() {
            match self.interaction {
                CanvasInteraction::Panning => {
                    let delta = response.drag_delta() / self.zoom;
                    self.camera_offset += delta;
                }
                CanvasInteraction::DraggingNode => {
                    let delta = response.drag_delta() / self.zoom;
                    for node_id in &self.selection.selected_nodes {
                        if let Some(node) = self.graph.nodes.get_mut(node_id) {
                            node.move_by([delta.x, delta.y]);
                        }
                    }
                }
                CanvasInteraction::Selecting => {
                    // Update selection box
                }
                CanvasInteraction::DraggingConnection { .. } => {
                    // Connection is being drawn in draw()
                }
                _ => {}
            }
        }

        // Handle drag end
        if response.drag_stopped() {
            match self.interaction {
                CanvasInteraction::DraggingConnection { source_node, source_pin } => {
                    // Try to complete connection
                    if let Some(pos) = self.last_mouse_pos {
                        if let Some((target_node, target_pin)) = self.find_pin_at(pos, rect) {
                            let connection = Connection::new(
                                source_node,
                                source_pin,
                                target_node,
                                target_pin,
                            );
                            self.graph.add_connection(connection);
                        }
                    }
                }
                CanvasInteraction::Selecting => {
                    // Select nodes in selection box
                    if let (Some(start), Some(end)) = (self.drag_start, self.last_mouse_pos) {
                        let selection_rect = Rect::from_two_pos(start, end);
                        for (node_id, node) in &self.graph.nodes {
                            let node_rect = self.get_node_screen_rect(node, rect);
                            if selection_rect.intersects(node_rect) {
                                self.selection.add_to_selection(*node_id);
                            }
                        }
                    }
                }
                _ => {}
            }
            self.interaction = CanvasInteraction::None;
            self.drag_start = None;
        }

        // Handle delete key
        if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
            let to_remove: Vec<_> = self.selection.selected_nodes.iter().copied().collect();
            for node_id in to_remove {
                self.graph.remove_node(node_id);
            }
            self.selection.clear();
        }

        // Handle duplicate (Ctrl+D)
        if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::D)) {
            self.duplicate_selected();
        }

        // Handle Ctrl+A (select all)
        if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::A)) {
            self.selection.selected_nodes = self.graph.nodes.keys().copied().collect();
        }
    }

    /// Duplicate selected nodes
    fn duplicate_selected(&mut self) {
        let mut id_mapping: HashMap<NodeId, NodeId> = HashMap::new();
        let mut new_nodes = Vec::new();

        // Clone nodes with offset
        for node_id in &self.selection.selected_nodes {
            if let Some(node) = self.graph.nodes.get(node_id) {
                let mut new_node = node.clone();
                new_node.id = NodeId::new();
                new_node.position[0] += 20.0;
                new_node.position[1] += 20.0;
                id_mapping.insert(*node_id, new_node.id);
                new_nodes.push(new_node);
            }
        }

        // Add new nodes
        for node in &new_nodes {
            self.graph.add_node(node.clone());
        }

        // Clone connections between duplicated nodes
        let connections_to_clone: Vec<_> = self
            .graph
            .connections
            .iter()
            .filter(|c| {
                id_mapping.contains_key(&c.source_node) && id_mapping.contains_key(&c.target_node)
            })
            .cloned()
            .collect();

        for conn in connections_to_clone {
            if let (Some(&new_source), Some(&new_target)) =
                (id_mapping.get(&conn.source_node), id_mapping.get(&conn.target_node))
            {
                let new_conn = Connection::new(
                    new_source,
                    conn.source_pin,
                    new_target,
                    conn.target_pin,
                );
                self.graph.add_connection(new_conn);
            }
        }

        // Select new nodes
        self.selection.selected_nodes = new_nodes.iter().map(|n| n.id).collect();
    }

    /// Draw the background
    fn draw_background(&self, ui: &mut egui::Ui, rect: &Rect) {
        ui.painter().rect_filled(*rect, 0.0, self.style.background_color);
    }

    /// Draw the grid
    fn draw_grid(&self, ui: &mut egui::Ui, rect: &Rect) {
        let painter = ui.painter();
        let grid_size = self.style.grid_size * self.zoom;

        let offset_x = (self.camera_offset.x * self.zoom) % grid_size;
        let offset_y = (self.camera_offset.y * self.zoom) % grid_size;

        let mut x = rect.min.x + offset_x;
        while x < rect.max.x {
            painter.line_segment(
                [pos2(x, rect.min.y), pos2(x, rect.max.y)],
                Stroke::new(1.0, self.style.grid_color),
            );
            x += grid_size;
        }

        let mut y = rect.min.y + offset_y;
        while y < rect.max.y {
            painter.line_segment(
                [pos2(rect.min.x, y), pos2(rect.max.x, y)],
                Stroke::new(1.0, self.style.grid_color),
            );
            y += grid_size;
        }
    }

    /// Draw all connections
    fn draw_connections(&self, ui: &mut egui::Ui, rect: &Rect) {
        for conn in &self.graph.connections {
            if let (Some(source_node), Some(target_node)) =
                (self.graph.nodes.get(&conn.source_node), self.graph.nodes.get(&conn.target_node))
            {
                if let (Some(source_pin), Some(target_pin)) = (
                    source_node.get_pin(conn.source_pin),
                    target_node.get_pin(conn.target_pin),
                ) {
                    let start = self.get_pin_screen_position(source_node, source_pin, rect);
                    let end = self.get_pin_screen_position(target_node, target_pin, rect);
                    let color = source_pin.pin_type.color();
                    self.draw_connection_curve(ui, start, end, color);
                }
            }
        }
    }

    /// Draw a curved connection line
    fn draw_connection_curve(&self, ui: &mut egui::Ui, start: Pos2, end: Pos2, color: Color32) {
        let control_offset = ((end.x - start.x) / 2.0).abs().max(50.0) * self.zoom;

        let cp1 = pos2(start.x + control_offset, start.y);
        let cp2 = pos2(end.x - control_offset, end.y);

        let points: Vec<Pos2> = (0..=20)
            .map(|i| {
                let t = i as f32 / 20.0;
                let t2 = t * t;
                let t3 = t2 * t;
                let mt = 1.0 - t;
                let mt2 = mt * mt;
                let mt3 = mt2 * mt;

                pos2(
                    mt3 * start.x + 3.0 * mt2 * t * cp1.x + 3.0 * mt * t2 * cp2.x + t3 * end.x,
                    mt3 * start.y + 3.0 * mt2 * t * cp1.y + 3.0 * mt * t2 * cp2.y + t3 * end.y,
                )
            })
            .collect();

        ui.painter()
            .add(egui::Shape::line(points, Stroke::new(self.style.connection_thickness * self.zoom, color)));
    }

    /// Draw a straight connection line
    fn draw_connection_line(&self, ui: &mut egui::Ui, start: Pos2, end: Pos2, color: Color32) {
        ui.painter().line_segment(
            [start, end],
            Stroke::new(self.style.connection_thickness * self.zoom, color),
        );
    }

    /// Draw all nodes
    fn draw_nodes(&mut self, ui: &mut egui::Ui, rect: &Rect) {
        // Collect nodes to avoid borrow issues
        let nodes: Vec<_> = self.graph.nodes.values().cloned().collect();

        for node in nodes {
            self.draw_node(ui, &node, rect);
        }
    }

    /// Draw a single node
    fn draw_node(&mut self, ui: &mut egui::Ui, node: &Node, rect: &Rect) {
        let screen_rect = self.get_node_screen_rect(node, rect);
        let is_selected = self.selection.is_selected(node.id);
        let is_hovered = self.selection.hovered_node == Some(node.id);

        let category_color = node.category_color();

        // Node shadow
        if self.style.node_shadow {
            ui.painter().rect_filled(
                screen_rect.translate(vec2(3.0, 3.0)),
                self.style.node_corner_radius * self.zoom,
                Color32::BLACK.gamma_multiply(0.5),
            );
        }

        // Node background
        ui.painter().rect_filled(
            screen_rect,
            self.style.node_corner_radius * self.zoom,
            Color32::from_gray(45),
        );

        // Selection highlight
        if is_selected {
            ui.painter().rect_stroke(
                screen_rect.expand(2.0 * self.zoom),
                self.style.node_corner_radius * self.zoom,
                Stroke::new(2.0 * self.zoom, self.style.selection_color),
            );
        }

        // Hover highlight
        if is_hovered && !is_selected {
            ui.painter().rect_stroke(
                screen_rect.expand(1.0 * self.zoom),
                self.style.node_corner_radius * self.zoom,
                Stroke::new(1.0 * self.zoom, Color32::WHITE.gamma_multiply(0.5)),
            );
        }

        // Header
        let header_height = self.style.node_header_height * self.zoom;
        let header_rect = Rect::from_min_size(
            screen_rect.min,
            vec2(screen_rect.width(), header_height),
        );
        ui.painter().rect_filled(
            header_rect,
            egui::Rounding::same(self.style.node_corner_radius * self.zoom)
                .at_least(egui::Rounding::same(0.0).ne),
            category_color,
        );

        // Title
        let title = node.display_name();
        let font_size = 14.0 * self.zoom;
        ui.painter().text(
            header_rect.min + vec2(10.0 * self.zoom, header_height / 2.0),
            egui::Align2::LEFT_CENTER,
            &title,
            egui::FontId::proportional(font_size),
            Color32::WHITE,
        );

        // Draw pins
        self.draw_pins(ui, node, rect, &screen_rect);
    }

    /// Draw pins for a node
    fn draw_pins(&mut self, ui: &mut egui::Ui, node: &Node, _rect: &Rect, node_rect: &Rect) {
        let pin_radius = self.style.node_pin_radius * self.zoom;
        let pin_spacing = self.style.node_pin_spacing * self.zoom;
        let header_height = self.style.node_header_height * self.zoom;

        // Input pins (left side)
        for (i, pin) in node.inputs.iter().enumerate() {
            let y = node_rect.min.y + header_height + pin_spacing * (i as f32 + 0.5);
            let pos = pos2(node_rect.min.x, y);
            self.draw_pin(ui, node.id, pin, pos, pin_radius, true);
        }

        // Output pins (right side)
        for (i, pin) in node.outputs.iter().enumerate() {
            let y = node_rect.min.y + header_height + pin_spacing * (i as f32 + 0.5);
            let pos = pos2(node_rect.max.x, y);
            self.draw_pin(ui, node.id, pin, pos, pin_radius, false);
        }
    }

    /// Draw a single pin
    fn draw_pin(&mut self, ui: &mut egui::Ui, node_id: NodeId, pin: &Pin, pos: Pos2, radius: f32, is_input: bool) {
        let color = pin.pin_type.color();

        // Pin circle
        ui.painter().circle_filled(pos, radius, color);
        ui.painter().circle_stroke(pos, radius, Stroke::new(1.0, Color32::WHITE));

        // Pin label
        let label_offset = if is_input { radius + 5.0 } else { -(radius + 5.0) };
        let label_pos = pos2(pos.x + label_offset, pos.y);
        let align = if is_input {
            egui::Align2::LEFT_CENTER
        } else {
            egui::Align2::RIGHT_CENTER
        };

        let font_size = 11.0 * self.zoom;
        ui.painter().text(
            label_pos,
            align,
            &pin.name,
            egui::FontId::proportional(font_size),
            Color32::LIGHT_GRAY,
        );

        // Check hover
        if let Some(mouse_pos) = self.last_mouse_pos {
            if mouse_pos.distance(pos) < radius * 1.5 {
                self.selection.hovered_pin = Some((node_id, pin.id));
            }
        }
    }

    /// Draw the minimap
    fn draw_minimap(&self, ui: &mut egui::Ui, rect: &Rect) {
        let minimap_size = vec2(150.0, 100.0);
        let minimap_pos = pos2(rect.max.x - minimap_size.x - 10.0, rect.max.y - minimap_size.y - 10.0);
        let minimap_rect = Rect::from_min_size(minimap_pos, minimap_size);

        // Background
        ui.painter().rect_filled(minimap_rect, 4.0, Color32::from_gray(30));
        ui.painter().rect_stroke(minimap_rect, 4.0, Stroke::new(1.0, Color32::from_gray(60)));

        // Calculate bounds
        if self.graph.nodes.is_empty() {
            return;
        }

        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for node in self.graph.nodes.values() {
            min_x = min_x.min(node.position[0]);
            max_x = max_x.max(node.position[0]);
            min_y = min_y.min(node.position[1]);
            max_y = max_y.max(node.position[1]);
        }

        let bounds_width = (max_x - min_x).max(100.0);
        let bounds_height = (max_y - min_y).max(100.0);

        let scale_x = (minimap_size.x - 10.0) / bounds_width;
        let scale_y = (minimap_size.y - 10.0) / bounds_height;
        let scale = scale_x.min(scale_y);

        // Draw nodes on minimap
        for node in self.graph.nodes.values() {
            let x = minimap_pos.x + 5.0 + (node.position[0] - min_x) * scale;
            let y = minimap_pos.y + 5.0 + (node.position[1] - min_y) * scale;
            let node_rect = Rect::from_center_size(
                pos2(x, y),
                vec2(8.0, 6.0),
            );
            ui.painter().rect_filled(node_rect, 1.0, node.category_color());
        }

        // Draw viewport rectangle
        let viewport_x = minimap_pos.x + 5.0 + (-self.camera_offset.x - min_x) * scale;
        let viewport_y = minimap_pos.y + 5.0 + (-self.camera_offset.y - min_y) * scale;
        let viewport_width = rect.width() / self.zoom * scale;
        let viewport_height = rect.height() / self.zoom * scale;

        let viewport_rect = Rect::from_min_size(
            pos2(viewport_x, viewport_y),
            vec2(viewport_width, viewport_height),
        );
        ui.painter().rect_stroke(
            viewport_rect.intersect(minimap_rect),
            1.0,
            Stroke::new(1.0, Color32::WHITE),
        );
    }

    /// Get the screen rectangle for a node
    fn get_node_screen_rect(&self, node: &Node, rect: &Rect) -> Rect {
        let pos = self.canvas_to_screen(pos2(node.position[0], node.position[1]), rect);

        // Calculate node height based on pin count
        let pin_count = node.inputs.len().max(node.outputs.len());
        let height = self.style.node_header_height + pin_count as f32 * self.style.node_pin_spacing + 10.0;

        // Calculate width based on content
        let title_width = node.display_name().len() as f32 * 8.0;
        let max_pin_name_width = node
            .inputs
            .iter()
            .chain(node.outputs.iter())
            .map(|p| p.name.len() as f32 * 6.0)
            .fold(0.0, f32::max);
        let width = (title_width.max(max_pin_name_width) + 40.0).max(self.style.node_min_width);

        Rect::from_min_size(pos, vec2(width * self.zoom, height * self.zoom))
    }

    /// Get the screen position of a pin
    fn get_pin_screen_position(&self, node: &Node, pin: &Pin, rect: &Rect) -> Pos2 {
        let node_rect = self.get_node_screen_rect(node, rect);
        let header_height = self.style.node_header_height * self.zoom;
        let pin_spacing = self.style.node_pin_spacing * self.zoom;

        // Find pin index
        let is_input = node.inputs.iter().position(|p| p.id == pin.id);
        let is_output = node.outputs.iter().position(|p| p.id == pin.id);

        let index = is_input.or(is_output).unwrap_or(0);
        let y = node_rect.min.y + header_height + pin_spacing * (index as f32 + 0.5);

        let x = if is_input.is_some() {
            node_rect.min.x
        } else {
            node_rect.max.x
        };

        pos2(x, y)
    }

    /// Find a node at a screen position
    fn find_node_at(&self, screen_pos: Pos2, rect: &Rect) -> Option<NodeId> {
        for (id, node) in &self.graph.nodes {
            let node_rect = self.get_node_screen_rect(node, rect);
            if node_rect.contains(screen_pos) {
                return Some(*id);
            }
        }
        None
    }

    /// Find a pin at a screen position
    fn find_pin_at(&self, screen_pos: Pos2, rect: &Rect) -> Option<(NodeId, PinId)> {
        let pin_radius = self.style.node_pin_radius * self.zoom * 1.5;

        for (node_id, node) in &self.graph.nodes {
            for pin in node.inputs.iter().chain(node.outputs.iter()) {
                let pin_pos = self.get_pin_screen_position(node, pin, rect);
                if screen_pos.distance(pin_pos) < pin_radius {
                    return Some((*node_id, pin.id));
                }
            }
        }
        None
    }

    /// Zoom in
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.2).min(3.0);
    }

    /// Zoom out
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.2).max(0.25);
    }

    /// Reset zoom
    pub fn reset_zoom(&mut self) {
        self.zoom = 1.0;
    }

    /// Center view on all nodes
    pub fn frame_all(&mut self, rect: &Rect) {
        if self.graph.nodes.is_empty() {
            self.camera_offset = Vec2::ZERO;
            return;
        }

        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for node in self.graph.nodes.values() {
            min_x = min_x.min(node.position[0]);
            max_x = max_x.max(node.position[0]);
            min_y = min_y.min(node.position[1]);
            max_y = max_y.max(node.position[1]);
        }

        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        self.camera_offset = vec2(-center_x + rect.center().x, -center_y + rect.center().y);
    }

    /// Get selected nodes
    pub fn selected_nodes(&self) -> impl Iterator<Item = &Node> {
        self.selection
            .selected_nodes
            .iter()
            .filter_map(move |id| self.graph.nodes.get(id))
    }

    /// Get the first selected node
    pub fn first_selected_node(&self) -> Option<&Node> {
        self.selection.selected_nodes.iter().next().and_then(|id| self.graph.nodes.get(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::nodes::{NodeId, NodeType, Pin, PinType};

    #[test]
    fn test_graph_operations() {
        let mut graph = NodeGraph::new();

        let node1 = Node::new(NodeType::OnInteract, [0.0, 0.0]);
        let node2 = Node::new(NodeType::MoveEntity { x: 10, y: 0, relative: false }, [200.0, 0.0]);

        let id1 = graph.add_node(node1);
        let id2 = graph.add_node(node2);

        assert_eq!(graph.nodes.len(), 2);

        // Test connection
        let conn = Connection::new(id1, graph.nodes[&id1].outputs[0].id, id2, graph.nodes[&id2].inputs[0].id);
        assert!(graph.add_connection(conn));
        assert_eq!(graph.connections.len(), 1);

        // Remove node
        graph.remove_node(id1);
        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.connections.is_empty());
    }

    #[test]
    fn test_canvas_coordinates() {
        let canvas = NodeCanvas::new();
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));

        let screen_pos = pos2(400.0, 300.0);
        let canvas_pos = canvas.screen_to_canvas(screen_pos, &rect);
        let back_to_screen = canvas.canvas_to_screen(canvas_pos, &rect);

        assert!((screen_pos.x - back_to_screen.x).abs() < 0.01);
        assert!((screen_pos.y - back_to_screen.y).abs() < 0.01);
    }
}
