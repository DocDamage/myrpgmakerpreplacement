//! Patrol Path Editor
//!
//! Visual editor for creating and managing NPC patrol routes.
//! Features:
//! - Visual path editing on map with click-to-add waypoints
//! - Drag to move waypoints
//! - Right-click to delete waypoints
//! - Connect waypoints with lines
//! - Waypoint properties (wait time, animation trigger, dialogue)
//! - Path properties (loop mode, color, speed)
//! - NPC assignment/unassignment
//! - Real-time preview with animated NPC movement

use dde_core::pathfinding::{PatrolLoopType, PatrolPath};
use glam::IVec2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique ID for patrol paths
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PatrolPathId(pub u64);

impl PatrolPathId {
    /// Generate a new unique ID
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for PatrolPathId {
    fn default() -> Self {
        Self::new()
    }
}

/// Enhanced waypoint with additional properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Waypoint {
    /// Position on map (grid coordinates)
    pub position: IVec2,
    /// Wait time at this waypoint (in ticks)
    pub wait_ticks: u32,
    /// Optional animation trigger when reaching this waypoint
    pub animation_trigger: Option<String>,
    /// Optional dialogue to trigger when reaching this waypoint
    pub dialogue: Option<String>,
}

impl Waypoint {
    /// Create a new waypoint at position
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            position: IVec2::new(x, y),
            wait_ticks: 0,
            animation_trigger: None,
            dialogue: None,
        }
    }

    /// Create with wait time
    pub fn with_wait(mut self, ticks: u32) -> Self {
        self.wait_ticks = ticks;
        self
    }

    /// Create with animation trigger
    pub fn with_animation(mut self, anim: impl Into<String>) -> Self {
        self.animation_trigger = Some(anim.into());
        self
    }

    /// Create with dialogue
    pub fn with_dialogue(mut self, dialogue: impl Into<String>) -> Self {
        self.dialogue = Some(dialogue.into());
        self
    }
}

impl Default for Waypoint {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Patrol path with enhanced editor properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatrolPathData {
    /// Path ID
    pub id: PatrolPathId,
    /// Path name
    pub name: String,
    /// Waypoints in order
    pub waypoints: Vec<Waypoint>,
    /// Loop mode
    pub loop_mode: PatrolLoopType,
    /// Path color for visualization
    pub color: [u8; 3],
    /// Speed modifier (1.0 = normal)
    pub speed_modifier: f32,
    /// Whether path is active
    pub active: bool,
    /// Associated map ID
    pub map_id: u32,
}

impl PatrolPathData {
    /// Create a new patrol path
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: PatrolPathId::new(),
            name: name.into(),
            waypoints: Vec::new(),
            loop_mode: PatrolLoopType::PingPong,
            color: [100, 200, 255],
            speed_modifier: 1.0,
            active: true,
            map_id: 1,
        }
    }

    /// Add a waypoint
    pub fn add_waypoint(&mut self, x: i32, y: i32) -> usize {
        self.waypoints.push(Waypoint::new(x, y));
        self.waypoints.len() - 1
    }

    /// Remove a waypoint by index
    pub fn remove_waypoint(&mut self, index: usize) -> Option<Waypoint> {
        if index < self.waypoints.len() {
            Some(self.waypoints.remove(index))
        } else {
            None
        }
    }

    /// Move waypoint to new position
    pub fn move_waypoint(&mut self, index: usize, x: i32, y: i32) -> bool {
        if let Some(wp) = self.waypoints.get_mut(index) {
            wp.position = IVec2::new(x, y);
            true
        } else {
            false
        }
    }

    /// Get waypoint at position (within tolerance)
    pub fn get_waypoint_at(&self, pos: IVec2, tolerance: i32) -> Option<usize> {
        self.waypoints.iter().position(|wp| {
            (wp.position.x - pos.x).abs() <= tolerance && 
            (wp.position.y - pos.y).abs() <= tolerance
        })
    }

    /// Convert to runtime PatrolPath
    pub fn to_patrol_path(&self) -> Option<PatrolPath> {
        if self.waypoints.len() < 2 {
            return None;
        }
        let positions: Vec<IVec2> = self.waypoints.iter()
            .map(|wp| wp.position)
            .collect();
        Some(PatrolPath::new(positions))
    }

    /// Get total path length (in grid cells)
    pub fn total_length(&self) -> f32 {
        if self.waypoints.len() < 2 {
            return 0.0;
        }
        let mut length = 0.0;
        for i in 1..self.waypoints.len() {
            let prev = self.waypoints[i - 1].position.as_vec2();
            let curr = self.waypoints[i].position.as_vec2();
            length += prev.distance(curr);
        }
        length
    }
}

impl Default for PatrolPathData {
    fn default() -> Self {
        Self::new("New Patrol Path")
    }
}

/// NPC info for assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcInfo {
    /// NPC unique ID
    pub id: u64,
    /// NPC display name
    pub name: String,
    /// Current map location
    pub map_id: u32,
    /// Current position
    pub position: IVec2,
    /// Currently assigned patrol path (if any)
    pub assigned_path: Option<PatrolPathId>,
}

impl NpcInfo {
    /// Create new NPC info
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            map_id: 1,
            position: IVec2::ZERO,
            assigned_path: None,
        }
    }
}

/// Editor tool mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolMode {
    /// Select and move waypoints
    Select,
    /// Add new waypoints
    Add,
    /// Delete waypoints
    Delete,
}

impl ToolMode {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            ToolMode::Select => "Select",
            ToolMode::Add => "Add",
            ToolMode::Delete => "Delete",
        }
    }

    /// Get icon
    pub fn icon(&self) -> &'static str {
        match self {
            ToolMode::Select => "🔍",
            ToolMode::Add => "➕",
            ToolMode::Delete => "🗑️",
        }
    }
}

/// Drag state for UI interactions
#[derive(Debug, Clone, Copy, PartialEq)]
enum DragState {
    /// Not dragging
    None,
    /// Dragging a waypoint
    DraggingWaypoint { path_id: PatrolPathId, waypoint_index: usize, start_pos: egui::Pos2 },
    /// Panning the view
    Panning { last_pos: egui::Pos2 },
}

/// Preview state for animation
#[derive(Debug, Clone)]
pub struct PreviewState {
    /// Whether preview is playing
    pub playing: bool,
    /// Current position along path (0.0 to 1.0)
    pub progress: f32,
    /// Current waypoint index
    pub current_waypoint: usize,
    /// Animation direction (1 = forward, -1 = backward)
    pub direction: i32,
    /// Speed multiplier
    pub speed: f32,
    /// Current world position
    pub position: glam::Vec2,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self {
            playing: false,
            progress: 0.0,
            current_waypoint: 0,
            direction: 1,
            speed: 2.0,
            position: glam::Vec2::ZERO,
        }
    }
}

/// Map view state for rendering
#[derive(Debug, Clone)]
pub struct MapViewState {
    /// Camera offset (pan)
    pub offset: egui::Vec2,
    /// Zoom level
    pub zoom: f32,
    /// Grid size in pixels (at zoom 1.0)
    pub grid_size: f32,
    /// Show grid
    pub show_grid: bool,
    /// Map width in tiles
    pub map_width: i32,
    /// Map height in tiles
    pub map_height: i32,
}

impl Default for MapViewState {
    fn default() -> Self {
        Self {
            offset: egui::Vec2::ZERO,
            zoom: 1.0,
            grid_size: 32.0,
            show_grid: true,
            map_width: 50,
            map_height: 50,
        }
    }
}

impl MapViewState {
    /// Convert grid position to screen position
    pub fn grid_to_screen(&self, grid_pos: IVec2, rect: &egui::Rect) -> egui::Pos2 {
        let x = rect.min.x + self.offset.x + grid_pos.x as f32 * self.grid_size * self.zoom;
        let y = rect.min.y + self.offset.y + grid_pos.y as f32 * self.grid_size * self.zoom;
        egui::pos2(x, y)
    }

    /// Convert screen position to grid position
    pub fn screen_to_grid(&self, screen_pos: egui::Pos2, rect: &egui::Rect) -> IVec2 {
        let x = (screen_pos.x - rect.min.x - self.offset.x) / (self.grid_size * self.zoom);
        let y = (screen_pos.y - rect.min.y - self.offset.y) / (self.grid_size * self.zoom);
        IVec2::new(x.round() as i32, y.round() as i32)
    }

    /// Zoom in
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.25).min(4.0);
    }

    /// Zoom out
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.25).max(0.25);
    }

    /// Reset zoom
    pub fn reset_zoom(&mut self) {
        self.zoom = 1.0;
    }
}

/// Patrol Path Editor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatrolEditor {
    /// Whether the editor is visible
    visible: bool,
    /// All patrol paths
    pub paths: HashMap<PatrolPathId, PatrolPathData>,
    /// Currently selected path
    selected_path: Option<PatrolPathId>,
    /// Currently selected waypoint index
    selected_waypoint: Option<usize>,
    /// Available NPCs for assignment
    pub npcs: Vec<NpcInfo>,
    /// Current tool mode
    tool_mode: ToolMode,
    /// Map view state
    #[serde(skip)]
    map_view: MapViewState,
    /// Drag state
    #[serde(skip)]
    drag_state: DragState,
    /// Preview state for animation
    #[serde(skip)]
    preview: PreviewState,
    /// Whether to show path preview
    show_preview: bool,
    /// Next path number for auto-naming
    next_path_number: u32,
    /// Pending delete path ID (for context menu)
    #[serde(skip)]
    pending_delete: Option<PatrolPathId>,
}

impl PatrolEditor {
    /// Create a new patrol editor
    pub fn new() -> Self {
        Self {
            visible: false,
            paths: HashMap::new(),
            selected_path: None,
            selected_waypoint: None,
            npcs: Vec::new(),
            tool_mode: ToolMode::Select,
            map_view: MapViewState::default(),
            drag_state: DragState::None,
            preview: PreviewState::default(),
            show_preview: true,
            next_path_number: 1,
            pending_delete: None,
        }
    }

    /// Show the editor
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the editor
    pub fn hide(&mut self) {
        self.visible = false;
        self.preview.playing = false;
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        if self.visible {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Create a new patrol path
    pub fn create_path(&mut self, name: Option<String>) -> PatrolPathId {
        let name = name.unwrap_or_else(|| {
            let name = format!("Patrol Path {}", self.next_path_number);
            self.next_path_number += 1;
            name
        });
        
        let path = PatrolPathData::new(name);
        let id = path.id;
        self.paths.insert(id, path);
        self.selected_path = Some(id);
        self.selected_waypoint = None;
        id
    }

    /// Delete a patrol path
    pub fn delete_path(&mut self, id: PatrolPathId) {
        // Unassign from all NPCs
        for npc in &mut self.npcs {
            if npc.assigned_path == Some(id) {
                npc.assigned_path = None;
            }
        }
        
        self.paths.remove(&id);
        
        if self.selected_path == Some(id) {
            self.selected_path = self.paths.keys().next().copied();
            self.selected_waypoint = None;
        }
    }

    /// Get selected path
    pub fn selected_path(&self) -> Option<&PatrolPathData> {
        self.selected_path.and_then(|id| self.paths.get(&id))
    }

    /// Get selected path mutable
    pub fn selected_path_mut(&mut self) -> Option<&mut PatrolPathData> {
        self.selected_path.and_then(|id| self.paths.get_mut(&id))
    }

    /// Add NPC to editor
    pub fn add_npc(&mut self, npc: NpcInfo) {
        self.npcs.push(npc);
    }

    /// Remove NPC from editor
    pub fn remove_npc(&mut self, npc_id: u64) {
        self.npcs.retain(|n| n.id != npc_id);
    }

    /// Assign path to NPC
    pub fn assign_path_to_npc(&mut self, path_id: PatrolPathId, npc_id: u64) -> bool {
        if !self.paths.contains_key(&path_id) {
            return false;
        }
        
        if let Some(npc) = self.npcs.iter_mut().find(|n| n.id == npc_id) {
            npc.assigned_path = Some(path_id);
            true
        } else {
            false
        }
    }

    /// Unassign path from NPC
    pub fn unassign_path_from_npc(&mut self, npc_id: u64) -> bool {
        if let Some(npc) = self.npcs.iter_mut().find(|n| n.id == npc_id) {
            npc.assigned_path = None;
            true
        } else {
            false
        }
    }

    /// Get NPCs assigned to a path
    pub fn get_assigned_npcs(&self, path_id: PatrolPathId) -> Vec<&NpcInfo> {
        self.npcs.iter()
            .filter(|n| n.assigned_path == Some(path_id))
            .collect()
    }

    /// Draw the editor UI
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        // Update preview animation
        if self.preview.playing {
            self.update_preview(ctx.input(|i| i.stable_dt));
        }

        egui::Window::new("🚶 Patrol Path Editor")
            .default_size([1200.0, 800.0])
            .resizable(true)
            .show(ctx, |ui| {
                self.draw_ui(ui);
            });
    }

    /// Draw the main UI
    fn draw_ui(&mut self, ui: &mut egui::Ui) {
        // Top toolbar
        egui::TopBottomPanel::top("patrol_toolbar")
            .exact_height(50.0)
            .show_inside(ui, |ui| {
                self.draw_toolbar(ui);
            });

        // Left panel: Path list
        egui::SidePanel::left("patrol_path_list")
            .default_width(220.0)
            .show_inside(ui, |ui| {
                self.draw_path_list(ui);
            });

        // Right panel: Properties
        egui::SidePanel::right("patrol_properties")
            .default_width(280.0)
            .show_inside(ui, |ui| {
                self.draw_properties_panel(ui);
            });

        // Central panel: Map view
        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.draw_map_view(ui);
        });
    }

    /// Draw the toolbar
    fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Path selector
            ui.label("Path:");
            egui::ComboBox::from_id_source("path_selector")
                .selected_text(
                    self.selected_path()
                        .map(|p| p.name.as_str())
                        .unwrap_or("Select Path..."),
                )
                .width(200.0)
                .show_ui(ui, |ui| {
                    for (id, path) in &self.paths {
                        let selected = self.selected_path == Some(*id);
                        if ui.selectable_label(selected, &path.name).clicked() {
                            self.selected_path = Some(*id);
                            self.selected_waypoint = None;
                        }
                    }
                });

            ui.separator();

            // Tool mode buttons
            ui.label("Tool:");
            for mode in [ToolMode::Select, ToolMode::Add, ToolMode::Delete] {
                let selected = self.tool_mode == mode;
                if ui.selectable_label(selected, format!("{} {}", mode.icon(), mode.name()))
                    .clicked() {
                    self.tool_mode = mode;
                }
            }

            ui.separator();

            // Zoom controls
            ui.label("Zoom:");
            if ui.button("-").clicked() {
                self.map_view.zoom_out();
            }
            ui.label(format!("{:.0}%", self.map_view.zoom * 100.0));
            if ui.button("+").clicked() {
                self.map_view.zoom_in();
            }

            ui.separator();

            // Preview controls
            ui.checkbox(&mut self.show_preview, "Preview");
            if self.show_preview {
                let play_text = if self.preview.playing { "⏸" } else { "▶" };
                if ui.button(play_text).clicked() {
                    self.preview.playing = !self.preview.playing;
                }
                if ui.button("⏮").clicked() {
                    self.reset_preview();
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕").clicked() {
                    self.hide();
                }
                if ui.button("➕ New Path").clicked() {
                    self.create_path(None);
                }
            });
        });
    }

    /// Draw the path list panel
    fn draw_path_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("Patrol Paths");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (id, path) in &self.paths {
                let is_selected = self.selected_path == Some(*id);
                
                let response = ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    
                    ui.horizontal(|ui| {
                        // Color indicator
                        let color = egui::Color32::from_rgb(path.color[0], path.color[1], path.color[2]);
                        let (color_rect, _) = ui.allocate_exact_size(
                            egui::vec2(16.0, 16.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(color_rect, 4.0, color);
                        
                        ui.vertical(|ui| {
                            let name_text = if is_selected {
                                egui::RichText::new(&path.name).strong()
                            } else {
                                egui::RichText::new(&path.name)
                            };
                            ui.label(name_text);
                            ui.label(format!("{} waypoints", path.waypoints.len()))
                                .on_hover_text(format!("Loop: {:?}", path.loop_mode));
                        });
                    });
                });

                let response = response.response;
                if response.clicked() {
                    self.selected_path = Some(*id);
                    self.selected_waypoint = None;
                }

                // Context menu for delete
                response.context_menu(|ui| {
                    if ui.button("🗑️ Delete Path").clicked() {
                        self.pending_delete = Some(*id);
                        ui.close_menu();
                    }
                    if ui.button("📋 Duplicate").clicked() {
                        self.duplicate_path(*id);
                        ui.close_menu();
                    }
                });
            }
        });

        // Handle pending delete
        if let Some(id) = self.pending_delete {
            self.delete_path(id);
            self.pending_delete = None;
        }
    }

    /// Duplicate a path
    fn duplicate_path(&mut self, id: PatrolPathId) {
        if let Some(original) = self.paths.get(&id).cloned() {
            let mut new_path = original;
            new_path.id = PatrolPathId::new();
            new_path.name = format!("{} (Copy)", original.name);
            let new_id = new_path.id;
            self.paths.insert(new_id, new_path);
            self.selected_path = Some(new_id);
        }
    }

    /// Draw the properties panel
    fn draw_properties_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Properties");
        ui.separator();

        // Path properties
        if let Some(path) = self.selected_path_mut() {
            // Name
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut path.name);
            });
            ui.separator();

            // Color picker
            ui.label("Path Color:");
            ui.horizontal(|ui| {
                let mut color = egui::Color32::from_rgb(path.color[0], path.color[1], path.color[2]);
                egui::widgets::color_picker::color_edit_button_srgb(
                    ui,
                    &mut path.color,
                );
            });
            ui.separator();

            // Loop mode
            ui.label("Loop Mode:");
            for mode in [PatrolLoopType::Loop, PatrolLoopType::PingPong, PatrolLoopType::Once] {
                let selected = path.loop_mode == mode;
                let label = match mode {
                    PatrolLoopType::Loop => "🔁 Loop",
                    PatrolLoopType::PingPong => "🏓 Ping Pong",
                    PatrolLoopType::Once => "⏹️ Once",
                };
                if ui.selectable_label(selected, label).clicked() {
                    path.loop_mode = mode;
                }
            }
            ui.separator();

            // Speed modifier
            ui.horizontal(|ui| {
                ui.label("Speed:");
                ui.add(egui::Slider::new(&mut path.speed_modifier, 0.1..=3.0));
            });
            ui.separator();

            // Waypoint properties (if selected)
            if let Some(wp_index) = self.selected_waypoint {
                if let Some(waypoint) = path.waypoints.get_mut(wp_index) {
                    ui.label(format!("Waypoint {}", wp_index + 1));
                    ui.separator();

                    // Position
                    ui.horizontal(|ui| {
                        ui.label("X:");
                        ui.add(egui::DragValue::new(&mut waypoint.position.x).speed(1));
                        ui.label("Y:");
                        ui.add(egui::DragValue::new(&mut waypoint.position.y).speed(1));
                    });

                    // Wait time
                    ui.horizontal(|ui| {
                        ui.label("Wait (ticks):");
                        ui.add(egui::DragValue::new(&mut waypoint.wait_ticks).speed(1));
                    });

                    // Animation trigger
                    ui.horizontal(|ui| {
                        ui.label("Animation:");
                        let mut anim = waypoint.animation_trigger.clone().unwrap_or_default();
                        if ui.text_edit_singleline(&mut anim).lost_focus() {
                            waypoint.animation_trigger = if anim.is_empty() { None } else { Some(anim) };
                        }
                    });

                    // Dialogue
                    ui.horizontal(|ui| {
                        ui.label("Dialogue:");
                        let mut dialogue = waypoint.dialogue.clone().unwrap_or_default();
                        if ui.text_edit_singleline(&mut dialogue).lost_focus() {
                            waypoint.dialogue = if dialogue.is_empty() { None } else { Some(dialogue) };
                        }
                    });

                    ui.separator();

                    // Delete waypoint button
                    if ui.button("🗑️ Delete Waypoint").clicked() {
                        path.remove_waypoint(wp_index);
                        self.selected_waypoint = None;
                    }
                }
            } else {
                ui.label("Select a waypoint to edit properties");
            }

            ui.separator();

            // Path stats
            ui.label(egui::RichText::new("Path Stats").strong());
            ui.label(format!("Waypoints: {}", path.waypoints.len()));
            ui.label(format!("Total length: {:.1} tiles", path.total_length()));
            
            // Assigned NPCs
            let assigned = self.get_assigned_npcs(path.id);
            if !assigned.is_empty() {
                ui.label("Assigned NPCs:");
                for npc in assigned {
                    ui.label(format!("• {}", npc.name));
                }
            }
        } else {
            ui.label("Select a path to edit properties");
        }

        ui.separator();

        // NPC Assignment section
        ui.heading("NPC Assignment");
        ui.separator();

        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            for npc in &mut self.npcs {
                ui.horizontal(|ui| {
                    let is_assigned = npc.assigned_path == self.selected_path;
                    
                    if ui.checkbox(&mut { is_assigned }, &npc.name).changed() {
                        if is_assigned {
                            // Unassign
                            npc.assigned_path = None;
                        } else if let Some(path_id) = self.selected_path {
                            npc.assigned_path = Some(path_id);
                        }
                    }
                    
                    if let Some(path_id) = npc.assigned_path {
                        if let Some(path) = self.paths.get(&path_id) {
                            ui.label(format!("({})", path.name))
                                .on_hover_text(&path.name);
                        }
                    }
                });
            }
        });
    }

    /// Draw the map view
    fn draw_map_view(&mut self, ui: &mut egui::Ui) {
        let available_rect = ui.available_rect_before_wrap();
        
        // Map view background
        let bg_color = egui::Color32::from_rgb(40, 40, 45);
        ui.painter().rect_filled(available_rect, 0.0, bg_color);

        // Grid
        if self.map_view.show_grid {
            self.draw_grid(ui, available_rect);
        }

        // Draw all paths
        for (id, path) in &self.paths {
            let is_selected = self.selected_path == Some(*id);
            self.draw_path(ui, available_rect, path, is_selected);
        }

        // Draw preview NPC
        if self.show_preview {
            self.draw_preview_npc(ui, available_rect);
        }

        // Handle input
        self.handle_map_input(ui, available_rect);
    }

    /// Draw the grid
    fn draw_grid(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        let painter = ui.painter();
        let grid_color = egui::Color32::from_rgba_premultiplied(100, 100, 100, 50);
        let major_grid_color = egui::Color32::from_rgba_premultiplied(120, 120, 120, 80);

        let start_x = ((rect.min.x - self.map_view.offset.x) / (self.map_view.grid_size * self.map_view.zoom)).floor() as i32;
        let end_x = ((rect.max.x - self.map_view.offset.x) / (self.map_view.grid_size * self.map_view.zoom)).ceil() as i32;
        let start_y = ((rect.min.y - self.map_view.offset.y) / (self.map_view.grid_size * self.map_view.zoom)).floor() as i32;
        let end_y = ((rect.max.y - self.map_view.offset.y) / (self.map_view.grid_size * self.map_view.zoom)).ceil() as i32;

        // Vertical lines
        for x in start_x..=end_x {
            let screen_x = rect.min.x + self.map_view.offset.x + x as f32 * self.map_view.grid_size * self.map_view.zoom;
            if screen_x < rect.min.x || screen_x > rect.max.x {
                continue;
            }
            let color = if x % 10 == 0 { major_grid_color } else { grid_color };
            painter.line_segment(
                [egui::pos2(screen_x, rect.min.y), egui::pos2(screen_x, rect.max.y)],
                egui::Stroke::new(1.0, color),
            );
        }

        // Horizontal lines
        for y in start_y..=end_y {
            let screen_y = rect.min.y + self.map_view.offset.y + y as f32 * self.map_view.grid_size * self.map_view.zoom;
            if screen_y < rect.min.y || screen_y > rect.max.y {
                continue;
            }
            let color = if y % 10 == 0 { major_grid_color } else { grid_color };
            painter.line_segment(
                [egui::pos2(rect.min.x, screen_y), egui::pos2(rect.max.x, screen_y)],
                egui::Stroke::new(1.0, color),
            );
        }
    }

    /// Draw a single path
    fn draw_path(&self, ui: &mut egui::Ui, rect: egui::Rect, path: &PatrolPathData, is_selected: bool) {
        let painter = ui.painter();
        let color = egui::Color32::from_rgb(path.color[0], path.color[1], path.color[2]);
        let waypoint_radius = if is_selected { 8.0 } else { 6.0 };
        let line_width = if is_selected { 3.0 } else { 2.0 };

        // Draw connecting lines
        if path.waypoints.len() >= 2 {
            let line_color = if is_selected { color } else { color.linear_multiply(0.6) };
            
            for i in 1..path.waypoints.len() {
                let start = self.map_view.grid_to_screen(path.waypoints[i - 1].position, &rect);
                let end = self.map_view.grid_to_screen(path.waypoints[i].position, &rect);
                
                painter.line_segment(
                    [start, end],
                    egui::Stroke::new(line_width, line_color),
                );

                // Draw direction arrow at midpoint
                if is_selected {
                    let mid = egui::pos2((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
                    let angle = (end.y - start.y).atan2(end.x - start.x);
                    self.draw_arrow(painter, mid, angle, line_color);
                }
            }

            // Draw loop closing line if Loop mode
            if path.loop_mode == PatrolLoopType::Loop && path.waypoints.len() > 2 {
                let start = self.map_view.grid_to_screen(path.waypoints.last().unwrap().position, &rect);
                let end = self.map_view.grid_to_screen(path.waypoints.first().unwrap().position, &rect);
                let loop_color = line_color.linear_multiply(0.5);
                painter.line_segment(
                    [start, end],
                    egui::Stroke::new(line_width * 0.8, loop_color),
                );
            }
        }

        // Draw waypoints
        for (i, waypoint) in path.waypoints.iter().enumerate() {
            let pos = self.map_view.grid_to_screen(waypoint.position, &rect);
            let is_waypoint_selected = is_selected && self.selected_waypoint == Some(i);
            
            // Waypoint circle
            let wp_color = if is_waypoint_selected {
                egui::Color32::YELLOW
            } else if is_selected {
                color
            } else {
                color.linear_multiply(0.7)
            };
            
            painter.circle_filled(pos, waypoint_radius, wp_color);
            painter.circle_stroke(pos, waypoint_radius, egui::Stroke::new(2.0, egui::Color32::WHITE));

            // Waypoint number (if selected)
            if is_selected {
                painter.text(
                    pos - egui::vec2(0.0, waypoint_radius + 10.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{}", i + 1),
                    egui::FontId::proportional(10.0),
                    egui::Color32::WHITE,
                );
            }

            // Special indicators
            if waypoint.wait_ticks > 0 {
                painter.text(
                    pos + egui::vec2(0.0, waypoint_radius + 10.0),
                    egui::Align2::CENTER_CENTER,
                    "⏱",
                    egui::FontId::proportional(10.0),
                    egui::Color32::YELLOW,
                );
            }
            if waypoint.dialogue.is_some() {
                painter.text(
                    pos + egui::vec2(waypoint_radius + 8.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    "💬",
                    egui::FontId::proportional(10.0),
                    egui::Color32::LIGHT_BLUE,
                );
            }
        }
    }

    /// Draw direction arrow
    fn draw_arrow(&self, painter: &egui::Painter, pos: egui::Pos2, angle: f32, color: egui::Color32) {
        let size = 8.0;
        let arrow_angle = std::f32::consts::PI / 6.0;
        
        let p1 = egui::pos2(
            pos.x - size * (angle + arrow_angle).cos(),
            pos.y - size * (angle + arrow_angle).sin(),
        );
        let p2 = egui::pos2(
            pos.x - size * (angle - arrow_angle).cos(),
            pos.y - size * (angle - arrow_angle).sin(),
        );
        
        painter.line_segment([pos, p1], egui::Stroke::new(2.0, color));
        painter.line_segment([pos, p2], egui::Stroke::new(2.0, color));
    }

    /// Draw preview NPC
    fn draw_preview_npc(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        let Some(path_id) = self.selected_path else { return };
        let Some(path) = self.paths.get(&path_id) else { return };
        if path.waypoints.is_empty() { return; }

        let painter = ui.painter();
        let pos = self.map_view.grid_to_screen(
            IVec2::new(self.preview.position.x as i32, self.preview.position.y as i32),
            &rect,
        );

        // Draw NPC as a circle
        let npc_color = egui::Color32::from_rgb(path.color[0], path.color[1], path.color[2]);
        painter.circle_filled(pos, 10.0, npc_color);
        painter.circle_stroke(pos, 10.0, egui::Stroke::new(2.0, egui::Color32::WHITE));
        
        // Draw NPC label
        painter.text(
            pos - egui::vec2(0.0, 15.0),
            egui::Align2::CENTER_BOTTOM,
            "NPC",
            egui::FontId::proportional(10.0),
            egui::Color32::WHITE,
        );
    }

    /// Handle map input (clicks, drags)
    fn handle_map_input(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let response = ui.interact(rect, ui.id().with("map_view"), egui::Sense::click_and_drag());

        // Handle right-click for delete
        if response.secondary_clicked() {
            let mouse_pos = response.interact_pointer_pos().unwrap_or_default();
            let grid_pos = self.map_view.screen_to_grid(mouse_pos, &rect);
            
            if let Some(path_id) = self.selected_path {
                if let Some(path) = self.paths.get_mut(&path_id) {
                    if let Some(index) = path.get_waypoint_at(grid_pos, 1) {
                        path.remove_waypoint(index);
                        if self.selected_waypoint == Some(index) {
                            self.selected_waypoint = None;
                        }
                    }
                }
            }
        }

        // Handle tool mode interactions
        match self.tool_mode {
            ToolMode::Add => {
                if response.clicked() {
                    if let Some(mouse_pos) = response.interact_pointer_pos() {
                        let grid_pos = self.map_view.screen_to_grid(mouse_pos, &rect);
                        
                        if let Some(path_id) = self.selected_path {
                            if let Some(path) = self.paths.get_mut(&path_id) {
                                // Don't add if too close to existing waypoint
                                if path.get_waypoint_at(grid_pos, 1).is_none() {
                                    let index = path.add_waypoint(grid_pos.x, grid_pos.y);
                                    self.selected_waypoint = Some(index);
                                }
                            }
                        }
                    }
                }
            }
            ToolMode::Select => {
                if response.drag_started() {
                    if let Some(mouse_pos) = response.interact_pointer_pos() {
                        let grid_pos = self.map_view.screen_to_grid(mouse_pos, &rect);
                        
                        // Check if clicking on a waypoint
                        if let Some(path_id) = self.selected_path {
                            if let Some(path) = self.paths.get(&path_id) {
                                if let Some(index) = path.get_waypoint_at(grid_pos, 1) {
                                    self.drag_state = DragState::DraggingWaypoint {
                                        path_id,
                                        waypoint_index: index,
                                        start_pos: mouse_pos,
                                    };
                                    self.selected_waypoint = Some(index);
                                } else {
                                    self.drag_state = DragState::Panning { last_pos: mouse_pos };
                                }
                            }
                        } else {
                            self.drag_state = DragState::Panning { last_pos: mouse_pos };
                        }
                    }
                }

                if response.dragged() {
                    match self.drag_state {
                        DragState::DraggingWaypoint { path_id, waypoint_index, .. } => {
                            if let Some(mouse_pos) = response.interact_pointer_pos() {
                                let grid_pos = self.map_view.screen_to_grid(mouse_pos, &rect);
                                
                                if let Some(path) = self.paths.get_mut(&path_id) {
                                    path.move_waypoint(waypoint_index, grid_pos.x, grid_pos.y);
                                }
                            }
                        }
                        DragState::Panning { last_pos } => {
                            if let Some(mouse_pos) = response.interact_pointer_pos() {
                                let delta = mouse_pos - last_pos;
                                self.map_view.offset += delta;
                                self.drag_state = DragState::Panning { last_pos: mouse_pos };
                            }
                        }
                        _ => {}
                    }
                }

                if response.drag_stopped() {
                    self.drag_state = DragState::None;
                }

                if response.clicked() {
                    if let Some(mouse_pos) = response.interact_pointer_pos() {
                        let grid_pos = self.map_view.screen_to_grid(mouse_pos, &rect);
                        
                        // Try to select a waypoint
                        if let Some(path_id) = self.selected_path {
                            if let Some(path) = self.paths.get(&path_id) {
                                self.selected_waypoint = path.get_waypoint_at(grid_pos, 1);
                            }
                        }
                    }
                }
            }
            ToolMode::Delete => {
                if response.clicked() {
                    if let Some(mouse_pos) = response.interact_pointer_pos() {
                        let grid_pos = self.map_view.screen_to_grid(mouse_pos, &rect);
                        
                        if let Some(path_id) = self.selected_path {
                            if let Some(path) = self.paths.get_mut(&path_id) {
                                if let Some(index) = path.get_waypoint_at(grid_pos, 1) {
                                    path.remove_waypoint(index);
                                    if self.selected_waypoint == Some(index) {
                                        self.selected_waypoint = None;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Handle scroll for zoom
        if response.hovered() {
            ui.input(|i| {
                if i.raw_scroll_delta.y > 0.0 {
                    self.map_view.zoom_in();
                } else if i.raw_scroll_delta.y < 0.0 {
                    self.map_view.zoom_out();
                }
            });
        }
    }

    /// Update preview animation
    fn update_preview(&mut self, dt: f32) {
        let Some(path_id) = self.selected_path else { return };
        let Some(path) = self.paths.get(&path_id) else { return };
        if path.waypoints.len() < 2 { return; }

        let speed = self.preview.speed * path.speed_modifier * dt;
        let current_wp_index = self.preview.current_waypoint;
        let next_wp_index = match path.loop_mode {
            PatrolLoopType::Loop => (current_wp_index + 1) % path.waypoints.len(),
            PatrolLoopType::PingPong => {
                if self.preview.direction > 0 {
                    (current_wp_index + 1).min(path.waypoints.len() - 1)
                } else {
                    current_wp_index.saturating_sub(1)
                }
            }
            PatrolLoopType::Once => (current_wp_index + 1).min(path.waypoints.len() - 1),
        };

        let current_wp = &path.waypoints[current_wp_index];
        let next_wp = &path.waypoints[next_wp_index];

        // Move towards next waypoint
        let current_pos = current_wp.position.as_vec2();
        let target_pos = next_wp.position.as_vec2();
        let direction = target_pos - current_pos;
        let distance = direction.length();

        if distance > 0.0 {
            let move_amount = speed.min(distance);
            self.preview.position += direction.normalize() * move_amount;

            // Check if reached waypoint
            if self.preview.position.distance(target_pos) < 0.1 {
                self.preview.current_waypoint = next_wp_index;
                
                // Handle direction change for ping-pong
                if path.loop_mode == PatrolLoopType::PingPong {
                    if next_wp_index == 0 || next_wp_index == path.waypoints.len() - 1 {
                        self.preview.direction *= -1;
                    }
                }
                
                // Handle stop for once mode
                if path.loop_mode == PatrolLoopType::Once && next_wp_index == path.waypoints.len() - 1 {
                    self.preview.playing = false;
                }
            }
        }
    }

    /// Reset preview to start
    fn reset_preview(&mut self) {
        self.preview.playing = false;
        self.preview.current_waypoint = 0;
        self.preview.direction = 1;
        
        if let Some(path_id) = self.selected_path {
            if let Some(path) = self.paths.get(&path_id) {
                if let Some(first_wp) = path.waypoints.first() {
                    self.preview.position = first_wp.position.as_vec2();
                }
            }
        }
    }

    /// Export path to JSON
    pub fn export_path_to_json(&self, path_id: PatrolPathId) -> Option<String> {
        self.paths.get(&path_id)
            .and_then(|p| serde_json::to_string_pretty(p).ok())
    }

    /// Import path from JSON
    pub fn import_path_from_json(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let path: PatrolPathData = serde_json::from_str(json)?;
        self.paths.insert(path.id, path);
        Ok(())
    }

    /// Get all paths as slice
    pub fn get_paths(&self) -> Vec<&PatrolPathData> {
        self.paths.values().collect()
    }

    /// Get path by ID
    pub fn get_path(&self, id: PatrolPathId) -> Option<&PatrolPathData> {
        self.paths.get(&id)
    }

    /// Get mutable path by ID
    pub fn get_path_mut(&mut self, id: PatrolPathId) -> Option<&mut PatrolPathData> {
        self.paths.get_mut(&id)
    }
}

impl Default for PatrolEditor {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for Editor integration
pub trait PatrolEditorExt {
    /// Draw the NPC menu with Patrol Paths option
    fn draw_npc_menu_with_patrol(&mut self, ui: &mut egui::Ui);
    /// Draw the patrol editor window
    fn draw_patrol_editor(&mut self, ctx: &egui::Context);
    /// Get mutable reference to patrol editor
    fn patrol_editor_mut(&mut self) -> &mut PatrolEditor;
    /// Get reference to patrol editor
    fn patrol_editor(&self) -> &PatrolEditor;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patrol_editor_new() {
        let editor = PatrolEditor::new();
        assert!(!editor.is_visible());
        assert!(editor.paths.is_empty());
    }

    #[test]
    fn test_create_path() {
        let mut editor = PatrolEditor::new();
        let id = editor.create_path(Some("Test Path".to_string()));
        assert_eq!(editor.paths.len(), 1);
        assert!(editor.paths.contains_key(&id));
    }

    #[test]
    fn test_waypoint_operations() {
        let mut path = PatrolPathData::new("Test");
        
        // Add waypoints
        let idx1 = path.add_waypoint(10, 10);
        let idx2 = path.add_waypoint(20, 20);
        assert_eq!(path.waypoints.len(), 2);
        
        // Move waypoint
        assert!(path.move_waypoint(idx1, 15, 15));
        assert_eq!(path.waypoints[idx1].position, IVec2::new(15, 15));
        
        // Get waypoint at position
        assert_eq!(path.get_waypoint_at(IVec2::new(15, 15), 1), Some(idx1));
        assert_eq!(path.get_waypoint_at(IVec2::new(20, 20), 1), Some(idx2));
        assert_eq!(path.get_waypoint_at(IVec2::new(100, 100), 1), None);
        
        // Remove waypoint
        path.remove_waypoint(idx1);
        assert_eq!(path.waypoints.len(), 1);
    }

    #[test]
    fn test_npc_assignment() {
        let mut editor = PatrolEditor::new();
        let path_id = editor.create_path(None);
        
        editor.add_npc(NpcInfo::new(1, "NPC 1"));
        editor.add_npc(NpcInfo::new(2, "NPC 2"));
        
        // Assign path
        assert!(editor.assign_path_to_npc(path_id, 1));
        assert_eq!(editor.npcs[0].assigned_path, Some(path_id));
        
        // Get assigned NPCs
        let assigned = editor.get_assigned_npcs(path_id);
        assert_eq!(assigned.len(), 1);
        assert_eq!(assigned[0].id, 1);
        
        // Unassign
        assert!(editor.unassign_path_from_npc(1));
        assert_eq!(editor.npcs[0].assigned_path, None);
    }

    #[test]
    fn test_waypoint_properties() {
        let mut wp = Waypoint::new(10, 10)
            .with_wait(30)
            .with_animation("wave")
            .with_dialogue("hello");
        
        assert_eq!(wp.position, IVec2::new(10, 10));
        assert_eq!(wp.wait_ticks, 30);
        assert_eq!(wp.animation_trigger, Some("wave".to_string()));
        assert_eq!(wp.dialogue, Some("hello".to_string()));
    }

    #[test]
    fn test_path_total_length() {
        let mut path = PatrolPathData::new("Test");
        path.add_waypoint(0, 0);
        path.add_waypoint(10, 0);
        path.add_waypoint(10, 10);
        
        assert_eq!(path.total_length(), 20.0);
    }

    #[test]
    fn test_patrol_path_id_generation() {
        let id1 = PatrolPathId::new();
        let id2 = PatrolPathId::new();
        assert_ne!(id1.0, id2.0);
    }

    #[test]
    fn test_map_view_conversions() {
        let map_view = MapViewState::default();
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1000.0, 1000.0));
        
        let grid_pos = IVec2::new(10, 10);
        let screen_pos = map_view.grid_to_screen(grid_pos, &rect);
        let back_to_grid = map_view.screen_to_grid(screen_pos, &rect);
        
        assert_eq!(grid_pos, back_to_grid);
    }
}
