//! Performance Profiler Panel
//!
//! Editor UI for viewing real-time performance metrics and budgets.

use std::collections::VecDeque;
use std::time::Duration;

/// Profiler panel UI state
pub struct ProfilerPanel {
    /// Whether panel is visible
    visible: bool,
    /// Selected tab
    selected_tab: ProfilerTab,
    /// Show advanced profiler
    advanced_mode: bool,
    /// Frame time history for graph
    frame_time_history: VecDeque<f32>,
    /// Memory history for graph
    memory_history: VecDeque<f32>,
    /// Graph max history
    max_history: usize,
    /// Last update time
    last_update: f32,
    /// Update interval (seconds)
    update_interval: f32,
}

/// Profiler tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfilerTab {
    Overview,
    Systems,
    Memory,
    Advanced,
}

/// Interface for profiler data
pub trait ProfilerInterface {
    /// Check if profiling is enabled
    fn is_enabled(&self) -> bool;
    /// Toggle profiling
    fn toggle(&mut self);
    /// Get current FPS
    fn fps(&self) -> f32;
    /// Get average frame time
    fn average_frame_time(&self, frames: usize) -> f32;
    /// Get frame time history
    fn frame_time_history(&self) -> Vec<f32>;
    /// Get entity count
    fn entity_count(&self) -> usize;
    /// Get particle count
    fn particle_count(&self) -> usize;
    /// Get budget status for a system
    fn budget_status(&self, system: &str) -> BudgetStatus;
    /// Get all budget statuses
    fn all_budget_statuses(&self) -> Vec<(&str, BudgetStatus)>;
    /// Get current memory usage (MB)
    fn memory_mb(&self) -> f32;
    /// Get Lua memory (MB)
    fn lua_memory_mb(&self) -> f32;
    /// Export to CSV
    fn export_csv(&self, path: &str) -> Result<(), String>;
}

/// Budget status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetStatus {
    Good,
    Warning,
    OverBudget,
    Unknown,
}

impl BudgetStatus {
    fn color(&self) -> egui::Color32 {
        match self {
            BudgetStatus::Good => egui::Color32::GREEN,
            BudgetStatus::Warning => egui::Color32::YELLOW,
            BudgetStatus::OverBudget => egui::Color32::RED,
            BudgetStatus::Unknown => egui::Color32::GRAY,
        }
    }

    fn text(&self) -> &'static str {
        match self {
            BudgetStatus::Good => "OK",
            BudgetStatus::Warning => "WARN",
            BudgetStatus::OverBudget => "OVER",
            BudgetStatus::Unknown => "---",
        }
    }
}

impl ProfilerPanel {
    /// Create a new profiler panel
    pub fn new() -> Self {
        Self {
            visible: false,
            selected_tab: ProfilerTab::Overview,
            advanced_mode: false,
            frame_time_history: VecDeque::with_capacity(120),
            memory_history: VecDeque::with_capacity(120),
            max_history: 120,
            last_update: 0.0,
            update_interval: 0.1, // Update 10 times per second
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

    /// Update panel data
    pub fn update(&mut self, dt: f32, profiler: &dyn ProfilerInterface) {
        self.last_update += dt;
        
        if self.last_update >= self.update_interval && profiler.is_enabled() {
            self.last_update = 0.0;
            
            // Update frame time history
            let frame_time = profiler.average_frame_time(1);
            self.frame_time_history.push_back(frame_time);
            if self.frame_time_history.len() > self.max_history {
                self.frame_time_history.pop_front();
            }
            
            // Update memory history
            let memory = profiler.memory_mb();
            self.memory_history.push_back(memory);
            if self.memory_history.len() > self.max_history {
                self.memory_history.pop_front();
            }
        }
    }

    /// Draw the profiler panel UI
    pub fn draw(&mut self, ctx: &egui::Context, profiler: &mut dyn ProfilerInterface) {
        if !self.visible {
            return;
        }

        let mut visible = self.visible;
        egui::Window::new("📊 Performance Profiler")
            .open(&mut visible)
            .resizable(true)
            .default_size([600.0, 450.0])
            .show(ctx, |ui| {
                self.draw_panel_content(ui, profiler);
            });
        self.visible = visible;
    }

    /// Draw panel content
    fn draw_panel_content(&mut self, ui: &mut egui::Ui, profiler: &mut dyn ProfilerInterface) {
        // Header with enable toggle
        ui.horizontal(|ui| {
            ui.heading("Performance Profiler");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut enabled = profiler.is_enabled();
                if ui.checkbox(&mut enabled, "Enabled").changed() {
                    profiler.toggle();
                }
                
                ui.checkbox(&mut self.advanced_mode, "Advanced");
            });
        });

        ui.separator();

        // Tab bar
        ui.horizontal(|ui| {
            self.tab_button(ui, "📈 Overview", ProfilerTab::Overview);
            self.tab_button(ui, "⚙️ Systems", ProfilerTab::Systems);
            self.tab_button(ui, "🧠 Memory", ProfilerTab::Memory);
            if self.advanced_mode {
                self.tab_button(ui, "🔬 Advanced", ProfilerTab::Advanced);
            }
        });

        ui.separator();

        // Tab content
        match self.selected_tab {
            ProfilerTab::Overview => self.draw_overview_tab(ui, profiler),
            ProfilerTab::Systems => self.draw_systems_tab(ui, profiler),
            ProfilerTab::Memory => self.draw_memory_tab(ui, profiler),
            ProfilerTab::Advanced => self.draw_advanced_tab(ui, profiler),
        }
    }

    /// Draw tab button
    fn tab_button(&mut self, ui: &mut egui::Ui, label: &str, tab: ProfilerTab) {
        let selected = self.selected_tab == tab;
        if ui.selectable_label(selected, label).clicked() {
            self.selected_tab = tab;
        }
    }

    /// Draw overview tab
    fn draw_overview_tab(&mut self, ui: &mut egui::Ui, profiler: &dyn ProfilerInterface) {
        if !profiler.is_enabled() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(egui::RichText::new("Profiler is disabled").size(18.0).weak());
                ui.label("Enable profiling to see performance metrics.");
            });
            return;
        }

        // FPS and frame time
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("FPS");
                let fps = profiler.fps();
                let fps_color = if fps >= 55.0 {
                    egui::Color32::GREEN
                } else if fps >= 30.0 {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::RED
                };
                ui.label(egui::RichText::new(format!("{:.1}", fps)).size(32.0).color(fps_color));
            });

            ui.vertical(|ui| {
                ui.label("Frame Time");
                let frame_time = profiler.average_frame_time(60);
                let ft_color = if frame_time <= 18.0 {
                    egui::Color32::GREEN
                } else if frame_time <= 33.0 {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::RED
                };
                ui.label(egui::RichText::new(format!("{:.2} ms", frame_time)).size(32.0).color(ft_color));
            });

            ui.vertical(|ui| {
                ui.label("Entities");
                ui.label(egui::RichText::new(profiler.entity_count().to_string()).size(32.0));
            });

            ui.vertical(|ui| {
                ui.label("Memory");
                ui.label(egui::RichText::new(format!("{:.1} MB", profiler.memory_mb())).size(24.0));
            });
        });

        ui.add_space(20.0);

        // Frame time graph
        ui.heading("Frame Time History");
        self.draw_frame_time_graph(ui);

        ui.add_space(20.0);

        // Budget status summary
        ui.heading("Budget Status");
        let statuses = profiler.all_budget_statuses();
        
        egui::Grid::new("budget_status_grid")
            .num_columns(3)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                for (system, status) in statuses {
                    ui.label(system);
                    ui.colored_label(status.color(), status.text());
                    ui.end_row();
                }
            });

        ui.add_space(20.0);

        // Export button
        if ui.button("📁 Export to CSV").clicked() {
            // Would open file dialog in real implementation
            // For now, just export to a default path
            match profiler.export_csv("profile_data.csv") {
                Ok(_) => {}
                Err(_) => {}
            }
        }
    }

    /// Draw frame time graph
    fn draw_frame_time_graph(&self, ui: &mut egui::Ui) {
        let available_width = ui.available_width();
        let height = 100.0;

        let (rect, _response) = ui.allocate_exact_size(
            egui::vec2(available_width, height),
            egui::Sense::hover(),
        );

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            // Background
            painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);

            if self.frame_time_history.len() >= 2 {
                let max_time = self.frame_time_history.iter().copied().fold(0.0, f32::max).max(33.0);
                
                // Draw 60fps line (16.67ms)
                let fps60_y = rect.bottom() - (16.67 / max_time) * rect.height();
                painter.line_segment(
                    [egui::pos2(rect.left(), fps60_y), egui::pos2(rect.right(), fps60_y)],
                    egui::Stroke::new(1.0, egui::Color32::GREEN.gamma_multiply(0.3)),
                );
                
                // Draw 30fps line (33.33ms)
                let fps30_y = rect.bottom() - (33.33 / max_time) * rect.height();
                painter.line_segment(
                    [egui::pos2(rect.left(), fps30_y), egui::pos2(rect.right(), fps30_y)],
                    egui::Stroke::new(1.0, egui::Color32::YELLOW.gamma_multiply(0.3)),
                );

                // Draw frame time line
                let points: Vec<_> = self.frame_time_history
                    .iter()
                    .enumerate()
                    .map(|(i, &time)| {
                        let x = rect.left() + (i as f32 / (self.frame_time_history.len() - 1) as f32) * rect.width();
                        let y = rect.bottom() - (time / max_time) * rect.height();
                        egui::pos2(x, y)
                    })
                    .collect();

                for i in 0..points.len().saturating_sub(1) {
                    let color = if self.frame_time_history[i] > 33.0 {
                        egui::Color32::RED
                    } else if self.frame_time_history[i] > 16.67 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::GREEN
                    };
                    painter.line_segment(
                        [points[i], points[i + 1]],
                        egui::Stroke::new(2.0, color),
                    );
                }
            }

            // Labels
            painter.text(
                rect.left_top() + egui::vec2(4.0, 4.0),
                egui::Align2::LEFT_TOP,
                "16ms (60fps)",
                egui::FontId::default(),
                egui::Color32::GREEN.gamma_multiply(0.7),
            );
        }
    }

    /// Draw systems tab
    fn draw_systems_tab(&mut self, ui: &mut egui::Ui, profiler: &dyn ProfilerInterface) {
        if !profiler.is_enabled() {
            ui.label("Enable profiling to see system metrics.");
            return;
        }

        ui.heading("System Performance");
        ui.label("Per-system timing and budget compliance.");
        
        ui.add_space(10.0);

        // System breakdown would go here
        // For now, show budget statuses
        let statuses = profiler.all_budget_statuses();
        
        for (system, status) in statuses {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(system);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(status.color(), status.text());
                    });
                });
            });
        }
    }

    /// Draw memory tab
    fn draw_memory_tab(&mut self, ui: &mut egui::Ui, profiler: &dyn ProfilerInterface) {
        if !profiler.is_enabled() {
            ui.label("Enable profiling to see memory metrics.");
            return;
        }

        ui.heading("Memory Usage");
        ui.add_space(10.0);

        // Memory stats
        egui::Grid::new("memory_stats_grid")
            .num_columns(2)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                ui.label("Total Memory:");
                ui.label(format!("{:.2} MB", profiler.memory_mb()));
                ui.end_row();

                ui.label("Lua Memory:");
                ui.label(format!("{:.2} MB", profiler.lua_memory_mb()));
                ui.end_row();
            });

        ui.add_space(20.0);

        // Memory graph
        ui.heading("Memory History");
        self.draw_memory_graph(ui);
    }

    /// Draw memory graph
    fn draw_memory_graph(&self, ui: &mut egui::Ui) {
        let available_width = ui.available_width();
        let height = 100.0;

        let (rect, _response) = ui.allocate_exact_size(
            egui::vec2(available_width, height),
            egui::Sense::hover(),
        );

        if ui.is_rect_visible(rect) && self.memory_history.len() >= 2 {
            let painter = ui.painter();

            // Background
            painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);

            let max_mem = self.memory_history.iter().copied().fold(0.0, f32::max).max(100.0);

            // Draw memory line
            let points: Vec<_> = self.memory_history
                .iter()
                .enumerate()
                .map(|(i, &mem)| {
                    let x = rect.left() + (i as f32 / (self.memory_history.len() - 1) as f32) * rect.width();
                    let y = rect.bottom() - (mem / max_mem) * rect.height();
                    egui::pos2(x, y)
                })
                .collect();

            for i in 0..points.len().saturating_sub(1) {
                painter.line_segment(
                    [points[i], points[i + 1]],
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255)),
                );
            }

            // Label
            painter.text(
                rect.left_top() + egui::vec2(4.0, 4.0),
                egui::Align2::LEFT_TOP,
                format!("Peak: {:.1} MB", max_mem),
                egui::FontId::default(),
                ui.visuals().text_color(),
            );
        }
    }

    /// Draw advanced tab
    fn draw_advanced_tab(&mut self, ui: &mut egui::Ui, _profiler: &dyn ProfilerInterface) {
        ui.heading("Advanced Metrics");
        ui.label("Hierarchical profiling and GPU timing.");
        
        ui.add_space(10.0);
        
        ui.label("(Advanced profiling data would appear here)");
        ui.label("- Hierarchical call tree");
        ui.label("- GPU timing");
        ui.label("- 99th percentile frame times");
        ui.label("- System counters");
    }
}

impl Default for ProfilerPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProfiler {
        enabled: bool,
        fps: f32,
        frame_time: f32,
        entity_count: usize,
        memory_mb: f32,
    }

    impl MockProfiler {
        fn new() -> Self {
            Self {
                enabled: true,
                fps: 60.0,
                frame_time: 16.67,
                entity_count: 100,
                memory_mb: 150.0,
            }
        }
    }

    impl ProfilerInterface for MockProfiler {
        fn is_enabled(&self) -> bool {
            self.enabled
        }

        fn toggle(&mut self) {
            self.enabled = !self.enabled;
        }

        fn fps(&self) -> f32 {
            self.fps
        }

        fn average_frame_time(&self, _frames: usize) -> f32 {
            self.frame_time
        }

        fn frame_time_history(&self) -> Vec<f32> {
            vec![16.0, 17.0, 16.5, 17.2, 16.8]
        }

        fn entity_count(&self) -> usize {
            self.entity_count
        }

        fn particle_count(&self) -> usize {
            500
        }

        fn budget_status(&self, _system: &str) -> BudgetStatus {
            BudgetStatus::Good
        }

        fn all_budget_statuses(&self) -> Vec<(&str, BudgetStatus)> {
            vec![
                ("frame", BudgetStatus::Good),
                ("render", BudgetStatus::Good),
                ("simulation", BudgetStatus::Warning),
            ]
        }

        fn memory_mb(&self) -> f32 {
            self.memory_mb
        }

        fn lua_memory_mb(&self) -> f32 {
            10.0
        }

        fn export_csv(&self, _path: &str) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn test_panel_creation() {
        let panel = ProfilerPanel::new();
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_panel_toggle() {
        let mut panel = ProfilerPanel::new();
        assert!(!panel.is_visible());

        panel.toggle();
        assert!(panel.is_visible());

        panel.toggle();
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_budget_status() {
        assert_eq!(BudgetStatus::Good.text(), "OK");
        assert_eq!(BudgetStatus::Warning.text(), "WARN");
        assert_eq!(BudgetStatus::OverBudget.text(), "OVER");
        assert_eq!(BudgetStatus::Unknown.text(), "---");
    }

    #[test]
    fn test_mock_profiler() {
        let mut profiler = MockProfiler::new();
        
        assert!(profiler.is_enabled());
        assert_eq!(profiler.fps(), 60.0);
        assert_eq!(profiler.entity_count(), 100);
        
        profiler.toggle();
        assert!(!profiler.is_enabled());
    }
}
