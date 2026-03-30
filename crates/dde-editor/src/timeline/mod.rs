//! Cutscene Timeline Editor
//!
//! A timeline-based editor for creating cinematic cutscenes (like Adobe Premiere for RPGs).
//!
//! # Features
//!
//! - **Multi-track timeline**: Camera, Entity, Audio, Effect, and Dialogue tracks
//! - **Keyframe interpolation**: Linear, Step, and Bezier interpolation with easing functions
//! - **Real-time preview**: See your cutscene play out as you edit
//! - **Export to events**: Convert timelines to game runtime events
//! - **Video export**: Export to PNG sequence for marketing videos
//!
//! # Example
//!
//! ```rust
//! use dde_editor::timeline::{TimelineEditor, TrackType, TrackValue};
//! use dde_editor::timeline::keyframes::{CameraValue, Keyframe};
//!
//! // Create a new timeline
//! let mut timeline = TimelineEditor::new();
//!
//! // Add a camera track
//! timeline.add_track(TrackType::Camera, "Main Camera");
//!
//! // Add keyframes
//! if let Some(track) = timeline.tracks.first_mut() {
//!     let value = TrackValue::Camera(CameraValue::default());
//!     track.add_keyframe(0.0, value.clone());
//!     track.add_keyframe(5.0, value);
//! }
//! ```

pub mod editor;
pub mod export;
pub mod keyframes;
pub mod preview;
pub mod tracks;

pub use editor::TimelineEditor;
pub use export::{export_to_events, CutsceneData, CutsceneEvent, ExportOptions};
pub use keyframes::{EasingFunction, Interpolation, Keyframe, TrackId, TrackValue};
pub use preview::{PreviewCamera, PreviewFrame, PreviewRenderer};
pub use tracks::{Track, TrackGroup, TrackType};

use serde::{Deserialize, Serialize};

/// A complete cutscene definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cutscene {
    /// Unique identifier
    pub id: uuid::Uuid,
    /// Human-readable name
    pub name: String,
    /// Description of the cutscene
    pub description: String,
    /// Timeline data
    pub timeline: TimelineEditor,
    /// Tags for organization
    pub tags: Vec<String>,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Modified timestamp
    pub modified_at: chrono::DateTime<chrono::Utc>,
}

impl Cutscene {
    /// Create a new cutscene
    pub fn new(name: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            description: String::new(),
            timeline: TimelineEditor::new(),
            tags: Vec::new(),
            created_at: now,
            modified_at: now,
        }
    }

    /// Create with specific duration
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.timeline.duration = duration;
        self
    }

    /// Mark as modified
    pub fn touch(&mut self) {
        self.modified_at = chrono::Utc::now();
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        let tag = tag.into();
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Remove a tag
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Import from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Get duration in seconds
    pub fn duration(&self) -> f32 {
        self.timeline.duration
    }

    /// Get track count by type
    pub fn track_counts(&self) -> std::collections::HashMap<TrackType, usize> {
        let mut counts = std::collections::HashMap::new();
        for track in &self.timeline.tracks {
            *counts.entry(track.track_type).or_insert(0) += 1;
        }
        counts
    }

    /// Get total keyframe count
    pub fn total_keyframes(&self) -> usize {
        self.timeline
            .tracks
            .iter()
            .map(|t| t.keyframe_count())
            .sum()
    }

    /// Check if cutscene is empty (no tracks)
    pub fn is_empty(&self) -> bool {
        self.timeline.tracks.is_empty()
    }
}

impl Default for Cutscene {
    fn default() -> Self {
        Self::new("Untitled Cutscene")
    }
}

/// Cutscene library for managing multiple cutscenes
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CutsceneLibrary {
    /// All cutscenes
    pub cutscenes: Vec<Cutscene>,
    /// Currently selected cutscene ID
    pub selected_id: Option<uuid::Uuid>,
}

impl CutsceneLibrary {
    /// Create a new empty library
    pub fn new() -> Self {
        Self {
            cutscenes: Vec::new(),
            selected_id: None,
        }
    }

    /// Add a cutscene
    pub fn add(&mut self, cutscene: Cutscene) {
        self.cutscenes.push(cutscene);
    }

    /// Remove a cutscene by ID
    pub fn remove(&mut self, id: uuid::Uuid) -> Option<Cutscene> {
        if let Some(pos) = self.cutscenes.iter().position(|c| c.id == id) {
            let removed = self.cutscenes.remove(pos);
            if self.selected_id == Some(id) {
                self.selected_id = None;
            }
            Some(removed)
        } else {
            None
        }
    }

    /// Get a cutscene by ID
    pub fn get(&self, id: uuid::Uuid) -> Option<&Cutscene> {
        self.cutscenes.iter().find(|c| c.id == id)
    }

    /// Get a cutscene by ID (mutable)
    pub fn get_mut(&mut self, id: uuid::Uuid) -> Option<&mut Cutscene> {
        self.cutscenes.iter_mut().find(|c| c.id == id)
    }

    /// Get the selected cutscene
    pub fn selected(&self) -> Option<&Cutscene> {
        self.selected_id.and_then(|id| self.get(id))
    }

    /// Get the selected cutscene (mutable)
    pub fn selected_mut(&mut self) -> Option<&mut Cutscene> {
        self.selected_id.and_then(|id| self.get_mut(id))
    }

    /// Select a cutscene
    pub fn select(&mut self, id: uuid::Uuid) {
        if self.cutscenes.iter().any(|c| c.id == id) {
            self.selected_id = Some(id);
        }
    }

    /// Create a new cutscene and select it
    pub fn create_new(&mut self, name: impl Into<String>) -> uuid::Uuid {
        let cutscene = Cutscene::new(name);
        let id = cutscene.id;
        self.cutscenes.push(cutscene);
        self.selected_id = Some(id);
        id
    }

    /// Get all cutscene names
    pub fn names(&self) -> Vec<&str> {
        self.cutscenes.iter().map(|c| c.name.as_str()).collect()
    }

    /// Search cutscenes by name or tag
    pub fn search(&self, query: &str) -> Vec<&Cutscene> {
        let query = query.to_lowercase();
        self.cutscenes
            .iter()
            .filter(|c| {
                c.name.to_lowercase().contains(&query)
                    || c.tags.iter().any(|t| t.to_lowercase().contains(&query))
            })
            .collect()
    }

    /// Sort cutscenes by name
    pub fn sort_by_name(&mut self) {
        self.cutscenes.sort_by(|a, b| a.name.cmp(&b.name));
    }

    /// Sort cutscenes by modification date
    pub fn sort_by_modified(&mut self) {
        self.cutscenes
            .sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    }

    /// Export library to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Import library from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Timeline playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackState {
    /// Stopped at beginning
    #[default]
    Stopped,
    /// Playing
    Playing,
    /// Paused
    Paused,
    /// Scrubbing (dragging playhead)
    Scrubbing,
    /// Previewing single frame
    Previewing,
}

impl PlaybackState {
    /// Check if timeline is currently playing
    pub fn is_playing(&self) -> bool {
        matches!(self, PlaybackState::Playing)
    }

    /// Check if timeline is active (playing or previewing)
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            PlaybackState::Playing | PlaybackState::Scrubbing | PlaybackState::Previewing
        )
    }
}

/// Time display format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeFormat {
    /// Seconds with decimals (0.00)
    Seconds,
    /// SMPTE format (00:00:00:00)
    Smpte { fps: f32 },
    /// Frames
    Frames { fps: f32 },
}

impl TimeFormat {
    /// Format time as string
    pub fn format(&self, time: f32) -> String {
        match self {
            TimeFormat::Seconds => format!("{:.2}", time),
            TimeFormat::Smpte { fps } => {
                let total_frames = (time * *fps) as i32;
                let fps_i = *fps as i32;
                let hours = total_frames / (3600 * fps_i);
                let minutes = (total_frames % (3600 * fps_i)) / (60 * fps_i);
                let seconds = (total_frames % (60 * fps_i)) / fps_i;
                let frames = total_frames % fps_i;
                format!("{:02}:{:02}:{:02}:{:02}", hours, minutes, seconds, frames)
            }
            TimeFormat::Frames { fps } => {
                format!("{}", (time * fps) as i32)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cutscene_new() {
        let cutscene = Cutscene::new("Test Cutscene");
        assert_eq!(cutscene.name, "Test Cutscene");
        assert!(!cutscene.id.to_string().is_empty());
    }

    #[test]
    fn test_cutscene_tags() {
        let mut cutscene = Cutscene::new("Test");
        cutscene.add_tag("intro");
        cutscene.add_tag("cinematic");
        assert_eq!(cutscene.tags.len(), 2);

        cutscene.remove_tag("intro");
        assert_eq!(cutscene.tags.len(), 1);
    }

    #[test]
    fn test_cutscene_library() {
        let mut library = CutsceneLibrary::new();
        let id = library.create_new("Test Cutscene");

        assert_eq!(library.cutscenes.len(), 1);
        assert_eq!(library.selected_id, Some(id));

        let cutscene = library.get(id).unwrap();
        assert_eq!(cutscene.name, "Test Cutscene");
    }

    #[test]
    fn test_cutscene_search() {
        let mut library = CutsceneLibrary::new();
        let mut cutscene = Cutscene::new("Intro Cinematic");
        cutscene.add_tag("tutorial");
        library.add(cutscene);

        library.add(Cutscene::new("Boss Battle"));

        let results = library.search("intro");
        assert_eq!(results.len(), 1);

        let results = library.search("tutorial");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_time_format() {
        let format = TimeFormat::Seconds;
        assert_eq!(format.format(5.5), "5.50");

        let format = TimeFormat::Frames { fps: 30.0 };
        assert_eq!(format.format(1.0), "30");

        let format = TimeFormat::Smpte { fps: 30.0 };
        assert_eq!(format.format(3661.0), "01:01:01:00");
    }

    #[test]
    fn test_playback_state() {
        assert!(PlaybackState::Playing.is_playing());
        assert!(!PlaybackState::Paused.is_playing());
        assert!(!PlaybackState::Stopped.is_playing());
    }
}
