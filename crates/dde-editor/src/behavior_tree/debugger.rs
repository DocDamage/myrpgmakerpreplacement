//! Behavior tree debugger
//!
//! This module provides runtime debugging capabilities for behavior trees,
//! including visual overlays and detailed inspection panels.

use std::collections::{HashMap, VecDeque};

use dde_core::ai::{BehaviorTreeRunner, BtStatus, NodeId};
use dde_core::Entity;
use egui::{Color32, Pos2, Rect, RichText, Ui, Vec2};

/// Debug visualizer for running behavior trees
#[derive(Debug, Clone)]
pub struct BtDebugger {
    /// Entity being debugged
    target_entity: Option<Entity>,
    /// Current status of each node
    node_status: HashMap<NodeId, BtStatus>,
    /// Last tick's execution path
    execution_path: Vec<NodeId>,
    /// History of status changes
    status_history: VecDeque<StatusChange>,
    /// Auto-refresh rate (updates per second)
    refresh_rate: f32,
    /// Time since last refresh
    time_since_refresh: f32,
    /// Whether to auto-step
    auto_step: bool,
    /// Paused state
    paused: bool,
    /// Breakpoints
    breakpoints: Vec<NodeId>,
    /// Step request flag
    step_requested: bool,
}

impl Default for BtDebugger {
    fn default() -> Self {
        Self::new()
    }
}

impl BtDebugger {
    /// Create a new behavior tree debugger
    pub fn new() -> Self {
        Self {
            target_entity: None,
            node_status: HashMap::new(),
            execution_path: Vec::new(),
            status_history: VecDeque::with_capacity(100),
            refresh_rate: 10.0, // 10 updates per second
            time_since_refresh: 0.0,
            auto_step: true,
            paused: false,
            breakpoints: Vec::new(),
            step_requested: false,
        }
    }

    /// Set the target entity to debug
    pub fn set_target(&mut self, entity: Entity) {
        self.target_entity = Some(entity);
        self.clear_state();
    }

    /// Clear the debug target
    pub fn clear_target(&mut self) {
        self.target_entity = None;
        self.clear_state();
    }

    /// Get the current target entity
    pub fn target_entity(&self) -> Option<Entity> {
        self.target_entity
    }

    /// Clear debug state
    fn clear_state(&mut self) {
        self.node_status.clear();
        self.execution_path.clear();
        self.status_history.clear();
    }

    /// Update debug info from running BT
    pub fn update(&mut self, dt: f32, entity: Entity, runner: &BehaviorTreeRunner) {
        if self.target_entity != Some(entity) {
            return;
        }

        self.time_since_refresh += dt;

        if self.time_since_refresh < 1.0 / self.refresh_rate {
            return;
        }
        self.time_since_refresh = 0.0;

        // Update execution path
        let new_path: Vec<NodeId> = runner.current_path().to_vec();

        // Detect changes and record to history
        for node_id in &new_path {
            let new_status = BtStatus::Running;
            if let Some(&old_status) = self.node_status.get(node_id) {
                if old_status != new_status {
                    self.record_change(*node_id, old_status, new_status);
                }
            } else {
                self.record_change(*node_id, BtStatus::Success, new_status);
            }
        }

        self.execution_path = new_path;

        // Get running node status
        if let Some(running_id) = runner.running_node() {
            self.node_status.insert(running_id, BtStatus::Running);
        }

        // Update blackboard values display
        // (This is done on-demand in draw_panel)
    }

    /// Record a status change
    fn record_change(&mut self, node_id: NodeId, from: BtStatus, to: BtStatus) {
        let change = StatusChange {
            timestamp: std::time::Instant::now(),
            node_id,
            from,
            to,
        };

        if self.status_history.len() >= 100 {
            self.status_history.pop_back();
        }
        self.status_history.push_front(change);

        self.node_status.insert(node_id, to);
    }

    /// Set refresh rate
    pub fn set_refresh_rate(&mut self, rate: f32) {
        self.refresh_rate = rate.clamp(1.0, 60.0);
    }

    /// Toggle pause
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Check if should tick (respects pause/step)
    pub fn should_tick(&mut self) -> bool {
        if self.step_requested {
            self.step_requested = false;
            return true;
        }
        !self.paused && self.auto_step
    }

    /// Request a single step
    pub fn step(&mut self) {
        self.step_requested = true;
    }

    /// Toggle breakpoint
    pub fn toggle_breakpoint(&mut self, node_id: NodeId) {
        if let Some(pos) = self.breakpoints.iter().position(|&id| id == node_id) {
            self.breakpoints.remove(pos);
        } else {
            self.breakpoints.push(node_id);
        }
    }

    /// Check if a node has a breakpoint
    pub fn has_breakpoint(&self, node_id: NodeId) -> bool {
        self.breakpoints.contains(&node_id)
    }

    /// Draw detailed debugger panel
    pub fn draw_panel(&mut self, ui: &mut Ui, runner: Option<&BehaviorTreeRunner>) {
        ui.horizontal(|ui| {
            ui.heading("Behavior Tree Debugger");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear").clicked() {
                    self.clear_state();
                }
            });
        });

        ui.separator();

        // Target info
        if let Some(entity) = self.target_entity {
            ui.label(format!("Target: {:?}", entity));
        } else {
            ui.label(RichText::new("No target selected").color(Color32::YELLOW));
        }

        ui.separator();

        // Playback controls
        ui.horizontal(|ui| {
            if ui
                .button(if self.paused { "▶ Play" } else { "⏸ Pause" })
                .clicked()
            {
                self.toggle_pause();
            }

            if ui.button("⏭ Step").clicked() {
                self.step();
            }

            ui.checkbox(&mut self.auto_step, "Auto-step");

            ui.add(
                egui::Slider::new(&mut self.refresh_rate, 1.0..=60.0)
                    .text("Refresh rate")
                    .suffix(" Hz"),
            );
        });

        ui.separator();

        // Execution path
        ui.collapsing("Execution Path", |ui| {
            if self.execution_path.is_empty() {
                ui.label("No active execution path");
            } else {
                for (i, node_id) in self.execution_path.iter().enumerate() {
                    let indent = "  ".repeat(i);
                    let status = self
                        .node_status
                        .get(node_id)
                        .copied()
                        .unwrap_or(BtStatus::Failure);
                    let color = status_color(status);
                    ui.label(RichText::new(format!("{}└─ {:?}", indent, node_id)).color(color));
                }
            }
        });

        // Blackboard
        if let Some(runner) = runner {
            ui.collapsing("Blackboard", |ui| {
                let blackboard = runner.blackboard();
                let keys: Vec<_> = blackboard.keys().collect();
                if keys.is_empty() {
                    ui.label("No variables set");
                } else {
                    for key in keys {
                        // Display the key and a placeholder for value (actual type unknown)
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(key).monospace().small());
                            ui.label("=");
                            // Try to get value as string representation
                            let value_str = blackboard.get_string(key)
                                .map(|s| s.to_string())
                                .or_else(|| blackboard.get_bool(key).map(|b| b.to_string()))
                                .or_else(|| blackboard.get_int(key).map(|i| i.to_string()))
                                .or_else(|| blackboard.get_float(key).map(|f| format!("{:.2}", f)))
                                .unwrap_or_else(|| "<complex>".to_string());
                            ui.label(RichText::new(value_str).color(Color32::LIGHT_BLUE));
                        });
                    }
                }
            });
        }

        // Status history
        ui.collapsing("Status History", |ui| {
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for change in &self.status_history {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{:?}", change.node_id))
                                    .monospace()
                                    .small(),
                            );
                            ui.label(
                                RichText::new(format!("{:?}", change.from))
                                    .color(status_color(change.from))
                                    .small(),
                            );
                            ui.label("→");
                            ui.label(
                                RichText::new(format!("{:?}", change.to))
                                    .color(status_color(change.to))
                                    .small(),
                            );
                        });
                    }
                });
        });

        // Breakpoints
        ui.collapsing("Breakpoints", |ui| {
            if self.breakpoints.is_empty() {
                ui.label("No breakpoints set");
            } else {
                for node_id in &self.breakpoints {
                    ui.horizontal(|ui| {
                        ui.label(format!("{:?}", node_id));
                        if ui.button("Remove").clicked() {
                            // This would need to be handled differently in practice
                        }
                    });
                }
            }
        });
    }

    /// Draw debug overlay on game view
    pub fn draw_entity_overlay(
        &self,
        painter: &egui::Painter,
        entity_screen_pos: Pos2,
        entity: Entity,
        runner: Option<&BehaviorTreeRunner>,
    ) {
        if self.target_entity != Some(entity) {
            return;
        }

        let Some(runner) = runner else {
            return;
        };

        // Draw status bubble above entity
        let bubble_rect = Rect::from_center_size(
            entity_screen_pos - Vec2::new(0.0, 40.0),
            Vec2::new(120.0, 30.0),
        );

        // Background
        painter.rect_filled(bubble_rect, 5.0, Color32::from_black_alpha(200));
        painter.rect_stroke(bubble_rect, 5.0, (1.0, Color32::WHITE));

        // Current action text
        let current_node = runner
            .running_node()
            .map(|id| format!("Node {:?}", id))
            .unwrap_or_else(|| "Idle".to_string());

        painter.text(
            bubble_rect.center(),
            egui::Align2::CENTER_CENTER,
            current_node,
            egui::FontId::proportional(12.0),
            Color32::WHITE,
        );

        // Status indicator
        let status = runner
            .running_node()
            .and_then(|id| self.node_status.get(&id))
            .copied()
            .unwrap_or(BtStatus::Running);

        let status_color = status_color(status);
        painter.circle_filled(
            bubble_rect.right_top() + Vec2::new(-10.0, 10.0),
            6.0,
            status_color,
        );
    }

    /// Get status color for a node
    pub fn node_status_color(&self, node_id: NodeId) -> Color32 {
        self.node_status
            .get(&node_id)
            .copied()
            .map(status_color)
            .unwrap_or(Color32::GRAY)
    }

    /// Check if a node is in the current execution path
    pub fn is_in_execution_path(&self, node_id: NodeId) -> bool {
        self.execution_path.contains(&node_id)
    }
}

/// Status change record
#[derive(Debug, Clone, Copy)]
pub struct StatusChange {
    pub timestamp: std::time::Instant,
    pub node_id: NodeId,
    pub from: BtStatus,
    pub to: BtStatus,
}

/// Get color for a status
pub fn status_color(status: BtStatus) -> Color32 {
    match status {
        BtStatus::Success => Color32::from_rgb(100, 200, 100), // Green
        BtStatus::Failure => Color32::from_rgb(200, 100, 100), // Red
        BtStatus::Running => Color32::from_rgb(255, 200, 50),  // Yellow
    }
}

/// Get a darker variant for borders/backgrounds
pub fn status_color_dark(status: BtStatus) -> Color32 {
    match status {
        BtStatus::Success => Color32::from_rgb(50, 100, 50),
        BtStatus::Failure => Color32::from_rgb(100, 50, 50),
        BtStatus::Running => Color32::from_rgb(150, 120, 25),
    }
}

/// Draw a node with debug status overlay
pub fn draw_node_with_status(
    ui: &mut Ui,
    _node_rect: Rect,
    node_id: NodeId,
    debugger: Option<&BtDebugger>,
    f: impl FnOnce(&mut Ui),
) {
    // Draw the node content
    let mut frame = egui::Frame::group(ui.style())
        .fill(Color32::from_gray(40))
        .stroke(egui::Stroke::new(1.0, Color32::GRAY));

    // Apply debug styling if debugger is active
    if let Some(dbg) = debugger {
        let status_color = dbg.node_status_color(node_id);
        let is_in_path = dbg.is_in_execution_path(node_id);

        let stroke_width = if is_in_path { 3.0 } else { 1.0 };
        let stroke_color = if is_in_path {
            status_color
        } else {
            Color32::GRAY
        };

        frame = frame.stroke(egui::Stroke::new(stroke_width, stroke_color));

        if dbg.has_breakpoint(node_id) {
            // Add breakpoint indicator
            frame = frame.fill(status_color_dark(
                dbg.node_status
                    .get(&node_id)
                    .copied()
                    .unwrap_or(BtStatus::Running),
            ));
        }
    }

    frame.show(ui, |ui| {
        f(ui);

        // Draw status indicator dot
        if let Some(dbg) = debugger {
            let status = dbg.node_status.get(&node_id).copied();
            if let Some(s) = status {
                let dot_pos = ui.min_rect().left_top() + Vec2::new(4.0, 4.0);
                ui.painter().circle_filled(dot_pos, 4.0, status_color(s));
            }
        }
    });
}

/// Visual overlay for entity showing BT state
pub fn draw_entity_debug(
    painter: &egui::Painter,
    entity_pos: Pos2,
    current_action: &str,
    status: BtStatus,
) {
    // Draw text bubble with current action
    let bubble_size = Vec2::new(100.0, 24.0);
    let bubble_rect = Rect::from_center_size(entity_pos - Vec2::new(0.0, 30.0), bubble_size);

    // Background
    let bg_color = status_color_dark(status);
    painter.rect_filled(bubble_rect, 4.0, bg_color);
    painter.rect_stroke(bubble_rect, 4.0, (1.0, status_color(status)));

    // Text
    painter.text(
        bubble_rect.center(),
        egui::Align2::CENTER_CENTER,
        current_action,
        egui::FontId::proportional(10.0),
        Color32::WHITE,
    );

    // Status dot
    painter.circle_filled(
        bubble_rect.right_top() + Vec2::new(-8.0, 8.0),
        4.0,
        status_color(status),
    );
}

/// Debug statistics for behavior tree execution
#[derive(Debug, Default, Clone)]
pub struct BtDebugStats {
    pub total_ticks: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub running_count: u64,
    pub avg_tick_time_ms: f32,
}

impl BtDebugStats {
    pub fn record_tick(&mut self, status: BtStatus, duration_ms: f32) {
        self.total_ticks += 1;

        match status {
            BtStatus::Success => self.success_count += 1,
            BtStatus::Failure => self.failure_count += 1,
            BtStatus::Running => self.running_count += 1,
        }

        // Rolling average
        let alpha = 0.1;
        self.avg_tick_time_ms = self.avg_tick_time_ms * (1.0 - alpha) + duration_ms * alpha;
    }

    pub fn success_rate(&self) -> f32 {
        if self.total_ticks == 0 {
            0.0
        } else {
            self.success_count as f32 / self.total_ticks as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debugger_creation() {
        let debugger = BtDebugger::new();
        assert!(debugger.target_entity().is_none());
        assert!(!debugger.paused);
    }

    #[test]
    fn test_debugger_target() {
        let mut debugger = BtDebugger::new();
        let entity = Entity::DANGLING;

        debugger.set_target(entity);
        assert_eq!(debugger.target_entity(), Some(entity));

        debugger.clear_target();
        assert!(debugger.target_entity().is_none());
    }

    #[test]
    fn test_breakpoints() {
        let mut debugger = BtDebugger::new();
        let node_id = NodeId::new();

        assert!(!debugger.has_breakpoint(node_id));

        debugger.toggle_breakpoint(node_id);
        assert!(debugger.has_breakpoint(node_id));

        debugger.toggle_breakpoint(node_id);
        assert!(!debugger.has_breakpoint(node_id));
    }

    #[test]
    fn test_status_colors() {
        assert_eq!(
            status_color(BtStatus::Success),
            Color32::from_rgb(100, 200, 100)
        );
        assert_eq!(
            status_color(BtStatus::Failure),
            Color32::from_rgb(200, 100, 100)
        );
        assert_eq!(
            status_color(BtStatus::Running),
            Color32::from_rgb(255, 200, 50)
        );
    }

    #[test]
    fn test_debug_stats() {
        let mut stats = BtDebugStats::default();

        stats.record_tick(BtStatus::Success, 1.0);
        stats.record_tick(BtStatus::Success, 1.0);
        stats.record_tick(BtStatus::Failure, 1.0);

        assert_eq!(stats.total_ticks, 3);
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.failure_count, 1);
        assert!((stats.success_rate() - 0.6666667).abs() < 0.001);
    }
}
