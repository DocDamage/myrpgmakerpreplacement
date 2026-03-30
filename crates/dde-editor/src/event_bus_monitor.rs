//! Event Bus Monitor Panel
//!
//! Real-time debugging panel for monitoring events flowing through the event bus.
//! Provides filtering, statistics, and export capabilities.

use dde_core::events::{Event, EventBus, EventFilter, EventType, SubscriptionId};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Maximum number of events to keep in the log
const MAX_EVENT_HISTORY: usize = 10000;
/// Default maximum payload display length
const DEFAULT_PAYLOAD_TRUNCATION: usize = 200;
/// Statistics window size for events per second calculation
const EPS_WINDOW_SECONDS: usize = 5;

/// A recorded event entry in the monitor
#[derive(Debug, Clone)]
pub struct EventEntry {
    /// Unique sequence ID
    pub sequence: u64,
    /// Timestamp when the event was received
    pub timestamp: Instant,
    /// Event type category
    pub event_type: EventType,
    /// String representation of the event payload
    pub payload: String,
    /// Associated entity ID if any (parsed from payload)
    pub entity_id: Option<String>,
    /// Event priority (if available)
    pub priority: Option<String>,
}

impl EventEntry {
    /// Format timestamp as HH:MM:SS.mmm
    pub fn format_timestamp(&self) -> String {
        let elapsed = self.timestamp.elapsed();
        let total_millis = elapsed.as_millis();
        let hours = (total_millis / 3_600_000) % 24;
        let minutes = (total_millis / 60_000) % 60;
        let seconds = (total_millis / 1_000) % 60;
        let millis = total_millis % 1_000;
        format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
    }

    /// Get truncated payload for display
    pub fn truncated_payload(&self, max_len: usize) -> String {
        if self.payload.len() <= max_len {
            self.payload.clone()
        } else {
            format!("{}...", &self.payload[..max_len])
        }
    }

    /// Get color for event type
    pub fn type_color(&self) -> egui::Color32 {
        match self.event_type {
            EventType::World => egui::Color32::from_rgb(100, 200, 100),    // Green
            EventType::Battle => egui::Color32::from_rgb(200, 100, 100),   // Red
            EventType::Ui => egui::Color32::from_rgb(100, 150, 200),       // Blue
            EventType::Audio => egui::Color32::from_rgb(200, 150, 100),    // Orange
            EventType::Ai => egui::Color32::from_rgb(150, 100, 200),       // Purple
            EventType::Quest => egui::Color32::from_rgb(200, 200, 100),    // Yellow
            EventType::Input => egui::Color32::from_rgb(100, 200, 200),    // Cyan
            EventType::System => egui::Color32::from_rgb(150, 150, 150),   // Gray
            EventType::Custom(_) => egui::Color32::from_rgb(200, 100, 200), // Magenta
        }
    }
}

/// Filter settings for the event monitor
#[derive(Debug, Clone)]
pub struct EventFilterSettings {
    /// Filter by event type (None = all types)
    pub event_type_filter: Option<EventType>,
    /// Filter by entity ID substring
    pub entity_id_filter: String,
    /// Text search in payload
    pub text_filter: String,
    /// Minimum priority level to show
    pub min_priority: Option<String>,
    /// Invert filter (show only non-matching)
    pub invert_filter: bool,
}

impl Default for EventFilterSettings {
    fn default() -> Self {
        Self {
            event_type_filter: None,
            entity_id_filter: String::new(),
            text_filter: String::new(),
            min_priority: None,
            invert_filter: false,
        }
    }
}

impl EventFilterSettings {
    /// Check if an event matches the filter criteria
    pub fn matches(&self, entry: &EventEntry) -> bool {
        let mut matches = true;

        // Event type filter
        if let Some(ref ty) = self.event_type_filter {
            if entry.event_type != *ty {
                matches = false;
            }
        }

        // Entity ID filter
        if !self.entity_id_filter.is_empty() {
            let has_entity = entry
                .entity_id
                .as_ref()
                .map(|e| e.to_lowercase().contains(&self.entity_id_filter.to_lowercase()))
                .unwrap_or(false);
            if !has_entity {
                matches = false;
            }
        }

        // Text search in payload
        if !self.text_filter.is_empty() {
            let text_matches = entry
                .payload
                .to_lowercase()
                .contains(&self.text_filter.to_lowercase());
            if !text_matches {
                matches = false;
            }
        }

        if self.invert_filter {
            matches = !matches;
        }

        matches
    }

    /// Check if any filter is active
    pub fn is_active(&self) -> bool {
        self.event_type_filter.is_some()
            || !self.entity_id_filter.is_empty()
            || !self.text_filter.is_empty()
            || self.min_priority.is_some()
            || self.invert_filter
    }

    /// Clear all filters
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// Event statistics tracker
#[derive(Debug, Default)]
pub struct EventStatistics {
    /// Total events recorded
    pub total_count: u64,
    /// Events per second over time windows
    pub eps_history: VecDeque<(Instant, u64)>,
    /// Count per event type
    pub type_counts: HashMap<EventType, u64>,
    /// Count per entity ID
    pub entity_counts: HashMap<String, u64>,
    /// Last calculation time
    last_calculation: Instant,
}

impl EventStatistics {
    /// Create new statistics tracker
    pub fn new() -> Self {
        Self {
            total_count: 0,
            eps_history: VecDeque::with_capacity(EPS_WINDOW_SECONDS * 10),
            type_counts: HashMap::new(),
            entity_counts: HashMap::new(),
            last_calculation: Instant::now(),
        }
    }

    /// Record an event
    pub fn record(&mut self, entry: &EventEntry) {
        self.total_count += 1;

        // Record timestamp for EPS calculation
        self.eps_history.push_back((entry.timestamp, self.total_count));

        // Clean old entries (older than EPS_WINDOW_SECONDS)
        let cutoff = Instant::now() - Duration::from_secs(EPS_WINDOW_SECONDS as u64);
        while self
            .eps_history
            .front()
            .map(|(t, _)| *t < cutoff)
            .unwrap_or(false)
        {
            self.eps_history.pop_front();
        }

        // Count by type
        *self.type_counts.entry(entry.event_type).or_insert(0) += 1;

        // Count by entity
        if let Some(ref entity) = entry.entity_id {
            *self.entity_counts.entry(entity.clone()).or_insert(0) += 1;
        }
    }

    /// Calculate current events per second
    pub fn events_per_second(&self) -> f64 {
        if self.eps_history.len() < 2 {
            return 0.0;
        }

        let first = self.eps_history.front().unwrap();
        let last = self.eps_history.back().unwrap();

        let time_diff = last.0.duration_since(first.0).as_secs_f64();
        let count_diff = last.1 - first.1;

        if time_diff > 0.0 {
            count_diff as f64 / time_diff
        } else {
            0.0
        }
    }

    /// Get top event types by frequency
    pub fn top_event_types(&self, limit: usize) -> Vec<(EventType, u64)> {
        let mut types: Vec<_> = self.type_counts.iter().map(|(k, v)| (*k, *v)).collect();
        types.sort_by(|a, b| b.1.cmp(&a.1));
        types.truncate(limit);
        types
    }

    /// Get top entities by event count
    pub fn top_entities(&self, limit: usize) -> Vec<(String, u64)> {
        let mut entities: Vec<_> = self.entity_counts.iter().map(|(k, v)| (k.clone(), *v)).collect();
        entities.sort_by(|a, b| b.1.cmp(&a.1));
        entities.truncate(limit);
        entities
    }

    /// Get count for a specific event type
    pub fn type_count(&self, event_type: EventType) -> u64 {
        self.type_counts.get(&event_type).copied().unwrap_or(0)
    }

    /// Clear all statistics
    pub fn clear(&mut self) {
        self.total_count = 0;
        self.eps_history.clear();
        self.type_counts.clear();
        self.entity_counts.clear();
        self.last_calculation = Instant::now();
    }
}

/// Event Bus Monitor panel for debugging
pub struct EventBusMonitor {
    /// Whether the panel is visible
    visible: bool,
    /// Event log entries
    events: VecDeque<EventEntry>,
    /// Filter settings
    filters: EventFilterSettings,
    /// Statistics
    statistics: EventStatistics,
    /// Whether monitoring is paused
    paused: bool,
    /// Payload truncation length
    truncation_length: usize,
    /// Selected event for detail view
    selected_event: Option<usize>,
    /// Auto-scroll to bottom
    auto_scroll: bool,
    /// Show statistics panel
    show_statistics: bool,
    /// Event sequence counter
    sequence_counter: Arc<AtomicU64>,
    /// Subscription ID for the event bus
    subscription: Option<SubscriptionId>,
    /// Pending events from subscription callback
    pending_events: Arc<Mutex<Vec<EventEntry>>>, 
    /// Last update time for EPS smoothing
    last_update: Instant,
    /// Smoothed EPS value
    smoothed_eps: f64,
}

impl Default for EventBusMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBusMonitor {
    /// Create a new event bus monitor
    pub fn new() -> Self {
        Self {
            visible: false,
            events: VecDeque::with_capacity(MAX_EVENT_HISTORY),
            filters: EventFilterSettings::default(),
            statistics: EventStatistics::new(),
            paused: false,
            truncation_length: DEFAULT_PAYLOAD_TRUNCATION,
            selected_event: None,
            auto_scroll: true,
            show_statistics: true,
            sequence_counter: Arc::new(AtomicU64::new(1)),
            subscription: None,
            pending_events: Arc::new(Mutex::new(Vec::new())),
            last_update: Instant::now(),
            smoothed_eps: 0.0,
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

    /// Pause monitoring
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume monitoring
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Toggle pause state
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Check if paused
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.events.clear();
        self.statistics.clear();
        self.selected_event = None;
    }

    /// Subscribe to an event bus to receive events
    pub fn subscribe_to_bus(&mut self, bus: &EventBus) {
        // Unsubscribe from previous if any
        if let Some(sub) = self.subscription {
            bus.unsubscribe(sub);
        }

        let pending = Arc::clone(&self.pending_events);
        let counter = Arc::clone(&self.sequence_counter);

        // Subscribe to all events
        let sub = bus.subscribe(EventFilter::All, move |event| {
            let entry = EventEntry {
                sequence: counter.fetch_add(1, Ordering::SeqCst),
                timestamp: Instant::now(),
                event_type: event.event_type(),
                payload: format!("{:?}", event),
                entity_id: Self::extract_entity_id(event),
                priority: None,
            };
            pending.lock().push(entry);
        });

        self.subscription = Some(sub);
    }

    /// Unsubscribe from the event bus
    pub fn unsubscribe(&mut self, bus: &EventBus) {
        if let Some(sub) = self.subscription.take() {
            bus.unsubscribe(sub);
        }
    }

    /// Extract entity ID from event if possible
    fn extract_entity_id(event: &dyn Event) -> Option<String> {
        // Try to extract entity ID from common event patterns
        let payload = format!("{:?}", event);

        // Look for Entity(...) pattern
        if let Some(start) = payload.find("Entity(") {
            if let Some(end) = payload[start..].find(')') {
                return Some(payload[start..start + end + 1].to_string());
            }
        }

        // Look for entity: Entity(...) pattern
        if let Some(idx) = payload.find("entity:") {
            let rest = &payload[idx + 7..];
            if let Some(start) = rest.find("Entity(") {
                if let Some(end) = rest[start..].find(')') {
                    return Some(rest[start..start + end + 1].to_string());
                }
            }
        }

        None
    }

    /// Add a manual event entry (for testing or external events)
    pub fn add_event(&mut self, event_type: EventType, payload: impl Into<String>) {
        if self.paused {
            return;
        }

        let entry = EventEntry {
            sequence: self.sequence_counter.fetch_add(1, Ordering::SeqCst),
            timestamp: Instant::now(),
            event_type,
            payload: payload.into(),
            entity_id: None,
            priority: None,
        };

        self.add_entry(entry);
    }

    /// Add an entry to the log
    fn add_entry(&mut self, entry: EventEntry) {
        // Update statistics
        self.statistics.record(&entry);

        // Add to log
        self.events.push_back(entry);

        // Trim if over capacity
        while self.events.len() > MAX_EVENT_HISTORY {
            self.events.pop_front();
        }
    }

    /// Process any pending events from the subscription callback
    fn process_pending_events(&mut self) {
        if self.paused {
            return;
        }

        if let Ok(mut pending) = self.pending_events.lock() {
            for entry in pending.drain(..) {
                self.add_entry(entry);
            }
        }
    }

    /// Update the monitor (call each frame)
    pub fn update(&mut self, _dt: f32) {
        self.process_pending_events();

        // Update smoothed EPS
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        if elapsed >= 0.1 {
            // Update every 100ms
            let current_eps = self.statistics.events_per_second();
            self.smoothed_eps = self.smoothed_eps * 0.8 + current_eps * 0.2;
            self.last_update = now;
        }
    }

    /// Export events to a file
    pub fn export_to_file(&self, path: &str) -> Result<(), String> {
        use std::io::Write;

        let mut file = std::fs::File::create(path).map_err(|e| e.to_string())?;

        // Write header
        writeln!(file, "sequence,timestamp,event_type,entity_id,payload").map_err(|e| e.to_string())?;

        // Write events
        for entry in &self.events {
            let timestamp = entry.timestamp.elapsed().as_millis();
            let entity = entry.entity_id.as_deref().unwrap_or("");
            let payload = entry.payload.replace('"', "\"");
            writeln!(
                file,
                "{},{},{:?},{},\"{}\"",
                entry.sequence, timestamp, entry.event_type, entity, payload
            )
            .map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    /// Get filtered events
    fn filtered_events(&self) -> Vec<&EventEntry> {
        if !self.filters.is_active() {
            return self.events.iter().collect();
        }

        self.events.iter().filter(|e| self.filters.matches(e)).collect()
    }

    /// Draw the monitor panel
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        let mut visible = self.visible;
        egui::Window::new("📡 Event Bus Monitor")
            .open(&mut visible)
            .resizable(true)
            .default_size([900.0, 600.0])
            .show(ctx, |ui| {
                self.draw_panel_content(ui);
            });
        self.visible = visible;
    }

    /// Draw panel content
    fn draw_panel_content(&mut self, ui: &mut egui::Ui) {
        // Header with controls
        ui.horizontal(|ui| {
            ui.heading("Event Bus Monitor");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Pause/Resume button
                let pause_text = if self.paused { "▶ Resume" } else { "⏸ Pause" };
                if ui.button(pause_text).clicked() {
                    self.toggle_pause();
                }

                // Clear button
                if ui.button("🗑 Clear").clicked() {
                    self.clear();
                }

                // Export button
                if ui.button("💾 Export").clicked() {
                    // Default export path
                    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                    let path = format!("event_log_{}.csv", timestamp);
                    match self.export_to_file(&path) {
                        Ok(_) => tracing::info!("Exported events to {}", path),
                        Err(e) => tracing::error!("Failed to export: {}", e),
                    }
                }

                // Statistics toggle
                ui.checkbox(&mut self.show_statistics, "Stats");

                // Auto-scroll toggle
                ui.checkbox(&mut self.auto_scroll, "Auto-scroll");
            });
        });

        ui.separator();

        // Statistics bar
        self.draw_statistics_bar(ui);

        ui.separator();

        // Filter bar
        self.draw_filter_bar(ui);

        ui.separator();

        // Main content area
        if self.show_statistics {
            egui::SidePanel::right("event_stats_panel")
                .resizable(true)
                .default_width(250.0)
                .show_inside(ui, |ui| {
                    self.draw_statistics_panel(ui);
                });
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.draw_event_log(ui);
        });

        // Detail view at bottom
        if self.selected_event.is_some() {
            egui::TopBottomPanel::bottom("event_detail_panel")
                .resizable(true)
                .default_height(150.0)
                .show_inside(ui, |ui| {
                    self.draw_event_detail(ui);
                });
        }
    }

    /// Draw statistics bar
    fn draw_statistics_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Total events
            ui.label(format!("Total: {}", self.statistics.total_count));

            ui.separator();

            // Events per second
            let eps = self.smoothed_eps;
            let eps_color = if eps > 1000.0 {
                egui::Color32::RED
            } else if eps > 100.0 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::GREEN
            };
            ui.label("EPS:");
            ui.colored_label(eps_color, format!("{:.1}", eps));

            ui.separator();

            // Logged events count
            ui.label(format!("Logged: {}", self.events.len()));

            ui.separator();

            // Paused indicator
            if self.paused {
                ui.colored_label(egui::Color32::YELLOW, "⏸ PAUSED");
            }

            // Filter indicator
            if self.filters.is_active() {
                ui.colored_label(egui::Color32::LIGHT_BLUE, "🔍 FILTERED");
            }
        });
    }

    /// Draw filter bar
    fn draw_filter_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Event type filter
            ui.label("Type:");
            egui::ComboBox::from_id_salt("event_type_filter")
                .selected_text(
                    self.filters
                        .event_type_filter
                        .map(|t| format!("{:?}", t))
                        .unwrap_or_else(|| "All".to_string()),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.filters.event_type_filter, None, "All");
                    ui.selectable_value(
                        &mut self.filters.event_type_filter,
                        Some(EventType::World),
                        "World",
                    );
                    ui.selectable_value(
                        &mut self.filters.event_type_filter,
                        Some(EventType::Battle),
                        "Battle",
                    );
                    ui.selectable_value(
                        &mut self.filters.event_type_filter,
                        Some(EventType::Ui),
                        "UI",
                    );
                    ui.selectable_value(
                        &mut self.filters.event_type_filter,
                        Some(EventType::Audio),
                        "Audio",
                    );
                    ui.selectable_value(
                        &mut self.filters.event_type_filter,
                        Some(EventType::Ai),
                        "AI",
                    );
                    ui.selectable_value(
                        &mut self.filters.event_type_filter,
                        Some(EventType::Quest),
                        "Quest",
                    );
                    ui.selectable_value(
                        &mut self.filters.event_type_filter,
                        Some(EventType::Input),
                        "Input",
                    );
                    ui.selectable_value(
                        &mut self.filters.event_type_filter,
                        Some(EventType::System),
                        "System",
                    );
                });

            ui.separator();

            // Entity ID filter
            ui.label("Entity:");
            ui.add(
                egui::TextEdit::singleline(&mut self.filters.entity_id_filter)
                    .desired_width(100.0)
                    .hint_text("ID"),
            );

            ui.separator();

            // Text filter
            ui.label("Search:");
            ui.add(
                egui::TextEdit::singleline(&mut self.filters.text_filter)
                    .desired_width(150.0)
                    .hint_text("text"),
            );

            ui.separator();

            // Invert filter
            ui.checkbox(&mut self.filters.invert_filter, "Invert");

            // Clear filters button
            if ui.button("Clear Filters").clicked() {
                self.filters.clear();
            }
        });
    }

    /// Draw statistics panel
    fn draw_statistics_panel(&self, ui: &mut egui::Ui) {
        ui.heading("Statistics");
        ui.separator();

        // Events per second
        ui.label("Events Per Second");
        ui.label(
            egui::RichText::new(format!("{:.1}", self.smoothed_eps))
                .size(24.0)
                .color(egui::Color32::LIGHT_BLUE),
        );

        ui.add_space(10.0);

        // Total events
        ui.label("Total Events");
        ui.label(
            egui::RichText::new(format!("{}", self.statistics.total_count))
                .size(20.0)
                .color(egui::Color32::WHITE),
        );

        ui.add_space(10.0);
        ui.separator();

        // Top event types
        ui.label("Top Event Types");
        let top_types = self.statistics.top_event_types(5);
        for (event_type, count) in top_types {
            ui.horizontal(|ui| {
                let color = match event_type {
                    EventType::World => egui::Color32::GREEN,
                    EventType::Battle => egui::Color32::RED,
                    EventType::Ui => egui::Color32::BLUE,
                    EventType::Audio => egui::Color32::ORANGE,
                    EventType::Ai => egui::Color32::PURPLE,
                    EventType::Quest => egui::Color32::YELLOW,
                    EventType::Input => egui::Color32::CYAN,
                    EventType::System => egui::Color32::GRAY,
                    EventType::Custom(_) => egui::Color32::MAGENTA,
                };
                ui.colored_label(color, format!("{:?}", event_type));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{}", count));
                });
            });
        }

        ui.add_space(10.0);
        ui.separator();

        // Event type breakdown
        ui.label("Event Type Counts");
        egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
            for event_type in [
                EventType::World,
                EventType::Battle,
                EventType::Ui,
                EventType::Audio,
                EventType::Ai,
                EventType::Quest,
                EventType::Input,
                EventType::System,
            ] {
                let count = self.statistics.type_count(event_type);
                if count > 0 {
                    ui.horizontal(|ui| {
                        ui.label(format!("{:?}:", event_type));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("{}", count));
                        });
                    });
                }
            }
        });
    }

    /// Draw the event log table
    fn draw_event_log(&mut self, ui: &mut egui::Ui) {
        let filtered = self.filtered_events();

        // Show filtered count if filtering
        if self.filters.is_active() {
            ui.label(format!("Showing {} of {} events", filtered.len(), self.events.len()));
            ui.separator();
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .stick_to_bottom(self.auto_scroll)
            .show(ui, |ui| {
                egui::Grid::new("event_log_grid")
                    .num_columns(5)
                    .spacing([10.0, 2.0])
                    .striped(true)
                    .show(ui, |ui| {
                        // Header
                        ui.label("#");
                        ui.label("Time");
                        ui.label("Type");
                        ui.label("Entity");
                        ui.label("Payload");
                        ui.end_row();

                        // Events
                        for entry in filtered.iter().rev().take(1000) {
                            let is_selected = self.selected_event == Some(entry.sequence as usize);

                            // Sequence number
                            let seq_response = ui.selectable_label(
                                is_selected,
                                format!("{}", entry.sequence),
                            );

                            // Timestamp
                            let time_response =
                                ui.selectable_label(is_selected, entry.format_timestamp());

                            // Event type with color
                            let type_response = ui.colored_label(
                                entry.type_color(),
                                format!("{:?}", entry.event_type),
                            );

                            // Entity ID
                            let entity_text = entry.entity_id.as_deref().unwrap_or("-");
                            let entity_response = ui.selectable_label(is_selected, entity_text);

                            // Truncated payload
                            let payload = entry.truncated_payload(self.truncation_length);
                            let payload_response =
                                ui.selectable_label(is_selected, payload).on_hover_text(&entry.payload);

                            // Handle selection
                            if seq_response.clicked()
                                || time_response.clicked()
                                || entity_response.clicked()
                                || payload_response.clicked()
                            {
                                self.selected_event = Some(entry.sequence as usize);
                            }

                            ui.end_row();
                        }
                    });
            });
    }

    /// Draw event detail view
    fn draw_event_detail(&self, ui: &mut egui::Ui) {
        let Some(selected_seq) = self.selected_event else {
            ui.label("Select an event to view details");
            return;
        };

        // Find the event
        let Some(entry) = self
            .events
            .iter()
            .find(|e| e.sequence as usize == selected_seq)
        else {
            ui.label("Event not found");
            return;
        };

        ui.horizontal(|ui| {
            ui.heading(format!("Event #{}", entry.sequence));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("Type: {:?}", entry.event_type));
            });
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.monospace(format!("Timestamp: {:?}", entry.timestamp));
            ui.monospace(format!("Type: {:?}", entry.event_type));
            if let Some(ref entity) = entry.entity_id {
                ui.monospace(format!("Entity: {}", entity));
            }
            ui.separator();
            ui.label("Payload:");
            ui.monospace(&entry.payload);
        });
    }
}

/// Extension trait for Editor to add Event Bus Monitor integration
pub trait EventBusMonitorExt {
    /// Draw the Debug menu with Event Bus Monitor option
    fn draw_debug_menu(&mut self, ui: &mut egui::Ui);
    /// Toggle the event bus monitor
    fn toggle_event_bus_monitor(&mut self);
    /// Check if event bus monitor is visible
    fn is_event_bus_monitor_visible(&self) -> bool;
}

// Implementation for the Editor struct would be in lib.rs
// This trait allows external integration

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_entry_format_timestamp() {
        let entry = EventEntry {
            sequence: 1,
            timestamp: Instant::now(),
            event_type: EventType::World,
            payload: "test".to_string(),
            entity_id: None,
            priority: None,
        };

        // Just verify it doesn't panic and returns a string
        let formatted = entry.format_timestamp();
        assert!(!formatted.is_empty());
        assert!(formatted.contains(':'));
    }

    #[test]
    fn test_event_entry_truncated_payload() {
        let entry = EventEntry {
            sequence: 1,
            timestamp: Instant::now(),
            event_type: EventType::World,
            payload: "a".repeat(300),
            entity_id: None,
            priority: None,
        };

        let truncated = entry.truncated_payload(50);
        assert_eq!(truncated.len(), 53); // 50 + "..."
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_filter_settings_matches() {
        let mut filters = EventFilterSettings::default();

        let entry = EventEntry {
            sequence: 1,
            timestamp: Instant::now(),
            event_type: EventType::World,
            payload: "Entity(123) moved".to_string(),
            entity_id: Some("Entity(123)".to_string()),
            priority: None,
        };

        // Default matches all
        assert!(filters.matches(&entry));

        // Filter by type
        filters.event_type_filter = Some(EventType::Battle);
        assert!(!filters.matches(&entry));

        filters.event_type_filter = Some(EventType::World);
        assert!(filters.matches(&entry));

        // Filter by entity
        filters.event_type_filter = None;
        filters.entity_id_filter = "456".to_string();
        assert!(!filters.matches(&entry));

        filters.entity_id_filter = "123".to_string();
        assert!(filters.matches(&entry));

        // Filter by text
        filters.entity_id_filter = String::new();
        filters.text_filter = "jumped".to_string();
        assert!(!filters.matches(&entry));

        filters.text_filter = "moved".to_string();
        assert!(filters.matches(&entry));

        // Invert filter
        filters.invert_filter = true;
        assert!(!filters.matches(&entry));
    }

    #[test]
    fn test_statistics_record() {
        let mut stats = EventStatistics::new();

        let entry1 = EventEntry {
            sequence: 1,
            timestamp: Instant::now(),
            event_type: EventType::World,
            payload: "test".to_string(),
            entity_id: Some("Entity(1)".to_string()),
            priority: None,
        };

        let entry2 = EventEntry {
            sequence: 2,
            timestamp: Instant::now(),
            event_type: EventType::Battle,
            payload: "test".to_string(),
            entity_id: Some("Entity(1)".to_string()),
            priority: None,
        };

        stats.record(&entry1);
        assert_eq!(stats.total_count, 1);
        assert_eq!(stats.type_count(EventType::World), 1);
        assert_eq!(stats.type_count(EventType::Battle), 0);

        stats.record(&entry2);
        assert_eq!(stats.total_count, 2);
        assert_eq!(stats.type_count(EventType::World), 1);
        assert_eq!(stats.type_count(EventType::Battle), 1);
    }

    #[test]
    fn test_monitor_add_event() {
        let mut monitor = EventBusMonitor::new();

        monitor.add_event(EventType::World, "test event");
        assert_eq!(monitor.events.len(), 1);

        monitor.pause();
        monitor.add_event(EventType::World, "should not be added");
        assert_eq!(monitor.events.len(), 1);

        monitor.resume();
        monitor.add_event(EventType::World, "another event");
        assert_eq!(monitor.events.len(), 2);
    }

    #[test]
    fn test_monitor_clear() {
        let mut monitor = EventBusMonitor::new();

        monitor.add_event(EventType::World, "test");
        monitor.add_event(EventType::Battle, "test");
        assert_eq!(monitor.events.len(), 2);

        monitor.clear();
        assert_eq!(monitor.events.len(), 0);
        assert_eq!(monitor.statistics.total_count, 0);
    }

    #[test]
    fn test_monitor_visibility() {
        let mut monitor = EventBusMonitor::new();

        assert!(!monitor.is_visible());

        monitor.show();
        assert!(monitor.is_visible());

        monitor.hide();
        assert!(!monitor.is_visible());

        monitor.toggle();
        assert!(monitor.is_visible());

        monitor.toggle();
        assert!(!monitor.is_visible());
    }
}
