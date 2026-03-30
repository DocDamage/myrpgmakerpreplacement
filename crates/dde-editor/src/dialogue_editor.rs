//! Dialogue Tree Editor
//!
//! A comprehensive visual node editor for creating and editing NPC dialogue trees.
//! Provides drag-and-drop node editing, connection management, property editing,
//! preview mode, and database integration.
//!
//! ## Features
//!
//! - Visual node canvas with pan/zoom
//! - Multiple node types (Text, Choice, Condition, Action, Branch, End)
//! - Connection management with bezier curves
//! - Property panel for selected nodes
//! - Preview mode for testing dialogues
//! - Save/load to database
//!
//! ## Usage
//!
//! ```rust,ignore
//! use dde_editor::dialogue_editor::DialogueEditor;
//!
//! let mut editor = DialogueEditor::new();
//! editor.new_tree();
//!
//! // In your egui UI loop:
//! // editor.draw(ctx);
//! ```

use dde_core::systems::dialogue::{
    ActionType, BranchMode, ConditionOp, DialogueChoice, DialogueCondition, DialogueEffect,
    DialogueMetadata, DialogueNode, DialogueNodeType, DialogueTree, NodeConnection, NodePosition,
    PortraitPosition, ValidationError,
};
use egui::{
    pos2, vec2, Align2, Color32, DragValue, Id, Key, Painter, PointerButton, Pos2, Rect, Response,
    RichText, Rounding, Sense, Stroke, Ui, Vec2,
};
use std::collections::HashMap;

/// The main dialogue tree editor
#[derive(Debug)]
pub struct DialogueEditor {
    /// Currently edited tree
    tree: Option<DialogueTree>,
    /// Currently selected node ID
    selected_node: Option<String>,
    /// Node being dragged
    dragging_node: Option<String>,
    /// Connection being drawn (source node ID)
    drawing_connection: Option<(String, ConnectionSource)>,
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
    /// Modified flag for unsaved changes
    modified: bool,
    /// Current file path
    file_path: Option<std::path::PathBuf>,
    /// Editor mode
    mode: EditorMode,
    /// Preview session state
    preview_state: Option<PreviewState>,
    /// Undo/redo history
    history: Vec<HistoryEntry>,
    /// Current history position
    history_pos: usize,
    /// Max history size
    max_history: usize,
    /// Show validation errors
    show_validation: bool,
    /// Validation errors
    validation_errors: Vec<ValidationError>,
    /// Node ID counter
    node_id_counter: u64,
    /// Show debug overlays
    show_debug_info: bool,
    /// New tree dialog state
    new_tree_dialog: Option<NewTreeDialog>,
    /// Database connection (optional)
    db_connection: Option<DbConnection>,
}

/// Connection source type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionSource {
    Output,
    Choice(usize),
}

/// Editor mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorMode {
    #[default]
    Edit,
    Preview,
}

/// Preview session state
#[derive(Debug, Clone)]
struct PreviewState {
    current_node: String,
    history: Vec<(String, String, bool)>, // (speaker, text, is_player)
    variables: HashMap<String, serde_json::Value>,
    choices: Vec<DialogueChoice>,
    completed: bool,
}

/// History entry for undo/redo
#[derive(Debug, Clone)]
struct HistoryEntry {
    tree: DialogueTree,
    description: String,
}

/// New tree dialog state
#[derive(Debug, Clone, Default)]
struct NewTreeDialog {
    id: String,
    name: String,
}

/// Database connection placeholder
#[derive(Debug, Clone)]
struct DbConnection {
    // Would contain actual database connection
    connected: bool,
}

impl Default for DialogueEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl DialogueEditor {
    /// Create a new dialogue editor
    pub fn new() -> Self {
        Self {
            tree: None,
            selected_node: None,
            dragging_node: None,
            drawing_connection: None,
            canvas_offset: Vec2::new(100.0, 100.0),
            canvas_zoom: 1.0,
            show_grid: true,
            grid_size: 20.0,
            show_minimap: false,
            palette_filter: String::new(),
            modified: false,
            file_path: None,
            mode: EditorMode::Edit,
            preview_state: None,
            history: Vec::new(),
            history_pos: 0,
            max_history: 50,
            show_validation: false,
            validation_errors: Vec::new(),
            node_id_counter: 1,
            show_debug_info: false,
            new_tree_dialog: None,
            db_connection: None,
        }
    }

    /// Check if a tree is loaded
    pub fn has_tree(&self) -> bool {
        self.tree.is_some()
    }

    /// Get the current tree
    pub fn tree(&self) -> Option<&DialogueTree> {
        self.tree.as_ref()
    }

    /// Get mutable tree
    pub fn tree_mut(&mut self) -> Option<&mut DialogueTree> {
        self.tree.as_mut()
    }

    /// Check if there are unsaved changes
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Get the current file path
    pub fn file_path(&self) -> Option<&std::path::Path> {
        self.file_path.as_deref()
    }

    /// Create a new empty dialogue tree
    pub fn new_tree(&mut self) {
        self.new_tree_dialog = Some(NewTreeDialog {
            id: format!("dialogue_{}", self.node_id_counter),
            name: "New Dialogue".to_string(),
        });
        self.node_id_counter += 1;
    }

    /// Finalize creating a new tree from dialog
    fn finalize_new_tree(&mut self, dialog: NewTreeDialog) {
        let tree = DialogueTree::new(&dialog.id, &dialog.name);
        self.save_to_history("New tree");
        self.tree = Some(tree);
        self.selected_node = None;
        self.modified = true;
        self.file_path = None;
        self.new_tree_dialog = None;
        self.preview_state = None;
    }

    /// Load a dialogue tree
    pub fn load_tree(&mut self, tree: DialogueTree) {
        self.save_to_history("Load tree");
        self.tree = Some(tree);
        self.selected_node = None;
        self.modified = false;
        self.preview_state = None;
    }

    /// Get the tree as JSON
    pub fn to_json(&self) -> Option<String> {
        self.tree.as_ref()?.to_json().ok()
    }

    /// Load from JSON
    pub fn from_json(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let tree = DialogueTree::from_json(json)?;
        self.load_tree(tree);
        Ok(())
    }

    /// Toggle preview mode
    pub fn toggle_preview(&mut self) {
        match self.mode {
            EditorMode::Edit => {
                if let Some(ref tree) = self.tree {
                    self.mode = EditorMode::Preview;
                    self.preview_state = Some(PreviewState {
                        current_node: tree.root_node.clone(),
                        history: Vec::new(),
                        variables: HashMap::new(),
                        choices: Vec::new(),
                        completed: false,
                    });
                    self.update_preview();
                }
            }
            EditorMode::Preview => {
                self.mode = EditorMode::Edit;
                self.preview_state = None;
            }
        }
    }

    /// Update preview state based on current node
    fn update_preview(&mut self) {
        let Some(ref tree) = self.tree else { return };
        let Some(ref mut state) = self.preview_state else { return };
        
        let node = match tree.get_node(&state.current_node) {
            Some(n) => n.clone(),
            None => {
                state.completed = true;
                return;
            }
        };

        match &node.node_type {
            DialogueNodeType::NpcText => {
                state.choices.clear();
                let speaker = node.speaker.clone().unwrap_or_else(|| "NPC".to_string());
                state.history.push((speaker.clone(), node.text.clone(), false));
                
                // Move to next if there's only one connection
                if node.connections.len() == 1 {
                    state.current_node = node.connections[0].target_node.clone();
                    self.update_preview();
                }
            }
            DialogueNodeType::PlayerChoice => {
                state.choices = node.choices.clone();
            }
            DialogueNodeType::Condition => {
                // In preview, take first path
                if let Some(conn) = node.connections.first() {
                    state.current_node = conn.target_node.clone();
                    self.update_preview();
                } else {
                    state.completed = true;
                }
            }
            DialogueNodeType::Action => {
                if let Some(conn) = node.connections.first() {
                    state.current_node = conn.target_node.clone();
                    self.update_preview();
                } else {
                    state.completed = true;
                }
            }
            DialogueNodeType::Branch { mode } => {
                use rand::seq::SliceRandom;
                
                let target = match mode {
                    BranchMode::Random | BranchMode::Weighted => {
                        node.connections.choose(&mut rand::thread_rng())
                    }
                    _ => node.connections.first(),
                };
                
                if let Some(conn) = target {
                    state.current_node = conn.target_node.clone();
                    self.update_preview();
                } else {
                    state.completed = true;
                }
            }
            DialogueNodeType::End => {
                state.completed = true;
            }
        }
    }

    /// Select a choice in preview mode
    fn preview_select_choice(&mut self, index: usize) {
        let Some(ref mut state) = self.preview_state else { return };
        let Some(ref tree) = self.tree else { return };
        
        if index >= state.choices.len() {
            return;
        }
        
        let choice = &state.choices[index];
        state.history.push(("Player".to_string(), choice.text.clone(), true));
        
        // Find current node and get next
        if let Some(node) = tree.get_node(&state.current_node) {
            if let Some(ref next) = choice.next_node {
                state.current_node = next.clone();
            } else if let Some(conn) = node.connections.first() {
                state.current_node = conn.target_node.clone();
            } else {
                state.completed = true;
                return;
            }
            self.update_preview();
        }
    }

    /// Advance preview
    fn preview_advance(&mut self) {
        let Some(ref mut state) = self.preview_state else { return };
        if state.completed || !state.choices.is_empty() {
            return;
        }
        
        let Some(ref tree) = self.tree else { return };
        if let Some(node) = tree.get_node(&state.current_node) {
            if let Some(conn) = node.connections.first() {
                state.current_node = conn.target_node.clone();
                self.update_preview();
            } else {
                state.completed = true;
            }
        }
    }

    /// Restart preview
    fn preview_restart(&mut self) {
        if let Some(ref tree) = self.tree {
            self.preview_state = Some(PreviewState {
                current_node: tree.root_node.clone(),
                history: Vec::new(),
                variables: HashMap::new(),
                choices: Vec::new(),
                completed: false,
            });
            self.update_preview();
        }
    }

    /// Draw the complete editor UI
    pub fn draw(&mut self, ctx: &egui::Context) {
        // Handle new tree dialog
        if let Some(dialog) = self.new_tree_dialog.clone() {
            egui::Window::new("New Dialogue Tree")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Dialogue ID:");
                    ui.text_edit_singleline(&mut self.new_tree_dialog.as_mut().unwrap().id);
                    ui.label("Dialogue Name:");
                    ui.text_edit_singleline(&mut self.new_tree_dialog.as_mut().unwrap().name);
                    
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            let dialog = self.new_tree_dialog.take().unwrap();
                            self.finalize_new_tree(dialog);
                        }
                        if ui.button("Cancel").clicked() {
                            self.new_tree_dialog = None;
                        }
                    });
                });
        }

        // Main editor window
        egui::Window::new("Dialogue Tree Editor")
            .default_size([1400.0, 900.0])
            .show(ctx, |ui| {
                match self.mode {
                    EditorMode::Edit => self.draw_edit_mode(ui),
                    EditorMode::Preview => self.draw_preview_mode(ui),
                }
            });
    }

    /// Draw edit mode UI
    fn draw_edit_mode(&mut self, ui: &mut Ui) {
        // Top toolbar
        self.draw_toolbar(ui);
        ui.separator();

        if self.tree.is_none() {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.heading("No Dialogue Tree Loaded");
                ui.label("Click 'New' to create a dialogue or 'Open' to load one");
            });
            return;
        }

        // Main editor area with side panels
        egui::SidePanel::left("dialogue_palette_panel")
            .default_width(220.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                self.draw_palette(ui);
            });

        egui::SidePanel::right("dialogue_properties_panel")
            .default_width(320.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                self.draw_properties_panel(ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.draw_canvas(ui);
        });

        // Validation errors window
        if self.show_validation {
            self.draw_validation_window(ui.ctx());
        }
    }

    /// Draw preview mode UI
    fn draw_preview_mode(&mut self, ui: &mut Ui) {
        // Preview toolbar
        ui.horizontal(|ui| {
            if ui.button("◀ Back to Edit").clicked() {
                self.toggle_preview();
            }
            ui.separator();
            if ui.button("↻ Restart").clicked() {
                self.preview_restart();
            }
            if ui.button("▶ Continue").clicked() {
                self.preview_advance();
            }
            ui.separator();
            
            if let Some(ref state) = self.preview_state {
                if state.completed {
                    ui.label(RichText::new("✓ Dialogue Complete").color(Color32::GREEN));
                }
            }
        });
        ui.separator();

        let Some(ref state) = self.preview_state else { return };

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_width(ui.available_width());

            // Dialogue history
            for (speaker, text, is_player) in &state.history {
                let (bg_color, text_color, align) = if *is_player {
                    (Color32::from_rgb(60, 100, 60), Color32::WHITE, egui::Align::RIGHT)
                } else {
                    (Color32::from_rgb(60, 60, 80), Color32::WHITE, egui::Align::LEFT)
                };

                ui.with_layout(
                    egui::Layout::top_down(align).with_cross_align(egui::Align::Min),
                    |ui| {
                        let frame = egui::Frame::none()
                            .fill(bg_color)
                            .rounding(Rounding::same(8.0))
                            .inner_margin(12.0);

                        frame.show(ui, |ui| {
                            ui.set_max_width(600.0);
                            ui.label(RichText::new(speaker.as_str()).strong().color(Color32::YELLOW));
                            ui.label(RichText::new(text.as_str()).color(text_color));
                        });
                        ui.add_space(8.0);
                    },
                );
            }

            // Show choices or continue button
            if !state.completed {
                if !state.choices.is_empty() {
                    ui.separator();
                    ui.label(RichText::new("Select a response:").strong());
                    ui.add_space(8.0);

                    for (i, choice) in state.choices.iter().enumerate() {
                        if ui
                            .button(RichText::new(format!("→ {}", choice.text)).size(16.0))
                            .clicked()
                        {
                            self.preview_select_choice(i);
                        }
                        if let Some(ref tooltip) = choice.tooltip {
                            ui.label(RichText::new(tooltip).small().italics().color(Color32::GRAY));
                        }
                        ui.add_space(4.0);
                    }
                } else {
                    ui.add_space(16.0);
                    if ui.button(RichText::new("▶ Continue").size(16.0)).clicked() {
                        self.preview_advance();
                    }
                }
            }

            ui.add_space(50.0);
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
                    self.save_to_file();
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
                if ui.add_enabled(self.db_connection.is_some(), egui::Button::new("💾 Save to Database")).clicked() {
                    self.save_to_database();
                    ui.close_menu();
                }
                if ui.add_enabled(self.db_connection.is_some(), egui::Button::new("📂 Load from Database")).clicked() {
                    self.load_from_database();
                    ui.close_menu();
                }
            });

            ui.separator();

            // Edit operations
            ui.menu_button("Edit", |ui| {
                let can_undo = self.history_pos > 0;
                let can_redo = self.history_pos < self.history.len();

                if ui.add_enabled(can_undo, egui::Button::new("↩ Undo (Ctrl+Z)")).clicked() {
                    self.undo();
                    ui.close_menu();
                }
                if ui.add_enabled(can_redo, egui::Button::new("↪ Redo (Ctrl+Y)")).clicked() {
                    self.redo();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("🗑 Delete Selected (Del)").clicked() {
                    if let Some(id) = self.selected_node.clone() {
                        self.delete_node(&id);
                    }
                    ui.close_menu();
                }
                if ui.button("📋 Duplicate (Ctrl+D)").clicked() {
                    self.duplicate_selected();
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
                    self.canvas_offset = Vec2::new(100.0, 100.0);
                    self.canvas_zoom = 1.0;
                    ui.close_menu();
                }
            });

            ui.separator();

            // Validate
            if ui.button("✓ Validate").clicked() {
                self.validate_tree();
            }

            // Show error count if any
            if !self.validation_errors.is_empty() {
                let color = if self.validation_errors.iter().any(|e| {
                    matches!(e, ValidationError::MissingRootNode | ValidationError::InvalidConnection { .. })
                }) {
                    Color32::RED
                } else {
                    Color32::YELLOW
                };
                if ui.button(RichText::new(format!("⚠ {} Issues", self.validation_errors.len())).color(color)).clicked() {
                    self.show_validation = true;
                }
            }

            ui.separator();

            // Preview button
            let preview_text = if self.mode == EditorMode::Preview {
                "✕ Stop Preview"
            } else {
                "▶ Preview"
            };
            if ui.button(preview_text).clicked() {
                self.toggle_preview();
            }

            // Right-aligned info
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.modified {
                    ui.label(RichText::new("●").color(Color32::YELLOW));
                }
                ui.label(format!("{:.0}%", self.canvas_zoom * 100.0));
                if let Some(ref tree) = self.tree {
                    ui.label(format!("{} nodes", tree.nodes.len()));
                }
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
            // NPC Text Node
            if self.palette_button(ui, "🗨 Text (NPC)", "NPC dialogue text", [100, 150, 255]) {
                self.add_node_at_center(DialogueNode::new("", DialogueNodeType::NpcText));
            }

            // Choice Node
            if self.palette_button(ui, "⚡ Choice (Player)", "Player decision point", [100, 200, 100]) {
                self.add_node_at_center(DialogueNode::new("", DialogueNodeType::PlayerChoice));
            }

            // Condition Node
            if self.palette_button(ui, "? Condition", "Check game state/variables", [255, 200, 100]) {
                self.add_node_at_center(DialogueNode::new("", DialogueNodeType::Condition));
            }

            // Action Node
            if self.palette_button(ui, "⚙ Action", "Execute effects/actions", [200, 100, 200]) {
                self.add_node_at_center(DialogueNode::new("", DialogueNodeType::Action));
            }

            // Branch Node
            ui.collapsing("🔀 Branch", |ui| {
                if ui.button("  Random").clicked() {
                    self.add_node_at_center(DialogueNode::new_branch("", BranchMode::Random));
                }
                if ui.button("  Weighted").clicked() {
                    self.add_node_at_center(DialogueNode::new_branch("", BranchMode::Weighted));
                }
                if ui.button("  Sequential").clicked() {
                    self.add_node_at_center(DialogueNode::new_branch("", BranchMode::Sequential));
                }
                if ui.button("  First Valid").clicked() {
                    self.add_node_at_center(DialogueNode::new_branch("", BranchMode::FirstValid));
                }
            });

            // End Node
            if self.palette_button(ui, "⏹ End", "End of dialogue", [150, 150, 150]) {
                self.add_node_at_center(DialogueNode::new_end(""));
            }
        });

        ui.separator();
        ui.label(
            RichText::new("Double-click canvas to add node\nDrag to connect nodes")
                .small()
                .color(Color32::GRAY),
        );
    }

    /// Draw a palette button with color
    fn palette_button(
        &self,
        ui: &mut Ui,
        text: &str,
        tooltip: &str,
        color: [u8; 3],
    ) -> bool {
        let color32 = Color32::from_rgb(color[0], color[1], color[2]);
        let button = egui::Button::new(RichText::new(text).color(Color32::WHITE))
            .fill(color32.gamma_multiply(0.7));
        ui.add(button).on_hover_text(tooltip).clicked()
    }

    /// Add a node at the center of the canvas
    fn add_node_at_center(&mut self, template: DialogueNode) {
        let Some(ref mut tree) = self.tree else { return };
        
        let id = format!("node_{}", self.node_id_counter);
        self.node_id_counter += 1;
        
        let canvas_center = self.screen_to_world(pos2(400.0, 300.0));
        
        let mut node = template;
        node.id = id.clone();
        node.position = NodePosition::new(canvas_center.x, canvas_center.y);
        
        self.save_to_history("Add node");
        tree.add_node(node);
        self.selected_node = Some(id);
        self.modified = true;
    }

    /// Draw the main canvas area
    fn draw_canvas(&mut self, ui: &mut Ui) {
        let available_rect = ui.available_rect_before_wrap();
        let canvas_id = Id::new("dialogue_canvas");

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
        self.draw_connections(ui);

        // Draw nodes
        self.draw_nodes(ui);

        // Draw connection being created
        if let Some((ref source_id, source_type)) = &self.drawing_connection {
            if let Some(ref tree) = self.tree {
                if let Some(source_node) = tree.get_node(source_id) {
                    let source_pos = self.world_to_screen(pos2(source_node.position.x, source_node.position.y));
                    let output_pos = match source_type {
                        ConnectionSource::Output => source_pos + Vec2::new(0.0, 30.0),
                        ConnectionSource::Choice(i) => source_pos + Vec2::new(-60.0 + i as f32 * 40.0, 40.0),
                    };

                    let end_pos = ui.input(|i| i.pointer.hover_pos()).unwrap_or(output_pos);

                    self.draw_bezier_connection(
                        painter,
                        output_pos,
                        end_pos,
                        Color32::from_rgb(255, 200, 50),
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
            if ui.button("Add Text Node").clicked() {
                if let Some(pos) = canvas_response.interact_pointer_pos() {
                    self.add_node_at_position(
                        DialogueNode::new("", DialogueNodeType::NpcText),
                        pos,
                    );
                }
                ui.close_menu();
            }
            if ui.button("Add Choice Node").clicked() {
                if let Some(pos) = canvas_response.interact_pointer_pos() {
                    self.add_node_at_position(
                        DialogueNode::new("", DialogueNodeType::PlayerChoice),
                        pos,
                    );
                }
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Frame All").clicked() {
                self.frame_all();
                ui.close_menu();
            }
        });

        // Handle keyboard shortcuts
        self.handle_keyboard_shortcuts(ui);

        // Draw minimap
        if self.show_minimap {
            self.draw_minimap(ui, available_rect);
        }
    }

    /// Add a node at a specific screen position
    fn add_node_at_position(&mut self, template: DialogueNode, screen_pos: Pos2) {
        let Some(ref mut tree) = self.tree else { return };
        
        let id = format!("node_{}", self.node_id_counter);
        self.node_id_counter += 1;
        
        let world_pos = self.screen_to_world(screen_pos);
        
        let mut node = template;
        node.id = id.clone();
        node.position = NodePosition::new(world_pos.x, world_pos.y);
        
        self.save_to_history("Add node");
        tree.add_node(node);
        self.selected_node = Some(id);
        self.modified = true;
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
                [pos2(x, rect.top()), pos2(x, rect.bottom())],
                Stroke::new(1.0, grid_color),
            );
            x += grid_step;
        }

        let mut y = rect.top() + offset_y;
        while y < rect.bottom() {
            painter.line_segment(
                [pos2(rect.left(), y), pos2(rect.right(), y)],
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
                [pos2(x, rect.top()), pos2(x, rect.bottom())],
                Stroke::new(1.0, major_grid_color),
            );
            x += major_step;
        }

        y = rect.top() + major_offset_y;
        while y < rect.bottom() {
            painter.line_segment(
                [pos2(rect.left(), y), pos2(rect.right(), y)],
                Stroke::new(1.0, major_grid_color),
            );
            y += major_step;
        }
    }

    /// Draw connections between nodes
    fn draw_connections(&self, ui: &mut Ui) {
        let Some(ref tree) = self.tree else { return };
        let painter = ui.painter();

        for (node_id, node) in &tree.nodes {
            let parent_pos = self.world_to_screen(pos2(node.position.x, node.position.y));
            let parent_output = parent_pos + Vec2::new(0.0, 30.0);

            // Draw node output connections
            for (i, conn) in node.connections.iter().enumerate() {
                if let Some(target) = tree.get_node(&conn.target_node) {
                    let child_pos = self.world_to_screen(pos2(target.position.x, target.position.y));
                    let child_input = child_pos - Vec2::new(0.0, 30.0);

                    let color = if conn.condition.is_some() {
                        Color32::from_rgb(255, 200, 100) // Orange for conditional
                    } else {
                        Color32::from_gray(120)
                    };

                    self.draw_bezier_connection(painter, parent_output, child_input, color);

                    // Draw label if present
                    if let Some(ref label) = conn.label {
                        let mid = parent_output + (child_input - parent_output) * 0.5;
                        painter.text(
                            mid,
                            Align2::CENTER_CENTER,
                            label,
                            egui::FontId::proportional(10.0),
                            Color32::LIGHT_GRAY,
                        );
                    }

                    // Draw weight for weighted branches
                    if conn.weight != 1.0 {
                        let weight_pos = parent_output + (child_input - parent_output) * 0.3;
                        painter.circle_filled(weight_pos, 10.0, Color32::from_gray(60));
                        painter.text(
                            weight_pos,
                            Align2::CENTER_CENTER,
                            format!("{:.0}", conn.weight),
                            egui::FontId::proportional(9.0),
                            Color32::WHITE,
                        );
                    }
                }
            }

            // Draw choice connections
            if matches!(node.node_type, DialogueNodeType::PlayerChoice) {
                for (i, choice) in node.choices.iter().enumerate() {
                    if let Some(ref target_id) = choice.next_node {
                        if let Some(target) = tree.get_node(target_id) {
                            let choice_offset = Vec2::new(-50.0 + i as f32 * 40.0, 40.0);
                            let choice_pos = parent_pos + choice_offset;
                            let child_pos = self.world_to_screen(pos2(target.position.x, target.position.y));
                            let child_input = child_pos - Vec2::new(0.0, 30.0);

                            self.draw_bezier_connection(
                                painter,
                                choice_pos,
                                child_input,
                                Color32::from_rgb(100, 200, 100),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Draw a bezier curve connection
    fn draw_bezier_connection(
        &self,
        painter: &Painter,
        start: Pos2,
        end: Pos2,
        color: Color32,
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

        // Draw arrow head
        let dir = (end - cp2).normalized();
        let arrow_size = 8.0 * self.canvas_zoom;
        let arrow_pos = end;
        let p1 = arrow_pos - dir.rotated(std::f32::consts::PI / 6.0) * arrow_size;
        let p2 = arrow_pos - dir.rotated(-std::f32::consts::PI / 6.0) * arrow_size;

        painter.add(egui::Shape::convex_polygon(
            vec![arrow_pos, p1, p2],
            color,
            Stroke::NONE,
        ));
    }

    /// Draw all nodes
    fn draw_nodes(&mut self, ui: &mut Ui) {
        let nodes: Vec<_> = if let Some(ref tree) = self.tree {
            tree.nodes.values().cloned().collect()
        } else {
            return;
        };

        for node in nodes {
            self.draw_node(ui, &node);
        }
    }

    /// Draw a single node
    fn draw_node(&mut self, ui: &mut Ui, node: &DialogueNode) {
        let screen_pos = self.world_to_screen(pos2(node.position.x, node.position.y));
        let node_size = self.get_node_size(node);
        let node_rect = Rect::from_center_size(screen_pos, node_size);
        let node_id = Id::new(&node.id);

        // Node interaction
        let response = ui.interact(node_rect, node_id, Sense::click_and_drag());

        // Handle dragging
        if response.dragged_by(PointerButton::Primary) && self.dragging_node.is_none() {
            self.dragging_node = Some(node.id.clone());
        }

        if self.dragging_node.as_ref() == Some(&node.id) {
            if response.drag_stopped() {
                self.dragging_node = None;
                self.save_to_history("Move node");
            } else {
                let delta = response.drag_delta() / self.canvas_zoom;
                if let Some(ref mut tree) = self.tree {
                    if let Some(n) = tree.get_node_mut(&node.id) {
                        n.position.x += delta.x;
                        n.position.y += delta.y;
                        // Snap to grid
                        if self.show_grid {
                            n.position.x = (n.position.x / self.grid_size).round() * self.grid_size;
                            n.position.y = (n.position.y / self.grid_size).round() * self.grid_size;
                        }
                        self.modified = true;
                    }
                }
            }
        }

        // Handle selection
        if response.clicked() {
            self.selected_node = Some(node.id.clone());
        }

        // Handle double-click for connection
        if response.double_clicked() {
            self.drawing_connection = Some((node.id.clone(), ConnectionSource::Output));
        }

        // Draw the node
        self.draw_node_visual(ui, node_rect, node, response.hovered());

        // Context menu
        response.context_menu(|ui| {
            if ui.button("Duplicate").clicked() {
                self.duplicate_node(&node.id);
                ui.close_menu();
            }
            if ui.button("Delete").clicked() {
                self.delete_node(&node.id);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Connect...").clicked() {
                self.drawing_connection = Some((node.id.clone(), ConnectionSource::Output));
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Set as Root").clicked() {
                if let Some(ref mut tree) = self.tree {
                    tree.set_root_node(node.id.clone());
                    self.modified = true;
                }
                ui.close_menu();
            }
        });

        // Handle connection completion
        if response.clicked()
            && self.drawing_connection.is_some()
            && self.drawing_connection.as_ref().unwrap().0 != node.id
        {
            if let Some((source_id, _)) = self.drawing_connection.take() {
                self.create_connection(&source_id, &node.id);
            }
        }
    }

    /// Get the display size for a node
    fn get_node_size(&self, node: &DialogueNode) -> Vec2 {
        let base_width = 180.0;
        let base_height = match &node.node_type {
            DialogueNodeType::PlayerChoice => 60.0 + node.choices.len() as f32 * 15.0,
            _ => 70.0,
        };
        Vec2::new(base_width * self.canvas_zoom, base_height * self.canvas_zoom)
    }

    /// Draw the visual representation of a node
    fn draw_node_visual(&self, ui: &mut Ui, rect: Rect, node: &DialogueNode, hovered: bool) {
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
        let is_selected = self.selected_node.as_ref() == Some(&node.id);
        let is_root = self.tree.as_ref().map(|t| t.root_node == node.id).unwrap_or(false);
        
        if is_selected {
            painter.rect_stroke(
                rect.expand(4.0),
                6.0,
                Stroke::new(2.0, Color32::WHITE),
            );
        }

        // Root indicator
        if is_root {
            painter.rect_stroke(
                rect.expand(6.0),
                8.0,
                Stroke::new(2.0, Color32::YELLOW),
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
        let bg_color = if is_selected {
            Color32::from_gray(55)
        } else {
            Color32::from_gray(45)
        };
        painter.rect_filled(rect, 6.0, bg_color);

        // Header bar
        let header_rect = Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + 24.0 * self.canvas_zoom));
        painter.rect_filled(header_rect, Rounding::from(6.0).ne, color32);

        // Icon based on node type
        let icon = match node.node_type {
            DialogueNodeType::NpcText => "🗨",
            DialogueNodeType::PlayerChoice => "⚡",
            DialogueNodeType::Condition => "?",
            DialogueNodeType::Action => "⚙",
            DialogueNodeType::Branch { .. } => "🔀",
            DialogueNodeType::End => "⏹",
        };

        painter.text(
            header_rect.left_center() + Vec2::new(10.0 * self.canvas_zoom, 0.0),
            Align2::LEFT_CENTER,
            icon,
            egui::FontId::proportional(14.0 * self.canvas_zoom),
            Color32::WHITE,
        );

        // Title
        let title = node.display_name();
        let truncated_title = if title.len() > 20 {
            format!("{}...", &title[..17])
        } else {
            title
        };
        
        painter.text(
            header_rect.center(),
            Align2::CENTER_CENTER,
            truncated_title,
            egui::FontId::proportional(11.0 * self.canvas_zoom),
            Color32::WHITE,
        );

        // Node type label
        let type_label = match node.node_type {
            DialogueNodeType::NpcText => "Text",
            DialogueNodeType::PlayerChoice => "Choice",
            DialogueNodeType::Condition => "Condition",
            DialogueNodeType::Action => "Action",
            DialogueNodeType::Branch { mode } => match mode {
                BranchMode::Random => "Random",
                BranchMode::Weighted => "Weighted",
                BranchMode::Sequential => "Sequential",
                BranchMode::FirstValid => "First Valid",
            },
            DialogueNodeType::End => "End",
        };
        
        painter.text(
            rect.center_bottom() - Vec2::new(0.0, 8.0 * self.canvas_zoom),
            Align2::CENTER_BOTTOM,
            type_label,
            egui::FontId::proportional(9.0 * self.canvas_zoom),
            Color32::GRAY,
        );

        // Output socket (bottom center)
        let output_pos = rect.center_bottom();
        painter.circle_filled(output_pos, 6.0 * self.canvas_zoom, Color32::from_gray(80));
        painter.circle_stroke(
            output_pos,
            6.0 * self.canvas_zoom,
            Stroke::new(1.0, Color32::WHITE),
        );

        // Input socket (top center)
        let input_pos = rect.center_top();
        painter.circle_filled(input_pos, 6.0 * self.canvas_zoom, Color32::from_gray(80));
        painter.circle_stroke(
            input_pos,
            6.0 * self.canvas_zoom,
            Stroke::new(1.0, Color32::WHITE),
        );

        // Choice sockets for choice nodes
        if matches!(node.node_type, DialogueNodeType::PlayerChoice) {
            for i in 0..node.choices.len().min(4) {
                let x_offset = -40.0 + i as f32 * 27.0;
                let choice_pos = rect.center_bottom() + Vec2::new(x_offset * self.canvas_zoom, 0.0);
                painter.circle_filled(choice_pos, 5.0 * self.canvas_zoom, Color32::from_rgb(100, 200, 100));
                painter.circle_stroke(
                    choice_pos,
                    5.0 * self.canvas_zoom,
                    Stroke::new(1.0, Color32::WHITE),
                );
            }
        }
    }

    /// Draw the properties panel
    fn draw_properties_panel(&mut self, ui: &mut Ui) {
        ui.heading("Properties");
        ui.separator();

        let Some(ref mut tree) = self.tree else {
            ui.label("No tree loaded");
            return;
        };

        // Tree properties if no node selected
        if self.selected_node.is_none() {
            ui.collapsing("Tree Properties", |ui| {
                ui.horizontal(|ui| {
                    ui.label("ID:");
                    ui.text_edit_singleline(&mut tree.id);
                });
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut tree.name);
                });
                ui.horizontal(|ui| {
                    ui.label("Version:");
                    ui.text_edit_singleline(&mut tree.metadata.version);
                });
                ui.label("Description:");
                let mut desc = tree.metadata.description.clone().unwrap_or_default();
                if ui.text_edit_multiline(&mut desc).changed() {
                    tree.metadata.description = if desc.is_empty() { None } else { Some(desc) };
                    self.modified = true;
                }
                ui.horizontal(|ui| {
                    ui.label("Tags:");
                    let tags = tree.metadata.tags.join(", ");
                    ui.label(tags);
                });
            });
            ui.separator();
            return;
        }

        let selected_id = self.selected_node.clone().unwrap();
        let node = match tree.get_node_mut(&selected_id) {
            Some(n) => n,
            None => {
                ui.label(RichText::new("Node not found").color(Color32::RED));
                self.selected_node = None;
                return;
            }
        };

        // Node info header
        ui.horizontal(|ui| {
            let color = node.color();
            let color32 = Color32::from_rgb(color[0], color[1], color[2]);
            ui.label(
                RichText::new(match node.node_type {
                    DialogueNodeType::NpcText => "🗨",
                    DialogueNodeType::PlayerChoice => "⚡",
                    DialogueNodeType::Condition => "?",
                    DialogueNodeType::Action => "⚙",
                    DialogueNodeType::Branch { .. } => "🔀",
                    DialogueNodeType::End => "⏹",
                })
                .size(20.0)
                .color(color32),
            );
            ui.vertical(|ui| {
                ui.label(RichText::new(node.id.clone()).strong());
            });
        });

        ui.separator();

        // Node ID
        ui.horizontal(|ui| {
            ui.label("ID:");
            let mut new_id = node.id.clone();
            if ui.text_edit_singleline(&mut new_id).changed() && new_id != node.id && !new_id.is_empty() {
                // Update connections
                let old_id = node.id.clone();
                node.id = new_id.clone();
                
                // Update all references
                for n in tree.nodes.values_mut() {
                    for conn in n.connections.iter_mut() {
                        if conn.target_node == old_id {
                            conn.target_node = new_id.clone();
                        }
                    }
                    for choice in n.choices.iter_mut() {
                        if choice.next_node.as_ref() == Some(&old_id) {
                            choice.next_node = Some(new_id.clone());
                        }
                    }
                }
                
                // Update root if needed
                if tree.root_node == old_id {
                    tree.root_node = new_id.clone();
                }
                
                self.selected_node = Some(new_id);
                self.modified = true;
            }
        });

        ui.separator();

        // Common properties
        match &mut node.node_type {
            DialogueNodeType::NpcText => {
                ui.label("Speaker:");
                let mut speaker = node.speaker.clone().unwrap_or_default();
                if ui.text_edit_singleline(&mut speaker).changed() {
                    node.speaker = if speaker.is_empty() { None } else { Some(speaker) };
                    self.modified = true;
                }

                ui.label("Text:");
                if ui.text_edit_multiline(&mut node.text).changed() {
                    self.modified = true;
                }

                ui.separator();

                ui.collapsing("Visual & Audio", |ui| {
                    ui.label("Portrait:");
                    let mut portrait = node.portrait.clone().unwrap_or_default();
                    if ui.text_edit_singleline(&mut portrait).changed() {
                        node.portrait = if portrait.is_empty() { None } else { Some(portrait) };
                        self.modified = true;
                    }

                    ui.horizontal(|ui| {
                        ui.label("Position:");
                        egui::ComboBox::from_id_salt("portrait_pos")
                            .selected_text(format!("{:?}", node.portrait_position))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut node.portrait_position, PortraitPosition::None, "None");
                                ui.selectable_value(&mut node.portrait_position, PortraitPosition::Left, "Left");
                                ui.selectable_value(&mut node.portrait_position, PortraitPosition::Right, "Right");
                                ui.selectable_value(&mut node.portrait_position, PortraitPosition::Fullscreen, "Fullscreen");
                            });
                    });

                    ui.label("Voiceover:");
                    let mut voiceover = node.voiceover.clone().unwrap_or_default();
                    if ui.text_edit_singleline(&mut voiceover).changed() {
                        node.voiceover = if voiceover.is_empty() { None } else { Some(voiceover) };
                        self.modified = true;
                    }

                    ui.label("Sound Effect:");
                    let mut sfx = node.sound_effect.clone().unwrap_or_default();
                    if ui.text_edit_singleline(&mut sfx).changed() {
                        node.sound_effect = if sfx.is_empty() { None } else { Some(sfx) };
                        self.modified = true;
                    }

                    ui.label("Animation:");
                    let mut anim = node.animation_trigger.clone().unwrap_or_default();
                    if ui.text_edit_singleline(&mut anim).changed() {
                        node.animation_trigger = if anim.is_empty() { None } else { Some(anim) };
                        self.modified = true;
                    }

                    ui.label("Emotion:");
                    if ui.text_edit_singleline(&mut node.emotion).changed() {
                        self.modified = true;
                    }
                });
            }

            DialogueNodeType::PlayerChoice => {
                ui.label("Choices:");
                let mut to_remove = None;
                let mut to_update = None;

                for (i, choice) in node.choices.iter_mut().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}.", i + 1));
                            if ui.button("🗑").clicked() {
                                to_remove = Some(i);
                            }
                        });
                        
                        ui.label("Text:");
                        if ui.text_edit_singleline(&mut choice.text).changed() {
                            self.modified = true;
                        }
                        
                        ui.label("Target Node:");
                        let mut target = choice.next_node.clone().unwrap_or_default();
                        if ui.text_edit_singleline(&mut target).changed() {
                            choice.next_node = if target.is_empty() { None } else { Some(target) };
                            self.modified = true;
                        }
                        
                        ui.label("Tooltip:");
                        let mut tooltip = choice.tooltip.clone().unwrap_or_default();
                        if ui.text_edit_singleline(&mut tooltip).changed() {
                            choice.tooltip = if tooltip.is_empty() { None } else { Some(tooltip) };
                            self.modified = true;
                        }
                    });
                    ui.add_space(4.0);
                }

                if let Some(i) = to_remove {
                    node.choices.remove(i);
                    self.modified = true;
                }

                if ui.button("+ Add Choice").clicked() {
                    let id = format!("choice_{}", node.choices.len());
                    node.choices.push(DialogueChoice::new(&id, "New choice..."));
                    self.modified = true;
                }
            }

            DialogueNodeType::Condition => {
                ui.label("Conditions (AND logic):");
                let mut to_remove = None;

                for (i, cond) in node.conditions.iter_mut().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            if cond.negate {
                                ui.label("NOT");
                            }
                            if ui.button("🗑").clicked() {
                                to_remove = Some(i);
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Variable:");
                            ui.text_edit_singleline(&mut cond.variable);
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Operator:");
                            egui::ComboBox::from_id_salt(format!("cond_op_{}", i))
                                .selected_text(format!("{:?}", cond.operator))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut cond.operator, ConditionOp::Eq, "==");
                                    ui.selectable_value(&mut cond.operator, ConditionOp::Neq, "!=");
                                    ui.selectable_value(&mut cond.operator, ConditionOp::Gt, ">");
                                    ui.selectable_value(&mut cond.operator, ConditionOp::Gte, ">=");
                                    ui.selectable_value(&mut cond.operator, ConditionOp::Lt, "<");
                                    ui.selectable_value(&mut cond.operator, ConditionOp::Lte, "<=");
                                });
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Value:");
                            let mut val_str = cond.value.to_string();
                            if ui.text_edit_singleline(&mut val_str).changed() {
                                if let Ok(num) = val_str.parse::<i64>() {
                                    cond.value = serde_json::json!(num);
                                } else if let Ok(num) = val_str.parse::<f64>() {
                                    cond.value = serde_json::json!(num);
                                } else if val_str == "true" {
                                    cond.value = serde_json::json!(true);
                                } else if val_str == "false" {
                                    cond.value = serde_json::json!(false);
                                } else {
                                    cond.value = serde_json::json!(val_str);
                                }
                                self.modified = true;
                            }
                        });

                        ui.checkbox(&mut cond.negate, "Negate");
                    });
                    ui.add_space(4.0);
                }

                if let Some(i) = to_remove {
                    node.conditions.remove(i);
                    self.modified = true;
                }

                if ui.button("+ Add Condition").clicked() {
                    node.conditions.push(DialogueCondition::default());
                    self.modified = true;
                }

                ui.separator();
                ui.label("Connections:");
                ui.label("Connect output to nodes for true/false branches");
            }

            DialogueNodeType::Action => {
                ui.label("On Enter Effects:");
                let mut to_remove = None;

                for (i, effect) in node.on_enter_effects.iter_mut().enumerate() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(format!("{:?}", effect.action_type));
                            if ui.button("🗑").clicked() {
                                to_remove = Some(i);
                            }
                        });
                        
                        ui.horizontal(|ui| {
                            ui.label("Target:");
                            ui.text_edit_singleline(&mut effect.target);
                        });
                    });
                    ui.add_space(4.0);
                }

                if let Some(i) = to_remove {
                    node.on_enter_effects.remove(i);
                    self.modified = true;
                }

                ui.menu_button("+ Add Effect", |ui| {
                    if ui.button("Set Variable").clicked() {
                        node.on_enter_effects.push(DialogueEffect::new(ActionType::SetVariable, ""));
                        self.modified = true;
                        ui.close_menu();
                    }
                    if ui.button("Give Item").clicked() {
                        node.on_enter_effects.push(DialogueEffect::new(ActionType::GiveItem, ""));
                        self.modified = true;
                        ui.close_menu();
                    }
                    if ui.button("Trigger Animation").clicked() {
                        node.on_enter_effects.push(DialogueEffect::new(ActionType::TriggerAnimation, ""));
                        self.modified = true;
                        ui.close_menu();
                    }
                    if ui.button("Play Sound").clicked() {
                        node.on_enter_effects.push(DialogueEffect::new(ActionType::PlaySound, ""));
                        self.modified = true;
                        ui.close_menu();
                    }
                });
            }

            DialogueNodeType::Branch { mode } => {
                ui.horizontal(|ui| {
                    ui.label("Branch Mode:");
                    egui::ComboBox::from_id_salt("branch_mode")
                        .selected_text(format!("{:?}", mode))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(mode, BranchMode::Random, "Random");
                            ui.selectable_value(mode, BranchMode::Weighted, "Weighted");
                            ui.selectable_value(mode, BranchMode::Sequential, "Sequential");
                            ui.selectable_value(mode, BranchMode::FirstValid, "First Valid");
                        });
                });

                if *mode == BranchMode::Weighted {
                    ui.label("Set weights in connections panel");
                }
            }

            DialogueNodeType::End => {
                ui.label("End node - dialogue terminates here");
            }
        }

        ui.separator();

        // Connections editor
        if !matches!(node.node_type, DialogueNodeType::End) {
            ui.collapsing("Connections", |ui| {
                let mut to_remove = None;
                let mut to_update = None;

                for (i, conn) in node.connections.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("→ {}", conn.target_node));
                        if conn.condition.is_some() {
                            ui.label("(conditional)");
                        }
                        if ui.button("🗑").clicked() {
                            to_remove = Some(i);
                        }
                    });
                    
                    if matches!(node.node_type, DialogueNodeType::Branch { mode: BranchMode::Weighted }) {
                        ui.horizontal(|ui| {
                            ui.label("Weight:");
                            ui.add(DragValue::new(&mut conn.weight).speed(0.1));
                        });
                    }
                    
                    ui.horizontal(|ui| {
                        ui.label("Label:");
                        let mut label = conn.label.clone().unwrap_or_default();
                        if ui.text_edit_singleline(&mut label).changed() {
                            conn.label = if label.is_empty() { None } else { Some(label) };
                            self.modified = true;
                        }
                    });
                }

                if let Some(i) = to_remove {
                    node.connections.remove(i);
                    self.modified = true;
                }

                ui.label("Double-click node to start connection");
            });
        }

        ui.separator();

        // Comments
        ui.label("Comments:");
        let mut comments = node.comments.clone().unwrap_or_default();
        if ui.text_edit_multiline(&mut comments).changed() {
            node.comments = if comments.is_empty() { None } else { Some(comments) };
            self.modified = true;
        }

        ui.separator();

        // Position
        ui.horizontal(|ui| {
            ui.label("Position:");
            ui.add(DragValue::new(&mut node.position.x).prefix("X: ").speed(1.0));
            ui.add(DragValue::new(&mut node.position.y).prefix("Y: ").speed(1.0));
        });

        ui.separator();

        // Actions
        ui.horizontal(|ui| {
            if ui.button("🗑 Delete Node").clicked() {
                self.delete_node(&node.id.clone());
            }
            if ui.button("📋 Duplicate").clicked() {
                self.duplicate_node(&node.id.clone());
            }
        });
    }

    /// Draw the validation errors window
    fn draw_validation_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Validation Issues")
            .collapsible(true)
            .resizable(true)
            .open(&mut self.show_validation)
            .show(ctx, |ui| {
                if self.validation_errors.is_empty() {
                    ui.label(RichText::new("✓ No issues found").color(Color32::GREEN));
                } else {
                    for error in &self.validation_errors {
                        let color = match error {
                            ValidationError::MissingRootNode => Color32::RED,
                            ValidationError::InvalidConnection { .. } => Color32::RED,
                            _ => Color32::YELLOW,
                        };
                        ui.label(RichText::new(error.to_string()).color(color));
                    }
                }
            });
    }

    /// Draw the minimap
    fn draw_minimap(&self, ui: &mut Ui, rect: Rect) {
        let minimap_size = vec2(150.0, 100.0);
        let minimap_pos = pos2(
            rect.max.x - minimap_size.x - 10.0,
            rect.max.y - minimap_size.y - 10.0,
        );
        let minimap_rect = Rect::from_min_size(minimap_pos, minimap_size);

        let painter = ui.painter();

        // Background
        painter.rect_filled(minimap_rect, 4.0, Color32::from_gray(30));
        painter.rect_stroke(minimap_rect, 4.0, Stroke::new(1.0, Color32::from_gray(60)));

        let Some(ref tree) = self.tree else { return };
        if tree.nodes.is_empty() {
            return;
        }

        // Calculate bounds
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for node in tree.nodes.values() {
            min_x = min_x.min(node.position.x);
            max_x = max_x.max(node.position.x);
            min_y = min_y.min(node.position.y);
            max_y = max_y.max(node.position.y);
        }

        let bounds_width = (max_x - min_x).max(100.0);
        let bounds_height = (max_y - min_y).max(100.0);

        let scale_x = (minimap_size.x - 10.0) / bounds_width;
        let scale_y = (minimap_size.y - 10.0) / bounds_height;
        let scale = scale_x.min(scale_y);

        // Draw nodes on minimap
        for node in tree.nodes.values() {
            let x = minimap_pos.x + 5.0 + (node.position.x - min_x) * scale;
            let y = minimap_pos.y + 5.0 + (node.position.y - min_y) * scale;
            let color = node.color();
            let node_rect = Rect::from_center_size(pos2(x, y), vec2(8.0, 6.0));
            painter.rect_filled(node_rect, 1.0, Color32::from_rgb(color[0], color[1], color[2]));
        }

        // Draw viewport rectangle
        let viewport_x = minimap_pos.x + 5.0 + (-self.canvas_offset.x / self.canvas_zoom - min_x) * scale;
        let viewport_y = minimap_pos.y + 5.0 + (-self.canvas_offset.y / self.canvas_zoom - min_y) * scale;
        let viewport_width = rect.width() / self.canvas_zoom * scale;
        let viewport_height = rect.height() / self.canvas_zoom * scale;

        let viewport_rect = Rect::from_min_size(
            pos2(viewport_x, viewport_y),
            vec2(viewport_width, viewport_height),
        );
        painter.rect_stroke(
            viewport_rect.intersect(minimap_rect),
            1.0,
            Stroke::new(1.0, Color32::WHITE),
        );
    }

    /// Handle keyboard shortcuts
    fn handle_keyboard_shortcuts(&mut self, ui: &mut Ui) {
        ui.input(|i| {
            // Delete selected node
            if i.key_pressed(Key::Delete) {
                if let Some(id) = self.selected_node.clone() {
                    self.delete_node(&id);
                }
            }

            // Undo
            if i.modifiers.ctrl && i.key_pressed(Key::Z) {
                self.undo();
            }

            // Redo
            if i.modifiers.ctrl && i.key_pressed(Key::Y) {
                self.redo();
            }

            // Duplicate
            if i.modifiers.ctrl && i.key_pressed(Key::D) {
                if let Some(id) = self.selected_node.clone() {
                    self.duplicate_node(&id);
                }
            }

            // Frame all
            if i.key_pressed(Key::F) {
                self.frame_all();
            }

            // Escape cancels connection drawing
            if i.key_pressed(Key::Escape) {
                self.drawing_connection = None;
            }
        });
    }

    /// Create a connection between nodes
    fn create_connection(&mut self, source_id: &str, target_id: &str) {
        let Some(ref mut tree) = self.tree else { return };
        
        if let Some(source) = tree.get_node_mut(source_id) {
            // Check if connection already exists
            if !source.connections.iter().any(|c| c.target_node == target_id) {
                source.connections.push(NodeConnection::to(target_id));
                self.modified = true;
                self.save_to_history("Create connection");
            }
        }
    }

    /// Delete a node
    fn delete_node(&mut self, node_id: &str) {
        let Some(ref mut tree) = self.tree else { return };
        
        if tree.remove_node(node_id).is_some() {
            if self.selected_node.as_ref() == Some(node_id) {
                self.selected_node = None;
            }
            self.modified = true;
            self.save_to_history("Delete node");
        }
    }

    /// Duplicate a node
    fn duplicate_node(&mut self, node_id: &str) {
        let Some(ref mut tree) = self.tree else { return };
        
        if let Some(node) = tree.get_node(node_id).cloned() {
            let new_id = format!("{}_copy", node_id);
            let mut new_node = node.clone();
            new_node.id = new_id.clone();
            new_node.position.x += 50.0;
            new_node.position.y += 50.0;
            new_node.connections.clear(); // Don't copy connections
            
            tree.add_node(new_node);
            self.selected_node = Some(new_id);
            self.modified = true;
            self.save_to_history("Duplicate node");
        }
    }

    /// Duplicate selected nodes
    fn duplicate_selected(&mut self) {
        if let Some(id) = self.selected_node.clone() {
            self.duplicate_node(&id);
        }
    }

    /// Frame all nodes in view
    fn frame_all(&mut self) {
        let Some(ref tree) = self.tree else { return };
        let Some((min_x, max_x, min_y, max_y)) = tree.get_bounds() else { return };

        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        self.canvas_offset = Vec2::new(-center_x + 400.0, -center_y + 300.0);
        self.canvas_zoom = 1.0;
    }

    /// Validate the tree
    fn validate_tree(&mut self) {
        if let Some(ref tree) = self.tree {
            self.validation_errors = tree.validate();
            self.show_validation = !self.validation_errors.is_empty();
        }
    }

    /// Save to history for undo
    fn save_to_history(&mut self, description: &str) {
        let Some(ref tree) = self.tree else { return };
        
        // Remove any redo history
        if self.history_pos < self.history.len() {
            self.history.truncate(self.history_pos);
        }

        // Add new entry
        self.history.push(HistoryEntry {
            tree: tree.clone(),
            description: description.to_string(),
        });

        // Limit history size
        if self.history.len() > self.max_history {
            self.history.remove(0);
        } else {
            self.history_pos += 1;
        }
    }

    /// Undo
    fn undo(&mut self) {
        if self.history_pos > 0 {
            self.history_pos -= 1;
            if let Some(entry) = self.history.get(self.history_pos) {
                self.tree = Some(entry.tree.clone());
                self.modified = true;
            }
        }
    }

    /// Redo
    fn redo(&mut self) {
        if self.history_pos < self.history.len() {
            if let Some(entry) = self.history.get(self.history_pos) {
                self.tree = Some(entry.tree.clone());
                self.history_pos += 1;
                self.modified = true;
            }
        }
    }

    /// World to screen coordinate conversion
    fn world_to_screen(&self, world_pos: Pos2) -> Pos2 {
        pos2(
            world_pos.x * self.canvas_zoom + self.canvas_offset.x,
            world_pos.y * self.canvas_zoom + self.canvas_offset.y,
        )
    }

    /// Screen to world coordinate conversion
    fn screen_to_world(&self, screen_pos: Pos2) -> Pos2 {
        pos2(
            (screen_pos.x - self.canvas_offset.x) / self.canvas_zoom,
            (screen_pos.y - self.canvas_offset.y) / self.canvas_zoom,
        )
    }

    // Placeholder methods for file operations
    fn save_to_file(&mut self) {
        // Would implement actual file save
        self.modified = false;
    }

    fn save_as_dialog(&mut self) {
        // Would implement save as dialog
    }

    fn open_dialog(&mut self) {
        // Would implement open dialog
    }

    fn export_json(&mut self) {
        // Would implement JSON export
    }

    fn import_json(&mut self) {
        // Would implement JSON import
    }

    fn save_to_database(&mut self) {
        // Would implement database save
    }

    fn load_from_database(&mut self) {
        // Would implement database load
    }
}

impl Default for EditorMode {
    fn default() -> Self {
        EditorMode::Edit
    }
}
