//! Main timeline editor UI
//!
//! Provides the timeline editing interface with tracks, keyframes, and playback controls.

use super::keyframes::{Keyframe, TrackId, TrackValue};
use super::preview::{PreviewFrame, PreviewRenderer};
use super::tracks::{Track, TrackGroup, TrackType};
use egui::Stroke;
use hecs::World;
use serde::{Deserialize, Serialize};

/// Main timeline editor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEditor {
    /// Current time (playhead position)
    pub playhead: f32,
    /// Total duration in seconds
    pub duration: f32,
    /// Zoom level (pixels per second)
    pub zoom: f32,
    /// All tracks
    pub tracks: Vec<Track>,
    /// Track groups
    pub groups: Vec<TrackGroup>,
    /// Selected keyframe/selection
    pub selection: Option<Selection>,
    /// Playback state
    pub playing: bool,
    /// Loop playback
    pub loop_playback: bool,
    /// Snap to grid
    pub snap_enabled: bool,
    /// Grid size in seconds
    pub snap_grid: f32,
    /// Time offset for scrolling
    pub scroll_offset: f32,
    /// Track list scroll offset
    pub track_scroll: f32,
    /// Show waveform for audio tracks
    pub show_waveforms: bool,
    /// Show keyframe values on timeline
    pub show_values: bool,
    /// Current tool mode
    pub tool: TimelineTool,
    /// Drag state
    #[serde(skip)]
    pub drag_state: Option<DragState>,
    /// Clipboard for copy/paste
    #[serde(skip)]
    pub clipboard: Option<Vec<Keyframe>>,
    /// Undo/redo history
    pub history: TimelineHistory,
    /// Preview renderer
    #[serde(skip)]
    pub preview_renderer: Option<PreviewRenderer>,
}

/// Current timeline tool
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimelineTool {
    /// Select and move keyframes
    Select,
    /// Add keyframes
    Add,
    /// Delete keyframes
    Delete,
    /// Move timeline view
    Hand,
}

impl TimelineTool {
    /// Get cursor icon for this tool
    pub fn cursor(&self) -> egui::CursorIcon {
        match self {
            TimelineTool::Select => egui::CursorIcon::Default,
            TimelineTool::Add => egui::CursorIcon::Crosshair,
            TimelineTool::Delete => egui::CursorIcon::NotAllowed,
            TimelineTool::Hand => egui::CursorIcon::Grab,
        }
    }
}

/// Selection state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Selection {
    /// Single keyframe selected
    Single {
        track_id: TrackId,
        keyframe_index: usize,
    },
    /// Multiple keyframes selected
    Multiple {
        keyframes: Vec<(TrackId, usize)>,
    },
    /// Track selected
    Track(TrackId),
    /// Time range selected
    TimeRange {
        start: f32,
        end: f32,
        track_ids: Vec<TrackId>,
    },
}

/// Drag state for UI interactions
#[derive(Debug, Clone)]
pub enum DragState {
    /// Dragging playhead
    Playhead { start_x: f32, start_time: f32 },
    /// Dragging keyframe
    Keyframe { track_id: TrackId, index: usize, start_time: f32 },
    /// Panning timeline view
    Pan { start_offset: f32, start_x: f32 },
    /// Selecting time range
    SelectRange { start_time: f32, track_id: TrackId },
    /// Resizing timeline
    ResizeTimeline { start_duration: f32, start_x: f32 },
}

/// Undo/redo history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineHistory {
    /// Past states for undo
    pub undo_stack: Vec<TimelineState>,
    /// Future states for redo
    pub redo_stack: Vec<TimelineState>,
    /// Maximum history size
    pub max_size: usize,
}

/// A snapshot of timeline state for undo/redo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineState {
    pub tracks: Vec<Track>,
    pub duration: f32,
    pub timestamp: std::time::SystemTime,
}

impl TimelineEditor {
    /// Create a new timeline editor
    pub fn new() -> Self {
        Self {
            playhead: 0.0,
            duration: 60.0,
            zoom: 50.0,
            tracks: Vec::new(),
            groups: Vec::new(),
            selection: None,
            playing: false,
            loop_playback: false,
            snap_enabled: true,
            snap_grid: 0.5,
            scroll_offset: 0.0,
            track_scroll: 0.0,
            show_waveforms: true,
            show_values: false,
            tool: TimelineTool::Select,
            drag_state: None,
            clipboard: None,
            history: TimelineHistory::new(50),
            preview_renderer: None,
        }
    }

    /// Create a new timeline with specified duration
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    /// Draw the timeline UI
    pub fn draw_ui(&mut self, ui: &mut egui::Ui) {
        // Save state before modifications for undo
        if self.drag_state.is_none() && ui.input(|i| i.pointer.any_pressed()) {
            self.save_state();
        }

        egui::TopBottomPanel::top("timeline_toolbar").show_inside(ui, |ui| {
            self.draw_toolbar(ui);
        });

        egui::SidePanel::left("timeline_tracks").show_inside(ui, |ui| {
            self.draw_track_headers(ui);
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.draw_timeline_lanes(ui);
        });

        // Handle drag state
        self.handle_drag(ui);

        // Update playback
        if self.playing {
            self.tick(ui.input(|i| i.unstable_dt));
        }
    }

    /// Draw the toolbar with transport controls
    fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Transport controls
            let play_text = if self.playing { "⏸" } else { "▶" };
            if ui.button(play_text).on_hover_text("Play/Pause (Space)").clicked() {
                self.toggle_playback();
            }

            if ui.button("⏹").on_hover_text("Stop (Esc)").clicked() {
                self.stop();
            }

            ui.separator();

            // Previous/Next keyframe
            if ui.button("⏮").on_hover_text("Previous Keyframe").clicked() {
                self.jump_to_previous_keyframe();
            }
            if ui.button("⏭").on_hover_text("Next Keyframe").clicked() {
                self.jump_to_next_keyframe();
            }

            ui.separator();

            // Loop toggle
            let loop_text = if self.loop_playback { "🔁 On" } else { "🔁 Off" };
            if ui.button(loop_text).on_hover_text("Toggle Loop").clicked() {
                self.loop_playback = !self.loop_playback;
            }

            ui.separator();

            // Time display
            ui.label(format!("{:.2} / {:.2}s", self.playhead, self.duration));

            ui.separator();

            // Tools
            ui.label("Tool:");
            for (tool, name) in [
                (TimelineTool::Select, "Select"),
                (TimelineTool::Add, "Add"),
                (TimelineTool::Delete, "Delete"),
                (TimelineTool::Hand, "Hand"),
            ] {
                let selected = self.tool == tool;
                if ui.selectable_label(selected, name).clicked() {
                    self.tool = tool;
                }
            }

            ui.separator();

            // Snap toggle
            let snap_text = if self.snap_enabled { "🧲 On" } else { "🧲 Off" };
            if ui.button(snap_text).on_hover_text("Toggle Snap").clicked() {
                self.snap_enabled = !self.snap_enabled;
            }

            ui.separator();

            // Zoom controls
            ui.label("Zoom:");
            if ui.button("-").clicked() {
                self.zoom = (self.zoom / 1.2).max(1.0);
            }
            ui.label(format!("{:.0}px/s", self.zoom));
            if ui.button("+").clicked() {
                self.zoom = (self.zoom * 1.2).min(500.0);
            }

            ui.separator();

            // Fit to duration
            if ui.button("Fit").on_hover_text("Fit timeline to view").clicked() {
                self.fit_to_view();
            }
        });
    }

    /// Draw track headers (left panel)
    fn draw_track_headers(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // Add track button
            ui.menu_button("➕ Add Track", |ui| {
                if ui.button("📷 Camera Track").clicked() {
                    self.add_track(TrackType::Camera, "Camera");
                    ui.close_menu();
                }
                if ui.button("👤 Entity Track").clicked() {
                    self.add_track(TrackType::Entity, "Entity");
                    ui.close_menu();
                }
                if ui.button("🔊 Audio Track").clicked() {
                    self.add_track(TrackType::Audio, "Audio");
                    ui.close_menu();
                }
                if ui.button("✨ Effect Track").clicked() {
                    self.add_track(TrackType::Effect, "Effect");
                    ui.close_menu();
                }
                if ui.button("💬 Dialogue Track").clicked() {
                    self.add_track(TrackType::Dialogue, "Dialogue");
                    ui.close_menu();
                }
            });

            ui.separator();

            // Track list
            let track_height = 40.0;
            let track_ids: Vec<_> = self.tracks.iter().map(|t| t.id).collect();
            for track_id in track_ids {
                let track = self.tracks.iter().find(|t| t.id == track_id).unwrap();
                let is_selected = matches!(self.selection.as_ref(), Some(Selection::Track(id)) if *id == track_id);
                let track_name = track.name.clone();
                let track_icon = track.track_type.icon();
                let track_color = track.color();
                let is_muted = track.muted;
                let is_locked = track.locked;
                
                let mut mute_clicked = false;
                let mut lock_clicked = false;
                
                let response = ui.horizontal(|ui| {
                    ui.set_min_height(track_height);
                    
                    // Track icon and name
                    let color = egui::Color32::from_rgb(track_color[0], track_color[1], track_color[2]);
                    
                    ui.colored_label(color, track_icon);
                    
                    let name_response = ui.selectable_label(
                        is_selected,
                        &track_name
                    );

                    // Mute button
                    let mute_text = if is_muted { "🔇" } else { "🔊" };
                    mute_clicked = ui.button(mute_text).clicked();

                    // Lock button
                    let lock_text = if is_locked { "🔒" } else { "🔓" };
                    lock_clicked = ui.button(lock_text).clicked();

                    name_response
                });

                if response.inner.clicked() {
                    self.selection = Some(Selection::Track(track_id));
                }
                
                // Handle button clicks
                if mute_clicked {
                    self.toggle_track_mute(track_id);
                }
                if lock_clicked {
                    self.toggle_track_lock(track_id);
                }
            }
        });
    }

    /// Draw timeline lanes (central panel)
    fn draw_timeline_lanes(&mut self, ui: &mut egui::Ui) {
        let available_rect = ui.available_rect_before_wrap();
        
        // Draw time ruler
        let ruler_height = 30.0;
        let ruler_rect = egui::Rect::from_min_size(
            available_rect.min,
            egui::vec2(available_rect.width(), ruler_height),
        );
        self.draw_time_ruler(ui, ruler_rect);

        // Draw track lanes
        let lanes_rect = egui::Rect::from_min_size(
            egui::pos2(available_rect.min.x, available_rect.min.y + ruler_height),
            egui::vec2(available_rect.width(), available_rect.height() - ruler_height),
        );
        self.draw_lanes(ui, lanes_rect);
    }

    /// Draw the time ruler
    fn draw_time_ruler(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let painter = ui.painter_at(rect);
        
        // Background
        painter.rect_filled(rect, 0.0, ui.visuals().panel_fill);

        // Time markers
        let start_time = self.scroll_offset;
        let end_time = self.time_at_x(rect.max.x);
        let major_interval = self.calculate_major_interval();
        let minor_interval = major_interval / 5.0;

        let mut t = (start_time / major_interval).floor() * major_interval;
        while t <= end_time {
            let x = self.x_at_time(t, rect.min.x);
            
            // Major tick
            if x >= rect.min.x && x <= rect.max.x {
                painter.line_segment(
                    [egui::pos2(x, rect.max.y - 15.0), egui::pos2(x, rect.max.y)],
                    Stroke::new(1.0, ui.visuals().text_color()),
                );
                
                // Time label
                painter.text(
                    egui::pos2(x + 2.0, rect.min.y + 2.0),
                    egui::Align2::LEFT_TOP,
                    format!("{:.1}", t),
                    egui::FontId::monospace(10.0),
                    ui.visuals().text_color(),
                );
            }

            // Minor ticks
            for i in 1..5 {
                let minor_t = t + minor_interval * i as f32;
                if minor_t > end_time {
                    break;
                }
                let minor_x = self.x_at_time(minor_t, rect.min.x);
                if minor_x >= rect.min.x && minor_x <= rect.max.x {
                    painter.line_segment(
                        [egui::pos2(minor_x, rect.max.y - 8.0), egui::pos2(minor_x, rect.max.y)],
                        Stroke::new(1.0, ui.visuals().noninteractive().weak_bg_fill),
                    );
                }
            }

            t += major_interval;
        }

        // Draw playhead
        let playhead_x = self.x_at_time(self.playhead, rect.min.x);
        if playhead_x >= rect.min.x && playhead_x <= rect.max.x {
            painter.line_segment(
                [egui::pos2(playhead_x, rect.min.y), egui::pos2(playhead_x, rect.max.y)],
                Stroke::new(1.0, egui::Color32::RED),
            );
            
            // Playhead handle
            painter.circle_filled(
                egui::pos2(playhead_x, rect.min.y + 10.0),
                6.0,
                egui::Color32::RED,
            );
        }

        // Interaction for scrubbing
        let response = ui.interact(rect, ui.id().with("ruler"), egui::Sense::click_and_drag());
        
        if response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                let time = self.time_at_x(pos.x - rect.min.x);
                self.seek(time);
            }
        }
    }

    /// Draw the track lanes
    fn draw_lanes(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let painter = ui.painter_at(rect);
        let lane_height = 40.0;
        
        // Draw each track lane
        for (i, track) in self.tracks.iter().enumerate() {
            let lane_y = rect.min.y + i as f32 * lane_height - self.track_scroll;
            
            if lane_y + lane_height < rect.min.y || lane_y > rect.max.y {
                continue; // Cull off-screen lanes
            }

            let lane_rect = egui::Rect::from_min_size(
                egui::pos2(rect.min.x, lane_y),
                egui::vec2(rect.width(), lane_height - 2.0),
            );

            // Lane background
            let bg_color = if i % 2 == 0 {
                ui.visuals().panel_fill
            } else {
                ui.visuals().extreme_bg_color
            };
            painter.rect_filled(lane_rect, 0.0, bg_color);

            // Draw keyframes
            if !track.collapsed {
                for (kf_idx, keyframe) in track.keyframes.iter().enumerate() {
                    let kf_x = self.x_at_time(keyframe.time, rect.min.x);
                    
                    if kf_x >= rect.min.x && kf_x <= rect.max.x {
                        let is_selected = matches!(
                            self.selection.as_ref(),
                            Some(Selection::Single { track_id, keyframe_index })
                            if *track_id == track.id && *keyframe_index == kf_idx
                        );

                        let kf_rect = egui::Rect::from_center_size(
                            egui::pos2(kf_x, lane_y + lane_height / 2.0),
                            egui::vec2(12.0, 12.0),
                        );

                        let color = if is_selected {
                            egui::Color32::YELLOW
                        } else {
                            let c = track.color();
                            egui::Color32::from_rgb(c[0], c[1], c[2])
                        };

                        // Draw diamond shape
                        painter.add(egui::Shape::convex_polygon(
                            vec![
                                egui::pos2(kf_x, kf_rect.min.y),
                                egui::pos2(kf_rect.max.x, lane_y + lane_height / 2.0),
                                egui::pos2(kf_x, kf_rect.max.y),
                                egui::pos2(kf_rect.min.x, lane_y + lane_height / 2.0),
                            ],
                            color,
                            egui::Stroke::new(1.0, ui.visuals().text_color()),
                        ));

                        // Handle interaction
                        let kf_response = ui.interact(kf_rect, ui.id().with((track.id, kf_idx)), egui::Sense::click());
                        
                        if kf_response.clicked() {
                            self.selection = Some(Selection::Single {
                                track_id: track.id,
                                keyframe_index: kf_idx,
                            });
                        }
                    }
                }
            }

            // Current value indicator
            if let Some(_value) = track.value_at(self.playhead) {
                let line_x = self.x_at_time(self.playhead, rect.min.x);
                if line_x >= rect.min.x && line_x <= rect.max.x {
                    painter.line_segment(
                        [egui::pos2(line_x, lane_y + 5.0), egui::pos2(line_x, lane_y + lane_height - 5.0)],
                        Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(255, 255, 255, 100)),
                    );
                }
            }
        }

        // Draw playhead line across all lanes
        let playhead_x = self.x_at_time(self.playhead, rect.min.x);
        if playhead_x >= rect.min.x && playhead_x <= rect.max.x {
            painter.line_segment(
                [egui::pos2(playhead_x, rect.min.y), egui::pos2(playhead_x, rect.max.y)],
                Stroke::new(1.0, egui::Color32::RED.linear_multiply(0.5)),
            );
        }
    }

    /// Calculate appropriate major interval based on zoom
    fn calculate_major_interval(&self) -> f32 {
        let pixel_interval = self.zoom;
        
        if pixel_interval >= 200.0 {
            0.5
        } else if pixel_interval >= 100.0 {
            1.0
        } else if pixel_interval >= 50.0 {
            2.0
        } else if pixel_interval >= 20.0 {
            5.0
        } else if pixel_interval >= 10.0 {
            10.0
        } else {
            30.0
        }
    }

    /// Convert time to x coordinate
    fn x_at_time(&self, time: f32, offset: f32) -> f32 {
        offset + (time - self.scroll_offset) * self.zoom
    }

    /// Convert x coordinate to time
    fn time_at_x(&self, x: f32) -> f32 {
        x / self.zoom + self.scroll_offset
    }

    #[allow(dead_code)]
    /// Snap time to grid if enabled
    fn snap_time(&self, time: f32) -> f32 {
        if self.snap_enabled {
            (time / self.snap_grid).round() * self.snap_grid
        } else {
            time
        }
    }

    /// Handle drag interactions
    fn handle_drag(&mut self, ui: &egui::Ui) {
        // Process drag state
        if let Some(DragState::Playhead { .. }) = &self.drag_state {
            if !ui.input(|i| i.pointer.primary_down()) {
                self.drag_state = None;
            }
        }
    }

    /// Add a new track
    pub fn add_track(&mut self, track_type: TrackType, name: &str) {
        let track = Track::new(track_type, name);
        self.tracks.push(track);
        self.save_state();
    }

    /// Remove a track
    pub fn remove_track(&mut self, track_id: TrackId) {
        self.tracks.retain(|t| t.id != track_id);
        self.save_state();
    }

    /// Toggle track mute
    pub fn toggle_track_mute(&mut self, track_id: TrackId) {
        if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) {
            track.toggle_mute();
        }
    }

    /// Toggle track lock
    pub fn toggle_track_lock(&mut self, track_id: TrackId) {
        if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) {
            track.toggle_lock();
        }
    }

    /// Delete selected keyframe
    pub fn delete_selection(&mut self) {
        match &self.selection {
            Some(Selection::Single { track_id, keyframe_index }) => {
                if let Some(track) = self.tracks.iter_mut().find(|t| t.id == *track_id) {
                    track.remove_keyframe(*keyframe_index);
                    self.selection = None;
                    self.save_state();
                }
            }
            Some(Selection::Multiple { keyframes }) => {
                // Sort by index in reverse to avoid index shifting issues
                let mut to_delete: Vec<_> = keyframes.clone();
                to_delete.sort_by(|a, b| b.1.cmp(&a.1));
                
                for (track_id, index) in to_delete {
                    if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) {
                        track.remove_keyframe(index);
                    }
                }
                self.selection = None;
                self.save_state();
            }
            _ => {}
        }
    }

    /// Move playhead to time
    pub fn seek(&mut self, time: f32) {
        self.playhead = time.clamp(0.0, self.duration);
    }

    /// Update during playback
    pub fn tick(&mut self, dt: f32) {
        if self.playing {
            self.playhead += dt;
            
            if self.playhead > self.duration {
                if self.loop_playback {
                    self.playhead %= self.duration;
                } else {
                    self.playhead = self.duration;
                    self.playing = false;
                }
            }
        }
    }

    /// Toggle playback
    pub fn toggle_playback(&mut self) {
        self.playing = !self.playing;
    }

    /// Stop playback and reset to start
    pub fn stop(&mut self) {
        self.playing = false;
        self.playhead = 0.0;
    }

    /// Jump to previous keyframe
    pub fn jump_to_previous_keyframe(&mut self) {
        let mut prev_time: Option<f32> = None;
        
        for track in &self.tracks {
            if let Some(index) = track.keyframe_index_at(self.playhead) {
                if index > 0 {
                    let time = track.keyframes[index - 1].time;
                    if prev_time.is_none() || time > prev_time.unwrap() {
                        prev_time = Some(time);
                    }
                }
            }
        }
        
        if let Some(time) = prev_time {
            self.seek(time);
        }
    }

    /// Jump to next keyframe
    pub fn jump_to_next_keyframe(&mut self) {
        let mut next_time: Option<f32> = None;
        
        for track in &self.tracks {
            if let Some(index) = track.next_keyframe_index(self.playhead) {
                let time = track.keyframes[index].time;
                if next_time.is_none() || time < next_time.unwrap() {
                    next_time = Some(time);
                }
            }
        }
        
        if let Some(time) = next_time {
            self.seek(time);
        }
    }

    /// Fit timeline to view
    pub fn fit_to_view(&mut self) {
        // Calculate required zoom to fit duration
        // This would need actual view dimensions
        // For now, just reset scroll
        self.scroll_offset = 0.0;
    }

    /// Save current state to history
    fn save_state(&mut self) {
        let state = TimelineState {
            tracks: self.tracks.clone(),
            duration: self.duration,
            timestamp: std::time::SystemTime::now(),
        };
        self.history.push_undo(state);
    }

    /// Undo last change
    pub fn undo(&mut self) {
        if let Some(state) = self.history.pop_undo() {
            // Save current state to redo stack
            let current = TimelineState {
                tracks: self.tracks.clone(),
                duration: self.duration,
                timestamp: std::time::SystemTime::now(),
            };
            self.history.push_redo(current);
            
            // Restore state
            self.tracks = state.tracks;
            self.duration = state.duration;
        }
    }

    /// Redo last undone change
    pub fn redo(&mut self) {
        if let Some(state) = self.history.pop_redo() {
            // Save current state to undo stack
            let current = TimelineState {
                tracks: self.tracks.clone(),
                duration: self.duration,
                timestamp: std::time::SystemTime::now(),
            };
            self.history.push_undo(current);
            
            // Restore state
            self.tracks = state.tracks;
            self.duration = state.duration;
        }
    }

    /// Render preview at current playhead
    pub fn render_preview(&mut self, world: &World) -> Option<PreviewFrame> {
        if self.preview_renderer.is_none() {
            self.preview_renderer = Some(PreviewRenderer::new());
        }
        
        // Extract what we need before the mutable borrow
        let playhead = self.playhead;
        let tracks_clone = self.tracks.clone();
        let duration = self.duration;
        let zoom = self.zoom;
        let groups = self.groups.clone();
        let selection = self.selection.clone();
        let loop_playback = self.loop_playback;
        let snap_enabled = self.snap_enabled;
        let snap_grid = self.snap_grid;
        let scroll_offset = self.scroll_offset;
        let track_scroll = self.track_scroll;
        let show_waveforms = self.show_waveforms;
        let show_values = self.show_values;
        let tool = self.tool;
        let history = self.history.clone();
        
        if let Some(renderer) = &mut self.preview_renderer {
            // Use a temporary timeline for rendering to avoid borrow issues
            let temp_timeline = TimelineEditor {
                playhead,
                duration,
                zoom,
                tracks: tracks_clone,
                groups,
                selection,
                playing: false,
                loop_playback,
                snap_enabled,
                snap_grid,
                scroll_offset,
                track_scroll,
                show_waveforms,
                show_values,
                tool,
                drag_state: None,
                clipboard: None,
                history,
                preview_renderer: None,
            };
            Some(renderer.render(&temp_timeline, world))
        } else {
            None
        }
    }

    /// Get track by ID
    pub fn get_track(&self, id: TrackId) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == id)
    }

    /// Get track by ID (mutable)
    pub fn get_track_mut(&mut self, id: TrackId) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|t| t.id == id)
    }

    /// Get all values at current playhead time
    pub fn sample_at_playhead(&self) -> Vec<(TrackId, TrackValue)> {
        self.tracks.iter()
            .filter(|t| !t.muted)
            .filter_map(|t| t.value_at(self.playhead).map(|v| (t.id, v)))
            .collect()
    }
}

impl Default for TimelineEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TimelineHistory {
    fn default() -> Self {
        Self::new(50)
    }
}

impl TimelineHistory {
    /// Create new history with max size
    pub fn new(max_size: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_size,
        }
    }

    /// Push state to undo stack
    pub fn push_undo(&mut self, state: TimelineState) {
        if self.undo_stack.len() >= self.max_size {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(state);
        // Clear redo stack on new change
        self.redo_stack.clear();
    }

    /// Push state to redo stack
    pub fn push_redo(&mut self, state: TimelineState) {
        if self.redo_stack.len() >= self.max_size {
            self.redo_stack.remove(0);
        }
        self.redo_stack.push(state);
    }

    /// Pop from undo stack
    pub fn pop_undo(&mut self) -> Option<TimelineState> {
        self.undo_stack.pop()
    }

    /// Pop from redo stack
    pub fn pop_redo(&mut self) -> Option<TimelineState> {
        self.redo_stack.pop()
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use super::super::keyframes::CameraValue;

    #[test]
    fn test_timeline_new() {
        let timeline = TimelineEditor::new();
        assert_eq!(timeline.playhead, 0.0);
        assert_eq!(timeline.duration, 60.0);
        assert!(!timeline.playing);
    }

    #[test]
    fn test_timeline_seek() {
        let mut timeline = TimelineEditor::new();
        timeline.seek(30.0);
        assert_eq!(timeline.playhead, 30.0);
        
        // Test clamping
        timeline.seek(100.0);
        assert_eq!(timeline.playhead, 60.0);
        
        timeline.seek(-10.0);
        assert_eq!(timeline.playhead, 0.0);
    }

    #[test]
    fn test_timeline_playback() {
        let mut timeline = TimelineEditor::with_duration(TimelineEditor::new(), 10.0);
        timeline.playing = true;
        
        timeline.tick(1.0);
        assert_eq!(timeline.playhead, 1.0);
        
        // Test loop
        timeline.loop_playback = true;
        timeline.playhead = 9.5;
        timeline.tick(1.0);
        assert_eq!(timeline.playhead, 0.5);
        
        // Test stop at end
        timeline.loop_playback = false;
        timeline.playhead = 9.5;
        timeline.playing = true;
        timeline.tick(1.0);
        assert_eq!(timeline.playhead, 10.0);
        assert!(!timeline.playing);
    }

    #[test]
    fn test_add_track() {
        let mut timeline = TimelineEditor::new();
        timeline.add_track(TrackType::Camera, "Camera 1");
        assert_eq!(timeline.tracks.len(), 1);
        assert_eq!(timeline.tracks[0].track_type, TrackType::Camera);
    }

    #[test]
    fn test_delete_selection() {
        let mut timeline = TimelineEditor::new();
        timeline.add_track(TrackType::Camera, "Camera");
        
        let track_id = timeline.tracks[0].id;
        let value = TrackValue::Camera(CameraValue::default());
        timeline.tracks[0].add_keyframe(0.0, value);
        
        timeline.selection = Some(Selection::Single {
            track_id,
            keyframe_index: 0,
        });
        
        timeline.delete_selection();
        assert!(timeline.tracks[0].is_empty());
        assert!(timeline.selection.is_none());
    }

    #[test]
    fn test_snap_time() {
        let mut timeline = TimelineEditor::new();
        timeline.snap_enabled = true;
        timeline.snap_grid = 0.5;
        
        assert_eq!(timeline.snap_time(0.3), 0.5);
        assert_eq!(timeline.snap_time(0.6), 0.5);
        assert_eq!(timeline.snap_time(0.8), 1.0);
        
        timeline.snap_enabled = false;
        assert_eq!(timeline.snap_time(0.3), 0.3);
    }

    #[test]
    fn test_history() {
        let mut history = TimelineHistory::new(10);
        
        let state1 = TimelineState {
            tracks: vec![],
            duration: 10.0,
            timestamp: std::time::SystemTime::now(),
        };
        
        history.push_undo(state1);
        assert!(history.can_undo());
        
        let restored = history.pop_undo().unwrap();
        assert_eq!(restored.duration, 10.0);
        assert!(!history.can_undo());
    }
}
