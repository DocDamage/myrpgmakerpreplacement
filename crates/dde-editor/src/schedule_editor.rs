//! NPC Schedule Editor
//!
//! A timeline-based editor for creating and managing NPC daily routines.
//! Features a 24-hour timeline with drag-and-drop entry creation and editing.

use dde_core::pathfinding::{
    NpcSchedule, ScheduleEntry, ScheduleEntryType, ScheduleLocation, Weekday,
};
use egui::{Color32, Rect, Response, Rounding, Stroke, Vec2};
use glam::IVec2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique ID generator for schedule entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntryId(pub u64);

impl EntryId {
    /// Generate a new unique ID
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for EntryId {
    fn default() -> Self {
        Self::new()
    }
}

/// NPC info for selector
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
}

/// Visual style for the schedule editor
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ScheduleEditorStyle {
    /// Height of each entry row
    pub entry_height: f32,
    /// Height of the time ruler
    pub ruler_height: f32,
    /// Hour width in pixels (zoom level)
    pub hour_width: f32,
    /// Minimum hour width
    pub min_hour_width: f32,
    /// Maximum hour width
    pub max_hour_width: f32,
    /// Color for the timeline background
    pub timeline_bg: [u8; 3],
    /// Color for alternating rows
    pub alternate_row_bg: [u8; 3],
    /// Selection color
    pub selection_color: [u8; 3],
    /// Playhead color
    pub playhead_color: [u8; 3],
}

impl Default for ScheduleEditorStyle {
    fn default() -> Self {
        Self {
            entry_height: 40.0,
            ruler_height: 30.0,
            hour_width: 60.0,
            min_hour_width: 30.0,
            max_hour_width: 200.0,
            timeline_bg: [40, 40, 40],
            alternate_row_bg: [50, 50, 50],
            selection_color: [255, 200, 100],
            playhead_color: [255, 50, 50],
        }
    }
}

/// Drag state for UI interactions
#[derive(Debug, Clone, Copy, PartialEq)]
enum DragState {
    /// Creating new entry (dragging to set duration)
    Creating {
        start_time: f32,
        day: Weekday,
        entry_type: ScheduleEntryType,
    },
    /// Moving an existing entry
    Moving {
        entry_id: u64,
        day: Weekday,
        offset_x: f32,
    },
    /// Resizing an entry (left or right edge)
    Resizing {
        entry_id: u64,
        day: Weekday,
        edge: ResizeEdge,
        original_start: f32,
        original_end: f32,
    },
    /// Panning the timeline view
    Panning { last_x: f32 },
}

/// Which edge is being resized
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResizeEdge {
    Left,
    Right,
}

/// Property panel tab
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PropertyTab {
    /// Entry properties
    Entry,
    /// Schedule overview
    Overview,
    /// Path preview
    Path,
}

/// Map picker state
#[derive(Debug, Clone)]
struct MapPickerState {
    /// Available maps
    pub maps: Vec<MapInfo>,
    /// Selected map ID
    pub selected_map: u32,
    /// Picker is active
    pub active: bool,
    /// Callback for when a position is selected
    pub on_select: Option<Box<dyn Fn(IVec2, u32)>>,
}

impl Default for MapPickerState {
    fn default() -> Self {
        Self {
            maps: vec![MapInfo { id: 1, name: "Map001".to_string(), width: 20, height: 15 }],
            selected_map: 1,
            active: false,
            on_select: None,
        }
    }
}

/// Map information for picker
#[derive(Debug, Clone)]
struct MapInfo {
    pub id: u32,
    pub name: String,
    pub width: i32,
    pub height: i32,
}

/// NPC Schedule Editor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleEditor {
    /// Whether the editor window is visible
    visible: bool,
    /// Currently selected NPC
    selected_npc: Option<u64>,
    /// Currently selected schedule entry
    selected_entry: Option<(Weekday, u64)>,
    /// Currently selected day tab
    selected_day: Weekday,
    /// View offset (horizontal scroll)
    scroll_offset: f32,
    /// Editor visual style
    pub style: ScheduleEditorStyle,
    /// NPC schedules (NPC ID -> Schedule)
    pub schedules: HashMap<u64, NpcSchedule>,
    /// Available NPCs
    pub npcs: Vec<NpcInfo>,
    /// Current property tab
    #[serde(skip)]
    property_tab: PropertyTab,
    /// Drag state
    #[serde(skip)]
    drag_state: Option<DragState>,
    /// Map picker state
    #[serde(skip)]
    map_picker: MapPickerState,
    /// Current time for preview (0-24)
    preview_time: f32,
    /// Whether preview is playing
    preview_playing: bool,
    /// Show weekend/weekday pattern toggle
    use_weekend_pattern: bool,
    /// Entry type to create
    current_entry_type: ScheduleEntryType,
    /// Entry being edited (for property panel)
    #[serde(skip)]
    editing_entry: Option<ScheduleEntry>,
    /// Show path preview
    show_path_preview: bool,
}

impl ScheduleEditor {
    /// Create a new schedule editor
    pub fn new() -> Self {
        let mut schedules = HashMap::new();
        let npcs = Vec::new();

        Self {
            visible: false,
            selected_npc: None,
            selected_entry: None,
            selected_day: Weekday::Monday,
            scroll_offset: 0.0,
            style: ScheduleEditorStyle::default(),
            schedules,
            npcs,
            property_tab: PropertyTab::Entry,
            drag_state: None,
            map_picker: MapPickerState::default(),
            preview_time: 8.0,
            preview_playing: false,
            use_weekend_pattern: false,
            current_entry_type: ScheduleEntryType::Idle,
            editing_entry: None,
            show_path_preview: true,
        }
    }

    /// Show the schedule editor window
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the schedule editor window
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Check if editor is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Add an NPC to the editor
    pub fn add_npc(&mut self, npc: NpcInfo) {
        let npc_id = npc.id;
        self.npcs.push(npc);
        if self.selected_npc.is_none() {
            self.selected_npc = Some(npc_id);
        }
    }

    /// Remove an NPC from the editor
    pub fn remove_npc(&mut self, npc_id: u64) {
        self.npcs.retain(|n| n.id != npc_id);
        self.schedules.remove(&npc_id);
        if self.selected_npc == Some(npc_id) {
            self.selected_npc = self.npcs.first().map(|n| n.id);
        }
    }

    /// Get or create schedule for NPC
    pub fn get_schedule(&mut self, npc_id: u64) -> &mut NpcSchedule {
        self.schedules.entry(npc_id).or_insert_with(NpcSchedule::new)
    }

    /// Get schedule for NPC (immutable)
    pub fn get_schedule_ref(&self, npc_id: u64) -> Option<&NpcSchedule> {
        self.schedules.get(&npc_id)
    }

    /// Draw the schedule editor UI
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        egui::Window::new("🗓️ NPC Schedule Editor")
            .default_size([1200.0, 700.0])
            .resizable(true)
            .show(ctx, |ui| {
                self.draw_ui(ui);
            });
    }

    /// Draw the main editor UI
    fn draw_ui(&mut self, ui: &mut egui::Ui) {
        // Handle preview playback
        if self.preview_playing {
            self.preview_time += ui.input(|i| i.unstable_dt) * 2.0; // 2x speed for preview
            if self.preview_time >= 24.0 {
                self.preview_time = 0.0;
            }
        }

        // Update drag state
        self.handle_drag(ui);

        // Top panel: NPC selector and toolbar
        egui::TopBottomPanel::top("schedule_toolbar")
            .exact_height(60.0)
            .show_inside(ui, |ui| {
                self.draw_toolbar(ui);
            });

        // Left panel: NPC list
        egui::SidePanel::left("schedule_npc_list")
            .exact_width(200.0)
            .show_inside(ui, |ui| {
                self.draw_npc_list(ui);
            });

        // Right panel: Properties
        egui::SidePanel::right("schedule_properties")
            .exact_width(300.0)
            .show_inside(ui, |ui| {
                self.draw_properties_panel(ui);
            });

        // Central panel: Timeline
        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.draw_timeline(ui);
        });
    }

    /// Draw the toolbar
    fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // NPC selector dropdown
            ui.label("NPC:");
            egui::ComboBox::from_id_source("npc_selector")
                .selected_text(
                    self.selected_npc
                        .and_then(|id| self.npcs.iter().find(|n| n.id == id))
                        .map(|n| n.name.as_str())
                        .unwrap_or("Select NPC..."),
                )
                .show_ui(ui, |ui| {
                    for npc in &self.npcs {
                        if ui
                            .selectable_label(
                                self.selected_npc == Some(npc.id),
                                &npc.name,
                            )
                            .clicked()
                        {
                            self.selected_npc = Some(npc.id);
                            self.selected_entry = None;
                            self.editing_entry = None;
                        }
                    }
                });

            ui.separator();

            // Day tabs
            ui.label("Day:");
            for day in Weekday::all() {
                let is_selected = self.selected_day == day;
                let label = if ui.available_width() > 400.0 {
                    day.display_name()
                } else {
                    day.short_name()
                };
                if ui.selectable_label(is_selected, label).clicked() {
                    self.selected_day = day;
                    self.selected_entry = None;
                    self.editing_entry = None;
                }
            }

            ui.separator();

            // Entry type selector for new entries
            ui.label("New:");
            egui::ComboBox::from_id_source("entry_type_selector")
                .selected_text(self.current_entry_type.icon())
                .show_ui(ui, |ui| {
                    for entry_type in [
                        ScheduleEntryType::Work,
                        ScheduleEntryType::Sleep,
                        ScheduleEntryType::Eat,
                        ScheduleEntryType::Patrol,
                        ScheduleEntryType::Idle,
                        ScheduleEntryType::Custom,
                    ] {
                        if ui
                            .selectable_label(
                                self.current_entry_type == entry_type,
                                format!("{} {}", entry_type.icon(), entry_type.display_name()),
                            )
                            .clicked()
                        {
                            self.current_entry_type = entry_type;
                        }
                    }
                });

            ui.separator();

            // Zoom controls
            ui.label("Zoom:");
            if ui.button("-").clicked() {
                self.style.hour_width = (self.style.hour_width / 1.2)
                    .max(self.style.min_hour_width);
            }
            if ui.button("+").clicked() {
                self.style.hour_width = (self.style.hour_width * 1.2)
                    .min(self.style.max_hour_width);
            }

            ui.separator();

            // Preview controls
            ui.label("Preview:");
            let play_text = if self.preview_playing { "⏸" } else { "▶" };
            if ui.button(play_text).on_hover_text("Play/Pause Preview").clicked() {
                self.preview_playing = !self.preview_playing;
            }
            ui.label(format!("{:02.0}:{:02.0}", 
                self.preview_time.floor(), 
                (self.preview_time.fract() * 60.0).floor()
            ));

            ui.separator();

            // Weekend pattern toggle
            if ui
                .checkbox(&mut self.use_weekend_pattern, "Weekend Pattern")
                .changed()
            {
                if self.use_weekend_pattern {
                    if let Some(npc_id) = self.selected_npc {
                        let schedule = self.get_schedule(npc_id);
                        let weekday_entries = schedule.entries_for_day(Weekday::Monday).to_vec();
                        let weekend_entries = schedule.entries_for_day(Weekday::Saturday).to_vec();
                        schedule.set_weekend_pattern(weekday_entries, weekend_entries);
                    }
                }
            }

            // Show path preview toggle
            ui.checkbox(&mut self.show_path_preview, "Show Path");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕").clicked() {
                    self.hide();
                }
                if ui.button("📋 Copy Day").clicked() {
                    self.copy_to_all_days();
                }
                if ui.button("🗑️ Clear Day").clicked() {
                    self.clear_current_day();
                }
            });
        });
    }

    /// Draw the NPC list panel
    fn draw_npc_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("NPCs");
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for npc in &self.npcs {
                let is_selected = self.selected_npc == Some(npc.id);
                let response = ui.selectable_label(
                    is_selected,
                    format!("👤 {}", npc.name),
                );
                if response.clicked() {
                    self.selected_npc = Some(npc.id);
                    self.selected_entry = None;
                    self.editing_entry = None;
                }
                response.on_hover_text(format!("Map: {}, Pos: ({}, {})", 
                    npc.map_id, npc.position.x, npc.position.y));
            }
        });

        ui.separator();

        // Quick actions
        if ui.button("➕ Add NPC").clicked() {
            // This would typically open a dialog to select from game NPCs
            // For now, we'll add a placeholder
        }
    }

    /// Draw the timeline
    fn draw_timeline(&mut self, ui: &mut egui::Ui) {
        let available_rect = ui.available_rect_before_wrap();

        // Time ruler
        let ruler_rect = Rect::from_min_size(
            available_rect.min,
            Vec2::new(available_rect.width(), self.style.ruler_height),
        );
        self.draw_time_ruler(ui, ruler_rect);

        // Timeline content
        let content_rect = Rect::from_min_size(
            egui::pos2(available_rect.min.x, available_rect.min.y + self.style.ruler_height),
            Vec2::new(
                available_rect.width(),
                available_rect.height() - self.style.ruler_height,
            ),
        );
        self.draw_timeline_content(ui, content_rect);
    }

    /// Draw the time ruler
    fn draw_time_ruler(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        let bg_color = Color32::from_rgb(60, 60, 60);
        painter.rect_filled(rect, 0.0, bg_color);

        // Hour markers
        for hour in 0..=24 {
            let x = self.time_to_x(hour as f32, rect.min.x);
            if x < rect.min.x || x > rect.max.x {
                continue;
            }

            // Hour tick
            painter.line_segment(
                [egui::pos2(x, rect.max.y - 15.0), egui::pos2(x, rect.max.y)],
                Stroke::new(1.0, Color32::WHITE),
            );

            // Hour label
            painter.text(
                egui::pos2(x + 2.0, rect.min.y + 2.0),
                egui::Align2::LEFT_TOP,
                format!("{:02}:00", hour),
                egui::FontId::monospace(10.0),
                Color32::WHITE,
            );

            // Half-hour tick
            let half_x = self.time_to_x(hour as f32 + 0.5, rect.min.x);
            if half_x >= rect.min.x && half_x <= rect.max.x {
                painter.line_segment(
                    [egui::pos2(half_x, rect.max.y - 8.0), egui::pos2(half_x, rect.max.y)],
                    Stroke::new(1.0, Color32::GRAY),
                );
            }
        }

        // Draw playhead
        let playhead_x = self.time_to_x(self.preview_time, rect.min.x);
        if playhead_x >= rect.min.x && playhead_x <= rect.max.x {
            painter.line_segment(
                [egui::pos2(playhead_x, rect.min.y), egui::pos2(playhead_x, rect.max.y)],
                Stroke::new(2.0, Color32::from_rgb(
                    self.style.playhead_color[0],
                    self.style.playhead_color[1],
                    self.style.playhead_color[2],
                )),
            );
        }

        // Ruler interaction
        let response = ui.interact(rect, ui.id().with("ruler"), egui::Sense::click_and_drag());
        if response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                self.preview_time = self.x_to_time(pos.x - rect.min.x).clamp(0.0, 24.0);
            }
        }
    }

    /// Draw timeline content (schedule entries)
    fn draw_timeline_content(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        let bg_color = Color32::from_rgb(
            self.style.timeline_bg[0],
            self.style.timeline_bg[1],
            self.style.timeline_bg[2],
        );
        painter.rect_filled(rect, 0.0, bg_color);

        // Get entries for current day
        let entries: Vec<ScheduleEntry> = if let Some(npc_id) = self.selected_npc {
            self.get_schedule_ref(npc_id)
                .map(|s| s.entries_for_day(self.selected_day).to_vec())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Draw hour grid lines
        for hour in 0..=24 {
            let x = self.time_to_x(hour as f32, rect.min.x);
            if x >= rect.min.x && x <= rect.max.x {
                painter.line_segment(
                    [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                    Stroke::new(1.0, Color32::from_gray(80)),
                );
            }
        }

        // Draw entries
        for (i, entry) in entries.iter().enumerate() {
            let entry_rect = self.entry_rect(&entry, rect);
            
            // Skip if off-screen
            if entry_rect.max.x < rect.min.x || entry_rect.min.x > rect.max.x {
                continue;
            }

            let is_selected = self.selected_entry == Some((self.selected_day, entry.id));
            let color = Color32::from_rgb(
                entry.entry_type.color()[0],
                entry.entry_type.color()[1],
                entry.entry_type.color()[2],
            );

            // Entry background
            let bg_color = if is_selected {
                color.linear_multiply(1.3)
            } else {
                color.linear_multiply(0.8)
            };
            
            let rounding = Rounding::same(4.0);
            painter.rect_filled(entry_rect, rounding, bg_color);

            // Selection border
            if is_selected {
                let select_color = Color32::from_rgb(
                    self.style.selection_color[0],
                    self.style.selection_color[1],
                    self.style.selection_color[2],
                );
                painter.rect_stroke(entry_rect, rounding, Stroke::new(2.0, select_color));
            }

            // Entry icon
            painter.text(
                egui::pos2(entry_rect.min.x + 4.0, entry_rect.center().y),
                egui::Align2::LEFT_CENTER,
                entry.entry_type.icon(),
                egui::FontId::proportional(14.0),
                Color32::WHITE,
            );

            // Entry text (activity name)
            if entry_rect.width() > 40.0 {
                let text = if entry_rect.width() > 100.0 {
                    format!("{}: {}", 
                        entry.entry_type.display_name(),
                        &entry.activity.chars().take(20).collect::<String>()
                    )
                } else {
                    entry.entry_type.display_name().to_string()
                };
                
                painter.text(
                    egui::pos2(entry_rect.min.x + 24.0, entry_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    text,
                    egui::FontId::proportional(11.0),
                    Color32::WHITE,
                );
            }

            // Draw resize handles if selected
            if is_selected {
                let handle_width = 6.0;
                let left_handle = Rect::from_min_size(
                    entry_rect.min,
                    Vec2::new(handle_width, entry_rect.height()),
                );
                let right_handle = Rect::from_min_size(
                    egui::pos2(entry_rect.max.x - handle_width, entry_rect.min.y),
                    Vec2::new(handle_width, entry_rect.height()),
                );
                
                painter.rect_filled(left_handle, rounding, Color32::WHITE.linear_multiply(0.5));
                painter.rect_filled(right_handle, rounding, Color32::WHITE.linear_multiply(0.5));
            }

            // Entry interaction
            let response = ui.interact(
                entry_rect,
                ui.id().with(("entry", entry.id)),
                egui::Sense::click_and_drag(),
            );

            if response.clicked() {
                self.selected_entry = Some((self.selected_day, entry.id));
                self.editing_entry = Some(entry.clone());
            }

            // Handle drag for moving
            if response.dragged() {
                if self.drag_state.is_none() {
                    // Check if dragging edge for resize
                    let handle_width = 6.0;
                    let drag_pos = response.drag_delta().x;
                    
                    if let Some(pos) = response.interact_pointer_pos() {
                        if (pos.x - entry_rect.min.x).abs() < handle_width {
                            self.drag_state = Some(DragState::Resizing {
                                entry_id: entry.id,
                                day: self.selected_day,
                                edge: ResizeEdge::Left,
                                original_start: entry.start_time,
                                original_end: entry.end_time,
                            });
                        } else if (pos.x - entry_rect.max.x).abs() < handle_width {
                            self.drag_state = Some(DragState::Resizing {
                                entry_id: entry.id,
                                day: self.selected_day,
                                edge: ResizeEdge::Right,
                                original_start: entry.start_time,
                                original_end: entry.end_time,
                            });
                        } else {
                            self.drag_state = Some(DragState::Moving {
                                entry_id: entry.id,
                                day: self.selected_day,
                                offset_x: drag_pos,
                            });
                        }
                    }
                }
            }
        }

        // Draw empty area interaction for creating new entries
        let empty_response = ui.interact(
            rect,
            ui.id().with("timeline_bg"),
            egui::Sense::click_and_drag(),
        );

        if empty_response.drag_started() {
            if let Some(pos) = empty_response.interact_pointer_pos() {
                let time = self.x_to_time(pos.x - rect.min.x);
                self.drag_state = Some(DragState::Creating {
                    start_time: time,
                    day: self.selected_day,
                    entry_type: self.current_entry_type,
                });
            }
        }

        // Draw creation preview
        if let Some(DragState::Creating { start_time, day, entry_type }) = self.drag_state {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let end_time = self.x_to_time(pos.x - rect.min.x).clamp(0.0, 24.0);
                let min_time = start_time.min(end_time);
                let max_time = start_time.max(end_time);
                
                let preview_rect = Rect::from_min_size(
                    egui::pos2(self.time_to_x(min_time, rect.min.x), rect.min.y + 10.0),
                    Vec2::new(
                        (max_time - min_time) * self.style.hour_width,
                        rect.height() - 20.0,
                    ),
                );
                
                let color = Color32::from_rgb(
                    entry_type.color()[0],
                    entry_type.color()[1],
                    entry_type.color()[2],
                );
                painter.rect_filled(preview_rect, Rounding::same(4.0), color.linear_multiply(0.5));
            }
        }
    }

    /// Draw the properties panel
    fn draw_properties_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Properties");
        ui.separator();

        // Property tabs
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.property_tab == PropertyTab::Entry, "Entry")
                .clicked()
            {
                self.property_tab = PropertyTab::Entry;
            }
            if ui
                .selectable_label(self.property_tab == PropertyTab::Overview, "Overview")
                .clicked()
            {
                self.property_tab = PropertyTab::Overview;
            }
            if ui
                .selectable_label(self.property_tab == PropertyTab::Path, "Path")
                .clicked()
            {
                self.property_tab = PropertyTab::Path;
            }
        });

        ui.separator();

        match self.property_tab {
            PropertyTab::Entry => self.draw_entry_properties(ui),
            PropertyTab::Overview => self.draw_overview_properties(ui),
            PropertyTab::Path => self.draw_path_properties(ui),
        }
    }

    /// Draw entry properties
    fn draw_entry_properties(&mut self, ui: &mut egui::Ui) {
        let Some((day, entry_id)) = self.selected_entry else {
            ui.label("Select an entry to edit its properties.");
            ui.label("");
            ui.label("Tip: Drag on the timeline to create a new entry.");
            return;
        };

        let Some(npc_id) = self.selected_npc else { return };

        // Get the entry for editing
        let schedule = self.get_schedule(npc_id);
        let entries = if schedule.uniform_schedule {
            schedule.uniform_entries.clone()
        } else {
            schedule.entries_for_day(day).to_vec()
        };

        let Some(entry) = entries.iter().find(|e| e.id == entry_id).cloned() else {
            self.selected_entry = None;
            return;
        };

        // Entry type
        ui.horizontal(|ui| {
            ui.label("Type:");
            egui::ComboBox::from_id_source("prop_entry_type")
                .selected_text(format!("{} {}", entry.entry_type.icon(), entry.entry_type.display_name()))
                .show_ui(ui, |ui| {
                    for entry_type in [
                        ScheduleEntryType::Work,
                        ScheduleEntryType::Sleep,
                        ScheduleEntryType::Eat,
                        ScheduleEntryType::Patrol,
                        ScheduleEntryType::Idle,
                        ScheduleEntryType::Custom,
                    ] {
                        if ui
                            .selectable_label(
                                entry.entry_type == entry_type,
                                format!("{} {}", entry_type.icon(), entry_type.display_name()),
                            )
                            .clicked()
                        {
                            self.update_entry_field(npc_id, day, entry_id, |e| {
                                e.entry_type = entry_type;
                                if e.activity == e.entry_type.display_name() {
                                    e.activity = entry_type.display_name().to_string();
                                }
                            });
                        }
                    }
                });
        });

        ui.separator();

        // Time settings
        ui.label("Time Range");
        
        let mut start_time = entry.start_time;
        let mut end_time = entry.end_time;
        
        ui.horizontal(|ui| {
            ui.label("Start:");
            let hours = start_time.floor() as i32;
            let minutes = ((start_time.fract() * 60.0).round() as i32).clamp(0, 59);
            ui.label(format!("{:02}:{:02}", hours, minutes));
        });
        
        if ui.add(egui::Slider::new(&mut start_time, 0.0..=24.0).step_by(0.25)).changed() {
            self.update_entry_field(npc_id, day, entry_id, |e| e.start_time = start_time);
        }

        ui.horizontal(|ui| {
            ui.label("End:");
            let hours = end_time.floor() as i32;
            let minutes = ((end_time.fract() * 60.0).round() as i32).clamp(0, 59);
            ui.label(format!("{:02}:{:02}", hours, minutes));
        });
        
        if ui.add(egui::Slider::new(&mut end_time, 0.0..=24.0).step_by(0.25)).changed() {
            self.update_entry_field(npc_id, day, entry_id, |e| e.end_time = end_time);
        }

        ui.separator();

        // Activity name
        let mut activity = entry.activity.clone();
        ui.horizontal(|ui| {
            ui.label("Activity:");
            if ui.text_edit_singleline(&mut activity).lost_focus() {
                self.update_entry_field(npc_id, day, entry_id, |e| e.activity = activity.clone());
            }
        });

        ui.separator();

        // Location
        ui.label("Location");
        
        let mut x = entry.location.x;
        let mut y = entry.location.y;
        let mut map_id = entry.location.map_id;

        ui.horizontal(|ui| {
            ui.label("Map:");
            ui.add(egui::DragValue::new(&mut map_id).speed(1));
        });
        
        if map_id != entry.location.map_id {
            self.update_entry_field(npc_id, day, entry_id, |e| e.location.map_id = map_id);
        }

        ui.horizontal(|ui| {
            ui.label("X:");
            if ui.add(egui::DragValue::new(&mut x).speed(1)).changed() {
                self.update_entry_field(npc_id, day, entry_id, |e| e.location.x = x);
            }
            ui.label("Y:");
            if ui.add(egui::DragValue::new(&mut y).speed(1)).changed() {
                self.update_entry_field(npc_id, day, entry_id, |e| e.location.y = y);
            }
        });

        if ui.button("📍 Pick from Map").clicked() {
            self.map_picker.active = true;
        }

        ui.separator();

        // Dialogue trigger
        let mut dialogue = entry.dialogue_trigger.clone().unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label("Dialogue:");
            if ui.text_edit_singleline(&mut dialogue).lost_focus() {
                self.update_entry_field(npc_id, day, entry_id, |e| {
                    e.dialogue_trigger = if dialogue.is_empty() { None } else { Some(dialogue.clone()) }
                });
            }
        });

        // Interruptible toggle
        let mut interruptible = entry.interruptible;
        if ui.checkbox(&mut interruptible, "Interruptible").changed() {
            self.update_entry_field(npc_id, day, entry_id, |e| e.interruptible = interruptible);
        }

        ui.separator();

        // Patrol waypoints (for patrol entries)
        if entry.entry_type == ScheduleEntryType::Patrol {
            ui.label("Patrol Waypoints");
            ui.label(format!("{} waypoints defined", entry.patrol_waypoints.len()));
            
            for (i, wp) in entry.patrol_waypoints.iter().enumerate() {
                ui.label(format!("{}: ({}, {})", i + 1, wp.x, wp.y));
            }

            if ui.button("➕ Add Waypoint").clicked() {
                // Would open map picker for waypoint selection
            }
        }

        ui.separator();

        // Delete button
        ui.vertical_centered(|ui| {
            if ui.button("🗑️ Delete Entry").clicked() {
                if let Some(schedule) = self.schedules.get_mut(&npc_id) {
                    schedule.remove_entry(day, entry_id);
                    self.selected_entry = None;
                    self.editing_entry = None;
                }
            }
        });
    }

    /// Draw overview properties
    fn draw_overview_properties(&mut self, ui: &mut egui::Ui) {
        let Some(npc_id) = self.selected_npc else {
            ui.label("Select an NPC to view schedule overview.");
            return;
        };

        let schedule = self.get_schedule(npc_id);

        ui.label("Schedule Overview");
        ui.separator();

        // Uniform schedule toggle
        let mut uniform = schedule.uniform_schedule;
        if ui.checkbox(&mut uniform, "Same schedule every day").changed() {
            schedule.uniform_schedule = uniform;
        }

        // Validation
        ui.separator();
        ui.label("Validation");
        
        let errors = schedule.validate();
        if errors.is_empty() {
            ui.colored_label(Color32::GREEN, "✓ No issues found");
        } else {
            ui.colored_label(Color32::YELLOW, format!("⚠ {} issues found", errors.len()));
            egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                for error in &errors {
                    ui.label(format!("• {}", error));
                }
            });
        }

        // Statistics
        ui.separator();
        ui.label("Statistics");
        
        let days = if schedule.uniform_schedule {
            1
        } else {
            7
        };
        
        let mut total_entries = 0;
        let mut total_hours = 0.0;
        
        for day in Weekday::all().iter().take(days) {
            let entries = schedule.entries_for_day(*day);
            total_entries += entries.len();
            for entry in entries {
                total_hours += entry.duration();
            }
        }

        ui.label(format!("Total entries: {}", total_entries));
        ui.label(format!("Avg entries/day: {:.1}", total_entries as f32 / days as f32));
        ui.label(format!("Scheduled hours: {:.1}/day", total_hours / days as f32));
    }

    /// Draw path properties
    fn draw_path_properties(&mut self, ui: &mut egui::Ui) {
        let Some(npc_id) = self.selected_npc else {
            ui.label("Select an NPC to view movement path.");
            return;
        };

        ui.label("Movement Path Preview");
        ui.separator();

        ui.checkbox(&mut self.show_path_preview, "Show path on map");
        
        ui.separator();

        // Path visualization
        let schedule = self.get_schedule_ref(npc_id);
        if let Some(schedule) = schedule {
            let entries = schedule.entries_for_day(self.selected_day);
            
            ui.label(format!("Entries for {}", self.selected_day.display_name()));
            
            for entry in entries {
                let color = Color32::from_rgb(
                    entry.entry_type.color()[0],
                    entry.entry_type.color()[1],
                    entry.entry_type.color()[2],
                );
                
                ui.horizontal(|ui| {
                    ui.colored_label(color, entry.entry_type.icon());
                    ui.label(format!(
                        "{:02.0}:{:02.0} - {:02.0}:{:02.0}: {} at ({}, {})",
                        entry.start_time.floor(),
                        (entry.start_time.fract() * 60.0).floor(),
                        entry.end_time.floor(),
                        (entry.end_time.fract() * 60.0).floor(),
                        entry.activity,
                        entry.location.x,
                        entry.location.y
                    ));
                });
            }
        }
    }

    /// Handle drag interactions
    fn handle_drag(&mut self, ui: &egui::Ui) {
        let input = ui.input(|i| (i.pointer.any_released(), i.pointer.latest_pos()));
        
        if input.0 {
            // Drag ended
            if let Some(drag_state) = self.drag_state.take() {
                match drag_state {
                    DragState::Creating { start_time, day, entry_type } => {
                        if let Some(pos) = input.1 {
                            let end_time = self.x_to_time(pos.x - self.scroll_offset)
                                .clamp(0.0, 24.0);
                            let min_time = start_time.min(end_time);
                            let max_time = start_time.max(end_time);
                            
                            // Only create if duration is at least 15 minutes
                            if max_time - min_time >= 0.25 {
                                self.create_entry(day, min_time, max_time, entry_type);
                            }
                        }
                    }
                    DragState::Moving { entry_id, day, .. } => {
                        // Entry has been moved via the update during drag
                        // Finalize any state if needed
                        let _ = entry_id;
                        let _ = day;
                    }
                    DragState::Resizing { entry_id, day, .. } => {
                        // Entry has been resized via the update during drag
                        let _ = entry_id;
                        let _ = day;
                    }
                    DragState::Panning { .. } => {}
                }
            }
        } else if let Some(ref mut drag_state) = self.drag_state {
            // Update during drag
            if let Some(pos) = input.1 {
                match drag_state {
                    DragState::Moving { entry_id, day, .. } => {
                        let new_time = self.x_to_time(pos.x - self.scroll_offset);
                        self.move_entry(*day, *entry_id, new_time);
                    }
                    DragState::Resizing { entry_id, day, edge, original_start, original_end } => {
                        let time = self.x_to_time(pos.x - self.scroll_offset).clamp(0.0, 24.0);
                        
                        if let Some(npc_id) = self.selected_npc {
                            self.update_entry_field(npc_id, *day, *entry_id, |entry| {
                                match edge {
                                    ResizeEdge::Left => {
                                        entry.start_time = time.min(*original_end - 0.25);
                                    }
                                    ResizeEdge::Right => {
                                        entry.end_time = time.max(*original_start + 0.25);
                                    }
                                }
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Create a new schedule entry
    fn create_entry(&mut self, day: Weekday, start: f32, end: f32, entry_type: ScheduleEntryType) {
        let Some(npc_id) = self.selected_npc else { return };
        
        let schedule = self.get_schedule(npc_id);
        let entry_id = EntryId::new().0;
        
        let mut entry = ScheduleEntry::new(entry_id, start, end, entry_type);
        
        // Set default location to NPC's current position if available
        if let Some(npc) = self.npcs.iter().find(|n| n.id == npc_id) {
            entry.location = ScheduleLocation::new(npc.position.x, npc.position.y, npc.map_id);
        }
        
        schedule.add_entry(day, entry);
        self.selected_entry = Some((day, entry_id));
    }

    /// Move an entry to a new start time
    fn move_entry(&mut self, day: Weekday, entry_id: u64, new_start: f32) {
        let Some(npc_id) = self.selected_npc else { return };
        
        if let Some(schedule) = self.schedules.get_mut(&npc_id) {
            let entries = if schedule.uniform_schedule {
                &mut schedule.uniform_entries
            } else {
                schedule.schedules.get_mut(&day).unwrap_or(&mut schedule.uniform_entries)
            };
            
            if let Some(entry) = entries.iter_mut().find(|e| e.id == entry_id) {
                let duration = entry.duration();
                entry.start_time = new_start.clamp(0.0, 24.0 - duration);
                entry.end_time = entry.start_time + duration;
                if entry.end_time > 24.0 {
                    entry.end_time = 24.0;
                }
            }
        }
    }

    /// Update a field of an entry
    fn update_entry_field<F>(&mut self, npc_id: u64, day: Weekday, entry_id: u64, mut f: F)
    where
        F: FnMut(&mut ScheduleEntry),
    {
        if let Some(schedule) = self.schedules.get_mut(&npc_id) {
            let entries = if schedule.uniform_schedule {
                &mut schedule.uniform_entries
            } else {
                schedule.schedules.get_mut(&day).unwrap_or(&mut schedule.uniform_entries)
            };
            
            if let Some(entry) = entries.iter_mut().find(|e| e.id == entry_id) {
                f(entry);
                self.editing_entry = Some(entry.clone());
            }
        }
    }

    /// Copy current day's schedule to all days
    fn copy_to_all_days(&mut self) {
        let Some(npc_id) = self.selected_npc else { return };
        
        let schedule = self.get_schedule(npc_id);
        let current_entries = schedule.entries_for_day(self.selected_day).to_vec();
        
        schedule.uniform_schedule = false;
        for day in Weekday::all() {
            if day != self.selected_day {
                schedule.copy_day(self.selected_day, day);
            }
        }
    }

    /// Clear all entries for the current day
    fn clear_current_day(&mut self) {
        let Some(npc_id) = self.selected_npc else { return };
        
        if let Some(schedule) = self.schedules.get_mut(&npc_id) {
            if schedule.uniform_schedule {
                schedule.uniform_entries.clear();
            } else {
                schedule.schedules.insert(self.selected_day, Vec::new());
            }
            self.selected_entry = None;
            self.editing_entry = None;
        }
    }

    /// Convert time to x coordinate
    fn time_to_x(&self, time: f32, offset: f32) -> f32 {
        offset + time * self.style.hour_width - self.scroll_offset
    }

    /// Convert x coordinate to time
    fn x_to_time(&self, x: f32) -> f32 {
        (x + self.scroll_offset) / self.style.hour_width
    }

    /// Get rectangle for an entry
    fn entry_rect(&self, entry: &ScheduleEntry, container: Rect) -> Rect {
        let x = self.time_to_x(entry.start_time, container.min.x);
        let width = entry.duration() * self.style.hour_width;
        
        Rect::from_min_size(
            egui::pos2(x, container.min.y + 10.0),
            Vec2::new(width.max(5.0), self.style.entry_height),
        )
    }

    /// Get the schedule for an NPC (immutable)
    pub fn get_npc_schedule(&self, npc_id: u64) -> Option<&NpcSchedule> {
        self.schedules.get(&npc_id)
    }

    /// Set the schedule for an NPC
    pub fn set_npc_schedule(&mut self, npc_id: u64, schedule: NpcSchedule) {
        self.schedules.insert(npc_id, schedule);
    }

    /// Export schedule to JSON
    pub fn export_to_json(&self, npc_id: u64) -> Option<String> {
        self.schedules.get(&npc_id)
            .and_then(|s| serde_json::to_string_pretty(s).ok())
    }

    /// Import schedule from JSON
    pub fn import_from_json(&mut self, npc_id: u64, json: &str) -> Result<(), serde_json::Error> {
        let schedule: NpcSchedule = serde_json::from_str(json)?;
        self.schedules.insert(npc_id, schedule);
        Ok(())
    }
}

impl Default for ScheduleEditor {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for Editor to integrate schedule editor
pub trait ScheduleEditorExt {
    /// Draw the NPC menu
    fn draw_npc_menu(&mut self, ui: &mut egui::Ui);
    /// Draw the schedule editor window
    fn draw_schedule_editor(&mut self, ctx: &egui::Context);
    /// Get mutable reference to schedule editor
    fn schedule_editor_mut(&mut self) -> &mut ScheduleEditor;
    /// Get reference to schedule editor
    fn schedule_editor(&self) -> &ScheduleEditor;
}

// Note: This would be implemented on the Editor struct in lib.rs
// The implementation is provided here for reference
/*
impl ScheduleEditorExt for Editor {
    fn draw_npc_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("NPC", |ui| {
            if ui.button("🗓️ Schedule Editor...").clicked() {
                self.schedule_editor.show();
                ui.close_menu();
            }
            // ... other NPC menu items
        });
    }

    fn draw_schedule_editor(&mut self, ctx: &egui::Context) {
        self.schedule_editor.draw(ctx);
    }

    fn schedule_editor_mut(&mut self) -> &mut ScheduleEditor {
        &mut self.schedule_editor
    }

    fn schedule_editor(&self) -> &ScheduleEditor {
        &self.schedule_editor
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_editor_new() {
        let editor = ScheduleEditor::new();
        assert!(!editor.is_visible());
        assert!(editor.selected_npc.is_none());
    }

    #[test]
    fn test_entry_id_generation() {
        let id1 = EntryId::new();
        let id2 = EntryId::new();
        assert_ne!(id1.0, id2.0);
    }

    #[test]
    fn test_npc_info() {
        let npc = NpcInfo {
            id: 1,
            name: "Test NPC".to_string(),
            map_id: 1,
            position: IVec2::new(5, 10),
        };
        assert_eq!(npc.name, "Test NPC");
        assert_eq!(npc.position.x, 5);
        assert_eq!(npc.position.y, 10);
    }

    #[test]
    fn test_time_conversions() {
        let editor = ScheduleEditor::new();
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(1000.0, 100.0));
        
        let x = editor.time_to_x(12.0, rect.min.x);
        let time = editor.x_to_time(x - rect.min.x);
        assert!((time - 12.0).abs() < 0.01);
    }

    #[test]
    fn test_add_npc() {
        let mut editor = ScheduleEditor::new();
        let npc = NpcInfo {
            id: 1,
            name: "Test".to_string(),
            map_id: 1,
            position: IVec2::ZERO,
        };
        editor.add_npc(npc);
        assert_eq!(editor.npcs.len(), 1);
        assert_eq!(editor.selected_npc, Some(1));
    }

    #[test]
    fn test_schedule_entry_creation() {
        let entry = ScheduleEntry::new(1, 8.0, 17.0, ScheduleEntryType::Work);
        assert_eq!(entry.start_time, 8.0);
        assert_eq!(entry.end_time, 17.0);
        assert_eq!(entry.duration(), 9.0);
        assert_eq!(entry.entry_type, ScheduleEntryType::Work);
    }

    #[test]
    fn test_schedule_entry_contains_time() {
        let entry = ScheduleEntry::new(1, 9.0, 17.0, ScheduleEntryType::Work);
        assert!(entry.contains_time(12.0));
        assert!(entry.contains_time(9.0));
        assert!(!entry.contains_time(17.0));
        assert!(!entry.contains_time(8.0));
    }

    #[test]
    fn test_schedule_entry_wraps_midnight() {
        let entry = ScheduleEntry::new(1, 22.0, 6.0, ScheduleEntryType::Sleep);
        assert!(entry.contains_time(23.0));
        assert!(entry.contains_time(2.0));
        assert!(!entry.contains_time(12.0));
        assert_eq!(entry.duration(), 8.0);
    }

    #[test]
    fn test_weekday_helpers() {
        assert!(Weekday::Saturday.is_weekend());
        assert!(Weekday::Sunday.is_weekend());
        assert!(!Weekday::Monday.is_weekend());
        assert!(Weekday::Monday.is_weekday());
    }
}
