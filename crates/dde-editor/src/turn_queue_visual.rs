//! Turn Queue Visualization for Battle System
//!
//! Provides a real-time ATB (Active Time Battle) turn queue display with:
//! - ATB charge bars for each combatant
//! - Turn order indicators
//! - Character portraits and enemy icons
//! - Status effect indicators
//! - Interactive selection for detailed info
//!
//! # Usage
//! ```rust,ignore
//! // In your battle UI system:
//! let mut visualizer = TurnQueueVisualizer::new();
//! 
//! // Each frame, update with current turn queue data
//! visualizer.update(&turn_queue, &world);
//! 
//! // Draw the visualization
//! visualizer.draw(ctx, &mut selected_combatant);
//! ```

use dde_battle::turn_queue::{CombatantInfo, StatusEffectType, TurnQueue};
use dde_core::{Entity, World};
use egui::{Color32, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2};

/// Visualizer for the turn queue
pub struct TurnQueueVisualizer {
    /// Whether the panel is visible
    visible: bool,
    /// Window position (for floating mode)
    position: Option<Pos2>,
    /// Whether to show as floating overlay
    pub floating_mode: bool,
    /// Cached combatant display info
    combatants: Vec<CombatantDisplayInfo>,
    /// Currently hovered combatant
    hovered_entity: Option<Entity>,
    /// Show detailed info window
    show_details: bool,
    /// Detailed view entity
    details_entity: Option<Entity>,
    /// Animation time for bar pulsing
    animation_time: f32,
    /// Sort order for display
    pub sort_order: SortOrder,
}

/// Display info for a combatant
#[derive(Debug, Clone)]
pub struct CombatantDisplayInfo {
    /// Entity ID
    pub entity: Entity,
    /// Display name
    pub name: String,
    /// Is player character
    pub is_player: bool,
    /// Is alive
    pub is_alive: bool,
    /// Current ATB value (0-100)
    pub atb: f32,
    /// ATB fill rate
    pub atb_rate: f32,
    /// Current HP
    pub hp: i32,
    /// Max HP
    pub max_hp: i32,
    /// Level
    pub level: i32,
    /// Status effects
    pub status_effects: Vec<StatusEffectDisplay>,
    /// Portrait/Icon color (fallback when no texture)
    pub icon_color: Color32,
    /// Is currently selected
    pub is_selected: bool,
    /// Is ready to act (ATB full)
    pub is_ready: bool,
    /// Is currently active (taking turn)
    pub is_active: bool,
    /// Position in turn order
    pub turn_order: usize,
}

/// Display info for a status effect
#[derive(Debug, Clone)]
pub struct StatusEffectDisplay {
    /// Effect type
    pub effect_type: StatusEffectType,
    /// Remaining turns
    pub remaining_turns: u32,
    /// Potency
    pub potency: i32,
    /// Icon/emoji
    pub icon: &'static str,
    /// Color
    pub color: Color32,
}

/// Sort order for combatant display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// Sort by ATB value (highest first)
    AtbDescending,
    /// Sort by turn order (who acts next)
    TurnOrder,
    /// Players first, then enemies
    PlayersFirst,
    /// Original battle order
    BattleOrder,
}

/// Configuration for the visualizer appearance
#[derive(Debug, Clone)]
pub struct VisualizerConfig {
    /// Width of the combatant row
    pub row_width: f32,
    /// Height of each combatant row
    pub row_height: f32,
    /// Size of the portrait/icon
    pub icon_size: f32,
    /// Size of status effect icons
    pub status_icon_size: f32,
    /// Color for player ATB bars
    pub player_atb_color: Color32,
    /// Color for enemy ATB bars
    pub enemy_atb_color: Color32,
    /// Color for full ATB (ready to act)
    pub ready_atb_color: Color32,
    /// Color for active combatant
    pub active_color: Color32,
    /// Background color for player rows
    pub player_bg_color: Color32,
    /// Background color for enemy rows
    pub enemy_bg_color: Color32,
    /// Dead combatant opacity
    pub dead_opacity: f32,
}

impl Default for VisualizerConfig {
    fn default() -> Self {
        Self {
            row_width: 280.0,
            row_height: 60.0,
            icon_size: 48.0,
            status_icon_size: 16.0,
            player_atb_color: Color32::from_rgb(65, 105, 225),   // Royal Blue
            enemy_atb_color: Color32::from_rgb(178, 34, 34),     // Fire Brick
            ready_atb_color: Color32::from_rgb(50, 205, 50),     // Lime Green
            active_color: Color32::from_rgb(255, 215, 0),        // Gold
            player_bg_color: Color32::from_rgba_premultiplied(30, 60, 120, 200),
            enemy_bg_color: Color32::from_rgba_premultiplied(120, 30, 30, 200),
            dead_opacity: 0.4,
        }
    }
}

impl TurnQueueVisualizer {
    /// Create a new turn queue visualizer
    pub fn new() -> Self {
        Self {
            visible: true,
            position: None,
            floating_mode: true,
            combatants: Vec::new(),
            hovered_entity: None,
            show_details: false,
            details_entity: None,
            animation_time: 0.0,
            sort_order: SortOrder::TurnOrder,
        }
    }

    /// Show the visualizer
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the visualizer
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

    /// Set floating mode
    pub fn set_floating_mode(&mut self, floating: bool) {
        self.floating_mode = floating;
    }

    /// Check if in floating mode
    pub fn is_floating_mode(&self) -> bool {
        self.floating_mode
    }

    /// Set sort order
    pub fn set_sort_order(&mut self, order: SortOrder) {
        self.sort_order = order;
    }

    /// Update animation time (call each frame)
    pub fn update(&mut self, dt: f32) {
        self.animation_time += dt;
    }

    /// Update combatant data from turn queue
    pub fn update_from_queue(&mut self, queue: &TurnQueue, world: &World) {
        self.combatants.clear();

        // Get all combatants from queue
        let alive_entities = queue.alive_combatants();
        let active_entity = queue.active_entity();
        
        // Build display info for each combatant
        for entity in &alive_entities {
            if let Some(info) = queue.get_combatant(*entity) {
                let display_info = self.build_display_info(*entity, info, world, active_entity);
                self.combatants.push(display_info);
            }
        }

        // Sort combatants based on selected order
        self.sort_combatants();

        // Assign turn order
        for (i, combatant) in self.combatants.iter_mut().enumerate() {
            combatant.turn_order = i + 1;
        }
    }

    /// Build display info for a combatant
    fn build_display_info(
        &self,
        entity: Entity,
        info: &CombatantInfo,
        world: &World,
        active_entity: Option<Entity>,
    ) -> CombatantDisplayInfo {
        // Try to get name from world (would need a Name component)
        let name = format!("{}", 
            if info.is_player { "Player" } else { "Enemy" }
        );

        // Get HP if available
        let (hp, max_hp) = world.query_one::<&dde_core::components::Stats>(entity)
            .ok()
            .and_then(|mut q| q.get().map(|s| (s.hp, s.max_hp)))
            .unwrap_or((100, 100));

        // Get level if available
        let level = world.query_one::<&dde_core::components::battle::Level>(entity)
            .ok()
            .and_then(|mut q| q.get().map(|l| l.level))
            .unwrap_or(1);

        // Convert status effects
        let status_effects: Vec<StatusEffectDisplay> = info.status_effects
            .iter()
            .map(|se| StatusEffectDisplay {
                effect_type: se.effect_type,
                remaining_turns: se.remaining_turns,
                potency: se.potency,
                icon: get_status_icon_for_type(se.effect_type),
                color: get_status_color_for_type(se.effect_type),
            })
            .collect();

        // Determine icon color based on type
        let icon_color = if info.is_player {
            Color32::from_rgb(100, 150, 255)
        } else {
            Color32::from_rgb(255, 100, 100)
        };

        CombatantDisplayInfo {
            entity,
            name,
            is_player: info.is_player,
            is_alive: info.is_alive,
            atb: info.atb,
            atb_rate: info.atb_rate,
            hp,
            max_hp,
            level,
            status_effects,
            icon_color,
            is_selected: false,
            is_ready: info.atb >= 100.0,
            is_active: active_entity == Some(entity),
            turn_order: 0,
        }
    }

    /// Sort combatants based on current sort order
    fn sort_combatants(&mut self) {
        match self.sort_order {
            SortOrder::AtbDescending => {
                self.combatants.sort_by(|a, b| {
                    b.atb.partial_cmp(&a.atb).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortOrder::TurnOrder => {
                // Ready combatants first, then by ATB
                self.combatants.sort_by(|a, b| {
                    let a_ready = if a.is_ready { 2 } else if a.is_active { 1 } else { 0 };
                    let b_ready = if b.is_ready { 2 } else if b.is_active { 1 } else { 0 };
                    b_ready.cmp(&a_ready)
                        .then_with(|| b.atb.partial_cmp(&a.atb).unwrap_or(std::cmp::Ordering::Equal))
                });
            }
            SortOrder::PlayersFirst => {
                self.combatants.sort_by(|a, b| {
                    b.is_player.cmp(&a.is_player)
                        .then_with(|| b.atb.partial_cmp(&a.atb).unwrap_or(std::cmp::Ordering::Equal))
                });
            }
            SortOrder::BattleOrder => {
                // Keep current order (based on entity ID for stability)
                self.combatants.sort_by_key(|c| c.entity);
            }
        }
    }

    /// Draw the turn queue visualization
    pub fn draw(&mut self, ctx: &egui::Context, selected_entity: &mut Option<Entity>) {
        if !self.visible {
            return;
        }

        if self.floating_mode {
            self.draw_floating(ctx, selected_entity);
        } else {
            self.draw_panel(ctx, selected_entity);
        }

        // Draw details window if requested
        if self.show_details {
            self.draw_details_window(ctx);
        }
    }

    /// Draw as floating overlay
    fn draw_floating(&mut self, ctx: &egui::Context, selected_entity: &mut Option<Entity>) {
        let config = VisualizerConfig::default();
        
        let window = egui::Window::new("⚔️ Turn Queue")
            .resizable(true)
            .default_size([config.row_width + 40.0, 400.0])
            .collapsible(true);

        let window = if let Some(pos) = self.position {
            window.current_pos(pos)
        } else {
            window.default_pos(egui::pos2(10.0, 100.0))
        };

        window.show(ctx, |ui| {
            self.draw_controls(ui);
            ui.separator();
            self.draw_combatant_list(ui, selected_entity, &config);
        });
    }

    /// Draw as embedded panel
    fn draw_panel(&mut self, ctx: &egui::Context, selected_entity: &mut Option<Entity>) {
        let config = VisualizerConfig::default();
        
        egui::SidePanel::right("turn_queue_panel")
            .resizable(true)
            .default_width(config.row_width + 40.0)
            .show(ctx, |ui| {
                ui.heading("⚔️ Turn Queue");
                ui.separator();
                self.draw_controls(ui);
                ui.separator();
                self.draw_combatant_list(ui, selected_entity, &config);
            });
    }

    /// Draw control buttons
    fn draw_controls(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Sort:");
            
            let mut current_order = self.sort_order;
            egui::ComboBox::from_id_salt("sort_order")
                .selected_text(match self.sort_order {
                    SortOrder::AtbDescending => "ATB",
                    SortOrder::TurnOrder => "Turn Order",
                    SortOrder::PlayersFirst => "Players First",
                    SortOrder::BattleOrder => "Battle Order",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut current_order, SortOrder::TurnOrder, "Turn Order");
                    ui.selectable_value(&mut current_order, SortOrder::AtbDescending, "ATB");
                    ui.selectable_value(&mut current_order, SortOrder::PlayersFirst, "Players First");
                    ui.selectable_value(&mut current_order, SortOrder::BattleOrder, "Battle Order");
                });
            
            if current_order != self.sort_order {
                self.sort_order = current_order;
            }

            if ui.button("ℹ️").on_hover_text("Show detailed info").clicked() {
                self.show_details = !self.show_details;
            }
        });
    }

    /// Draw the list of combatants
    fn draw_combatant_list(
        &mut self,
        ui: &mut Ui,
        selected_entity: &mut Option<Entity>,
        config: &VisualizerConfig,
    ) {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for combatant in &self.combatants {
                    let response = self.draw_combatant_row(ui, combatant, config);
                    
                    // Handle click for selection
                    if response.clicked() {
                        *selected_entity = Some(combatant.entity);
                        self.details_entity = Some(combatant.entity);
                    }
                    
                    // Handle hover
                    if response.hovered() {
                        self.hovered_entity = Some(combatant.entity);
                    }
                }
            });
    }

    /// Draw a single combatant row
    fn draw_combatant_row(
        &self,
        ui: &mut Ui,
        combatant: &CombatantDisplayInfo,
        config: &VisualizerConfig,
    ) -> Response {
        let desired_size = Vec2::new(config.row_width, config.row_height);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            
            // Background
            let bg_color = if combatant.is_selected {
                Color32::from_rgb(100, 100, 50)
            } else if combatant.is_player {
                config.player_bg_color
            } else {
                config.enemy_bg_color
            };

            let opacity = if combatant.is_alive { 1.0 } else { config.dead_opacity };
            let bg_color = with_opacity(bg_color, opacity);

            // Rounded rectangle background
            painter.rect_filled(rect, Rounding::same(8.0), bg_color);

            // Border for active/ready combatants
            if combatant.is_active {
                let pulse = (self.animation_time * 4.0).sin() * 0.3 + 0.7;
                let border_color = Color32::from_rgba_premultiplied(
                    (255.0 * pulse) as u8,
                    215,
                    0,
                    255,
                );
                painter.rect_stroke(rect, Rounding::same(8.0), Stroke::new(3.0, border_color));
            } else if combatant.is_ready {
                painter.rect_stroke(rect, Rounding::same(8.0), Stroke::new(2.0, config.ready_atb_color));
            }

            // Layout calculation
            let margin = 8.0;
            let icon_x = rect.min.x + margin;
            let icon_y = rect.min.y + (rect.height() - config.icon_size) / 2.0;
            let icon_rect = Rect::from_min_size(
                Pos2::new(icon_x, icon_y),
                Vec2::new(config.icon_size, config.icon_size),
            );

            // Draw portrait/icon placeholder
            self.draw_combatant_icon(painter, icon_rect, combatant, config);

            // Info area
            let info_x = icon_rect.max.x + margin;
            let info_width = rect.max.x - info_x - margin;
            let info_rect = Rect::from_min_size(
                Pos2::new(info_x, rect.min.y + margin),
                Vec2::new(info_width, rect.height() - margin * 2.0),
            );

            // Draw name and info
            self.draw_combatant_info(painter, info_rect, combatant, config);

            // Draw status effects
            self.draw_status_effects(ui, rect, combatant, config);
        }

        response
    }

    /// Draw combatant icon/portrait
    fn draw_combatant_icon(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        combatant: &CombatantDisplayInfo,
        _config: &VisualizerConfig,
    ) {
        // Background circle for icon
        painter.circle_filled(rect.center(), rect.width() / 2.0, combatant.icon_color);

        // Icon symbol (emoji or letter)
        let symbol = if combatant.is_player { "🧙" } else { "👹" };
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            symbol,
            egui::FontId::proportional(rect.width() * 0.6),
            Color32::WHITE,
        );

        // Level indicator
        let level_pos = Pos2::new(rect.max.x - 12.0, rect.min.y + 12.0);
        painter.circle_filled(level_pos, 10.0, Color32::from_rgb(50, 50, 50));
        painter.text(
            level_pos,
            egui::Align2::CENTER_CENTER,
            combatant.level.to_string(),
            egui::FontId::proportional(10.0),
            Color32::WHITE,
        );
    }

    /// Draw combatant name and ATB bar
    fn draw_combatant_info(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        combatant: &CombatantDisplayInfo,
        config: &VisualizerConfig,
    ) {
        let line_height = rect.height() / 3.0;

        // Name row
        let name_color = if combatant.is_player {
            Color32::from_rgb(200, 220, 255)
        } else {
            Color32::from_rgb(255, 200, 200)
        };

        let name_text = if combatant.is_active {
            format!("▶ {}", combatant.name)
        } else if combatant.is_ready {
            format!("◆ {}", combatant.name)
        } else {
            combatant.name.clone()
        };

        painter.text(
            Pos2::new(rect.min.x, rect.min.y + line_height * 0.5),
            egui::Align2::LEFT_CENTER,
            name_text,
            egui::FontId::proportional(14.0),
            name_color,
        );

        // HP text
        let hp_text = format!("HP: {}/{}", combatant.hp, combatant.max_hp);
        let hp_color = if combatant.hp < combatant.max_hp / 4 {
            Color32::RED
        } else if combatant.hp < combatant.max_hp / 2 {
            Color32::YELLOW
        } else {
            Color32::GREEN
        };

        painter.text(
            Pos2::new(rect.max.x, rect.min.y + line_height * 0.5),
            egui::Align2::RIGHT_CENTER,
            hp_text,
            egui::FontId::proportional(11.0),
            hp_color,
        );

        // ATB Bar background
        let bar_y = rect.min.y + line_height * 1.5;
        let bar_height = line_height * 0.8;
        let bar_rect = Rect::from_min_size(
            Pos2::new(rect.min.x, bar_y - bar_height / 2.0),
            Vec2::new(rect.width(), bar_height),
        );

        painter.rect_filled(bar_rect, Rounding::same(4.0), Color32::from_rgb(40, 40, 40));

        // ATB Bar fill
        let atb_pct = (combatant.atb / 100.0).clamp(0.0, 1.0);
        let fill_width = rect.width() * atb_pct;
        let fill_rect = Rect::from_min_size(
            bar_rect.min,
            Vec2::new(fill_width, bar_height),
        );

        let bar_color = if combatant.is_ready {
            // Pulse effect for ready combatants
            let pulse = (self.animation_time * 3.0).sin() * 0.2 + 0.8;
            Color32::from_rgba_premultiplied(
                (config.ready_atb_color.r() as f32 * pulse) as u8,
                (config.ready_atb_color.g() as f32 * pulse) as u8,
                (config.ready_atb_color.b() as f32 * pulse) as u8,
                255,
            )
        } else if combatant.is_player {
            config.player_atb_color
        } else {
            config.enemy_atb_color
        };

        painter.rect_filled(fill_rect, Rounding::same(4.0), bar_color);

        // ATB percentage text
        let atb_text = format!("{:.0}%", combatant.atb);
        painter.text(
            bar_rect.center(),
            egui::Align2::CENTER_CENTER,
            atb_text,
            egui::FontId::proportional(10.0),
            Color32::WHITE,
        );

        // Turn order indicator
        if combatant.turn_order <= 3 {
            let order_colors = [
                Color32::GOLD,
                Color32::SILVER,
                Color32::from_rgb(205, 127, 50), // Bronze
            ];
            let order_pos = Pos2::new(rect.min.x - 5.0, rect.min.y + 5.0);
            painter.circle_filled(order_pos, 8.0, order_colors[combatant.turn_order - 1]);
            painter.text(
                order_pos,
                egui::Align2::CENTER_CENTER,
                combatant.turn_order.to_string(),
                egui::FontId::proportional(9.0),
                Color32::BLACK,
            );
        }
    }

    /// Draw status effect icons
    fn draw_status_effects(
        &self,
        ui: &mut Ui,
        row_rect: Rect,
        combatant: &CombatantDisplayInfo,
        config: &VisualizerConfig,
    ) {
        if combatant.status_effects.is_empty() {
            return;
        }

        let icon_spacing = config.status_icon_size + 2.0;
        let start_x = row_rect.max.x - 10.0 - (combatant.status_effects.len() as f32 * icon_spacing);
        let y = row_rect.max.y - config.status_icon_size - 4.0;

        for (i, effect) in combatant.status_effects.iter().enumerate() {
            let x = start_x + i as f32 * icon_spacing;
            let rect = Rect::from_min_size(
                Pos2::new(x, y),
                Vec2::new(config.status_icon_size, config.status_icon_size),
            );

            // Tooltip area
            let response = ui.interact(rect, ui.id().with(effect.effect_type as u8), Sense::hover());
            
            if response.hovered() {
                egui::show_tooltip(ui.ctx(), response.id, |ui| {
                    ui.label(format!("{} ({} turns)", 
                        format_status_name(effect.effect_type),
                        effect.remaining_turns
                    ));
                    if effect.potency > 0 {
                        ui.label(format!("Potency: {}", effect.potency));
                    }
                });
            }

            // Draw icon background
            ui.painter().rect_filled(rect, Rounding::same(4.0), effect.color);

            // Draw icon
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                effect.icon,
                egui::FontId::proportional(config.status_icon_size * 0.7),
                Color32::WHITE,
            );
        }
    }

    /// Draw detailed info window
    fn draw_details_window(&mut self, ctx: &egui::Context) {
        if let Some(entity) = self.details_entity {
            if let Some(combatant) = self.combatants.iter().find(|c| c.entity == entity) {
                egui::Window::new(format!("📊 {} Details", combatant.name))
                    .collapsible(true)
                    .resizable(true)
                    .default_size([300.0, 400.0])
                    .show(ctx, |ui| {
                        self.draw_detailed_info(ui, combatant);
                    });
            }
        }
    }

    /// Draw detailed combatant info
    fn draw_detailed_info(&self, ui: &mut Ui, combatant: &CombatantDisplayInfo) {
        egui::Grid::new("combatant_details")
            .num_columns(2)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                ui.label("Type:");
                ui.label(if combatant.is_player { "Player" } else { "Enemy" });
                ui.end_row();

                ui.label("Level:");
                ui.label(combatant.level.to_string());
                ui.end_row();

                ui.label("HP:");
                ui.label(format!("{}/{}", combatant.hp, combatant.max_hp));
                ui.end_row();

                ui.label("ATB:");
                ui.label(format!("{:.1}% (rate: {:.1})", combatant.atb, combatant.atb_rate));
                ui.end_row();

                ui.label("Status:");
                if combatant.is_active {
                    ui.colored_label(Color32::GOLD, "● Taking Turn");
                } else if combatant.is_ready {
                    ui.colored_label(Color32::GREEN, "◆ Ready");
                } else {
                    ui.label("○ Charging");
                }
                ui.end_row();

                ui.label("Entity ID:");
                ui.label(format!("{:?}", combatant.entity));
                ui.end_row();
            });

        ui.separator();
        ui.heading("Status Effects");

        if combatant.status_effects.is_empty() {
            ui.label("No active status effects");
        } else {
            for effect in &combatant.status_effects {
                ui.horizontal(|ui| {
                    ui.label(effect.icon);
                    ui.label(format_status_name(effect.effect_type));
                    ui.label(format!("({} turns)", effect.remaining_turns));
                    if effect.potency > 0 {
                        ui.label(format!("[{}]", effect.potency));
                    }
                });
            }
        }

        ui.separator();
        
        // ATB calculation info
        ui.heading("ATB Info");
        ui.label(format!("Current: {:.1}%", combatant.atb));
        ui.label(format!("Fill Rate: {:.2} per tick", combatant.atb_rate));
        
        if combatant.atb_rate > 0.0 && combatant.atb < 100.0 {
            let ticks_to_full = ((100.0 - combatant.atb) / combatant.atb_rate).ceil();
            ui.label(format!("Ticks to full: ~{:.0}", ticks_to_full));
        }
    }

    /// Get the currently hovered entity
    pub fn hovered_entity(&self) -> Option<Entity> {
        self.hovered_entity
    }

    /// Set selected entity
    pub fn set_selected(&mut self, entity: Option<Entity>) {
        for combatant in &mut self.combatants {
            combatant.is_selected = Some(combatant.entity) == entity;
        }
    }
}

impl Default for TurnQueueVisualizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to apply opacity to a color
fn with_opacity(color: Color32, opacity: f32) -> Color32 {
    Color32::from_rgba_premultiplied(
        color.r(),
        color.g(),
        color.b(),
        (color.a() as f32 * opacity) as u8,
    )
}

/// Get status icon for effect type
fn get_status_icon_for_type(effect: StatusEffectType) -> &'static str {
    // Map turn_queue StatusEffectType to status module StatusType
    match effect {
        StatusEffectType::Poison => "☠️",
        StatusEffectType::Burn => "🔥",
        StatusEffectType::Regen => "✨",
        StatusEffectType::Haste => "⚡",
        StatusEffectType::Slow => "🐌",
        StatusEffectType::Stun => "💫",
        StatusEffectType::Shield => "🛡️",
        StatusEffectType::AttackUp => "⚔️",
        StatusEffectType::AttackDown => "💔",
        StatusEffectType::DefenseUp => "🛡️",
        StatusEffectType::DefenseDown => "🕳️",
        StatusEffectType::MagicUp => "🔮",
        StatusEffectType::MagicDown => "📉",
        StatusEffectType::SpeedUp => "💨",
        StatusEffectType::SpeedDown => "🦥",
    }
}

/// Get status color for effect type
fn get_status_color_for_type(effect: StatusEffectType) -> Color32 {
    match effect {
        StatusEffectType::Poison => Color32::from_rgb(128, 0, 128),
        StatusEffectType::Burn => Color32::from_rgb(255, 69, 0),
        StatusEffectType::Regen => Color32::from_rgb(50, 205, 50),
        StatusEffectType::Haste => Color32::from_rgb(255, 215, 0),
        StatusEffectType::Slow => Color32::from_rgb(128, 128, 128),
        StatusEffectType::Stun => Color32::from_rgb(255, 255, 0),
        StatusEffectType::Shield => Color32::from_rgb(135, 206, 235),
        StatusEffectType::AttackUp => Color32::from_rgb(255, 99, 71),
        StatusEffectType::AttackDown => Color32::from_rgb(139, 0, 0),
        StatusEffectType::DefenseUp => Color32::from_rgb(65, 105, 225),
        StatusEffectType::DefenseDown => Color32::from_rgb(105, 105, 105),
        StatusEffectType::MagicUp => Color32::from_rgb(138, 43, 226),
        StatusEffectType::MagicDown => Color32::from_rgb(75, 0, 130),
        StatusEffectType::SpeedUp => Color32::from_rgb(0, 255, 255),
        StatusEffectType::SpeedDown => Color32::from_rgb(128, 128, 0),
    }
}

/// Format status effect name for display
fn format_status_name(effect: StatusEffectType) -> String {
    let name = match effect {
        StatusEffectType::Poison => "Poison",
        StatusEffectType::Burn => "Burn",
        StatusEffectType::Regen => "Regen",
        StatusEffectType::Haste => "Haste",
        StatusEffectType::Slow => "Slow",
        StatusEffectType::Stun => "Stun",
        StatusEffectType::Shield => "Shield",
        StatusEffectType::AttackUp => "Attack Up",
        StatusEffectType::AttackDown => "Attack Down",
        StatusEffectType::DefenseUp => "Defense Up",
        StatusEffectType::DefenseDown => "Defense Down",
        StatusEffectType::MagicUp => "Magic Up",
        StatusEffectType::MagicDown => "Magic Down",
        StatusEffectType::SpeedUp => "Speed Up",
        StatusEffectType::SpeedDown => "Speed Down",
    };
    name.to_string()
}

/// Extension trait for TurnQueue to provide visualization data
pub trait TurnQueueVisualizationExt {
    /// Get combatant names for display
    fn get_combatant_names(&self) -> Vec<(Entity, String, bool)>;
    /// Get ATB values for all combatants
    fn get_atb_values(&self) -> Vec<(Entity, f32)>;
    /// Get ready combatants sorted by ATB
    fn get_ready_sorted(&self) -> Vec<(Entity, f32)>;
}

impl TurnQueueVisualizationExt for TurnQueue {
    fn get_combatant_names(&self) -> Vec<(Entity, String, bool)> {
        self.alive_combatants()
            .into_iter()
            .filter_map(|entity| {
                self.get_combatant(entity).map(|c| {
                    let name = if c.is_player { "Player" } else { "Enemy" };
                    (entity, name.to_string(), c.is_player)
                })
            })
            .collect()
    }

    fn get_atb_values(&self) -> Vec<(Entity, f32)> {
        self.alive_combatants()
            .into_iter()
            .filter_map(|entity| {
                self.get_combatant(entity).map(|c| (entity, c.atb))
            })
            .collect()
    }

    fn get_ready_sorted(&self) -> Vec<(Entity, f32)> {
        let mut ready: Vec<(Entity, f32)> = self.alive_combatants()
            .into_iter()
            .filter_map(|entity| {
                self.get_combatant(entity)
                    .filter(|c| c.atb >= 100.0)
                    .map(|c| (entity, c.atb))
            })
            .collect();
        
        ready.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visualizer_creation() {
        let visualizer = TurnQueueVisualizer::new();
        assert!(visualizer.is_visible());
        assert!(visualizer.floating_mode);
    }

    #[test]
    fn test_sort_order() {
        let mut visualizer = TurnQueueVisualizer::new();
        
        visualizer.set_sort_order(SortOrder::AtbDescending);
        assert_eq!(visualizer.sort_order, SortOrder::AtbDescending);
        
        visualizer.set_sort_order(SortOrder::PlayersFirst);
        assert_eq!(visualizer.sort_order, SortOrder::PlayersFirst);
    }

    #[test]
    fn test_config_defaults() {
        let config = VisualizerConfig::default();
        assert!(config.row_width > 0.0);
        assert!(config.row_height > 0.0);
        assert!(config.icon_size > 0.0);
    }

    #[test]
    fn test_status_icons() {
        assert_eq!(get_status_icon_for_type(StatusEffectType::Poison), "☠️");
        assert_eq!(get_status_icon_for_type(StatusEffectType::Burn), "🔥");
        assert_eq!(get_status_icon_for_type(StatusEffectType::Haste), "⚡");
    }

    #[test]
    fn test_format_status_name() {
        assert_eq!(format_status_name(StatusEffectType::Poison), "Poison");
        assert_eq!(format_status_name(StatusEffectType::AttackUp), "Attack Up");
        assert_eq!(format_status_name(StatusEffectType::DefenseDown), "Defense Down");
    }
}
