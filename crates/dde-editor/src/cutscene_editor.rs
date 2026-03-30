//! Main Cutscene Editor Window
//!
//! Provides the complete cutscene editing interface with:
//! - Timeline editor (bottom panel)
//! - Preview viewport (top panel)
//! - Properties panel (right sidebar)
//! - Track controls (left sidebar)

use crate::timeline::{
    Cutscene, CutsceneLibrary, CutsceneData, CutsceneEvent,
    EasingFunction, Interpolation, PlaybackState, PreviewRenderer, TimelineEditor, Track, TrackType, TrackValue,
    export_to_events, TrackId,
};
use crate::timeline::keyframes::EffectType;
use dde_core::{Direction4, World};

/// Main cutscene editor window
pub struct CutsceneEditor {
    /// The cutscene library
    pub library: CutsceneLibrary,
    /// Currently open cutscene (if editing)
    pub current_cutscene: Option<Cutscene>,
    /// Timeline editor
    pub timeline: TimelineEditor,
    /// Preview renderer
    pub preview_renderer: PreviewRenderer,
    /// Editor state
    pub state: EditorState,
    /// UI layout state
    pub layout: LayoutState,
    /// Properties panel state
    pub properties: PropertiesPanel,
    /// Export settings
    pub export_settings: ExportSettings,
    /// Whether the editor is visible
    pub visible: bool,
}

/// Editor state
#[derive(Debug, Clone, Default)]
pub struct EditorState {
    /// Current playback state
    pub playback: PlaybackState,
    /// Selected track ID
    pub selected_track: Option<TrackId>,
    /// Current tool
    pub tool: EditorTool,
    /// Whether a cutscene is dirty (has unsaved changes)
    pub dirty: bool,
    /// Last saved timestamp
    pub last_saved: Option<std::time::SystemTime>,
}

/// Editor tools
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorTool {
    #[default]
    Select,
    Move,
    AddKeyframe,
    Delete,
}

/// Layout state for panels
#[derive(Debug, Clone)]
pub struct LayoutState {
    /// Preview panel height ratio (0.0-1.0)
    pub preview_ratio: f32,
    /// Properties panel width in pixels
    pub properties_width: f32,
    /// Track panel width in pixels
    pub track_panel_width: f32,
    /// Whether properties panel is visible
    pub show_properties: bool,
    /// Whether track panel is visible
    pub show_tracks: bool,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            preview_ratio: 0.5,
            properties_width: 300.0,
            track_panel_width: 200.0,
            show_properties: true,
            show_tracks: true,
        }
    }
}

/// Properties panel state
#[derive(Debug, Clone, Default)]
pub struct PropertiesPanel {
    /// Currently selected tab
    pub tab: PropertiesTab,
    /// Expanded sections
    pub expanded_sections: Vec<String>,
}

/// Properties tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PropertiesTab {
    #[default]
    Track,
    Keyframe,
    Cutscene,
    Export,
}

/// Export settings
#[derive(Debug, Clone)]
pub struct ExportSettings {
    /// Output directory
    pub output_dir: String,
    /// Frame rate for video export
    pub frame_rate: f32,
    /// Resolution width
    pub width: u32,
    /// Resolution height
    pub height: u32,
    /// Format
    pub format: ExportFormat,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            output_dir: String::from("./export"),
            frame_rate: 30.0,
            width: 1920,
            height: 1080,
            format: ExportFormat::PngSequence,
        }
    }
}

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    PngSequence,
    Json,
    Binary,
}

impl CutsceneEditor {
    /// Create a new cutscene editor
    pub fn new() -> Self {
        Self {
            library: CutsceneLibrary::new(),
            current_cutscene: None,
            timeline: TimelineEditor::new(),
            preview_renderer: PreviewRenderer::new(),
            state: EditorState::default(),
            layout: LayoutState::default(),
            properties: PropertiesPanel::default(),
            export_settings: ExportSettings::default(),
            visible: false,
        }
    }

    /// Open the editor
    pub fn open(&mut self) {
        self.visible = true;
    }

    /// Close the editor
    pub fn close(&mut self) {
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

    /// Create a new cutscene
    pub fn new_cutscene(&mut self, name: impl Into<String>) {
        let name = name.into();
        let mut cutscene = Cutscene::new(name);
        
        // Set default duration
        cutscene.timeline.duration = 30.0;
        
        // Add default camera track
        cutscene.timeline.add_track(TrackType::Camera, "Camera");
        
        self.current_cutscene = Some(cutscene);
        self.state.dirty = true;
        self.timeline = TimelineEditor::new();
        self.timeline.duration = 30.0;
        self.timeline.add_track(TrackType::Camera, "Camera");
    }

    /// Open an existing cutscene
    pub fn open_cutscene(&mut self, id: uuid::Uuid) {
        if let Some(cutscene) = self.library.get(id).cloned() {
            self.timeline = cutscene.timeline.clone();
            self.current_cutscene = Some(cutscene);
            self.state.dirty = false;
        }
    }

    /// Save current cutscene
    pub fn save_cutscene(&mut self) -> Option<uuid::Uuid> {
        if let Some(cutscene) = &mut self.current_cutscene {
            cutscene.timeline = self.timeline.clone();
            cutscene.touch();
            
            let id = cutscene.id;
            
            // Update in library
            if let Some(existing) = self.library.get_mut(id) {
                *existing = cutscene.clone();
            } else {
                self.library.add(cutscene.clone());
            }
            
            self.state.dirty = false;
            self.state.last_saved = Some(std::time::SystemTime::now());
            
            Some(id)
        } else {
            None
        }
    }

    /// Draw the cutscene editor UI
    pub fn draw(&mut self, ctx: &egui::Context, world: &World) {
        if !self.visible {
            return;
        }

        egui::Window::new("Cutscene Editor")
            .default_size([1200.0, 800.0])
            .show(ctx, |ui| {
                self.draw_menu_bar(ui);
                
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    egui::SidePanel::left("cutscene_tracks")
                        .default_width(self.layout.track_panel_width)
                        .resizable(true)
                        .show_inside(ui, |ui| {
                            self.draw_track_panel(ui);
                        });

                    egui::SidePanel::right("cutscene_properties")
                        .default_width(self.layout.properties_width)
                        .resizable(true)
                        .show_inside(ui, |ui| {
                            self.draw_properties_panel(ui);
                        });

                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        self.draw_main_content(ui, world);
                    });
                });
            });

        // Update preview if playing
        if self.timeline.playing {
            ctx.request_repaint();
        }
    }

    /// Draw menu bar
    fn draw_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New Cutscene").clicked() {
                    self.new_cutscene("Untitled Cutscene");
                    ui.close_menu();
                }
                if ui.button("Open...").clicked() {
                    // Would open file dialog
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Save").clicked() {
                    self.save_cutscene();
                    ui.close_menu();
                }
                if ui.button("Save As...").clicked() {
                    // Would open save dialog
                    ui.close_menu();
                }
            });

            ui.menu_button("Edit", |ui| {
                if ui.button("Undo").clicked() {
                    self.timeline.undo();
                    ui.close_menu();
                }
                if ui.button("Redo").clicked() {
                    self.timeline.redo();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Delete Selection").clicked() {
                    self.timeline.delete_selection();
                    ui.close_menu();
                }
            });

            ui.menu_button("Playback", |ui| {
                if ui.button("Play/Pause").clicked() {
                    self.timeline.toggle_playback();
                    ui.close_menu();
                }
                if ui.button("Stop").clicked() {
                    self.timeline.stop();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Loop: ").clicked() {
                    self.timeline.loop_playback = !self.timeline.loop_playback;
                    ui.close_menu();
                }
            });

            ui.menu_button("Export", |ui| {
                if ui.button("Export to Events").clicked() {
                    self.export_to_events();
                    ui.close_menu();
                }
                if ui.button("Export PNG Sequence...").clicked() {
                    // Would start video export
                    ui.close_menu();
                }
            });

            // Dirty indicator
            if self.state.dirty {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.colored_label(egui::Color32::YELLOW, "● Modified");
                });
            }
        });
    }

    /// Draw track panel (left sidebar)
    fn draw_track_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tracks");
        ui.separator();

        // Add track button
        ui.menu_button("➕ Add Track", |ui| {
            if ui.button("📷 Camera").clicked() {
                self.timeline.add_track(TrackType::Camera, "Camera");
                self.state.dirty = true;
                ui.close_menu();
            }
            if ui.button("👤 Entity").clicked() {
                self.timeline.add_track(TrackType::Entity, "Entity");
                self.state.dirty = true;
                ui.close_menu();
            }
            if ui.button("🔊 Audio").clicked() {
                self.timeline.add_track(TrackType::Audio, "Audio");
                self.state.dirty = true;
                ui.close_menu();
            }
            if ui.button("✨ Effect").clicked() {
                self.timeline.add_track(TrackType::Effect, "Effect");
                self.state.dirty = true;
                ui.close_menu();
            }
            if ui.button("💬 Dialogue").clicked() {
                self.timeline.add_track(TrackType::Dialogue, "Dialogue");
                self.state.dirty = true;
                ui.close_menu();
            }
        });

        ui.separator();

        // Track list - use index-based iteration to avoid borrow issues
        egui::ScrollArea::vertical().show(ui, |ui| {
            let track_count = self.timeline.tracks.len();
            for idx in 0..track_count {
                // Extract all the data we need from the track first
                let (track_id, is_selected, track_name, track_icon, track_color, is_muted, is_locked, is_visible, kf_count, duration) = {
                    let track = &self.timeline.tracks[idx];
                    (track.id, 
                     self.state.selected_track == Some(track.id),
                     track.name.clone(),
                     track.track_type.icon(),
                     track.color(),
                     track.muted,
                     track.locked,
                     track.visible,
                     track.keyframe_count(),
                     track.duration())
                };
                
                let color = egui::Color32::from_rgb(track_color[0], track_color[1], track_color[2]);

                egui::Frame::group(ui.style())
                    .fill(if is_selected {
                        ui.visuals().selection.bg_fill.linear_multiply(0.3)
                    } else {
                        ui.visuals().panel_fill
                    })
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Icon
                            ui.colored_label(color, track_icon);
                            
                            // Name
                            ui.label(&track_name);
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // Lock button - use response to trigger action
                                let lock_icon = if is_locked { "🔒" } else { "🔓" };
                                if ui.button(lock_icon).on_hover_text("Lock/Unlock").clicked() {
                                    if let Some(t) = self.timeline.tracks.iter_mut().find(|t| t.id == track_id) {
                                        t.toggle_lock();
                                    }
                                }
                                
                                // Mute button
                                let mute_icon = if is_muted { "🔇" } else { "🔊" };
                                if ui.button(mute_icon).on_hover_text("Mute/Unmute").clicked() {
                                    if let Some(t) = self.timeline.tracks.iter_mut().find(|t| t.id == track_id) {
                                        t.toggle_mute();
                                    }
                                }
                                
                                // Visibility button
                                let vis_icon = if is_visible { "👁" } else { "🚫" };
                                if ui.button(vis_icon).on_hover_text("Show/Hide").clicked() {
                                    if let Some(t) = self.timeline.tracks.iter_mut().find(|t| t.id == track_id) {
                                        t.visible = !t.visible;
                                    }
                                }
                            });
                        });
                        
                        // Keyframe count
                        ui.horizontal(|ui| {
                            ui.label(format!("{} keyframes", kf_count));
                            ui.label(format!("{:.1}s", duration));
                        });
                    });

                // Selection
                let response = ui.interact(
                    ui.min_rect(),
                    ui.id().with(track_id),
                    egui::Sense::click(),
                );
                
                if response.clicked() {
                    self.state.selected_track = Some(track_id);
                    self.properties.tab = PropertiesTab::Track;
                }
            }
        });
    }

    /// Draw a single track item (kept for potential future use)
    #[allow(dead_code)]
    fn draw_track_item(&mut self, ui: &mut egui::Ui, track: &mut Track) {
        let is_selected = self.state.selected_track == Some(track.id);
        
        let color = track.color();
        let color = egui::Color32::from_rgb(color[0], color[1], color[2]);

        egui::Frame::group(ui.style())
            .fill(if is_selected {
                ui.visuals().selection.bg_fill.linear_multiply(0.3)
            } else {
                ui.visuals().panel_fill
            })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Icon
                    ui.colored_label(color, track.track_type.icon());
                    
                    // Name (editable)
                    ui.label(&track.name);
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Lock button
                        let lock_icon = if track.locked { "🔒" } else { "🔓" };
                        if ui.button(lock_icon).on_hover_text("Lock/Unlock").clicked() {
                            track.toggle_lock();
                        }
                        
                        // Mute button
                        let mute_icon = if track.muted { "🔇" } else { "🔊" };
                        if ui.button(mute_icon).on_hover_text("Mute/Unmute").clicked() {
                            track.toggle_mute();
                        }
                        
                        // Visibility button
                        let vis_icon = if track.visible { "👁" } else { "🚫" };
                        if ui.button(vis_icon).on_hover_text("Show/Hide").clicked() {
                            track.visible = !track.visible;
                        }
                    });
                });
                
                // Keyframe count
                ui.horizontal(|ui| {
                    ui.label(format!("{} keyframes", track.keyframe_count()));
                    ui.label(format!("{:.1}s", track.duration()));
                });
            });

        // Selection
        let response = ui.interact(
            ui.min_rect(),
            ui.id().with(track.id),
            egui::Sense::click(),
        );
        
        if response.clicked() {
            self.state.selected_track = Some(track.id);
            self.properties.tab = PropertiesTab::Track;
        }
    }

    /// Draw properties panel (right sidebar)
    fn draw_properties_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Properties");
        ui.separator();

        // Tab bar
        ui.horizontal(|ui| {
            for (tab, name) in [
                (PropertiesTab::Track, "Track"),
                (PropertiesTab::Keyframe, "Keyframe"),
                (PropertiesTab::Cutscene, "Cutscene"),
                (PropertiesTab::Export, "Export"),
            ] {
                let selected = self.properties.tab == tab;
                if ui.selectable_label(selected, name).clicked() {
                    self.properties.tab = tab;
                }
            }
        });

        ui.separator();

        match self.properties.tab {
            PropertiesTab::Track => self.draw_track_properties(ui),
            PropertiesTab::Keyframe => self.draw_keyframe_properties(ui),
            PropertiesTab::Cutscene => self.draw_cutscene_properties(ui),
            PropertiesTab::Export => self.draw_export_properties(ui),
        }
    }

    /// Draw track properties
    fn draw_track_properties(&mut self, ui: &mut egui::Ui) {
        if let Some(track_id) = self.state.selected_track {
            if let Some(track) = self.timeline.get_track_mut(track_id) {
                ui.label("Track Properties");
                ui.separator();

                ui.label("Name:");
                ui.text_edit_singleline(&mut track.name);

                ui.label("Type: ");
                ui.label(track.track_type.display_name());

                ui.checkbox(&mut track.muted, "Muted");
                ui.checkbox(&mut track.locked, "Locked");
                ui.checkbox(&mut track.visible, "Visible");

                ui.separator();

                if ui.button("Clear All Keyframes").clicked() {
                    track.clear();
                    self.state.dirty = true;
                }

                if ui.button("Delete Track").clicked() {
                    self.timeline.remove_track(track_id);
                    self.state.selected_track = None;
                    self.state.dirty = true;
                }
            } else {
                ui.label("No track selected");
            }
        } else {
            ui.label("Select a track to edit properties");
        }
    }

    /// Draw keyframe properties
    fn draw_keyframe_properties(&mut self, ui: &mut egui::Ui) {
        // Extract what we need before borrowing
        let selection_info = self.timeline.selection.as_ref().and_then(|s| {
            if let crate::timeline::editor::Selection::Single { track_id, keyframe_index } = s {
                Some((*track_id, *keyframe_index))
            } else {
                None
            }
        });
        
        if let Some((track_id, keyframe_index)) = selection_info {
            // First pass: gather data from keyframe and draw UI elements that don't need self
            let value_needs_update = if let Some(track) = self.timeline.tracks.iter_mut().find(|t| t.id == track_id) {
                if let Some(keyframe) = track.keyframes.get_mut(keyframe_index) {
                    ui.label("Keyframe Properties");
                    ui.separator();

                    ui.label("Time:");
                    ui.add(egui::DragValue::new(&mut keyframe.time).speed(0.1));

                    ui.label("Interpolation:");
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{:?}", keyframe.interpolation))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut keyframe.interpolation, Interpolation::Step, "Step");
                            ui.selectable_value(&mut keyframe.interpolation, Interpolation::Linear, "Linear");
                            ui.selectable_value(
                                &mut keyframe.interpolation,
                                Interpolation::Bezier { control_in: 0.0, control_out: 1.0 },
                                "Bezier",
                            );
                        });

                    ui.label("Easing:");
                    egui::ComboBox::from_label("")
                        .selected_text(keyframe.easing.display_name())
                        .show_ui(ui, |ui| {
                            for easing in [
                                EasingFunction::Linear,
                                EasingFunction::EaseInQuad,
                                EasingFunction::EaseOutQuad,
                                EasingFunction::EaseInOutQuad,
                                EasingFunction::EaseInCubic,
                                EasingFunction::EaseOutElastic,
                            ] {
                                ui.selectable_value(&mut keyframe.easing, easing, easing.display_name());
                            }
                        });

                    // Value editor based on track type
                    ui.separator();
                    ui.label("Value:");
                    // Clone the value temporarily to avoid borrow issues
                    let mut value = keyframe.value.clone();
                    let value_ref = &mut value;
                    self.draw_value_editor(ui, value_ref);
                    Some((track_id, keyframe_index, value))
                } else {
                    None
                }
            } else {
                None
            };
            
            // Apply the value update if needed
            if let Some((tid, kidx, new_value)) = value_needs_update {
                if let Some(track) = self.timeline.tracks.iter_mut().find(|t| t.id == tid) {
                    if let Some(keyframe) = track.keyframes.get_mut(kidx) {
                        keyframe.value = new_value;
                    }
                }
            }
        } else {
            ui.label("Select a keyframe to edit properties");
        }
    }

    /// Draw value editor for keyframe
    fn draw_value_editor(&mut self, ui: &mut egui::Ui, value: &mut TrackValue) {
        match value {
            TrackValue::Camera(cam) => {
                ui.label("Position:");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut cam.position.x).prefix("X: ").speed(0.1));
                    ui.add(egui::DragValue::new(&mut cam.position.y).prefix("Y: ").speed(0.1));
                    ui.add(egui::DragValue::new(&mut cam.position.z).prefix("Z: ").speed(0.1));
                });
                ui.add(egui::Slider::new(&mut cam.zoom, 0.1..=5.0).text("Zoom"));
                ui.add(egui::Slider::new(&mut cam.rotation, -180.0..=180.0).text("Rotation"));
                ui.add(egui::Slider::new(&mut cam.shake_amount, 0.0..=1.0).text("Shake"));
                ui.add(egui::Slider::new(&mut cam.fade_alpha, 0.0..=1.0).text("Fade"));
            }
            TrackValue::Entity(ent) => {
                ui.label("Position:");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut ent.position.x).prefix("X: ").speed(0.1));
                    ui.add(egui::DragValue::new(&mut ent.position.y).prefix("Y: ").speed(0.1));
                    ui.add(egui::DragValue::new(&mut ent.position.z).prefix("Z: ").speed(0.1));
                });
                ui.checkbox(&mut ent.visible, "Visible");
                ui.label("Direction:");
                egui::ComboBox::from_label("")
                    .selected_text(format!("{:?}", ent.direction))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut ent.direction, Direction4::Down, "Down");
                        ui.selectable_value(&mut ent.direction, Direction4::Left, "Left");
                        ui.selectable_value(&mut ent.direction, Direction4::Right, "Right");
                        ui.selectable_value(&mut ent.direction, Direction4::Up, "Up");
                    });
            }
            TrackValue::Audio(audio) => {
                ui.add(egui::DragValue::new(&mut audio.audio_id).prefix("Audio ID: "));
                ui.add(egui::Slider::new(&mut audio.volume, 0.0..=1.0).text("Volume"));
                ui.add(egui::Slider::new(&mut audio.pitch, 0.5..=2.0).text("Pitch"));
            }
            TrackValue::Effect(effect) => {
                ui.label("Effect Type:");
                egui::ComboBox::from_label("")
                    .selected_text(format!("{:?}", effect.effect_type))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut effect.effect_type, EffectType::None, "None");
                        ui.selectable_value(&mut effect.effect_type, EffectType::Flash, "Flash");
                        ui.selectable_value(&mut effect.effect_type, EffectType::Shake, "Shake");
                        ui.selectable_value(&mut effect.effect_type, EffectType::Blur, "Blur");
                        ui.selectable_value(&mut effect.effect_type, EffectType::Bloom, "Bloom");
                        ui.selectable_value(&mut effect.effect_type, EffectType::Vignette, "Vignette");
                    });
                ui.add(egui::Slider::new(&mut effect.intensity, 0.0..=1.0).text("Intensity"));
            }
            TrackValue::Dialogue(dia) => {
                ui.label("Speaker:");
                ui.text_edit_singleline(&mut dia.speaker);
                ui.label("Text:");
                ui.text_edit_multiline(&mut dia.text);
                ui.checkbox(&mut dia.auto_advance, "Auto Advance");
                if dia.auto_advance {
                    ui.add(egui::DragValue::new(&mut dia.advance_delay_ms).prefix("Delay (ms): ").speed(100));
                }
            }
        }
    }

    /// Draw cutscene properties
    fn draw_cutscene_properties(&mut self, ui: &mut egui::Ui) {
        if let Some(cutscene) = &mut self.current_cutscene {
            ui.label("Cutscene Properties");
            ui.separator();

            ui.label("Name:");
            ui.text_edit_singleline(&mut cutscene.name);

            ui.label("Description:");
            ui.text_edit_multiline(&mut cutscene.description);

            ui.separator();

            ui.label("Duration:");
            ui.add(egui::DragValue::new(&mut self.timeline.duration).speed(1.0).suffix("s"));

            ui.checkbox(&mut self.timeline.loop_playback, "Loop Playback");

            ui.separator();

            ui.label("Tags:");
            for tag in &cutscene.tags {
                ui.label(format!("• {}", tag));
            }
        } else {
            ui.label("No cutscene open");
        }
    }

    /// Draw export properties
    fn draw_export_properties(&mut self, ui: &mut egui::Ui) {
        ui.label("Export Settings");
        ui.separator();

        ui.label("Format:");
        egui::ComboBox::from_label("")
            .selected_text(format!("{:?}", self.export_settings.format))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.export_settings.format, ExportFormat::PngSequence, "PNG Sequence");
                ui.selectable_value(&mut self.export_settings.format, ExportFormat::Json, "JSON");
                ui.selectable_value(&mut self.export_settings.format, ExportFormat::Binary, "Binary");
            });

        ui.label("Frame Rate:");
        ui.add(egui::DragValue::new(&mut self.export_settings.frame_rate).speed(1.0).suffix("fps"));

        ui.label("Resolution:");
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut self.export_settings.width).suffix("w"));
            ui.add(egui::DragValue::new(&mut self.export_settings.height).suffix("h"));
        });

        ui.label("Output Directory:");
        ui.text_edit_singleline(&mut self.export_settings.output_dir);

        ui.separator();

        if ui.button("Export Cutscene").clicked() {
            self.export_cutscene();
        }
    }

    /// Draw main content area
    fn draw_main_content(&mut self, ui: &mut egui::Ui, world: &World) {
        // Split into preview (top) and timeline (bottom)
        let available_height = ui.available_height();
        let preview_height = available_height * self.layout.preview_ratio;
        let timeline_height = available_height - preview_height;

        // Preview panel
        let preview_rect = ui.available_rect_before_wrap();
        let preview_rect = egui::Rect::from_min_size(
            preview_rect.min,
            egui::vec2(preview_rect.width(), preview_height - 4.0),
        );
        self.draw_preview_panel(ui, preview_rect, world);

        // Resize handle
        let handle_rect = egui::Rect::from_min_size(
            egui::pos2(preview_rect.min.x, preview_rect.max.y),
            egui::vec2(preview_rect.width(), 8.0),
        );
        let handle_response = ui.interact(handle_rect, ui.id().with("resize"), egui::Sense::drag());
        if handle_response.dragged() {
            let delta = handle_response.drag_delta().y;
            let new_ratio = (preview_height + delta) / available_height;
            self.layout.preview_ratio = new_ratio.clamp(0.1, 0.9);
        }
        ui.painter().rect_filled(handle_rect, 0.0, ui.visuals().widgets.inactive.bg_fill);

        // Timeline panel
        let timeline_rect = egui::Rect::from_min_size(
            egui::pos2(preview_rect.min.x, handle_rect.max.y),
            egui::vec2(preview_rect.width(), timeline_height - 4.0),
        );
        
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_min_size(timeline_rect.size());
            self.timeline.draw_ui(ui);
        });
    }

    /// Draw preview panel
    fn draw_preview_panel(&mut self, ui: &mut egui::Ui, rect: egui::Rect, world: &World) {
        let painter = ui.painter_at(rect);
        
        // Background
        painter.rect_filled(rect, 0.0, egui::Color32::BLACK);

        // Render preview
        if let Some(frame) = self.timeline.render_preview(world) {
            // Draw preview content
            // In a real implementation, this would render the actual scene
            // For now, draw placeholder text
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("Preview\nTime: {:.2}s\nCamera: {:.1}, {:.1}, {:.1}",
                    frame.time,
                    frame.camera.position.x,
                    frame.camera.position.y,
                    frame.camera.zoom
                ),
                egui::FontId::proportional(16.0),
                egui::Color32::WHITE,
            );

            // Draw effect overlay
            if frame.effect.effect_type != EffectType::None {
                let effect_color = egui::Color32::from_rgba_premultiplied(
                    (frame.effect.color[0] * 255.0) as u8,
                    (frame.effect.color[1] * 255.0) as u8,
                    (frame.effect.color[2] * 255.0) as u8,
                    (frame.effect.intensity * 100.0) as u8,
                );
                painter.rect_filled(rect, 0.0, effect_color);
            }
        } else {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No Preview",
                egui::FontId::proportional(20.0),
                egui::Color32::GRAY,
            );
        }

        // Preview controls overlay
        let controls_rect = egui::Rect::from_min_size(
            egui::pos2(rect.min.x + 10.0, rect.min.y + 10.0),
            egui::vec2(200.0, 30.0),
        );
        
        egui::Area::new(ui.id().with("preview_controls"))
            .fixed_pos(controls_rect.min)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    let play_text = if self.timeline.playing { "⏸" } else { "▶" };
                    if ui.button(play_text).clicked() {
                        self.timeline.toggle_playback();
                    }
                    if ui.button("⏹").clicked() {
                        self.timeline.stop();
                    }
                    ui.checkbox(&mut self.preview_renderer.show_debug, "Debug");
                    ui.checkbox(&mut self.preview_renderer.show_safe_frames, "Safe");
                });
            });
    }

    /// Export to game events
    fn export_to_events(&self) -> Vec<CutsceneEvent> {
        export_to_events(&self.timeline)
    }

    /// Export cutscene to file
    fn export_cutscene(&self) {
        match self.export_settings.format {
            ExportFormat::Json => {
                if let Some(cutscene) = &self.current_cutscene {
                    let data = CutsceneData::from_timeline(&self.timeline, &cutscene.name);
                    if let Ok(json) = data.to_json() {
                        // Would save to file
                        tracing::info!("Exported cutscene: {} bytes", json.len());
                    }
                }
            }
            ExportFormat::PngSequence => {
                // Would start video export
                tracing::info!("Starting PNG sequence export...");
            }
            ExportFormat::Binary => {
                // Would serialize to binary format
                tracing::info!("Exporting to binary format...");
            }
        }
    }

    /// Get current cutscene data
    pub fn current_cutscene_data(&self) -> Option<CutsceneData> {
        self.current_cutscene.as_ref()
            .map(|c| CutsceneData::from_timeline(&self.timeline, &c.name))
    }

    /// Check if there are unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.state.dirty
    }
}

impl Default for CutsceneEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::keyframes::CameraValue;

    #[test]
    fn test_cutscene_editor_new() {
        let editor = CutsceneEditor::new();
        assert!(!editor.visible);
        assert!(editor.current_cutscene.is_none());
    }

    #[test]
    fn test_cutscene_editor_open_close() {
        let mut editor = CutsceneEditor::new();
        editor.open();
        assert!(editor.visible);
        editor.close();
        assert!(!editor.visible);
    }

    #[test]
    fn test_new_cutscene() {
        let mut editor = CutsceneEditor::new();
        editor.new_cutscene("Test Cutscene");
        
        assert!(editor.current_cutscene.is_some());
        assert!(editor.state.dirty);
        assert_eq!(editor.timeline.tracks.len(), 1);
    }

    #[test]
    fn test_export_to_events() {
        let mut editor = CutsceneEditor::new();
        editor.new_cutscene("Test");
        
        // Add a keyframe
        let value = TrackValue::Camera(CameraValue::default());
        editor.timeline.tracks[0].add_keyframe(0.0, value.clone());
        editor.timeline.tracks[0].add_keyframe(5.0, value);
        
        let events = editor.export_to_events();
        assert!(!events.is_empty());
    }
}
