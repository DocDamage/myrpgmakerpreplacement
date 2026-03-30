//! Enhanced Performance Profiler Panel
//!
//! Comprehensive profiler UI with:
//! - Budget configuration with sliders
//! - Detailed breakdown tree view
//! - Historical time-series graphs
//! - Export capabilities (CSV/JSON)
//! - Advanced analytics (percentiles, variance, bottlenecks)
//! - Entity counts per system
//! - Memory profiling by category

use dde_core::profiler::BudgetStatus;
use dde_core::profiler::enhanced::{
    BudgetConfiguration, EnhancedFrameMetrics, EnhancedProfiler,
    EntityCounts, ExportRange, MemoryBreakdown, OptimizationSuggestion,
    SectionMetrics, SuggestionCategory, TimeSeriesMetric,
};
use std::collections::{HashMap, VecDeque};

/// Enhanced profiler panel UI state
pub struct ProfilerPanelEnhanced {
    /// Whether panel is visible
    visible: bool,
    /// Selected tab
    selected_tab: ProfilerTab,
    /// Frame time history for graph (cached)
    frame_time_history: VecDeque<f32>,
    /// FPS history for graph
    fps_history: VecDeque<f32>,
    /// Memory history for graph
    memory_history: VecDeque<f32>,
    /// Entity count history
    entity_history: VecDeque<usize>,
    /// Graph max history
    max_history: usize,
    /// Last update time
    last_update: f32,
    /// Update interval (seconds)
    update_interval: f32,
    /// Expanded sections in tree view
    expanded_sections: HashMap<String, bool>,
    /// Sort mode for section tree
    section_sort: SectionSort,
    /// Export range selection
    export_range: ExportRangeSelection,
    /// Custom export frame count
    custom_export_frames: usize,
    /// Graph time range (seconds)
    graph_time_range: f32,
    /// Show paused overlay
    show_paused: bool,
    /// Selected memory category filter
    memory_filter: MemoryFilter,
    /// Selected entity category filter
    entity_filter: EntityFilter,
    /// Show optimization suggestions
    show_suggestions: bool,
    /// Budget config being edited
    budget_edit: BudgetConfiguration,
    /// Whether budget editor is open
    budget_editor_open: bool,
    /// Scroll offset for tree view
    tree_scroll_offset: f32,
}

/// Profiler tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfilerTab {
    Overview,
    Budget,
    Breakdown,
    Graphs,
    Entities,
    Memory,
    Analytics,
    Export,
}

/// Section sort mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SectionSort {
    Name,
    CurrentTime,
    AverageTime,
    MaxTime,
    BudgetStatus,
}

/// Export range selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportRangeSelection {
    Last100,
    Last500,
    Last1000,
    All,
    Custom,
}

/// Memory category filter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemoryFilter {
    All,
    ECS,
    Assets,
    Scripts,
    DBCache,
}

/// Entity category filter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityFilter {
    All,
    NPCs,
    Items,
    Projectiles,
    Effects,
}

impl ProfilerPanelEnhanced {
    /// Create a new enhanced profiler panel
    pub fn new() -> Self {
        Self {
            visible: false,
            selected_tab: ProfilerTab::Overview,
            frame_time_history: VecDeque::with_capacity(3600),
            fps_history: VecDeque::with_capacity(3600),
            memory_history: VecDeque::with_capacity(3600),
            entity_history: VecDeque::with_capacity(3600),
            max_history: 3600,
            last_update: 0.0,
            update_interval: 0.05, // 20 updates per second
            expanded_sections: HashMap::new(),
            section_sort: SectionSort::AverageTime,
            export_range: ExportRangeSelection::Last500,
            custom_export_frames: 1000,
            graph_time_range: 60.0, // 60 seconds
            show_paused: false,
            memory_filter: MemoryFilter::All,
            entity_filter: EntityFilter::All,
            show_suggestions: true,
            budget_edit: BudgetConfiguration::default(),
            budget_editor_open: false,
            tree_scroll_offset: 0.0,
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

    /// Update panel data from profiler
    pub fn update(&mut self, dt: f32, profiler: &EnhancedProfiler) {
        self.last_update += dt;

        if self.last_update >= self.update_interval && profiler.is_enabled() && profiler.is_recording()
        {
            self.last_update = 0.0;
            self.show_paused = false;

            // Update histories
            if let Some(latest) = profiler.frame_history().back() {
                self.frame_time_history.push_back(latest.total_time_ms as f32);
                self.fps_history.push_back(latest.fps);
                self.memory_history.push_back(latest.memory.total_mb() as f32);
                self.entity_history.push_back(latest.entities.total);

                // Trim histories
                while self.frame_time_history.len() > self.max_history {
                    self.frame_time_history.pop_front();
                }
                while self.fps_history.len() > self.max_history {
                    self.fps_history.pop_front();
                }
                while self.memory_history.len() > self.max_history {
                    self.memory_history.pop_front();
                }
                while self.entity_history.len() > self.max_history {
                    self.entity_history.pop_front();
                }
            }
        } else if !profiler.is_recording() {
            self.show_paused = true;
        }
    }

    /// Draw the profiler panel UI
    pub fn draw(&mut self, ctx: &egui::Context, profiler: &mut EnhancedProfiler) {
        if !self.visible {
            return;
        }

        let mut visible = self.visible;
        
        egui::Window::new("📊 Performance Profiler (Enhanced)")
            .open(&mut visible)
            .resizable(true)
            .default_size([900.0, 650.0])
            .min_size([700.0, 400.0])
            .show(ctx, |ui| {
                self.draw_panel_content(ui, profiler);
            });
            
        self.visible = visible;
    }

    /// Draw panel content
    fn draw_panel_content(&mut self, ui: &mut egui::Ui, profiler: &mut EnhancedProfiler) {
        // Header with controls
        self.draw_header(ui, profiler);
        ui.separator();

        // Tab bar
        self.draw_tab_bar(ui);
        ui.separator();

        // Tab content
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                match self.selected_tab {
                    ProfilerTab::Overview => self.draw_overview_tab(ui, profiler),
                    ProfilerTab::Budget => self.draw_budget_tab(ui, profiler),
                    ProfilerTab::Breakdown => self.draw_breakdown_tab(ui, profiler),
                    ProfilerTab::Graphs => self.draw_graphs_tab(ui),
                    ProfilerTab::Entities => self.draw_entities_tab(ui, profiler),
                    ProfilerTab::Memory => self.draw_memory_tab(ui, profiler),
                    ProfilerTab::Analytics => self.draw_analytics_tab(ui, profiler),
                    ProfilerTab::Export => self.draw_export_tab(ui, profiler),
                }
            });
    }

    /// Draw header with controls
    fn draw_header(&mut self, ui: &mut egui::Ui, profiler: &mut EnhancedProfiler) {
        ui.horizontal(|ui| {
            ui.heading("Performance Profiler");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Recording controls
                let recording = profiler.is_recording();
                let button_text = if recording { "⏸ Pause" } else { "▶ Resume" };
                if ui.button(button_text).clicked() {
                    profiler.toggle_pause();
                }

                // Enable toggle
                let mut enabled = profiler.is_enabled();
                if ui.checkbox(&mut enabled, "Enabled").changed() {
                    profiler.toggle();
                }
            });
        });

        // Status bar
        ui.horizontal(|ui| {
            if profiler.is_enabled() {
                ui.colored_label(egui::Color32::GREEN, "● Profiling Active");
            } else {
                ui.colored_label(egui::Color32::RED, "○ Profiling Disabled");
            }

            if !profiler.is_recording() && profiler.is_enabled() {
                ui.add_space(10.0);
                ui.colored_label(egui::Color32::YELLOW, "⏸ Recording Paused");
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(latest) = profiler.frame_history().back() {
                    ui.label(format!("Frame #{} | {:.1} FPS", latest.frame, latest.fps));
                }
            });
        });
    }

    /// Draw tab bar
    fn draw_tab_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            self.tab_button(ui, "📈 Overview", ProfilerTab::Overview);
            self.tab_button(ui, "⚙️ Budget", ProfilerTab::Budget);
            self.tab_button(ui, "🔍 Breakdown", ProfilerTab::Breakdown);
            self.tab_button(ui, "📊 Graphs", ProfilerTab::Graphs);
            self.tab_button(ui, "👥 Entities", ProfilerTab::Entities);
            self.tab_button(ui, "🧠 Memory", ProfilerTab::Memory);
            self.tab_button(ui, "🔬 Analytics", ProfilerTab::Analytics);
            self.tab_button(ui, "💾 Export", ProfilerTab::Export);
        });
    }

    /// Draw tab button
    fn tab_button(&mut self, ui: &mut egui::Ui, label: &str, tab: ProfilerTab) {
        let selected = self.selected_tab == tab;
        if ui.selectable_label(selected, label).clicked() {
            self.selected_tab = tab;
        }
    }

    // ========================================================================
    // OVERVIEW TAB
    // ========================================================================

    /// Draw overview tab
    fn draw_overview_tab(&mut self, ui: &mut egui::Ui, profiler: &EnhancedProfiler) {
        if !profiler.is_enabled() {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label(
                    egui::RichText::new("Profiler is disabled")
                        .size(18.0)
                        .weak(),
                );
                ui.label("Enable profiling to see performance metrics.");
            });
            return;
        }

        // Main metrics cards
        ui.horizontal(|ui| {
            self.metric_card(ui, "FPS", |ui| {
                if let Some(latest) = profiler.frame_history().back() {
                    let fps = latest.fps;
                    let color = if fps >= 55.0 {
                        egui::Color32::GREEN
                    } else if fps >= 30.0 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::RED
                    };
                    ui.label(egui::RichText::new(format!("{:.1}", fps)).size(32.0).color(color));
                }
            });

            self.metric_card(ui, "Frame Time", |ui| {
                if let Some(latest) = profiler.frame_history().back() {
                    let ft = latest.total_time_ms;
                    let color = if ft <= 18.0 {
                        egui::Color32::GREEN
                    } else if ft <= 33.0 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::RED
                    };
                    ui.label(
                        egui::RichText::new(format!("{:.2} ms", ft))
                            .size(32.0)
                            .color(color),
                    );
                }
            });

            self.metric_card(ui, "Entities", |ui| {
                let count = profiler.entity_counts().total;
                ui.label(egui::RichText::new(format!("{}", count)).size(32.0));
            });

            self.metric_card(ui, "Memory", |ui| {
                if let Some(memory) = profiler.memory_history().back() {
                    let mb = memory.total_mb();
                    ui.label(egui::RichText::new(format!("{:.1} MB", mb)).size(24.0));
                }
            });
        });

        ui.add_space(20.0);

        // Mini frame time graph
        ui.heading("Frame Time History");
        self.draw_time_series_graph(
            ui,
            &self.frame_time_history.iter().copied().collect::<Vec<_>>(),
            "ms",
            Some(16.67),
            Some(33.33),
        );

        ui.add_space(20.0);

        // Quick budget status
        ui.heading("Budget Status");
        self.draw_budget_status_grid(ui, profiler);

        // Optimization suggestions preview
        if self.show_suggestions {
            ui.add_space(20.0);
            self.draw_suggestions_preview(ui, profiler);
        }
    }

    /// Draw a metric card
    fn metric_card(&self, ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
        egui::Frame::group(ui.style())
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.set_min_width(120.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(title).weak().size(12.0));
                    content(ui);
                });
            });
    }

    /// Draw budget status grid
    fn draw_budget_status_grid(&self, ui: &mut egui::Ui, profiler: &EnhancedProfiler) {
        let config = profiler.budget_config();

        egui::Grid::new("budget_status_grid")
            .num_columns(4)
            .spacing([20.0, 8.0])
            .show(ui, |ui| {
                // Headers
                ui.label(egui::RichText::new("System").strong());
                ui.label(egui::RichText::new("Current").strong());
                ui.label(egui::RichText::new("Budget").strong());
                ui.label(egui::RichText::new("Status").strong());
                ui.end_row();

                // Frame
                if let Some(latest) = profiler.frame_history().back() {
                    let ft = latest.total_time_ms;
                    let status = get_budget_status(ft, config.frame_budget_ms);
                    ui.label("Frame");
                    ui.label(format!("{:.2} ms", ft));
                    ui.label(format!("{:.1} ms", config.frame_budget_ms));
                    ui.colored_label(budget_status_color(&status), budget_budget_status_text(&status));
                    ui.end_row();
                }

                // Sections
                let budgets = [
                    ("Simulation", config.simulation_budget_ms),
                    ("Render", config.render_budget_ms),
                    ("Audio", config.audio_budget_ms),
                    ("Script", config.script_budget_ms),
                    ("UI", config.ui_budget_ms),
                    ("Pathfinding", config.pathfinding_budget_ms),
                ];

                for (name, budget) in budgets {
                    let actual = profiler
                        .section_timings()
                        .get(&name.to_lowercase())
                        .map(|t| t.average_ms())
                        .unwrap_or(0.0);
                    let status = get_budget_status(actual, budget);

                    ui.label(name);
                    ui.label(format!("{:.2} ms", actual));
                    ui.label(format!("{:.1} ms", budget));
                    ui.colored_label(budget_status_color(&status), status_text(&status));
                    ui.end_row();
                }
            });
    }

    /// Draw suggestions preview
    fn draw_suggestions_preview(&self, ui: &mut egui::Ui, profiler: &EnhancedProfiler) {
        let suggestions = profiler.optimization_suggestions();

        if suggestions.is_empty() {
            ui.label(egui::RichText::new("✓ No optimization suggestions").color(egui::Color32::GREEN));
            return;
        }

        ui.heading("💡 Optimization Suggestions");

        for (i, suggestion) in suggestions.iter().take(3).enumerate() {
            let color = match suggestion.priority {
                p if p > 0.7 => egui::Color32::RED,
                p if p > 0.4 => egui::Color32::YELLOW,
                _ => egui::Color32::GREEN,
            };

            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(30, 30, 30))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.colored_label(color, format!("{}.", i + 1));
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&suggestion.title).strong());
                            ui.label(&suggestion.description);
                        });
                    });
                });
        }
    }

    // ========================================================================
    // BUDGET TAB
    // ========================================================================

    /// Draw budget configuration tab
    fn draw_budget_tab(&mut self, ui: &mut egui::Ui, profiler: &mut EnhancedProfiler) {
        ui.heading("Budget Configuration");
        ui.label("Configure performance budgets and warning thresholds.");
        ui.add_space(15.0);

        // Target FPS slider
        ui.horizontal(|ui| {
            ui.label("Target FPS:");
            let mut target_fps = self.budget_edit.target_fps;
            if ui
                .add(egui::Slider::new(&mut target_fps, 30.0..=144.0).text("fps"))
                .changed()
            {
                self.budget_edit.target_fps = target_fps;
                self.budget_edit.frame_budget_ms = 1000.0 / target_fps as f64;
            }
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Section budgets
        ui.heading("Section Budgets");
        
        ui.horizontal(|ui| {
            ui.label("Simulation:");
            ui.add(
                egui::Slider::new(&mut self.budget_edit.simulation_budget_ms, 0.5..=10.0)
                    .text("ms"),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Render:");
            ui.add(
                egui::Slider::new(&mut self.budget_edit.render_budget_ms, 0.5..=10.0).text("ms"),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Audio:");
            ui.add(
                egui::Slider::new(&mut self.budget_edit.audio_budget_ms, 0.1..=5.0).text("ms"),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Script:");
            ui.add(
                egui::Slider::new(&mut self.budget_edit.script_budget_ms, 0.5..=10.0).text("ms"),
            );
        });

        ui.horizontal(|ui| {
            ui.label("UI:");
            ui.add(egui::Slider::new(&mut self.budget_edit.ui_budget_ms, 0.5..=10.0).text("ms"));
        });

        ui.horizontal(|ui| {
            ui.label("Pathfinding:");
            ui.add(
                egui::Slider::new(&mut self.budget_edit.pathfinding_budget_ms, 0.1..=5.0)
                    .text("ms"),
            );
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Thresholds
        ui.heading("Warning Thresholds");

        ui.horizontal(|ui| {
            ui.label("Warning:");
            ui.add(
                egui::Slider::new(&mut self.budget_edit.warning_threshold, 0.5..=0.95)
                    .text("ratio"),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Critical:");
            ui.add(
                egui::Slider::new(&mut self.budget_edit.critical_threshold, 0.8..=2.0)
                    .text("ratio"),
            );
        });

        ui.add_space(20.0);

        // Apply button
        if ui.button("✓ Apply Budget Configuration").clicked() {
            profiler.set_budget_config(self.budget_edit);
        }

        ui.add_space(10.0);

        // Reset button
        if ui.button("↺ Reset to Defaults").clicked() {
            self.budget_edit = BudgetConfiguration::default();
            profiler.set_budget_config(self.budget_edit);
        }
    }

    // ========================================================================
    // BREAKDOWN TAB
    // ========================================================================

    /// Draw detailed breakdown tab
    fn draw_breakdown_tab(&mut self, ui: &mut egui::Ui, profiler: &EnhancedProfiler) {
        ui.heading("Detailed Section Breakdown");

        // Sort controls
        ui.horizontal(|ui| {
            ui.label("Sort by:");
            ui.selectable_value(&mut self.section_sort, SectionSort::Name, "Name");
            ui.selectable_value(&mut self.section_sort, SectionSort::CurrentTime, "Current");
            ui.selectable_value(&mut self.section_sort, SectionSort::AverageTime, "Average");
            ui.selectable_value(&mut self.section_sort, SectionSort::MaxTime, "Max");
            ui.selectable_value(&mut self.section_sort, SectionSort::BudgetStatus, "Status");
        });

        ui.add_space(10.0);

        // Section tree
        let mut sections: Vec<_> = profiler
            .section_timings()
            .values()
            .map(|t| t.to_metrics(self.get_budget_for_section(&t.name)))
            .collect();

        // Sort
        match self.section_sort {
            SectionSort::Name => sections.sort_by(|a, b| a.name.cmp(&b.name)),
            SectionSort::CurrentTime => {
                sections.sort_by(|a, b| b.current_ms.partial_cmp(&a.current_ms).unwrap())
            }
            SectionSort::AverageTime => {
                sections.sort_by(|a, b| b.average_ms.partial_cmp(&a.average_ms).unwrap())
            }
            SectionSort::MaxTime => {
                sections.sort_by(|a, b| b.max_ms.partial_cmp(&a.max_ms).unwrap())
            }
            SectionSort::BudgetStatus => {
                sections.sort_by(|a, b| b.budget_status.cmp(&a.budget_status))
            }
        }

        // Headers
        egui::Grid::new("breakdown_headers")
            .num_columns(6)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Section").strong());
                ui.label(egui::RichText::new("Current").strong());
                ui.label(egui::RichText::new("Average").strong());
                ui.label(egui::RichText::new("Min").strong());
                ui.label(egui::RichText::new("Max").strong());
                ui.label(egui::RichText::new("Status").strong());
                ui.end_row();
            });

        ui.separator();

        // Tree rows
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                for section in sections {
                    self.draw_section_row(ui, &section, 0);
                }
            });
    }

    /// Draw a section tree row
    fn draw_section_row(&mut self, ui: &mut egui::Ui, section: &SectionMetrics, depth: usize) {
        let id = &section.name;
        let is_expanded = self.expanded_sections.get(id).copied().unwrap_or(false);
        let has_children = !section.children.is_empty();

        egui::Grid::new(format!("section_row_{}", id))
            .num_columns(6)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                // Section name with expand/collapse
                ui.horizontal(|ui| {
                    ui.add_space(depth as f32 * 20.0);

                    if has_children {
                        let button_text = if is_expanded { "▼" } else { "▶" };
                        if ui.button(button_text).clicked() {
                            self.expanded_sections
                                .insert(id.clone(), !is_expanded);
                        }
                    } else {
                        ui.add_space(24.0);
                    }

                    ui.label(&section.name);
                });

                // Metrics
                ui.label(format!("{:.2} ms", section.current_ms));
                ui.label(format!("{:.2} ms", section.average_ms));
                ui.label(format!("{:.2} ms", section.min_ms));
                ui.label(format!("{:.2} ms", section.max_ms));

                // Status
        
                ui.colored_label(budget_status_color(&section.budget_status), budget_status_text(&section.budget_status));

                ui.end_row();
            });

        // Children
        if is_expanded {
            for child in &section.children {
                self.draw_section_row(ui, child, depth + 1);
            }
        }
    }

    // ========================================================================
    // GRAPHS TAB
    // ========================================================================

    /// Draw graphs tab
    fn draw_graphs_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Historical Graphs");

        // Time range selection
        ui.horizontal(|ui| {
            ui.label("Time Range:");
            ui.selectable_value(&mut self.graph_time_range, 10.0, "10s");
            ui.selectable_value(&mut self.graph_time_range, 30.0, "30s");
            ui.selectable_value(&mut self.graph_time_range, 60.0, "1m");
            ui.selectable_value(&mut self.graph_time_range, 300.0, "5m");
        });

        if self.show_paused {
            ui.colored_label(egui::Color32::YELLOW, "⏸ Recording Paused - Graphs static");
        }

        ui.add_space(10.0);

        // Calculate frame count from time range (at 60fps)
        let frame_count = (self.graph_time_range * 60.0) as usize;

        // FPS Graph
        ui.heading("FPS Over Time");
        let fps_data: Vec<f32> = self
            .fps_history
            .iter()
            .rev()
            .take(frame_count)
            .copied()
            .collect();
        self.draw_time_series_graph(ui, &fps_data, "FPS", Some(30.0), Some(60.0));

        ui.add_space(20.0);

        // Frame Time Graph
        ui.heading("Frame Time Over Time");
        let ft_data: Vec<f32> = self
            .frame_time_history
            .iter()
            .rev()
            .take(frame_count)
            .copied()
            .collect();
        self.draw_time_series_graph(ui, &ft_data, "ms", Some(16.67), Some(33.33));

        ui.add_space(20.0);

        // Memory Graph
        ui.heading("Memory Usage Over Time");
        let mem_data: Vec<f32> = self
            .memory_history
            .iter()
            .rev()
            .take(frame_count)
            .copied()
            .collect();
        self.draw_time_series_graph(ui, &mem_data, "MB", None, None);
    }

    /// Draw a time series graph
    fn draw_time_series_graph(
        &self,
        ui: &mut egui::Ui,
        data: &[f32],
        unit: &str,
        threshold1: Option<f32>,
        threshold2: Option<f32>,
    ) {
        let available_width = ui.available_width();
        let height = 120.0;

        let (rect, _response) =
            ui.allocate_exact_size(egui::vec2(available_width, height), egui::Sense::hover());

        if !ui.is_rect_visible(rect) || data.len() < 2 {
            return;
        }

        let painter = ui.painter();

        // Background
        painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);

        let max_val = data.iter().copied().fold(0.0f32, f32::max).max(0.1);
        let min_val = data.iter().copied().fold(f32::MAX, f32::min).min(max_val);
        let range = max_val - min_val;

        // Draw threshold lines
        if let Some(t1) = threshold1 {
            let y = rect.bottom() - ((t1 - min_val) / range) * rect.height();
            if y >= rect.top() && y <= rect.bottom() {
                painter.line_segment(
                    [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                    egui::Stroke::new(1.0, egui::Color32::YELLOW.gamma_multiply(0.4)),
                );
            }
        }

        if let Some(t2) = threshold2 {
            let y = rect.bottom() - ((t2 - min_val) / range) * rect.height();
            if y >= rect.top() && y <= rect.bottom() {
                painter.line_segment(
                    [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                    egui::Stroke::new(1.0, egui::Color32::GREEN.gamma_multiply(0.4)),
                );
            }
        }

        // Draw data line
        let points: Vec<_> = data
            .iter()
            .enumerate()
            .map(|(i, &val)| {
                let x = rect.left()
                    + (i as f32 / (data.len().saturating_sub(1).max(1)) as f32) * rect.width();
                let y = rect.bottom() - ((val - min_val) / range) * rect.height();
                egui::pos2(x, y)
            })
            .collect();

        // Draw gradient line
        for i in 0..points.len().saturating_sub(1) {
            let color = if data[i] > threshold2.unwrap_or(f32::MAX) {
                egui::Color32::RED
            } else if data[i] > threshold1.unwrap_or(f32::MAX) {
                egui::Color32::YELLOW
            } else {
                egui::Color32::GREEN
            };
            painter.line_segment([points[i], points[i + 1]], egui::Stroke::new(2.0, color));
        }

        // Labels
        painter.text(
            rect.left_top() + egui::vec2(8.0, 4.0),
            egui::Align2::LEFT_TOP,
            format!("Max: {:.1} {}", max_val, unit),
            egui::FontId::default(),
            ui.visuals().text_color(),
        );

        painter.text(
            rect.left_bottom() + egui::vec2(8.0, -4.0),
            egui::Align2::LEFT_BOTTOM,
            format!("Min: {:.1} {}", min_val, unit),
            egui::FontId::default(),
            ui.visuals().text_color().gamma_multiply(0.7),
        );
    }

    // ========================================================================
    // ENTITIES TAB
    // ========================================================================

    /// Draw entities tab
    fn draw_entities_tab(&mut self, ui: &mut egui::Ui, profiler: &EnhancedProfiler) {
        ui.heading("Entity Counts");

        // Filter
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.selectable_value(&mut self.entity_filter, EntityFilter::All, "All");
            ui.selectable_value(&mut self.entity_filter, EntityFilter::NPCs, "NPCs");
            ui.selectable_value(&mut self.entity_filter, EntityFilter::Items, "Items");
            ui.selectable_value(&mut self.entity_filter, EntityFilter::Projectiles, "Projectiles");
            ui.selectable_value(&mut self.entity_filter, EntityFilter::Effects, "Effects");
        });

        ui.add_space(15.0);

        let counts = profiler.entity_counts();

        // Entity cards
        ui.horizontal_wrapped(|ui| {
            self.entity_card(ui, "Total", counts.total, egui::Color32::WHITE);
            self.entity_card(ui, "NPCs", counts.npcs, egui::Color32::from_rgb(100, 200, 255));
            self.entity_card(ui, "Items", counts.items, egui::Color32::from_rgb(255, 200, 100));
            self.entity_card(
                ui,
                "Projectiles",
                counts.projectiles,
                egui::Color32::from_rgb(255, 100, 100),
            );
            self.entity_card(
                ui,
                "Effects",
                counts.effects,
                egui::Color32::from_rgb(200, 100, 255),
            );
            self.entity_card(
                ui,
                "Players",
                counts.players,
                egui::Color32::from_rgb(100, 255, 100),
            );
            self.entity_card(ui, "Static", counts.static_entities, egui::Color32::GRAY);
        });

        ui.add_space(20.0);

        // Entity count graph
        ui.heading("Entity Count Over Time");

        let filtered_data: Vec<f32> = match self.entity_filter {
            EntityFilter::All => self.entity_history.iter().map(|&e| e as f32).collect(),
            _ => {
                // For filtered view, we'd need historical per-category data
                // For now, show current proportions
                self.entity_history.iter().map(|&e| e as f32).collect()
            }
        };

        self.draw_time_series_graph(&mut ui.child_ui(ui.available_rect_before_wrap(), *ui.layout()), &filtered_data, "entities", None, None);
    }

    /// Draw an entity count card
    fn entity_card(&self, ui: &mut egui::Ui, name: &str, count: usize, color: egui::Color32) {
        egui::Frame::group(ui.style())
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.set_min_width(100.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(name).size(12.0).weak());
                    ui.label(egui::RichText::new(format!("{}", count)).size(28.0).color(color));
                });
            });
    }

    // ========================================================================
    // MEMORY TAB
    // ========================================================================

    /// Draw memory tab
    fn draw_memory_tab(&mut self, ui: &mut egui::Ui, profiler: &EnhancedProfiler) {
        ui.heading("Memory Profiling");

        // Filter
        ui.horizontal(|ui| {
            ui.label("View:");
            ui.selectable_value(&mut self.memory_filter, MemoryFilter::All, "All");
            ui.selectable_value(&mut self.memory_filter, MemoryFilter::ECS, "ECS");
            ui.selectable_value(&mut self.memory_filter, MemoryFilter::Assets, "Assets");
            ui.selectable_value(&mut self.memory_filter, MemoryFilter::Scripts, "Scripts");
            ui.selectable_value(&mut self.memory_filter, MemoryFilter::DBCache, "DB Cache");
        });

        ui.add_space(15.0);

        if let Some(memory) = profiler.memory_history().back() {
            // Memory breakdown bars
            self.draw_memory_bar(ui, "Total RAM", memory.total_mb(), memory.total_mb(), egui::Color32::WHITE);
            self.draw_memory_bar(ui, "ECS World", memory.ecs_mb(), memory.total_mb(), egui::Color32::from_rgb(100, 200, 255));
            self.draw_memory_bar(ui, "Assets", memory.assets_mb(), memory.total_mb(), egui::Color32::from_rgb(255, 200, 100));
            self.draw_memory_bar(ui, "Textures", memory.textures_mb(), memory.total_mb(), egui::Color32::from_rgb(255, 150, 100));
            self.draw_memory_bar(ui, "Audio", memory.audio_mb(), memory.total_mb(), egui::Color32::from_rgb(100, 255, 150));
            self.draw_memory_bar(ui, "Scripts", memory.scripts_mb(), memory.total_mb(), egui::Color32::from_rgb(200, 100, 255));
            self.draw_memory_bar(ui, "DB Cache", memory.db_cache_mb(), memory.total_mb(), egui::Color32::from_rgb(255, 100, 200));

            // VRAM if available
            if let Some(vram) = memory.vram_mb() {
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
                self.draw_memory_bar(ui, "VRAM", vram, vram * 1.5, egui::Color32::from_rgb(255, 100, 100));
            }
        }

        ui.add_space(20.0);

        // Memory history graph
        ui.heading("Memory History");
        self.draw_time_series_graph(ui, &self.memory_history.iter().copied().collect::<Vec<_>>(), "MB", None, None);
    }

    /// Draw a memory usage bar
    fn draw_memory_bar(
        &self,
        ui: &mut egui::Ui,
        label: &str,
        value_mb: f64,
        max_mb: f64,
        color: egui::Color32,
    ) {
        let available_width = ui.available_width();
        let height = 24.0;

        ui.horizontal(|ui| {
            ui.label(format!("{}:", label));
            ui.label(format!("{:.1} MB", value_mb));
        });

        let (rect, _response) =
            ui.allocate_exact_size(egui::vec2(available_width, height), egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            // Background
            painter.rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);

            // Fill
            let fill_width = if max_mb > 0.0 {
                (value_mb / max_mb * rect.width() as f64).min(rect.width() as f64) as f32
            } else {
                0.0
            };

            if fill_width > 0.0 {
                let fill_rect =
                    egui::Rect::from_min_size(rect.min, egui::vec2(fill_width, rect.height()));
                painter.rect_filled(fill_rect, 2.0, color.gamma_multiply(0.7));
            }

            // Border
            painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, color));
        }

        ui.add_space(4.0);
    }

    // ========================================================================
    // ANALYTICS TAB
    // ========================================================================

    /// Draw analytics tab
    fn draw_analytics_tab(&mut self, ui: &mut egui::Ui, profiler: &EnhancedProfiler) {
        ui.heading("Advanced Analytics");

        // Statistics
        egui::CollapsingHeader::new("📊 Frame Time Statistics").default_open(true).show(ui, |ui| {
            if !profiler.frame_history().is_empty() {
                let p99 = profiler.percentile_frame_time(99.0);
                let p95 = profiler.percentile_frame_time(95.0);
                let p50 = profiler.percentile_frame_time(50.0);
                let variance = profiler.frame_time_variance();
                let std_dev = profiler.frame_time_std_dev();

                egui::Grid::new("stats_grid")
                    .num_columns(2)
                    .spacing([20.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("99th Percentile:");
                        ui.label(format!("{:.2} ms", p99));
                        ui.end_row();

                        ui.label("95th Percentile:");
                        ui.label(format!("{:.2} ms", p95));
                        ui.end_row();

                        ui.label("Median (50th):");
                        ui.label(format!("{:.2} ms", p50));
                        ui.end_row();

                        ui.label("Variance:");
                        ui.label(format!("{:.4} ms²", variance));
                        ui.end_row();

                        ui.label("Std Deviation:");
                        ui.label(format!("{:.2} ms", std_dev));
                        ui.end_row();
                    });
            } else {
                ui.label("No data available.");
            }
        });

        ui.add_space(15.0);

        egui::CollapsingHeader::new("🔴 Identified Bottlenecks").default_open(true).show(ui, |ui| {
            let bottlenecks = profiler.identify_bottlenecks();

            if bottlenecks.is_empty() {
                ui.label(egui::RichText::new("✓ No bottlenecks detected").color(egui::Color32::GREEN));
            } else {
                for bottleneck in bottlenecks {
                    let severity_color = if bottleneck.severity > 0.7 {
                        egui::Color32::RED
                    } else if bottleneck.severity > 0.4 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::from_rgb(255, 150, 0)
                    };

                    egui::Frame::group(ui.style())
                        .fill(egui::Color32::from_rgb(40, 20, 20))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.colored_label(severity_color, "⚠");
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(&bottleneck.name).strong().color(severity_color),
                                    );
                                    ui.label(format!(
                                        "{:.1}ms over budget ({:.1}% of frame)",
                                        bottleneck.over_budget_ms,
                                        bottleneck.percentage_of_frame * 100.0
                                    ));
                                });
                            });
                        });
                }
            }
        });

        ui.add_space(15.0);

        egui::CollapsingHeader::new("💡 Optimization Suggestions").default_open(true).show(ui, |ui| {
            let suggestions = profiler.optimization_suggestions();

            if suggestions.is_empty() {
                ui.label(egui::RichText::new("✓ No suggestions - performance is good!").color(egui::Color32::GREEN));
            } else {
                for suggestion in suggestions {
                    let category_emoji = match suggestion.category {
                        dde_core::profiler::enhanced::SuggestionCategory::Performance => "⚡",
                        dde_core::profiler::enhanced::SuggestionCategory::Memory => "🧠",
                        dde_core::profiler::enhanced::SuggestionCategory::Rendering => "🎨",
                        dde_core::profiler::enhanced::SuggestionCategory::Audio => "🔊",
                        dde_core::profiler::enhanced::SuggestionCategory::Script => "📜",
                        dde_core::profiler::enhanced::SuggestionCategory::ECS => "🧩",
                    };

                    let priority_color = if suggestion.priority > 0.7 {
                        egui::Color32::RED
                    } else if suggestion.priority > 0.4 {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::GREEN
                    };

                    egui::Frame::group(ui.style())
                        .fill(egui::Color32::from_rgb(30, 30, 30))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(category_emoji);
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(&suggestion.title)
                                            .strong()
                                            .color(priority_color),
                                    );
                                    ui.label(&suggestion.description);
                                });
                            });
                        });
                }
            }
        });
    }

    // ========================================================================
    // EXPORT TAB
    // ========================================================================

    /// Draw export tab
    fn draw_export_tab(&mut self, ui: &mut egui::Ui, profiler: &EnhancedProfiler) {
        ui.heading("Export Profile Data");
        ui.label("Export profiling data for external analysis.");

        ui.add_space(15.0);

        // Export range selection
        ui.heading("Export Range");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.export_range, ExportRangeSelection::Last100, "Last 100");
            ui.selectable_value(&mut self.export_range, ExportRangeSelection::Last500, "Last 500");
            ui.selectable_value(&mut self.export_range, ExportRangeSelection::Last1000, "Last 1000");
            ui.selectable_value(&mut self.export_range, ExportRangeSelection::All, "All");
            ui.selectable_value(&mut self.export_range, ExportRangeSelection::Custom, "Custom");
        });

        if self.export_range == ExportRangeSelection::Custom {
            ui.horizontal(|ui| {
                ui.label("Frames:");
                ui.add(egui::DragValue::new(&mut self.custom_export_frames).speed(10).range(1..=10000));
            });
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        // Export buttons
        ui.heading("Export Format");

        ui.horizontal(|ui| {
            if ui.button("📁 Export to CSV").clicked() {
                let range = self.get_export_range();
                let csv_data = profiler.export_csv(range);
                // In real implementation, this would save to file
                ui.ctx().output().copied_text = csv_data.clone();
                tracing::info!("CSV data copied to clipboard ({} bytes)", csv_data.len());
            }

            if ui.button("📄 Export to JSON").clicked() {
                let range = self.get_export_range();
                let json_data = profiler.export_json(range);
                ui.ctx().output().copied_text = json_data.clone();
                tracing::info!("JSON data copied to clipboard ({} bytes)", json_data.len());
            }
        });

        ui.add_space(15.0);

        // Export info
        let range = self.get_export_range();
        let frame_count = match range {
            ExportRange::LastN(n) => n.min(profiler.frame_history().len()),
            ExportRange::All => profiler.frame_history().len(),
            ExportRange::TimeRange(_, _) => profiler.frame_history().len(), // Simplified
        };

        ui.label(format!("Will export {} frames of data.", frame_count));

        ui.add_space(20.0);

        // Quick stats about available data
        ui.heading("Available Data");
        egui::Grid::new("export_stats")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                ui.label("Total Frames Recorded:");
                ui.label(format!("{}", profiler.frame_history().len()));
                ui.end_row();

                ui.label("Recording Duration:");
                let duration = profiler.frame_history().len() as f32 / 60.0;
                ui.label(format!("{:.1} seconds (at 60fps)", duration));
                ui.end_row();

                if let Some(first) = profiler.frame_history().front() {
                    ui.label("First Frame:");
                    ui.label(format!("#{}", first.frame));
                    ui.end_row();
                }

                if let Some(last) = profiler.frame_history().back() {
                    ui.label("Last Frame:");
                    ui.label(format!("#{}", last.frame));
                    ui.end_row();
                }
            });
    }

    /// Get export range from selection
    fn get_export_range(&self) -> ExportRange {
        match self.export_range {
            ExportRangeSelection::Last100 => ExportRange::LastN(100),
            ExportRangeSelection::Last500 => ExportRange::LastN(500),
            ExportRangeSelection::Last1000 => ExportRange::LastN(1000),
            ExportRangeSelection::All => ExportRange::All,
            ExportRangeSelection::Custom => ExportRange::LastN(self.custom_export_frames),
        }
    }

    /// Get budget for section name
    fn get_budget_for_section(&self, section: &str) -> f64 {
        match section.to_lowercase().as_str() {
            "simulation" | "tick" => self.budget_edit.simulation_budget_ms,
            "render" | "rendering" => self.budget_edit.render_budget_ms,
            "audio" => self.budget_edit.audio_budget_ms,
            "script" | "scripts" | "lua" => self.budget_edit.script_budget_ms,
            "ui" => self.budget_edit.ui_budget_ms,
            "pathfinding" => self.budget_edit.pathfinding_budget_ms,
            _ => self.budget_edit.frame_budget_ms,
        }
    }
}

impl Default for ProfilerPanelEnhanced {
    fn default() -> Self {
        Self::new()
    }
}

/// Get budget status for a value
fn get_budget_status(value: f64, budget: f64) -> BudgetStatus {
    if budget <= 0.0 {
        return BudgetStatus::Unknown;
    }

    let ratio = value / budget;
    if ratio <= 0.8 {
        BudgetStatus::Good
    } else if ratio <= 1.0 {
        BudgetStatus::Warning
    } else {
        BudgetStatus::OverBudget
    }
}

/// Get budget status text
fn budget_status_text(status: &BudgetStatus) -> &'static str {
    match status {
        BudgetStatus::Good => "OK",
        BudgetStatus::Warning => "WARN",
        BudgetStatus::OverBudget => "CRIT",
        BudgetStatus::Unknown => "---",
    }
}

/// Convert BudgetStatus to egui color
fn budget_status_color(status: &BudgetStatus) -> egui::Color32 {
    match status {
        BudgetStatus::Good => egui::Color32::GREEN,
        BudgetStatus::Warning => egui::Color32::YELLOW,
        BudgetStatus::OverBudget => egui::Color32::RED,
        BudgetStatus::Unknown => egui::Color32::GRAY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_creation() {
        let panel = ProfilerPanelEnhanced::new();
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_panel_toggle() {
        let mut panel = ProfilerPanelEnhanced::new();
        assert!(!panel.is_visible());

        panel.toggle();
        assert!(panel.is_visible());

        panel.toggle();
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_budget_status() {
        assert!(matches!(get_budget_status(10.0, 16.67), BudgetStatus::Good));
        assert!(matches!(get_budget_status(15.0, 16.67), BudgetStatus::Warning));
        assert!(matches!(get_budget_status(20.0, 16.67), BudgetStatus::OverBudget));
    }

    #[test]
    fn test_export_range_selection() {
        let mut panel = ProfilerPanelEnhanced::new();
        
        panel.export_range = ExportRangeSelection::Last100;
        assert!(matches!(panel.get_export_range(), ExportRange::LastN(100)));

        panel.export_range = ExportRangeSelection::All;
        assert!(matches!(panel.get_export_range(), ExportRange::All));

        panel.export_range = ExportRangeSelection::Custom;
        panel.custom_export_frames = 500;
        assert!(matches!(panel.get_export_range(), ExportRange::LastN(500)));
    }
}
