//! Replay Theater Panel
//!
//! Provides UI for:
//! - Replay library management
//! - Playback controls (play, pause, seek, speed)
//! - Replay recording
//! - Export/Import replays
//! - Thumbnail preview

use dde_core::replay::{
    PlayerInput, Replay, ReplayMetadata, ReplayPlayer, ReplayRecorder, ReplayState,
};
use dde_core::{World, WorldSerializer};
use egui::{Color32, ProgressBar, RichText, Ui};
use std::path::{Path, PathBuf};

/// Replay panel tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplayPanelTab {
    Library,
    Recording,
    Playback,
    Settings,
}

/// Replay library entry
#[derive(Debug, Clone)]
struct ReplayEntry {
    path: PathBuf,
    metadata: Option<ReplayMetadata>,
    file_size: u64,
}

/// Replay Theater Panel
pub struct ReplayPanel {
    current_tab: ReplayPanelTab,
    replay_directory: PathBuf,
    replay_library: Vec<ReplayEntry>,
    selected_replay: Option<usize>,
    player: Option<ReplayPlayer>,
    recorder: Option<ReplayRecorder>,
    state: ReplayState,
    recording_seed: u64,
    playback_speed: f32,
    seek_target: f32,
    status_message: Option<(String, bool)>,
    new_replay_name: String,
    new_replay_description: String,
    is_scrubbing: bool,
    show_entity_overlay: bool,
    frame_by_frame_mode: bool,
}

impl Default for ReplayPanel {
    fn default() -> Self {
        Self {
            current_tab: ReplayPanelTab::Library,
            replay_directory: PathBuf::from("replays"),
            replay_library: Vec::new(),
            selected_replay: None,
            player: None,
            recorder: None,
            state: ReplayState::Inactive,
            recording_seed: 0,
            playback_speed: 1.0,
            seek_target: 0.0,
            status_message: None,
            new_replay_name: "New Replay".to_string(),
            new_replay_description: String::new(),
            is_scrubbing: false,
            show_entity_overlay: true,
            frame_by_frame_mode: false,
        }
    }
}

impl ReplayPanel {
    /// Create with custom replay directory
    pub fn with_directory(replay_dir: impl Into<PathBuf>) -> Self {
        let replay_directory = replay_dir.into();
        let _ = std::fs::create_dir_all(&replay_directory);

        let mut panel = Self {
            replay_directory,
            ..Default::default()
        };

        panel.refresh_library();
        panel
    }

    /// Refresh the replay library
    pub fn refresh_library(&mut self) {
        self.replay_library.clear();

        if let Ok(entries) = std::fs::read_dir(&self.replay_directory) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "ddr") {
                    if let Ok(metadata) = entry.metadata() {
                        let file_size = metadata.len();
                        // Try to load metadata
                        let replay_metadata = Self::load_replay_metadata(&path);

                        self.replay_library.push(ReplayEntry {
                            path,
                            metadata: replay_metadata,
                            file_size,
                        });
                    }
                }
            }
        }

        // Sort by creation time (newest first)
        self.replay_library.sort_by(|a, b| {
            let a_time = a.metadata.as_ref().map(|m| m.created_at).unwrap_or(0);
            let b_time = b.metadata.as_ref().map(|m| m.created_at).unwrap_or(0);
            b_time.cmp(&a_time)
        });
    }

    /// Try to load just metadata from a replay file
    fn load_replay_metadata(path: &Path) -> Option<ReplayMetadata> {
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(replay) = Replay::from_bytes(&bytes) {
                return Some(replay.metadata);
            }
        }
        None
    }

    /// Set status message
    fn set_status(&mut self, message: impl Into<String>, is_error: bool) {
        self.status_message = Some((message.into(), is_error));
    }

    /// Draw the replay panel
    pub fn draw(&mut self, ui: &mut Ui, world: &World) {
        // Tab bar
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.current_tab == ReplayPanelTab::Library, "📚 Library")
                .clicked()
            {
                self.current_tab = ReplayPanelTab::Library;
                self.refresh_library();
            }
            if ui
                .selectable_label(self.current_tab == ReplayPanelTab::Recording, "⏺ Record")
                .clicked()
            {
                self.current_tab = ReplayPanelTab::Recording;
            }
            if ui
                .selectable_label(self.current_tab == ReplayPanelTab::Playback, "▶ Playback")
                .clicked()
            {
                self.current_tab = ReplayPanelTab::Playback;
            }
            if ui
                .selectable_label(self.current_tab == ReplayPanelTab::Settings, "⚙ Settings")
                .clicked()
            {
                self.current_tab = ReplayPanelTab::Settings;
            }
        });

        ui.separator();

        // Status message
        if let Some((msg, is_error)) = &self.status_message {
            let color = if *is_error {
                Color32::RED
            } else {
                Color32::GREEN
            };
            ui.label(RichText::new(msg).color(color));
            if ui.button("Clear").clicked() {
                self.status_message = None;
            }
            ui.separator();
        }

        // Tab content
        match self.current_tab {
            ReplayPanelTab::Library => self.draw_library_tab(ui),
            ReplayPanelTab::Recording => self.draw_recording_tab(ui, world),
            ReplayPanelTab::Playback => self.draw_playback_tab(ui),
            ReplayPanelTab::Settings => self.draw_settings_tab(ui),
        }
    }

    /// Draw library tab
    fn draw_library_tab(&mut self, ui: &mut Ui) {
        ui.heading("Replay Library");
        ui.add_space(10.0);

        if ui.button("🔄 Refresh").clicked() {
            self.refresh_library();
        }

        ui.label(format!(
            "Found {} replays in {:?}",
            self.replay_library.len(),
            self.replay_directory
        ));
        ui.add_space(10.0);

        if self.replay_library.is_empty() {
            ui.label("No replays found. Record some gameplay!");
        } else {
            // Collect all data first to avoid borrow issues
            let entries: Vec<_> = self
                .replay_library
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    let name = e
                        .metadata
                        .as_ref()
                        .map(|m| m.player_name.clone())
                        .unwrap_or_else(|| {
                            e.path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "Unknown".into())
                        });
                    let map = e
                        .metadata
                        .as_ref()
                        .map(|m| m.map_name.clone())
                        .unwrap_or_default();
                    let duration = e
                        .metadata
                        .as_ref()
                        .map(|m| m.formatted_duration())
                        .unwrap_or_default();
                    let time = e
                        .metadata
                        .as_ref()
                        .map(|m| m.formatted_time())
                        .unwrap_or_default();
                    let desc = e.metadata.as_ref().and_then(|m| m.description.clone());
                    let has_metadata = e.metadata.is_some();
                    (
                        i,
                        name,
                        map,
                        duration,
                        time,
                        desc,
                        e.file_size,
                        e.path.clone(),
                        has_metadata,
                    )
                })
                .collect();
            let selected = self.selected_replay;

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (index, name, map, duration, time, desc, file_size, path, has_metadata) in
                    entries
                {
                    let is_selected = selected == Some(index);

                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            // Selection radio
                            if ui.selectable_label(is_selected, "📹").clicked() {
                                self.selected_replay = Some(index);
                            }

                            // Replay info
                            ui.vertical(|ui| {
                                ui.label(RichText::new(name).strong());
                                if !map.is_empty() {
                                    ui.label(format!("Map: {}", map));
                                }
                                if !duration.is_empty() {
                                    ui.label(format!("Duration: {} | {}", duration, time));
                                }
                                if let Some(description) = desc {
                                    ui.label(RichText::new(description).italics());
                                }
                                ui.label(format!("Size: {} KB", file_size / 1024));
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("🗑 Delete").clicked() {
                                        if let Err(e) = std::fs::remove_file(&path) {
                                            self.set_status(format!("Delete failed: {}", e), true);
                                        } else {
                                            self.set_status("Replay deleted", false);
                                            self.refresh_library();
                                        }
                                    }

                                    if ui.button("📂 Load").clicked() && has_metadata {
                                        self.load_replay(index);
                                    }
                                },
                            );
                        });
                    });
                    ui.add_space(5.0);
                }
            });
        }
    }

    /// Load a replay for playback
    fn load_replay(&mut self, index: usize) {
        if let Some(entry) = self.replay_library.get(index) {
            match std::fs::read(&entry.path) {
                Ok(bytes) => match Replay::from_bytes(&bytes) {
                    Ok(replay) => {
                        self.player = Some(ReplayPlayer::new(replay));
                        self.state = ReplayState::Paused;
                        self.selected_replay = Some(index);
                        self.current_tab = ReplayPanelTab::Playback;
                        self.set_status("Replay loaded", false);
                    }
                    Err(e) => self.set_status(format!("Failed to parse replay: {}", e), true),
                },
                Err(e) => self.set_status(format!("Failed to read replay: {}", e), true),
            }
        }
    }

    /// Draw recording tab
    fn draw_recording_tab(&mut self, ui: &mut Ui, world: &World) {
        ui.heading("Record Replay");
        ui.add_space(10.0);

        match self.state {
            ReplayState::Inactive | ReplayState::Finished => {
                // Not recording - show start options
                ui.horizontal(|ui| {
                    ui.label("Player Name:");
                    ui.text_edit_singleline(&mut self.new_replay_name);
                });

                ui.horizontal(|ui| {
                    ui.label("Description:");
                    ui.text_edit_multiline(&mut self.new_replay_description);
                });

                ui.horizontal(|ui| {
                    ui.label("RNG Seed:");
                    ui.add(egui::DragValue::new(&mut self.recording_seed).speed(1));
                    if ui.button("Random").clicked() {
                        self.recording_seed = rand::random();
                    }
                });

                if ui.button("⏺ Start Recording").clicked() {
                    self.start_recording(world);
                }
            }
            ReplayState::Recording => {
                ui.label(RichText::new("🔴 RECORDING").color(Color32::RED).heading());

                if let Some(recorder) = &self.recorder {
                    ui.label(format!("Current tick: {}", recorder.current_tick()));
                }

                ui.horizontal(|ui| {
                    if ui.button("⏸ Pause").clicked() {
                        self.pause_recording();
                    }
                    if ui.button("⏹ Stop").clicked() {
                        self.stop_recording();
                    }
                });
            }
            ReplayState::Paused => {
                ui.label(RichText::new("⏸ PAUSED").color(Color32::YELLOW).heading());

                if let Some(recorder) = &self.recorder {
                    ui.label(format!("Current tick: {}", recorder.current_tick()));
                }

                ui.horizontal(|ui| {
                    if ui.button("▶ Resume").clicked() {
                        self.resume_recording();
                    }
                    if ui.button("⏹ Stop").clicked() {
                        self.stop_recording();
                    }
                });
            }
            _ => {
                ui.label("Recording not available during playback");
            }
        }
    }

    /// Start recording
    fn start_recording(&mut self, world: &World) {
        let snapshot = WorldSerializer::serialize(world, self.recording_seed, 0);
        let mut recorder = ReplayRecorder::new(self.recording_seed, snapshot);

        // Set metadata
        recorder.metadata_mut().player_name = self.new_replay_name.clone();
        recorder.metadata_mut().description = if self.new_replay_description.is_empty() {
            None
        } else {
            Some(self.new_replay_description.clone())
        };

        self.recorder = Some(recorder);
        self.state = ReplayState::Recording;
        self.set_status("Recording started", false);
    }

    /// Pause recording
    fn pause_recording(&mut self) {
        if let Some(recorder) = &mut self.recorder {
            recorder.pause();
        }
        self.state = ReplayState::Paused;
    }

    /// Resume recording
    fn resume_recording(&mut self) {
        if let Some(recorder) = &mut self.recorder {
            recorder.resume();
        }
        self.state = ReplayState::Recording;
    }

    /// Stop recording and save
    fn stop_recording(&mut self) {
        if let Some(recorder) = self.recorder.take() {
            let replay = recorder.finish();

            // Generate filename
            let filename = format!(
                "{}_{}.ddr",
                sanitize_filename(&self.new_replay_name),
                chrono::Local::now().format("%Y%m%d_%H%M%S")
            );
            let path = self.replay_directory.join(filename);

            match replay.to_bytes() {
                Ok(bytes) => match std::fs::write(&path, bytes) {
                    Ok(()) => {
                        self.set_status(format!("Saved replay to {:?}", path), false);
                        self.refresh_library();
                    }
                    Err(e) => self.set_status(format!("Failed to save: {}", e), true),
                },
                Err(e) => self.set_status(format!("Failed to serialize: {}", e), true),
            }
        }

        self.state = ReplayState::Finished;
    }

    /// Draw playback tab
    fn draw_playback_tab(&mut self, ui: &mut Ui) {
        ui.heading("Replay Playback");
        ui.add_space(10.0);

        if let Some(player) = &mut self.player {
            let replay = player.replay();

            // Metadata display
            ui.group(|ui| {
                ui.label(RichText::new(&replay.metadata.player_name).heading());
                ui.label(format!("Map: {}", replay.metadata.map_name));
                if let Some(desc) = &replay.metadata.description {
                    ui.label(desc);
                }
                ui.label(format!(
                    "Duration: {}",
                    replay.metadata.formatted_duration()
                ));
            });

            ui.add_space(10.0);

            // Progress bar
            let progress = player.progress();
            let current_tick = player.current_tick();
            let total_ticks = player.total_ticks();

            ui.label(format!(
                "Tick: {} / {} ({:.1}%)",
                current_tick,
                total_ticks,
                progress * 100.0
            ));

            // Scrubber
            let scrub_value = if self.is_scrubbing {
                self.seek_target
            } else {
                progress
            };

            let response = ui.add(
                ProgressBar::new(scrub_value)
                    .text(format!("{:.1}%", progress * 100.0))
                    .animate(player.is_playing()),
            );

            // Click to seek
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let rect = response.rect;
                    let new_progress = ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
                    let target_tick = (new_progress * total_ticks as f32) as u64;
                    let _ = player.seek_to(target_tick);
                }
            }

            ui.add_space(10.0);

            // Playback controls
            ui.horizontal(|ui| {
                // Play/Pause
                if player.is_playing() {
                    if ui.button("⏸ Pause").clicked() {
                        player.pause();
                    }
                } else if ui.button("▶ Play").clicked() {
                    player.play();
                }

                // Stop
                if ui.button("⏹ Stop").clicked() {
                    player.stop();
                }

                // Step controls
                if ui.button("⏮ Step Back").clicked() {
                    let _ = player.step_backward();
                }
                if ui.button("Step Forward ⏭").clicked() {
                    let _ = player.step_forward();
                }
            });

            // Speed control
            ui.horizontal(|ui| {
                ui.label("Speed:");
                if ui.button("0.5x").clicked() {
                    player.set_speed(0.5);
                }
                if ui.button("1x").clicked() {
                    player.set_speed(1.0);
                }
                if ui.button("2x").clicked() {
                    player.set_speed(2.0);
                }
                if ui.button("4x").clicked() {
                    player.set_speed(4.0);
                }

                ui.add(
                    egui::Slider::new(&mut self.playback_speed, 0.1..=10.0)
                        .text("x")
                        .logarithmic(true),
                );
                player.set_speed(self.playback_speed);
            });

            ui.add_space(10.0);

            // Display options
            ui.checkbox(&mut self.show_entity_overlay, "Show entity overlay");
            ui.checkbox(&mut self.frame_by_frame_mode, "Frame-by-frame mode");

            // Current inputs display
            if let Some(last_inputs) = player.next_tick() {
                if !last_inputs.is_empty() {
                    ui.add_space(10.0);
                    ui.label("Last Inputs:");
                    for input in last_inputs.iter().take(5) {
                        ui.label(format!("  • {:?}", input));
                    }
                }
            }

            if player.is_finished() {
                ui.add_space(10.0);
                ui.label(RichText::new("✓ Replay Finished").color(Color32::GREEN));
            }
        } else {
            ui.label("No replay loaded. Select one from the Library tab.");
        }
    }

    /// Draw settings tab
    fn draw_settings_tab(&mut self, ui: &mut Ui) {
        ui.heading("Replay Settings");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Replay Directory:");
            ui.label(self.replay_directory.to_string_lossy().as_ref());
        });

        if ui.button("🔄 Refresh Library").clicked() {
            self.refresh_library();
        }

        ui.add_space(10.0);
        ui.separator();

        // Total replays info
        let total_size: u64 = self.replay_library.iter().map(|e| e.file_size).sum();
        ui.label(format!("Total replays: {}", self.replay_library.len()));
        ui.label(format!("Total size: {} MB", total_size / 1_048_576));

        ui.add_space(10.0);
        ui.separator();

        // Danger zone
        ui.collapsing("⚠ Danger Zone", |ui| {
            if ui.button("Delete All Replays").clicked() {
                for entry in &self.replay_library {
                    let _ = std::fs::remove_file(&entry.path);
                }
                self.refresh_library();
                self.set_status("All replays deleted", false);
            }
        });
    }

    /// Get current player for external use
    pub fn player(&self) -> Option<&ReplayPlayer> {
        self.player.as_ref()
    }

    /// Get mutable player
    pub fn player_mut(&mut self) -> Option<&mut ReplayPlayer> {
        self.player.as_mut()
    }

    /// Check if recording
    pub fn is_recording(&self) -> bool {
        matches!(self.state, ReplayState::Recording)
    }

    /// Check if playing
    pub fn is_playing(&self) -> bool {
        self.player.as_ref().is_some_and(|p| p.is_playing())
    }

    /// Record inputs during recording
    pub fn record_inputs(&mut self, inputs: Vec<PlayerInput>) {
        if let Some(recorder) = &mut self.recorder {
            for input in inputs {
                recorder.record_input(input);
            }
            recorder.advance_tick();
        }
    }

    /// Get inputs during playback
    pub fn get_playback_inputs(&mut self) -> Option<Vec<PlayerInput>> {
        self.player.as_mut().and_then(|p| p.next_tick())
    }

    /// Get current state
    pub fn state(&self) -> ReplayState {
        self.state
    }
}

/// Sanitize string for use in filename
fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Hello World"), "Hello_World");
        assert_eq!(sanitize_filename("Test/Path"), "Test_Path");
        assert_eq!(sanitize_filename("File:Name"), "File_Name");
        assert_eq!(sanitize_filename("Valid-Name_123"), "Valid-Name_123");
    }

    #[test]
    fn test_replay_panel_default() {
        let panel = ReplayPanel::default();
        assert_eq!(panel.state, ReplayState::Inactive);
        assert!(panel.player.is_none());
        assert!(panel.recorder.is_none());
    }
}
