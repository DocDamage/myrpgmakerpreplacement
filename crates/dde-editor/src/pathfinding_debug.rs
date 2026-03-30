//! Pathfinding Debug View Overlay
//!
//! Provides a visual debugging interface for the A* pathfinding system:
//! - Grid overlay showing walkable/unwalkable tiles
//! - Computed path visualization
//! - Start/goal markers
//! - Open/closed set visualization (A* debug)
//! - Real-time pathfinding statistics
//! - Interactive path computation

use egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};
use glam::IVec2;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use dde_core::pathfinding::{Path, PathGrid};

/// Pathfinding debug overlay state
pub struct PathfindingDebugOverlay {
    /// Whether the overlay is visible
    visible: bool,
    /// Grid width
    grid_width: i32,
    /// Grid height
    grid_height: i32,
    /// Pathfinding grid
    grid: PathGrid,
    /// Current start position
    start_pos: Option<IVec2>,
    /// Current goal position
    goal_pos: Option<IVec2>,
    /// Computed path
    current_path: Option<Path>,
    /// A* debug data - open set (nodes to explore)
    open_set: HashSet<IVec2>,
    /// A* debug data - closed set (nodes already explored)
    closed_set: HashSet<IVec2>,
    /// G-scores for each node
    g_scores: HashMap<IVec2, i32>,
    /// F-scores for each node
    f_scores: HashMap<IVec2, i32>,
    /// Computation time of last path
    last_compute_time: Option<std::time::Duration>,
    /// Number of nodes explored in last computation
    nodes_explored: usize,
    /// Display toggles
    show_grid: bool,
    show_path: bool,
    show_collision: bool,
    show_avoidance: bool,
    show_open_closed: bool,
    show_costs: bool,
    /// Interactive mode state
    setting_mode: SettingMode,
    /// Camera offset for panning
    camera_offset: Vec2,
    /// Zoom level
    zoom: f32,
    /// Tile size in pixels (at zoom 1.0)
    base_tile_size: f32,
}

/// Mode for setting start/goal positions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingMode {
    /// Not setting any position
    None,
    /// Setting start position
    SettingStart,
    /// Setting goal position
    SettingGoal,
}

impl Default for PathfindingDebugOverlay {
    fn default() -> Self {
        Self::new(64, 64)
    }
}

impl PathfindingDebugOverlay {
    /// Create a new pathfinding debug overlay
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            visible: false,
            grid_width: width,
            grid_height: height,
            grid: PathGrid::new(width, height),
            start_pos: Some(IVec2::new(5, 5)),
            goal_pos: Some(IVec2::new(20, 20)),
            current_path: None,
            open_set: HashSet::new(),
            closed_set: HashSet::new(),
            g_scores: HashMap::new(),
            f_scores: HashMap::new(),
            last_compute_time: None,
            nodes_explored: 0,
            show_grid: true,
            show_path: true,
            show_collision: true,
            show_avoidance: false,
            show_open_closed: false,
            show_costs: false,
            setting_mode: SettingMode::None,
            camera_offset: Vec2::ZERO,
            zoom: 1.0,
            base_tile_size: 32.0,
        }
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Show the overlay
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the overlay
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Resize the grid
    pub fn resize_grid(&mut self, width: i32, height: i32) {
        self.grid_width = width;
        self.grid_height = height;
        self.grid = PathGrid::new(width, height);
        self.clear_path();
    }

    /// Get mutable reference to grid
    pub fn grid_mut(&mut self) -> &mut PathGrid {
        &mut self.grid
    }

    /// Get grid reference
    pub fn grid(&self) -> &PathGrid {
        &self.grid
    }

    /// Set tile walkable state
    pub fn set_tile_walkable(&mut self, x: i32, y: i32, walkable: bool) {
        self.grid.set_walkable(x, y, walkable);
    }

    /// Set tile cost
    pub fn set_tile_cost(&mut self, x: i32, y: i32, cost: f32) {
        self.grid.set_cost(x, y, cost);
    }

    /// Set entity avoidance at tile
    pub fn set_tile_occupied(&mut self, x: i32, y: i32, occupied: bool) {
        self.grid.set_occupied(x, y, occupied);
    }

    /// Clear the current path and debug data
    pub fn clear_path(&mut self) {
        self.current_path = None;
        self.open_set.clear();
        self.closed_set.clear();
        self.g_scores.clear();
        self.f_scores.clear();
        self.last_compute_time = None;
        self.nodes_explored = 0;
    }

    /// Compute path with debug information
    pub fn compute_path(&mut self) {
        let (start, goal) = match (self.start_pos, self.goal_pos) {
            (Some(s), Some(g)) => (s, g),
            _ => return,
        };

        self.clear_path();

        let start_time = Instant::now();
        let result = self.find_path_with_debug(start, goal);
        let elapsed = start_time.elapsed();

        self.last_compute_time = Some(elapsed);
        self.current_path = result;
    }

    /// Find path with debug data collection
    fn find_path_with_debug(&mut self, start: IVec2, goal: IVec2) -> Option<Path> {
        use std::collections::BinaryHeap;

        // Check bounds
        if !self.grid.is_walkable(start.x, start.y) || !self.grid.is_walkable(goal.x, goal.y) {
            return None;
        }

        // Already at goal
        if start == goal {
            return Some(Path {
                waypoints: vec![start],
                total_cost: 0.0,
            });
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct Node {
            position: IVec2,
            g_cost: i32,
            h_cost: i32,
            f_cost: i32,
        }

        impl Ord for Node {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                other.f_cost.cmp(&self.f_cost)
            }
        }

        impl PartialOrd for Node {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        let mut open_set = BinaryHeap::new();
        let mut closed_set = HashSet::new();
        let mut came_from: HashMap<IVec2, IVec2> = HashMap::new();
        let mut g_scores: HashMap<IVec2, i32> = HashMap::new();
        let mut f_scores: HashMap<IVec2, i32> = HashMap::new();

        let start_node = Node {
            position: start,
            g_cost: 0,
            h_cost: manhattan_distance(start, goal),
            f_cost: manhattan_distance(start, goal),
        };

        open_set.push(start_node);
        g_scores.insert(start, 0);
        f_scores.insert(start, manhattan_distance(start, goal));

        while let Some(current) = open_set.pop() {
            // Track closed set
            closed_set.insert(current.position);

            // Reached goal
            if current.position == goal {
                // Store debug info
                self.closed_set = closed_set.clone();
                self.g_scores = g_scores.clone();
                self.f_scores = f_scores.clone();
                self.nodes_explored = closed_set.len() + open_set.len();

                // Collect open set (remaining nodes)
                self.open_set = open_set.iter().map(|n| n.position).collect();

                let path = reconstruct_path(came_from, goal);
                let total_cost = *g_scores.get(&goal).unwrap_or(&0) as f32;
                return Some(Path {
                    waypoints: path,
                    total_cost,
                });
            }

            // Already processed
            if closed_set.contains(&current.position) && current.f_cost > *f_scores.get(&current.position).unwrap_or(&i32::MAX) {
                continue;
            }

            // Check neighbors
            for neighbor in self.get_neighbors(current.position) {
                if closed_set.contains(&neighbor) {
                    continue;
                }

                let cost = self.grid.get_cost(neighbor.x, neighbor.y);
                if cost.is_infinite() {
                    continue;
                }

                let tentative_g = current.g_cost + cost as i32;

                if tentative_g < *g_scores.get(&neighbor).unwrap_or(&i32::MAX) {
                    came_from.insert(neighbor, current.position);
                    g_scores.insert(neighbor, tentative_g);
                    let h = manhattan_distance(neighbor, goal);
                    let f = tentative_g + h;
                    f_scores.insert(neighbor, f);

                    open_set.push(Node {
                        position: neighbor,
                        g_cost: tentative_g,
                        h_cost: h,
                        f_cost: f,
                    });
                }
            }
        }

        // No path found - store debug info anyway
        self.closed_set = closed_set;
        self.open_set = open_set.iter().map(|n| n.position).collect();
        self.g_scores = g_scores;
        self.f_scores = f_scores;
        self.nodes_explored = self.closed_set.len() + self.open_set.len();

        None
    }

    /// Get walkable neighbors
    fn get_neighbors(&self, pos: IVec2) -> Vec<IVec2> {
        let dirs = [
            IVec2::new(0, 1),
            IVec2::new(0, -1),
            IVec2::new(-1, 0),
            IVec2::new(1, 0),
        ];

        dirs.iter()
            .filter_map(|&d| {
                let neighbor = pos + d;
                if self.grid.is_walkable(neighbor.x, neighbor.y) {
                    Some(neighbor)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Draw the debug overlay window
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        egui::Window::new("🔍 Pathfinding Debug")
            .default_size([1000.0, 700.0])
            .show(ctx, |ui| {
                self.draw_ui(ui);
            });
    }

    /// Draw the UI content
    fn draw_ui(&mut self, ui: &mut Ui) {
        // Top toolbar
        ui.horizontal(|ui| {
            ui.label("Grid Size:");
            ui.label(format!("{}x{}", self.grid_width, self.grid_height));
            
            ui.separator();
            
            if ui.button("🧹 Clear Path").clicked() {
                self.clear_path();
            }
            
            if ui.button("▶ Compute Path").clicked() {
                self.compute_path();
            }
            
            if ui.button("🔄 Reset Grid").clicked() {
                self.grid = PathGrid::new(self.grid_width, self.grid_height);
                self.clear_path();
            }
            
            ui.separator();
            
            // Zoom controls
            if ui.button("-").clicked() {
                self.zoom = (self.zoom / 1.25).max(0.25);
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if ui.button("+").clicked() {
                self.zoom = (self.zoom * 1.25).min(4.0);
            }
        });
        
        ui.separator();
        
        // Control panel and viewport
        egui::SidePanel::left("pf_debug_controls")
            .default_width(250.0)
            .show_inside(ui, |ui| {
                self.draw_controls(ui);
            });
        
        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.draw_viewport(ui);
        });
    }

    /// Draw control panel
    fn draw_controls(&mut self, ui: &mut Ui) {
        ui.heading("Display Options");
        ui.separator();
        
        ui.checkbox(&mut self.show_grid, "Show Grid");
        ui.checkbox(&mut self.show_path, "Show Path");
        ui.checkbox(&mut self.show_collision, "Show Collision Boxes");
        ui.checkbox(&mut self.show_avoidance, "Show Avoidance Zones");
        ui.checkbox(&mut self.show_open_closed, "Show Open/Closed Sets");
        ui.checkbox(&mut self.show_costs, "Show Tile Costs");
        
        ui.separator();
        ui.heading("Set Positions");
        
        // Setting mode buttons
        ui.horizontal(|ui| {
            let start_btn = ui.selectable_label(
                self.setting_mode == SettingMode::SettingStart,
                "📍 Set Start"
            );
            if start_btn.clicked() {
                self.setting_mode = if self.setting_mode == SettingMode::SettingStart {
                    SettingMode::None
                } else {
                    SettingMode::SettingStart
                };
            }
            
            let goal_btn = ui.selectable_label(
                self.setting_mode == SettingMode::SettingGoal,
                "🎯 Set Goal"
            );
            if goal_btn.clicked() {
                self.setting_mode = if self.setting_mode == SettingMode::SettingGoal {
                    SettingMode::None
                } else {
                    SettingMode::SettingGoal
                };
            }
        });
        
        ui.separator();
        ui.heading("Positions");
        
        if let Some(start) = self.start_pos {
            ui.label(format!("Start: ({}, {})", start.x, start.y));
        } else {
            ui.label("Start: Not set");
        }
        
        if let Some(goal) = self.goal_pos {
            ui.label(format!("Goal: ({}, {})", goal.x, goal.y));
        } else {
            ui.label("Goal: Not set");
        }
        
        ui.separator();
        ui.heading("Statistics");
        
        if let Some(path) = &self.current_path {
            ui.label(format!("Path Length: {}", path.len()));
            ui.label(format!("Total Cost: {:.1}", path.total_cost));
        } else {
            ui.label("Path Length: N/A");
            ui.label("Total Cost: N/A");
        }
        
        if let Some(time) = self.last_compute_time {
            ui.label(format!("Compute Time: {:?}", time));
        } else {
            ui.label("Compute Time: N/A");
        }
        
        ui.label(format!("Nodes Explored: {}", self.nodes_explored));
        
        ui.separator();
        ui.heading("Legend");
        
        self.draw_legend_item(ui, "Start", Color32::GREEN);
        self.draw_legend_item(ui, "Goal", Color32::RED);
        self.draw_legend_item(ui, "Path", Color32::YELLOW);
        self.draw_legend_item(ui, "Walkable", Color32::from_gray(100));
        self.draw_legend_item(ui, "Blocked", Color32::from_rgb(80, 30, 30));
        self.draw_legend_item(ui, "Open Set", Color32::from_rgb(100, 200, 100));
        self.draw_legend_item(ui, "Closed Set", Color32::from_rgb(100, 100, 200));
        self.draw_legend_item(ui, "Avoidance", Color32::from_rgba_premultiplied(255, 100, 0, 100));
    }

    /// Draw a legend item with color swatch
    fn draw_legend_item(&self, ui: &mut Ui, label: &str, color: Color32) {
        ui.horizontal(|ui| {
            let (rect, _response) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
            ui.painter().rect_filled(rect, 2.0, color);
            ui.label(label);
        });
    }

    /// Draw the main viewport with grid
    fn draw_viewport(&mut self, ui: &mut Ui) -> Response {
        let available_size = ui.available_size();
        let (response, painter) = ui.allocate_painter(available_size, Sense::drag());
        
        let rect = response.rect;
        let tile_size = self.base_tile_size * self.zoom;
        
        // Handle panning
        if response.dragged() {
            self.camera_offset -= response.drag_delta();
        }
        
        // Calculate visible range
        let start_x = (self.camera_offset.x / tile_size).floor() as i32;
        let start_y = (self.camera_offset.y / tile_size).floor() as i32;
        let end_x = start_x + (rect.width() / tile_size).ceil() as i32 + 1;
        let end_y = start_y + (rect.height() / tile_size).ceil() as i32 + 1;
        
        // Clamp to grid bounds
        let start_x = start_x.max(0).min(self.grid_width);
        let start_y = start_y.max(0).min(self.grid_height);
        let end_x = end_x.max(0).min(self.grid_width);
        let end_y = end_y.max(0).min(self.grid_height);
        
        // Draw grid tiles
        for y in start_y..end_y {
            for x in start_x..end_x {
                let screen_pos = self.tile_to_screen(x, y, rect, tile_size);
                let tile_rect = Rect::from_min_size(screen_pos, Vec2::splat(tile_size));
                
                // Determine tile color
                let color = self.get_tile_color(x, y);
                painter.rect_filled(tile_rect, 0.0, color);
                
                // Draw grid lines
                if self.show_grid {
                    painter.rect_stroke(
                        tile_rect,
                        0.0,
                        Stroke::new(0.5, Color32::from_rgba_premultiplied(255, 255, 255, 30)),
                    );
                }
                
                // Draw tile cost if enabled
                if self.show_costs {
                    let cost = self.grid.get_cost(x, y);
                    if cost != 1.0 && cost.is_finite() {
                        painter.text(
                            tile_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("{:.1}", cost),
                            egui::FontId::monospace(10.0),
                            Color32::WHITE,
                        );
                    }
                }
            }
        }
        
        // Draw path
        if self.show_path {
            self.draw_path(&painter, rect, tile_size);
        }
        
        // Draw start marker
        if let Some(start) = self.start_pos {
            let pos = self.tile_to_screen(start.x, start.y, rect, tile_size);
            self.draw_start_marker(&painter, pos, tile_size);
        }
        
        // Draw goal marker
        if let Some(goal) = self.goal_pos {
            let pos = self.tile_to_screen(goal.x, goal.y, rect, tile_size);
            self.draw_goal_marker(&painter, pos, tile_size);
        }
        
        // Handle mouse input
        if let Some(mouse_pos) = response.hover_pos() {
            let tile_coords = self.screen_to_tile(mouse_pos, rect, tile_size);
            
            // Highlight hovered tile
            if tile_coords.0 >= 0 
                && tile_coords.0 < self.grid_width 
                && tile_coords.1 >= 0 
                && tile_coords.1 < self.grid_height 
            {
                let highlight_pos = self.tile_to_screen(tile_coords.0, tile_coords.1, rect, tile_size);
                let highlight_rect = Rect::from_min_size(highlight_pos, Vec2::splat(tile_size));
                
                let highlight_color = match self.setting_mode {
                    SettingMode::SettingStart => Color32::from_rgba_premultiplied(0, 255, 0, 100),
                    SettingMode::SettingGoal => Color32::from_rgba_premultiplied(255, 0, 0, 100),
                    _ => Color32::from_rgba_premultiplied(255, 255, 0, 100),
                };
                
                painter.rect_filled(highlight_rect, 0.0, highlight_color);
                painter.rect_stroke(highlight_rect, 0.0, Stroke::new(2.0, Color32::WHITE));
                
                // Handle clicks
                if response.clicked() {
                    match self.setting_mode {
                        SettingMode::SettingStart => {
                            self.start_pos = Some(IVec2::new(tile_coords.0, tile_coords.1));
                            self.setting_mode = SettingMode::None;
                            self.compute_path();
                        }
                        SettingMode::SettingGoal => {
                            self.goal_pos = Some(IVec2::new(tile_coords.0, tile_coords.1));
                            self.setting_mode = SettingMode::None;
                            self.compute_path();
                        }
                        SettingMode::None => {
                            // Toggle walkable state on shift+click
                            if ui.ctx().input(|i| i.modifiers.shift) {
                                let current = self.grid.is_walkable(tile_coords.0, tile_coords.1);
                                self.grid.set_walkable(tile_coords.0, tile_coords.1, !current);
                                self.compute_path();
                            }
                        }
                    }
                }
            }
        }
        
        response
    }

    /// Get color for a tile based on its state
    fn get_tile_color(&self, x: i32, y: i32) -> Color32 {
        let pos = IVec2::new(x, y);
        
        // Show open/closed sets if enabled
        if self.show_open_closed {
            if self.open_set.contains(&pos) {
                return Color32::from_rgb(100, 200, 100);
            }
            if self.closed_set.contains(&pos) {
                return Color32::from_rgb(100, 100, 200);
            }
        }
        
        // Check if walkable
        if !self.grid.is_walkable(x, y) {
            return Color32::from_rgb(80, 30, 30);
        }
        
        // Check avoidance zones
        if self.show_avoidance {
            let cost = self.grid.get_cost(x, y);
            if cost > 1.0 {
                return Color32::from_rgba_premultiplied(255, 100, 0, 100);
            }
        }
        
        // Default walkable color
        Color32::from_gray(60)
    }

    /// Draw the computed path
    fn draw_path(&self, painter: &egui::Painter, rect: Rect, tile_size: f32) {
        if let Some(path) = &self.current_path {
            let path_color = Color32::YELLOW;
            let line_thickness = (tile_size * 0.3).max(2.0);
            
            // Draw path lines
            for i in 0..path.waypoints.len().saturating_sub(1) {
                let start = path.waypoints[i];
                let end = path.waypoints[i + 1];
                
                let start_pos = self.tile_to_screen(start.x, start.y, rect, tile_size)
                    + Vec2::splat(tile_size * 0.5);
                let end_pos = self.tile_to_screen(end.x, end.y, rect, tile_size)
                    + Vec2::splat(tile_size * 0.5);
                
                painter.line_segment(
                    [Pos2::new(start_pos.x, start_pos.y), Pos2::new(end_pos.x, end_pos.y)],
                    Stroke::new(line_thickness, path_color),
                );
            }
            
            // Draw waypoints
            for (i, waypoint) in path.waypoints.iter().enumerate() {
                let pos = self.tile_to_screen(waypoint.x, waypoint.y, rect, tile_size)
                    + Vec2::splat(tile_size * 0.5);
                
                let radius = tile_size * 0.15;
                let color = if i == 0 {
                    Color32::GREEN
                } else if i == path.waypoints.len() - 1 {
                    Color32::RED
                } else {
                    Color32::YELLOW
                };
                
                painter.circle_filled(Pos2::new(pos.x, pos.y), radius, color);
            }
        }
    }

    /// Draw start marker
    fn draw_start_marker(&self, painter: &egui::Painter, pos: Pos2, tile_size: f32) {
        let center = pos + Vec2::splat(tile_size * 0.5);
        let radius = tile_size * 0.35;
        
        // Draw circle
        painter.circle_filled(Pos2::new(center.x, center.y), radius, Color32::GREEN);
        painter.circle_stroke(
            Pos2::new(center.x, center.y),
            radius,
            Stroke::new(2.0, Color32::WHITE),
        );
        
        // Draw "S"
        painter.text(
            Pos2::new(center.x, center.y),
            egui::Align2::CENTER_CENTER,
            "S",
            egui::FontId::proportional(tile_size * 0.4),
            Color32::BLACK,
        );
    }

    /// Draw goal marker
    fn draw_goal_marker(&self, painter: &egui::Painter, pos: Pos2, tile_size: f32) {
        let center = pos + Vec2::splat(tile_size * 0.5);
        let radius = tile_size * 0.35;
        
        // Draw circle
        painter.circle_filled(Pos2::new(center.x, center.y), radius, Color32::RED);
        painter.circle_stroke(
            Pos2::new(center.x, center.y),
            radius,
            Stroke::new(2.0, Color32::WHITE),
        );
        
        // Draw "G"
        painter.text(
            Pos2::new(center.x, center.y),
            egui::Align2::CENTER_CENTER,
            "G",
            egui::FontId::proportional(tile_size * 0.4),
            Color32::WHITE,
        );
    }

    /// Convert tile coordinates to screen position
    fn tile_to_screen(&self, x: i32, y: i32, rect: Rect, tile_size: f32) -> Pos2 {
        Pos2::new(
            rect.min.x + (x as f32 * tile_size) - self.camera_offset.x,
            rect.min.y + (y as f32 * tile_size) - self.camera_offset.y,
        )
    }

    /// Convert screen position to tile coordinates
    fn screen_to_tile(&self, screen_pos: Pos2, rect: Rect, tile_size: f32) -> (i32, i32) {
        (
            ((screen_pos.x - rect.min.x + self.camera_offset.x) / tile_size) as i32,
            ((screen_pos.y - rect.min.y + self.camera_offset.y) / tile_size) as i32,
        )
    }
}

/// Manhattan distance heuristic
fn manhattan_distance(a: IVec2, b: IVec2) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

/// Reconstruct path from came_from map
fn reconstruct_path(came_from: HashMap<IVec2, IVec2>, goal: IVec2) -> Vec<IVec2> {
    let mut path = vec![goal];
    let mut current = goal;

    while let Some(&prev) = came_from.get(&current) {
        path.push(prev);
        current = prev;
    }

    path.reverse();
    path
}

/// Extension trait for integrating pathfinding debug with tilemap editor
pub trait PathfindingDebugExt {
    /// Toggle pathfinding debug overlay
    fn toggle_pathfinding_debug(&mut self);
    
    /// Show pathfinding debug overlay
    fn show_pathfinding_debug(&mut self);
    
    /// Hide pathfinding debug overlay
    fn hide_pathfinding_debug(&mut self);
    
    /// Check if pathfinding debug is visible
    fn is_pathfinding_debug_visible(&self) -> bool;
    
    /// Draw pathfinding debug overlay
    fn draw_pathfinding_debug(&mut self, ctx: &egui::Context);
    
    /// Sync pathfinding grid from tilemap collision layer
    fn sync_grid_from_tilemap(&mut self, map_width: u32, map_height: u32, is_walkable: impl Fn(i32, i32) -> bool);
}

/// Standalone pathfinding debug panel that can be used independently
pub struct PathfindingDebugPanel {
    overlay: PathfindingDebugOverlay,
}

impl Default for PathfindingDebugPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl PathfindingDebugPanel {
    /// Create a new debug panel
    pub fn new() -> Self {
        Self {
            overlay: PathfindingDebugOverlay::new(64, 64),
        }
    }

    /// Show the panel
    pub fn show(&mut self) {
        self.overlay.show();
    }

    /// Hide the panel
    pub fn hide(&mut self) {
        self.overlay.hide();
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.overlay.toggle();
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.overlay.is_visible()
    }

    /// Get mutable overlay reference
    pub fn overlay_mut(&mut self) -> &mut PathfindingDebugOverlay {
        &mut self.overlay
    }

    /// Get overlay reference
    pub fn overlay(&self) -> &PathfindingDebugOverlay {
        &self.overlay
    }

    /// Draw the panel
    pub fn draw(&mut self, ctx: &egui::Context) {
        self.overlay.draw(ctx);
    }

    /// Resize grid to match map dimensions
    pub fn resize_to_map(&mut self, width: u32, height: u32) {
        self.overlay.resize_grid(width as i32, height as i32);
    }

    /// Set tile walkable from collision data
    pub fn set_tile_from_collision(&mut self, x: i32, y: i32, has_collision: bool) {
        self.overlay.set_tile_walkable(x, y, !has_collision);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_creation() {
        let overlay = PathfindingDebugOverlay::new(32, 32);
        assert!(!overlay.is_visible());
        assert_eq!(overlay.grid_width, 32);
        assert_eq!(overlay.grid_height, 32);
    }

    #[test]
    fn test_overlay_toggle() {
        let mut overlay = PathfindingDebugOverlay::new(32, 32);
        assert!(!overlay.is_visible());
        
        overlay.toggle();
        assert!(overlay.is_visible());
        
        overlay.toggle();
        assert!(!overlay.is_visible());
    }

    #[test]
    fn test_set_positions() {
        let mut overlay = PathfindingDebugOverlay::new(32, 32);
        
        overlay.start_pos = Some(IVec2::new(5, 5));
        overlay.goal_pos = Some(IVec2::new(10, 10));
        
        assert_eq!(overlay.start_pos, Some(IVec2::new(5, 5)));
        assert_eq!(overlay.goal_pos, Some(IVec2::new(10, 10)));
    }

    #[test]
    fn test_path_computation() {
        let mut overlay = PathfindingDebugOverlay::new(10, 10);
        
        overlay.start_pos = Some(IVec2::new(0, 0));
        overlay.goal_pos = Some(IVec2::new(5, 5));
        
        overlay.compute_path();
        
        assert!(overlay.current_path.is_some());
        let path = overlay.current_path.unwrap();
        assert!(!path.waypoints.is_empty());
        assert_eq!(path.waypoints[0], IVec2::new(0, 0));
        assert_eq!(path.waypoints[path.len() - 1], IVec2::new(5, 5));
    }

    #[test]
    fn test_path_with_obstacles() {
        let mut overlay = PathfindingDebugOverlay::new(10, 10);
        
        // Block some tiles
        overlay.grid.set_walkable(3, 0, false);
        overlay.grid.set_walkable(3, 1, false);
        overlay.grid.set_walkable(3, 2, false);
        overlay.grid.set_walkable(3, 3, false);
        overlay.grid.set_walkable(3, 4, false);
        overlay.grid.set_walkable(3, 5, false);
        
        overlay.start_pos = Some(IVec2::new(0, 0));
        overlay.goal_pos = Some(IVec2::new(5, 5));
        
        overlay.compute_path();
        
        assert!(overlay.current_path.is_some());
    }

    #[test]
    fn test_blocked_path() {
        let mut overlay = PathfindingDebugOverlay::new(5, 5);
        
        // Create a wall
        for y in 0..5 {
            overlay.grid.set_walkable(2, y, false);
        }
        
        overlay.start_pos = Some(IVec2::new(0, 2));
        overlay.goal_pos = Some(IVec2::new(4, 2));
        
        overlay.compute_path();
        
        assert!(overlay.current_path.is_none());
    }

    #[test]
    fn test_debug_panel() {
        let mut panel = PathfindingDebugPanel::new();
        assert!(!panel.is_visible());
        
        panel.show();
        assert!(panel.is_visible());
        
        panel.hide();
        assert!(!panel.is_visible());
    }
}
