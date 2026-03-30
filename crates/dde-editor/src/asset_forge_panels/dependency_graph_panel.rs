//! Dependency Graph Viewer Panel
//!
//! UI for visualizing asset dependencies with graph display,
//! orphaned asset detection, and circular dependency warnings.

use dde_asset_forge::dependency_graph::{
    AssetId, AssetNode, AssetType, Dependency, DependencyAnalysis, DependencyGraph, DependencyKind,
};
use std::collections::{HashMap, HashSet};

/// Node position in the graph visualization
#[derive(Debug, Clone, Copy)]
pub struct NodePosition {
    pub x: f32,
    pub y: f32,
}

impl Default for NodePosition {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// Graph view state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphViewMode {
    All,
    ByType(AssetType),
    OrphansOnly,
    CircularDeps,
}

/// Selected asset info
#[derive(Debug, Clone)]
pub struct SelectedAssetInfo {
    pub asset_id: AssetId,
    pub dependencies: Vec<AssetId>,
    pub dependents: Vec<AssetId>,
    pub impact_count: usize,
}

/// Dependency Graph Viewer Panel
pub struct DependencyGraphPanel {
    /// Whether panel is visible
    visible: bool,
    /// The dependency graph
    graph: DependencyGraph,
    /// Node positions for visualization
    node_positions: HashMap<AssetId, NodePosition>,
    /// Current view mode
    view_mode: GraphViewMode,
    /// Selected asset
    selected_asset: Option<AssetId>,
    /// Zoom level
    zoom: f32,
    /// Pan offset
    pan: glam::Vec2,
    /// Is currently panning
    is_panning: bool,
    /// Last mouse position for panning
    last_mouse_pos: Option<egui::Pos2>,
    /// Show impact analysis
    show_impact_analysis: bool,
    /// Circular dependency warnings
    circular_warnings: Vec<String>,
    /// Filter text
    filter_text: String,
    /// Show node labels
    show_labels: bool,
    /// Node size multiplier
    node_size: f32,
    /// Show dependency kinds with different colors
    color_by_dependency_kind: bool,
    /// Analysis results
    analysis: Option<DependencyAnalysis>,
    /// Auto-layout enabled
    auto_layout: bool,
}

impl Default for DependencyGraphPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyGraphPanel {
    /// Create a new dependency graph panel
    pub fn new() -> Self {
        let mut panel = Self {
            visible: false,
            graph: DependencyGraph::new(),
            node_positions: HashMap::new(),
            view_mode: GraphViewMode::All,
            selected_asset: None,
            zoom: 1.0,
            pan: glam::Vec2::ZERO,
            is_panning: false,
            last_mouse_pos: None,
            show_impact_analysis: false,
            circular_warnings: Vec::new(),
            filter_text: String::new(),
            show_labels: true,
            node_size: 1.0,
            color_by_dependency_kind: true,
            analysis: None,
            auto_layout: true,
        };

        // Add sample data for demonstration
        panel.load_sample_data();
        panel
    }

    /// Load sample data for demonstration
    fn load_sample_data(&mut self) {
        // This would normally load from the actual project
        // For now, we'll create some sample assets
        
        let assets = vec![
            (AssetType::Map, "maps/level1.tmx"),
            (AssetType::Map, "maps/level2.tmx"),
            (AssetType::Tileset, "tilesets/terrain.tsx"),
            (AssetType::Tileset, "tilesets/buildings.tsx"),
            (AssetType::Texture, "textures/terrain.png"),
            (AssetType::Texture, "textures/buildings.png"),
            (AssetType::Texture, "textures/player.png"),
            (AssetType::SpriteSheet, "sprites/player_walk.png"),
            (AssetType::Prefab, "prefabs/player.json"),
            (AssetType::Audio, "audio/bgm_town.ogg"),
            (AssetType::Script, "scripts/level1.lua"),
            (AssetType::Script, "scripts/player_controller.lua"),
            // Orphaned asset (no dependents)
            (AssetType::Texture, "textures/unused_old.png"),
        ];

        for (asset_type, path) in assets {
            let id = AssetId::new(asset_type, path);
            let _ = self.graph.add_asset(id);
        }

        // Create some dependencies
        let level1 = AssetId::new(AssetType::Map, "maps/level1.tmx");
        let terrain_ts = AssetId::new(AssetType::Tileset, "tilesets/terrain.tsx");
        let terrain_tex = AssetId::new(AssetType::Texture, "textures/terrain.png");
        let player_prefab = AssetId::new(AssetType::Prefab, "prefabs/player.json");
        let player_sprite = AssetId::new(AssetType::SpriteSheet, "sprites/player_walk.png");
        let player_tex = AssetId::new(AssetType::Texture, "textures/player.png");
        let level1_script = AssetId::new(AssetType::Script, "scripts/level1.lua");
        let player_script = AssetId::new(AssetType::Script, "scripts/player_controller.lua");

        // level1 -> terrain_ts
        let _ = self.graph.add_dependency(
            &level1,
            terrain_ts.clone(),
            DependencyKind::Required,
            Some("background_layer".to_string()),
        );

        // terrain_ts -> terrain_tex
        let _ = self.graph.add_dependency(
            &terrain_ts,
            terrain_tex,
            DependencyKind::Required,
            None,
        );

        // level1 -> player_prefab
        let _ = self.graph.add_dependency(
            &level1,
            player_prefab.clone(),
            DependencyKind::Required,
            Some("player_start".to_string()),
        );

        // player_prefab -> player_sprite
        let _ = self.graph.add_dependency(
            &player_prefab,
            player_sprite,
            DependencyKind::Required,
            None,
        );

        // player_sprite -> player_tex
        let _ = self.graph.add_dependency(
            &AssetId::new(AssetType::SpriteSheet, "sprites/player_walk.png"),
            player_tex,
            DependencyKind::Required,
            None,
        );

        // level1 -> level1_script
        let _ = self.graph.add_dependency(
            &level1,
            level1_script,
            DependencyKind::Runtime,
            Some("map_logic".to_string()),
        );

        // player_prefab -> player_script
        let _ = self.graph.add_dependency(
            &player_prefab,
            player_script,
            DependencyKind::Runtime,
            Some("controller".to_string()),
        );

        self.compute_circular_warnings();
        self.run_analysis();
        if self.auto_layout {
            self.auto_layout_nodes();
        }
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

    /// Set the dependency graph
    pub fn set_graph(&mut self, graph: DependencyGraph) {
        self.graph = graph;
        self.compute_circular_warnings();
        self.run_analysis();
        if self.auto_layout {
            self.auto_layout_nodes();
        }
    }

    /// Get the dependency graph
    pub fn graph(&self) -> &DependencyGraph {
        &self.graph
    }

    /// Compute circular dependency warnings
    fn compute_circular_warnings(&mut self) {
        self.circular_warnings.clear();
        
        let errors = self.graph.validate();
        for error in errors {
            if error.contains("Circular") {
                self.circular_warnings.push(error);
            }
        }
    }

    /// Run dependency analysis
    fn run_analysis(&mut self) {
        self.analysis = Some(DependencyAnalysis::analyze(&self.graph));
    }

    /// Auto-layout nodes using a simple force-directed approach
    fn auto_layout_nodes(&mut self) {
        self.node_positions.clear();
        
        let assets: Vec<_> = self.graph.all_assets().keys().cloned().collect();
        let count = assets.len();
        
        if count == 0 {
            return;
        }

        // Arrange in a circle for initial positions
        let radius = 200.0f32;
        let center = glam::Vec2::new(400.0, 300.0);
        
        for (i, asset) in assets.iter().enumerate() {
            let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
            let pos = NodePosition {
                x: center.x + radius * angle.cos(),
                y: center.y + radius * angle.sin(),
            };
            self.node_positions.insert(asset.clone(), pos);
        }
    }

    /// Get filtered assets based on view mode
    fn get_filtered_assets(&self) -> Vec<&AssetId> {
        let all_assets: Vec<_> = self.graph.all_assets().keys().collect();
        
        match self.view_mode {
            GraphViewMode::All => all_assets,
            GraphViewMode::ByType(asset_type) => {
                all_assets.into_iter()
                    .filter(|id| id.asset_type == asset_type)
                    .collect()
            }
            GraphViewMode::OrphansOnly => {
                let orphans = self.graph.find_orphans(&[AssetType::Map, AssetType::Script]);
                orphans.into_iter().collect()
            }
            GraphViewMode::CircularDeps => {
                // Return assets involved in circular dependencies
                all_assets.into_iter()
                    .filter(|id| self.circular_warnings.iter().any(|w| {
                        w.contains(id.path.to_str().unwrap_or(""))
                    }))
                    .collect()
            }
        }
    }

    /// Get impact analysis for an asset (what would break if deleted)
    fn get_impact_analysis(&self, asset_id: &AssetId) -> Vec<AssetId> {
        let mut impacted = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = vec![asset_id.clone()];
        
        while let Some(current) = queue.pop() {
            if let Some(node) = self.graph.get(&current) {
                for dependent in &node.dependents {
                    if visited.insert(dependent.clone()) {
                        impacted.push(dependent.clone());
                        queue.push(dependent.clone());
                    }
                }
            }
        }
        
        impacted
    }

    /// Get color for asset type
    fn get_asset_type_color(asset_type: AssetType) -> egui::Color32 {
        match asset_type {
            AssetType::Data => egui::Color32::GRAY,
            AssetType::Texture => egui::Color32::from_rgb(100, 150, 255),
            AssetType::SpriteSheet => egui::Color32::from_rgb(150, 100, 255),
            AssetType::Audio => egui::Color32::from_rgb(255, 200, 100),
            AssetType::Music => egui::Color32::from_rgb(255, 150, 100),
            AssetType::Map => egui::Color32::from_rgb(100, 255, 150),
            AssetType::Tileset => egui::Color32::from_rgb(100, 200, 150),
            AssetType::Script => egui::Color32::from_rgb(255, 100, 150),
            AssetType::Prefab => egui::Color32::from_rgb(200, 100, 200),
            AssetType::Animation => egui::Color32::from_rgb(255, 255, 100),
            AssetType::Shader => egui::Color32::from_rgb(100, 255, 255),
            AssetType::Font => egui::Color32::from_rgb(200, 200, 200),
        }
    }

    /// Get color for dependency kind
    fn get_dependency_kind_color(kind: DependencyKind) -> egui::Color32 {
        match kind {
            DependencyKind::Required => egui::Color32::from_rgb(255, 100, 100),
            DependencyKind::Optional => egui::Color32::from_rgb(100, 255, 100),
            DependencyKind::Runtime => egui::Color32::from_rgb(100, 100, 255),
            DependencyKind::Reference => egui::Color32::from_rgb(200, 200, 200),
        }
    }

    /// Draw the panel
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        let mut visible = self.visible;
        egui::Window::new("🔗 Dependency Graph")
            .open(&mut visible)
            .resizable(true)
            .default_size([1000.0, 700.0])
            .show(ctx, |ui| {
                self.draw_panel_content(ui);
            });
        self.visible = visible;
    }

    /// Draw panel content
    fn draw_panel_content(&mut self, ui: &mut egui::Ui) {
        // Top toolbar
        self.draw_toolbar(ui);
        ui.separator();

        // Main content area
        egui::SidePanel::left("graph_sidebar")
            .resizable(true)
            .default_width(250.0)
            .show_inside(ui, |ui| {
                self.draw_sidebar(ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.draw_graph_canvas(ui);
        });

        // Impact analysis overlay
        if self.show_impact_analysis {
            if let Some(ref asset_id) = self.selected_asset {
                self.draw_impact_overlay(ui.ctx(), asset_id);
            }
        }
    }

    /// Draw toolbar
    fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Dependency Graph");
            
            ui.separator();
            
            // View mode selector
            ui.label("View:");
            egui::ComboBox::from_id_source("view_mode_combo")
                .selected_text(match self.view_mode {
                    GraphViewMode::All => "All Assets",
                    GraphViewMode::ByType(t) => match t {
                        AssetType::Texture => "Textures Only",
                        AssetType::Map => "Maps Only",
                        AssetType::Script => "Scripts Only",
                        AssetType::Audio => "Audio Only",
                        AssetType::Prefab => "Prefabs Only",
                        _ => "By Type",
                    },
                    GraphViewMode::OrphansOnly => "Orphans Only",
                    GraphViewMode::CircularDeps => "Circular Dependencies",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.view_mode, GraphViewMode::All, "All Assets");
                    ui.selectable_value(&mut self.view_mode, GraphViewMode::ByType(AssetType::Map), "Maps Only");
                    ui.selectable_value(&mut self.view_mode, GraphViewMode::ByType(AssetType::Texture), "Textures Only");
                    ui.selectable_value(&mut self.view_mode, GraphViewMode::ByType(AssetType::Script), "Scripts Only");
                    ui.selectable_value(&mut self.view_mode, GraphViewMode::OrphansOnly, "Orphans Only");
                    ui.selectable_value(&mut self.view_mode, GraphViewMode::CircularDeps, "Circular Dependencies");
                });
            
            ui.separator();
            
            // Zoom controls
            ui.label("Zoom:");
            if ui.button("-").clicked() {
                self.zoom = (self.zoom * 0.9).max(0.1);
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if ui.button("+").clicked() {
                self.zoom = (self.zoom * 1.1).min(5.0);
            }
            
            if ui.button("Reset").clicked() {
                self.zoom = 1.0;
                self.pan = glam::Vec2::ZERO;
            }
            
            ui.separator();
            
            // Layout button
            if ui.button("🔄 Auto Layout").clicked() {
                self.auto_layout_nodes();
            }
            
            ui.separator();
            
            // Options
            ui.checkbox(&mut self.show_labels, "Labels");
            ui.checkbox(&mut self.color_by_dependency_kind, "Color by Dep. Type");
        });
        
        // Warning banner for circular dependencies
        if !self.circular_warnings.is_empty() {
            ui.colored_label(
                egui::Color32::from_rgb(255, 100, 100),
                format!("⚠️ {} circular dependencies detected!", self.circular_warnings.len())
            );
        }
    }

    /// Draw sidebar
    fn draw_sidebar(&mut self, ui: &mut egui::Ui) {
        // Filter
        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.text_edit_singleline(&mut self.filter_text);
        });
        ui.separator();

        // Analysis summary
        if let Some(ref analysis) = self.analysis {
            ui.collapsing("📊 Statistics", |ui| {
                egui::Grid::new("stats_grid")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Total Assets:");
                        ui.label(analysis.total_assets.to_string());
                        ui.end_row();
                        
                        ui.label("Orphaned:");
                        ui.colored_label(
                            if analysis.orphaned_count > 0 { egui::Color32::YELLOW } else { egui::Color32::GREEN },
                            analysis.orphaned_count.to_string()
                        );
                        ui.end_row();
                        
                        ui.label("Circular Deps:");
                        ui.colored_label(
                            if analysis.circular_count > 0 { egui::Color32::RED } else { egui::Color32::GREEN },
                            analysis.circular_count.to_string()
                        );
                        ui.end_row();
                        
                        ui.label("Total Size:");
                        ui.label(Self::format_size(analysis.total_size));
                        ui.end_row();
                    });
                
                // By type breakdown
                ui.label("By Type:");
                for (asset_type, count) in &analysis.by_type {
                    ui.horizontal(|ui| {
                        let color = Self::get_asset_type_color(*asset_type);
                        ui.colored_label(color, "●");
                        ui.label(format!("{:?}: {}", asset_type, count));
                    });
                }
            });
            
            ui.separator();
        }

        // Asset list
        ui.heading("Assets");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            let filtered = self.get_filtered_assets();
            
            for asset_id in filtered {
                // Apply text filter
                if !self.filter_text.is_empty() {
                    let path_str = asset_id.path.to_string_lossy().to_lowercase();
                    if !path_str.contains(&self.filter_text.to_lowercase()) {
                        continue;
                    }
                }
                
                let is_selected = self.selected_asset.as_ref() == Some(asset_id);
                let node = self.graph.get(asset_id);
                
                let dep_count = node.map(|n| n.dependencies.len()).unwrap_or(0);
                let dependent_count = node.map(|n| n.dependents.len()).unwrap_or(0);
                
                let color = Self::get_asset_type_color(asset_id.asset_type);
                
                let response = ui.horizontal(|ui| {
                    ui.colored_label(color, "●");
                    let label = ui.selectable_label(
                        is_selected,
                        format!("{} ({} → {})", 
                            asset_id.filename().unwrap_or("?"),
                            dep_count,
                            dependent_count
                        )
                    );
                    label
                });
                
                if response.inner.clicked() {
                    self.selected_asset = Some(asset_id.clone());
                    self.show_impact_analysis = false;
                }
                
                // Tooltip with full path
                response.response.on_hover_text(asset_id.path.to_string_lossy());
            }
        });

        // Selected asset details
        if let Some(ref asset_id) = self.selected_asset {
            ui.separator();
            self.draw_selected_asset_info(ui, asset_id);
        }
    }

    /// Draw selected asset info
    fn draw_selected_asset_info(&mut self, ui: &mut egui::Ui, asset_id: &AssetId) {
        ui.heading("Selected Asset");
        ui.separator();
        
        let Some(node) = self.graph.get(asset_id) else {
            ui.label("Asset not found in graph");
            return;
        };
        
        ui.horizontal(|ui| {
            let color = Self::get_asset_type_color(asset_id.asset_type);
            ui.colored_label(color, "●");
            ui.label(asset_id.filename().unwrap_or("?"));
        });
        
        ui.monospace(asset_id.path.to_string_lossy());
        ui.separator();
        
        // Dependencies
        ui.label(format!("Dependencies ({}):", node.dependencies.len()));
        for dep in &node.dependencies {
            let dep_color = if self.color_by_dependency_kind {
                Self::get_dependency_kind_color(dep.kind)
            } else {
                Self::get_asset_type_color(dep.target.asset_type)
            };
            
            ui.horizontal(|ui| {
                ui.colored_label(dep_color, "→");
                ui.label(dep.target.filename().unwrap_or("?"));
                if let Some(ref ctx) = dep.context {
                    ui.label(format!("({})", ctx));
                }
            });
        }
        
        // Dependents
        ui.separator();
        ui.label(format!("Dependents ({}):", node.dependents.len()));
        for dependent in &node.dependents {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::LIGHT_GREEN, "←");
                ui.label(dependent.filename().unwrap_or("?"));
            });
        }
        
        // Impact analysis button
        ui.separator();
        if ui.button("🔍 Impact Analysis").clicked() {
            self.show_impact_analysis = true;
        }
        
        // Safe to delete check
        ui.separator();
        match self.graph.can_delete(asset_id) {
            Ok(_) => {
                ui.colored_label(egui::Color32::GREEN, "✓ Safe to delete (no dependents)");
            }
            Err(e) => {
                ui.colored_label(egui::Color32::YELLOW, format!("⚠️ {}", e));
            }
        }
    }

    /// Draw the graph canvas
    fn draw_graph_canvas(&mut self, ui: &mut egui::Ui) {
        let available_size = ui.available_size();
        
        let (response, painter) = ui.allocate_painter(available_size, egui::Sense::drag());
        
        let rect = response.rect;
        let center = rect.center();
        
        // Handle panning
        if response.dragged() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                if let Some(last_pos) = self.last_mouse_pos {
                    let delta = pointer_pos - last_pos;
                    self.pan += glam::Vec2::new(delta.x, delta.y);
                }
                self.last_mouse_pos = Some(pointer_pos);
                self.is_panning = true;
            }
        } else {
            self.last_mouse_pos = None;
            self.is_panning = false;
        }
        
        // Handle zoom with scroll
        if response.hovered() {
            let scroll_delta = ui.input(|i| i.raw_scroll_delta);
            if scroll_delta.y != 0.0 {
                let zoom_factor = (scroll_delta.y / 100.0).exp();
                self.zoom = (self.zoom * zoom_factor).clamp(0.1, 5.0);
            }
        }
        
        // Draw background grid
        self.draw_grid(&painter, rect);
        
        // Get assets to draw
        let assets_to_draw: Vec<_> = self.get_filtered_assets();
        
        // Draw edges first (behind nodes)
        for asset_id in &assets_to_draw {
            if let Some(node) = self.graph.get(asset_id) {
                for dep in &node.dependencies {
                    // Only draw if target is also in filtered view
                    if assets_to_draw.contains(&&dep.target) {
                        self.draw_edge(&painter, asset_id, &dep.target, &dep.kind, center);
                    }
                }
            }
        }
        
        // Draw nodes
        for asset_id in &assets_to_draw {
            self.draw_node(&painter, asset_id, center, response.hovered());
        }
        
        // Handle node selection click
        if response.clicked() && !self.is_panning {
            if let Some(pos) = response.interact_pointer_pos() {
                // Check if clicked on a node
                for asset_id in &assets_to_draw {
                    if let Some(node_pos) = self.node_positions.get(asset_id) {
                        let screen_pos = self.world_to_screen(*node_pos, center);
                        let distance = pos.distance(screen_pos);
                        
                        if distance < 15.0 * self.zoom * self.node_size {
                            self.selected_asset = Some((*asset_id).clone());
                            break;
                        }
                    }
                }
            }
        }
        
        // Draw instructions
        painter.text(
            rect.left_top() + egui::vec2(10.0, 10.0),
            egui::Align2::LEFT_TOP,
            "Drag to pan • Scroll to zoom • Click node to select",
            egui::FontId::default(),
            ui.visuals().weak_text_color(),
        );
    }

    /// Draw background grid
    fn draw_grid(&self, painter: &egui::Painter, rect: egui::Rect) {
        let grid_size = 50.0 * self.zoom;
        let offset_x = self.pan.x % grid_size;
        let offset_y = self.pan.y % grid_size;
        
        let stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(40));
        
        // Vertical lines
        let mut x = rect.left() + offset_x;
        while x < rect.right() {
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                stroke,
            );
            x += grid_size;
        }
        
        // Horizontal lines
        let mut y = rect.top() + offset_y;
        while y < rect.bottom() {
            painter.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                stroke,
            );
            y += grid_size;
        }
    }

    /// Draw an edge between two nodes
    fn draw_edge(
        &self,
        painter: &egui::Painter,
        from: &AssetId,
        to: &AssetId,
        kind: &DependencyKind,
        center: egui::Pos2,
    ) {
        let Some(from_pos) = self.node_positions.get(from) else { return };
        let Some(to_pos) = self.node_positions.get(to) else { return };
        
        let from_screen = self.world_to_screen(*from_pos, center);
        let to_screen = self.world_to_screen(*to_pos, center);
        
        let color = if self.color_by_dependency_kind {
            Self::get_dependency_kind_color(*kind)
        } else {
            egui::Color32::from_gray(150)
        };
        
        // Draw line
        painter.line_segment(
            [from_screen, to_screen],
            egui::Stroke::new(2.0 * self.zoom, color),
        );
        
        // Draw arrowhead
        let dir = (to_screen - from_screen).normalized();
        let arrow_pos = to_screen - dir * (15.0 * self.zoom * self.node_size);
        let perp = egui::vec2(-dir.y, dir.x);
        
        let arrow_size = 8.0 * self.zoom;
        painter.add(egui::Shape::convex_polygon(
            vec![
                arrow_pos,
                arrow_pos - dir * arrow_size + perp * arrow_size * 0.5,
                arrow_pos - dir * arrow_size - perp * arrow_size * 0.5,
            ],
            color,
            egui::Stroke::NONE,
        ));
    }

    /// Draw a node
    fn draw_node(
        &self,
        painter: &egui::Painter,
        asset_id: &AssetId,
        center: egui::Pos2,
        _hovered: bool,
    ) {
        let Some(pos) = self.node_positions.get(asset_id) else { return };
        let screen_pos = self.world_to_screen(*pos, center);
        
        let is_selected = self.selected_asset.as_ref() == Some(asset_id);
        let node = self.graph.get(asset_id);
        
        let base_color = Self::get_asset_type_color(asset_id.asset_type);
        let radius = 12.0 * self.zoom * self.node_size;
        
        // Selection glow
        if is_selected {
            painter.circle_filled(
                screen_pos,
                radius + 4.0,
                egui::Color32::YELLOW.gamma_multiply(0.5),
            );
        }
        
        // Node circle
        painter.circle_filled(screen_pos, radius, base_color);
        
        // Border
        let border_color = if is_selected {
            egui::Color32::YELLOW
        } else {
            egui::Color32::WHITE
        };
        painter.circle_stroke(screen_pos, radius, egui::Stroke::new(2.0, border_color));
        
        // Warning indicator for orphaned assets
        if let Some(node) = node {
            if node.dependents.is_empty() {
                // Check if it's a root type (not really orphaned)
                let is_root = matches!(asset_id.asset_type, AssetType::Map | AssetType::Script);
                if !is_root {
                    // Draw warning indicator
                    painter.circle_filled(
                        screen_pos + egui::vec2(radius * 0.7, -radius * 0.7),
                        radius * 0.3,
                        egui::Color32::YELLOW,
                    );
                }
            }
        }
        
        // Label
        if self.show_labels {
            let label = asset_id.filename().unwrap_or("?");
            painter.text(
                screen_pos + egui::vec2(0.0, radius + 5.0),
                egui::Align2::CENTER_TOP,
                label,
                egui::FontId::proportional(12.0 * self.zoom),
                egui::Color32::WHITE,
            );
        }
    }

    /// Draw impact analysis overlay
    fn draw_impact_overlay(&mut self, ctx: &egui::Context, asset_id: &AssetId) {
        let impacted = self.get_impact_analysis(asset_id);
        
        egui::Window::new("🔍 Impact Analysis")
            .collapsible(true)
            .resizable(true)
            .default_size([300.0, 400.0])
            .show(ctx, |ui| {
                ui.heading("Impact if Deleted");
                ui.separator();
                
                ui.label(format!("Asset: {}", asset_id.filename().unwrap_or("?")));
                ui.label(format!("Total impacted assets: {}", impacted.len()));
                
                ui.separator();
                
                if impacted.is_empty() {
                    ui.colored_label(egui::Color32::GREEN, "✓ No dependencies - safe to delete");
                } else {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        format!("⚠️ {} assets would be affected", impacted.len())
                    );
                    
                    ui.label("Affected assets:");
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        for affected in &impacted {
                            ui.horizontal(|ui| {
                                let color = Self::get_asset_type_color(affected.asset_type);
                                ui.colored_label(color, "●");
                                ui.label(affected.filename().unwrap_or("?"));
                            });
                        }
                    });
                }
                
                ui.separator();
                
                if ui.button("Close").clicked() {
                    self.show_impact_analysis = false;
                }
            });
    }

    /// Convert world position to screen position
    fn world_to_screen(&self, world: NodePosition, center: egui::Pos2) -> egui::Pos2 {
        egui::pos2(
            center.x + (world.x + self.pan.x) * self.zoom,
            center.y + (world.y + self.pan.y) * self.zoom,
        )
    }

    /// Convert screen position to world position
    fn screen_to_world(&self, screen: egui::Pos2, center: egui::Pos2) -> NodePosition {
        NodePosition {
            x: (screen.x - center.x) / self.zoom - self.pan.x,
            y: (screen.y - center.y) / self.zoom - self.pan.y,
        }
    }

    /// Format bytes to human-readable string
    fn format_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_creation() {
        let panel = DependencyGraphPanel::new();
        assert!(!panel.is_visible());
        assert_eq!(panel.zoom, 1.0);
    }

    #[test]
    fn test_panel_toggle() {
        let mut panel = DependencyGraphPanel::new();
        assert!(!panel.is_visible());
        
        panel.toggle();
        assert!(panel.is_visible());
        
        panel.toggle();
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_zoom_limits() {
        let mut panel = DependencyGraphPanel::new();
        
        panel.zoom = 10.0;
        // Zoom should clamp when drawn, not here
        
        panel.zoom = (panel.zoom * 1.1).min(5.0);
        assert_eq!(panel.zoom, 5.0);
        
        panel.zoom = (panel.zoom * 0.01).max(0.1);
        assert!(panel.zoom >= 0.1);
    }

    #[test]
    fn test_asset_type_colors() {
        let color = DependencyGraphPanel::get_asset_type_color(AssetType::Texture);
        assert_eq!(color, egui::Color32::from_rgb(100, 150, 255));
        
        let color = DependencyGraphPanel::get_asset_type_color(AssetType::Script);
        assert_eq!(color, egui::Color32::from_rgb(255, 100, 150));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(DependencyGraphPanel::format_size(0), "0.00 B");
        assert_eq!(DependencyGraphPanel::format_size(1024), "1.00 KB");
        assert_eq!(DependencyGraphPanel::format_size(1024 * 1024), "1.00 MB");
    }
}
