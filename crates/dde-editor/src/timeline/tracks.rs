//! Timeline track definitions
//!
//! Tracks hold keyframes for specific types of data (camera, entities, audio, etc.)

use super::keyframes::{Keyframe, TrackId, TrackValue};
use dde_core::Entity;
use serde::{Deserialize, Serialize};

/// Types of tracks in the timeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrackType {
    /// Camera position, zoom, shake, fade
    Camera,
    /// Entity movement, animation, visibility
    Entity,
    /// BGM, SFX, Voice
    Audio,
    /// Screen effects, particles
    Effect,
    /// Text, speaker, portrait
    Dialogue,
}

impl TrackType {
    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            TrackType::Camera => "Camera",
            TrackType::Entity => "Entity",
            TrackType::Audio => "Audio",
            TrackType::Effect => "Effect",
            TrackType::Dialogue => "Dialogue",
        }
    }

    /// Get icon/emoji for UI
    pub fn icon(&self) -> &'static str {
        match self {
            TrackType::Camera => "📷",
            TrackType::Entity => "👤",
            TrackType::Audio => "🔊",
            TrackType::Effect => "✨",
            TrackType::Dialogue => "💬",
        }
    }

    /// Get color for UI (RGB)
    pub fn color(&self) -> [u8; 3] {
        match self {
            TrackType::Camera => [255, 200, 100],   // Orange
            TrackType::Entity => [100, 200, 255],   // Blue
            TrackType::Audio => [100, 255, 150],    // Green
            TrackType::Effect => [255, 100, 200],   // Pink
            TrackType::Dialogue => [200, 150, 255], // Purple
        }
    }
}

/// A timeline track containing keyframes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackId,
    pub track_type: TrackType,
    pub name: String,
    /// Target entity (for entity tracks)
    pub target: Option<Entity>,
    /// Keyframes sorted by time
    pub keyframes: Vec<Keyframe>,
    /// Track is muted (not evaluated during playback)
    pub muted: bool,
    /// Track is locked (cannot be edited)
    pub locked: bool,
    /// Track color override (optional)
    pub color_override: Option<[u8; 3]>,
    /// Track collapsed in UI
    pub collapsed: bool,
    /// Track visible in UI
    pub visible: bool,
    /// Minimum time for this track (can extend timeline)
    pub min_time: f32,
    /// Maximum time for this track (can extend timeline)
    pub max_time: f32,
}

impl Track {
    /// Create a new track
    pub fn new(track_type: TrackType, name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            id: TrackId::new(),
            track_type,
            name,
            target: None,
            keyframes: Vec::new(),
            muted: false,
            locked: false,
            color_override: None,
            collapsed: false,
            visible: true,
            min_time: 0.0,
            max_time: 0.0,
        }
    }

    /// Create a track for a specific entity
    pub fn for_entity(entity: Entity, name: impl Into<String>) -> Self {
        let mut track = Self::new(TrackType::Entity, name);
        track.target = Some(entity);
        track
    }

    /// Get the value at a specific time
    pub fn value_at(&self, time: f32) -> Option<TrackValue> {
        if self.keyframes.is_empty() {
            return None;
        }

        // Find surrounding keyframes
        let prev = self.find_keyframe_before(time);
        let next = self.find_keyframe_after(time);

        match (prev, next) {
            (Some(prev), Some(next)) => {
                if prev.time == next.time {
                    Some(prev.value.clone())
                } else {
                    let t = (time - prev.time) / (next.time - prev.time);
                    Some(prev.interpolate(next, t.clamp(0.0, 1.0)))
                }
            }
            (Some(prev), None) => Some(prev.value.clone()),
            (None, Some(next)) => Some(next.value.clone()),
            (None, None) => None,
        }
    }

    /// Get the value at a specific time or return a default
    pub fn value_at_or_default(&self, time: f32, default: TrackValue) -> TrackValue {
        self.value_at(time).unwrap_or(default)
    }

    /// Find the keyframe before or at the given time
    fn find_keyframe_before(&self, time: f32) -> Option<&Keyframe> {
        self.keyframes.iter().rev().find(|kf| kf.time <= time)
    }

    /// Find the keyframe after the given time
    fn find_keyframe_after(&self, time: f32) -> Option<&Keyframe> {
        self.keyframes.iter().find(|kf| kf.time > time)
    }

    /// Add a keyframe at the given time
    /// Returns true if a new keyframe was added, false if an existing one was updated
    pub fn add_keyframe(&mut self, time: f32, value: TrackValue) -> bool {
        if self.locked {
            return false;
        }

        // Find insertion point
        match self
            .keyframes
            .binary_search_by(|kf| kf.time.partial_cmp(&time).unwrap())
        {
            Ok(index) => {
                // Update existing keyframe
                self.keyframes[index].value = value;
                false
            }
            Err(index) => {
                // Insert new keyframe
                let keyframe = Keyframe::new(time, value);
                self.keyframes.insert(index, keyframe);
                self.update_time_bounds();
                true
            }
        }
    }

    /// Add a keyframe with specific interpolation
    pub fn add_keyframe_with_interpolation(
        &mut self,
        time: f32,
        value: TrackValue,
        interpolation: super::keyframes::Interpolation,
    ) -> bool {
        if self.locked {
            return false;
        }

        match self
            .keyframes
            .binary_search_by(|kf| kf.time.partial_cmp(&time).unwrap())
        {
            Ok(index) => {
                self.keyframes[index].value = value;
                self.keyframes[index].interpolation = interpolation;
                false
            }
            Err(index) => {
                let keyframe = Keyframe::new(time, value).with_interpolation(interpolation);
                self.keyframes.insert(index, keyframe);
                self.update_time_bounds();
                true
            }
        }
    }

    /// Remove a keyframe at the given time
    pub fn remove_keyframe_at(&mut self, time: f32) -> Option<Keyframe> {
        if self.locked {
            return None;
        }

        match self
            .keyframes
            .binary_search_by(|kf| kf.time.partial_cmp(&time).unwrap())
        {
            Ok(index) => {
                let removed = self.keyframes.remove(index);
                self.update_time_bounds();
                Some(removed)
            }
            Err(_) => None,
        }
    }

    /// Remove a keyframe by index
    pub fn remove_keyframe(&mut self, index: usize) -> Option<Keyframe> {
        if self.locked || index >= self.keyframes.len() {
            return None;
        }

        let removed = self.keyframes.remove(index);
        self.update_time_bounds();
        Some(removed)
    }

    /// Get the index of the keyframe at or before the given time
    pub fn keyframe_index_at(&self, time: f32) -> Option<usize> {
        self.keyframes
            .iter()
            .enumerate()
            .rev()
            .find(|(_, kf)| kf.time <= time)
            .map(|(i, _)| i)
    }

    /// Get the next keyframe index after the given time
    pub fn next_keyframe_index(&self, time: f32) -> Option<usize> {
        self.keyframes
            .iter()
            .enumerate()
            .find(|(_, kf)| kf.time > time)
            .map(|(i, _)| i)
    }

    /// Move a keyframe to a new time
    pub fn move_keyframe(&mut self, old_time: f32, new_time: f32) -> bool {
        if self.locked {
            return false;
        }

        if let Some(index) = self.keyframe_index_at(old_time) {
            if (self.keyframes[index].time - old_time).abs() < f32::EPSILON {
                let mut keyframe = self.keyframes.remove(index);
                keyframe.time = new_time;

                // Re-insert at correct position
                let new_index = self
                    .keyframes
                    .binary_search_by(|kf| kf.time.partial_cmp(&new_time).unwrap())
                    .unwrap_or_else(|i| i);
                self.keyframes.insert(new_index, keyframe);
                self.update_time_bounds();
                return true;
            }
        }
        false
    }

    /// Get all keyframes in time range
    pub fn keyframes_in_range(&self, start: f32, end: f32) -> Vec<&Keyframe> {
        self.keyframes
            .iter()
            .filter(|kf| kf.time >= start && kf.time <= end)
            .collect()
    }

    /// Get the duration of this track
    pub fn duration(&self) -> f32 {
        if self.keyframes.is_empty() {
            0.0
        } else {
            self.keyframes.last().unwrap().time
        }
    }

    /// Update time bounds
    fn update_time_bounds(&mut self) {
        if self.keyframes.is_empty() {
            self.min_time = 0.0;
            self.max_time = 0.0;
        } else {
            self.min_time = self.keyframes.first().unwrap().time;
            self.max_time = self.keyframes.last().unwrap().time;
        }
    }

    /// Toggle mute state
    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }

    /// Toggle lock state
    pub fn toggle_lock(&mut self) {
        self.locked = !self.locked;
    }

    /// Toggle collapsed state
    pub fn toggle_collapsed(&mut self) {
        self.collapsed = !self.collapsed;
    }

    /// Get the color for this track
    pub fn color(&self) -> [u8; 3] {
        self.color_override
            .unwrap_or_else(|| self.track_type.color())
    }

    /// Get all keyframe times (for UI)
    pub fn keyframe_times(&self) -> Vec<f32> {
        self.keyframes.iter().map(|kf| kf.time).collect()
    }

    /// Check if this track has any keyframes at the given time (within epsilon)
    pub fn has_keyframe_at(&self, time: f32, epsilon: f32) -> bool {
        self.keyframes
            .iter()
            .any(|kf| (kf.time - time).abs() < epsilon)
    }

    /// Get the closest keyframe to the given time
    pub fn closest_keyframe(&self, time: f32) -> Option<&Keyframe> {
        self.keyframes.iter().min_by(|a, b| {
            let dist_a = (a.time - time).abs();
            let dist_b = (b.time - time).abs();
            dist_a.partial_cmp(&dist_b).unwrap()
        })
    }

    /// Duplicate a keyframe at the given time
    pub fn duplicate_keyframe(&mut self, time: f32, new_time: f32) -> bool {
        if self.locked {
            return false;
        }

        if let Some(kf) = self.closest_keyframe(time) {
            if (kf.time - time).abs() < 0.001 {
                let mut new_kf = kf.clone();
                new_kf.time = new_time;

                let index = self
                    .keyframes
                    .binary_search_by(|k| k.time.partial_cmp(&new_time).unwrap())
                    .unwrap_or_else(|i| i);
                self.keyframes.insert(index, new_kf);
                self.update_time_bounds();
                return true;
            }
        }
        false
    }

    /// Clear all keyframes
    pub fn clear(&mut self) {
        if !self.locked {
            self.keyframes.clear();
            self.update_time_bounds();
        }
    }

    /// Get the number of keyframes
    pub fn keyframe_count(&self) -> usize {
        self.keyframes.len()
    }

    /// Check if this track is empty
    pub fn is_empty(&self) -> bool {
        self.keyframes.is_empty()
    }
}

impl Default for Track {
    fn default() -> Self {
        Self::new(TrackType::Camera, "New Track")
    }
}

/// A group of tracks that can be collapsed/expanded together
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackGroup {
    pub name: String,
    pub track_ids: Vec<TrackId>,
    pub collapsed: bool,
    pub color: Option<[u8; 3]>,
}

impl TrackGroup {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            track_ids: Vec::new(),
            collapsed: false,
            color: None,
        }
    }

    pub fn add_track(&mut self, track_id: TrackId) {
        if !self.track_ids.contains(&track_id) {
            self.track_ids.push(track_id);
        }
    }

    pub fn remove_track(&mut self, track_id: TrackId) {
        self.track_ids.retain(|&id| id != track_id);
    }

    pub fn toggle_collapsed(&mut self) {
        self.collapsed = !self.collapsed;
    }
}

#[cfg(test)]
mod tests {
    use super::super::keyframes::{CameraValue, Interpolation};
    use super::*;

    #[test]
    fn test_track_add_keyframe() {
        let mut track = Track::new(TrackType::Camera, "Camera Track");

        let value = TrackValue::Camera(CameraValue::default());
        assert!(track.add_keyframe(0.0, value.clone()));
        assert_eq!(track.keyframe_count(), 1);

        // Adding at same time should update
        assert!(!track.add_keyframe(0.0, value));
        assert_eq!(track.keyframe_count(), 1);
    }

    #[test]
    fn test_track_value_at() {
        let mut track = Track::new(TrackType::Camera, "Camera Track");

        let value1 = TrackValue::Camera(CameraValue {
            zoom: 1.0,
            ..Default::default()
        });
        let value2 = TrackValue::Camera(CameraValue {
            zoom: 2.0,
            ..Default::default()
        });

        track.add_keyframe(0.0, value1);
        track.add_keyframe(1.0, value2);

        let result = track.value_at(0.5);
        assert!(result.is_some());
    }

    #[test]
    fn test_track_keyframe_ordering() {
        let mut track = Track::new(TrackType::Camera, "Camera Track");
        let value = TrackValue::Camera(CameraValue::default());

        track.add_keyframe(2.0, value.clone());
        track.add_keyframe(0.0, value.clone());
        track.add_keyframe(1.0, value);

        let times: Vec<f32> = track.keyframes.iter().map(|kf| kf.time).collect();
        assert_eq!(times, vec![0.0, 1.0, 2.0]);
    }

    #[test]
    fn test_track_locked() {
        let mut track = Track::new(TrackType::Camera, "Camera Track");
        track.locked = true;

        let value = TrackValue::Camera(CameraValue::default());
        assert!(!track.add_keyframe(0.0, value));
        assert!(track.is_empty());
    }

    #[test]
    fn test_track_move_keyframe() {
        let mut track = Track::new(TrackType::Camera, "Camera Track");
        let value = TrackValue::Camera(CameraValue::default());

        track.add_keyframe(0.0, value);
        assert!(track.move_keyframe(0.0, 1.0));

        assert!(track.has_keyframe_at(1.0, 0.001));
        assert!(!track.has_keyframe_at(0.0, 0.001));
    }
}
