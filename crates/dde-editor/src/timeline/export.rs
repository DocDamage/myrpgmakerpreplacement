//! Export timeline to game events
//!
//! Converts timeline tracks into a sequence of cutscene events that can be
//! executed by the game engine.

use super::keyframes::{EffectType, TrackValue, Vec3};
use super::tracks::Track;
use super::editor::TimelineEditor;
use dde_core::{Direction4, Entity};
use serde::{Deserialize, Serialize};

/// Export timeline to event sequence
pub fn export_to_events(timeline: &TimelineEditor) -> Vec<CutsceneEvent> {
    let mut events = Vec::new();

    for track in &timeline.tracks {
        if track.muted {
            continue;
        }

        let track_events = export_track(track);
        events.extend(track_events);
    }

    // Sort events by time
    events.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());

    // Optimize: combine adjacent identical values
    let events = optimize_events(events);

    events
}

/// Export a single track to events
fn export_track(track: &Track) -> Vec<CutsceneEvent> {
    let mut events = Vec::new();

    match track.track_type {
        super::tracks::TrackType::Camera => {
            events.extend(export_camera_track(track));
        }
        super::tracks::TrackType::Entity => {
            events.extend(export_entity_track(track));
        }
        super::tracks::TrackType::Audio => {
            events.extend(export_audio_track(track));
        }
        super::tracks::TrackType::Effect => {
            events.extend(export_effect_track(track));
        }
        super::tracks::TrackType::Dialogue => {
            events.extend(export_dialogue_track(track));
        }
    }

    events
}

/// Export camera track
fn export_camera_track(track: &Track) -> Vec<CutsceneEvent> {
    let mut events = Vec::new();
    let keyframes = &track.keyframes;

    if keyframes.is_empty() {
        return events;
    }

    // First keyframe - set initial camera state
    if let TrackValue::Camera(cam) = &keyframes[0].value {
        events.push(CutsceneEvent {
            time: keyframes[0].time,
            event_type: CutsceneEventType::CameraSet {
                position: cam.position,
                zoom: cam.zoom,
                rotation: cam.rotation,
            },
        });

        // Fade if needed
        if cam.fade_alpha > 0.0 {
            events.push(CutsceneEvent {
                time: keyframes[0].time,
                event_type: CutsceneEventType::ScreenFade {
                    target_alpha: cam.fade_alpha,
                    duration: 0.0,
                    color: [0.0, 0.0, 0.0],
                },
            });
        }
    }

    // Subsequent keyframes - create move events
    for i in 1..keyframes.len() {
        let prev = &keyframes[i - 1];
        let curr = &keyframes[i];
        let duration = curr.time - prev.time;

        match (&prev.value, &curr.value) {
            (TrackValue::Camera(prev_cam), TrackValue::Camera(curr_cam)) => {
                // Camera movement
                if prev_cam.position != curr_cam.position {
                    events.push(CutsceneEvent {
                        time: prev.time,
                        event_type: CutsceneEventType::CameraMove {
                            position: curr_cam.position,
                            duration,
                            easing: curr.easing,
                        },
                    });
                }

                // Zoom change
                if (prev_cam.zoom - curr_cam.zoom).abs() > f32::EPSILON {
                    events.push(CutsceneEvent {
                        time: prev.time,
                        event_type: CutsceneEventType::CameraZoom {
                            target: curr_cam.zoom,
                            duration,
                            easing: curr.easing,
                        },
                    });
                }

                // Rotation change
                if (prev_cam.rotation - curr_cam.rotation).abs() > f32::EPSILON {
                    events.push(CutsceneEvent {
                        time: prev.time,
                        event_type: CutsceneEventType::CameraRotate {
                            target: curr_cam.rotation,
                    duration,
                            easing: curr.easing,
                        },
                    });
                }

                // Shake
                if curr_cam.shake_amount > 0.0 && prev_cam.shake_amount != curr_cam.shake_amount {
                    events.push(CutsceneEvent {
                        time: curr.time,
                        event_type: CutsceneEventType::CameraShake {
                            amount: curr_cam.shake_amount,
                            duration: 0.5, // Default shake duration
                        },
                    });
                }

                // Fade
                if (prev_cam.fade_alpha - curr_cam.fade_alpha).abs() > f32::EPSILON {
                    events.push(CutsceneEvent {
                        time: prev.time,
                        event_type: CutsceneEventType::ScreenFade {
                            target_alpha: curr_cam.fade_alpha,
                            duration,
                            color: [0.0, 0.0, 0.0],
                        },
                    });
                }
            }
            _ => {}
        }
    }

    events
}

/// Export entity track
fn export_entity_track(track: &Track) -> Vec<CutsceneEvent> {
    let mut events = Vec::new();
    let keyframes = &track.keyframes;

    if keyframes.is_empty() {
        return events;
    }

    let entity = track.target.unwrap_or(Entity::DANGLING);

    // First keyframe - set initial state
    if let TrackValue::Entity(ent) = &keyframes[0].value {
        events.push(CutsceneEvent {
            time: keyframes[0].time,
            event_type: CutsceneEventType::EntitySetState {
                entity,
                position: ent.position,
                direction: ent.direction,
                visible: ent.visible,
            },
        });

        if let Some(anim_id) = ent.animation_id {
            events.push(CutsceneEvent {
                time: keyframes[0].time,
                event_type: CutsceneEventType::PlayAnimation { entity, anim_id },
            });
        }
    }

    // Subsequent keyframes
    for i in 1..keyframes.len() {
        let prev = &keyframes[i - 1];
        let curr = &keyframes[i];
        let duration = curr.time - prev.time;

        match (&prev.value, &curr.value) {
            (TrackValue::Entity(prev_ent), TrackValue::Entity(curr_ent)) => {
                // Movement
                if prev_ent.position != curr_ent.position {
                    events.push(CutsceneEvent {
                        time: prev.time,
                        event_type: CutsceneEventType::EntityMove {
                            entity,
                            position: curr_ent.position,
                            duration,
                            easing: curr.easing,
                        },
                    });
                }

                // Direction change
                if prev_ent.direction != curr_ent.direction {
                    events.push(CutsceneEvent {
                        time: curr.time,
                        event_type: CutsceneEventType::EntitySetDirection {
                            entity,
                            direction: curr_ent.direction,
                        },
                    });
                }

                // Visibility change
                if prev_ent.visible != curr_ent.visible {
                    events.push(CutsceneEvent {
                        time: curr.time,
                        event_type: CutsceneEventType::EntitySetVisibility {
                            entity,
                            visible: curr_ent.visible,
                        },
                    });
                }

                // Animation change
                if curr_ent.animation_id.is_some() && prev_ent.animation_id != curr_ent.animation_id {
                    events.push(CutsceneEvent {
                        time: curr.time,
                        event_type: CutsceneEventType::PlayAnimation {
                            entity,
                            anim_id: curr_ent.animation_id.unwrap(),
                        },
                    });
                }
            }
            _ => {}
        }
    }

    events
}

/// Export audio track
fn export_audio_track(track: &Track) -> Vec<CutsceneEvent> {
    let mut events = Vec::new();

    for keyframe in &track.keyframes {
        if let TrackValue::Audio(audio) = &keyframe.value {
            events.push(CutsceneEvent {
                time: keyframe.time,
                event_type: CutsceneEventType::PlaySound {
                    audio_id: audio.audio_id,
                    volume: audio.volume,
                    pitch: audio.pitch,
                },
            });
        }
    }

    events
}

/// Export effect track
fn export_effect_track(track: &Track) -> Vec<CutsceneEvent> {
    let mut events = Vec::new();
    let keyframes = &track.keyframes;

    if keyframes.is_empty() {
        return events;
    }

    // First keyframe - set initial effect state
    if let TrackValue::Effect(eff) = &keyframes[0].value {
        if eff.effect_type != EffectType::None {
            events.push(CutsceneEvent {
                time: keyframes[0].time,
                event_type: CutsceneEventType::ScreenEffect {
                    effect: eff.effect_type,
                    intensity: eff.intensity,
                    color: eff.color,
                },
            });
        }
    }

    // Subsequent keyframes
    for i in 1..keyframes.len() {
        let prev = &keyframes[i - 1];
        let curr = &keyframes[i];

        match (&prev.value, &curr.value) {
            (TrackValue::Effect(prev_eff), TrackValue::Effect(curr_eff)) => {
                // Effect type or intensity changed
                if prev_eff.effect_type != curr_eff.effect_type ||
                   (prev_eff.intensity - curr_eff.intensity).abs() > f32::EPSILON {
                    events.push(CutsceneEvent {
                        time: curr.time,
                        event_type: CutsceneEventType::ScreenEffect {
                            effect: curr_eff.effect_type,
                            intensity: curr_eff.intensity,
                            color: curr_eff.color,
                        },
                    });
                }
            }
            _ => {}
        }
    }

    events
}

/// Export dialogue track
fn export_dialogue_track(track: &Track) -> Vec<CutsceneEvent> {
    let mut events = Vec::new();

    for keyframe in &track.keyframes {
        if let TrackValue::Dialogue(dia) = &keyframe.value {
            events.push(CutsceneEvent {
                time: keyframe.time,
                event_type: CutsceneEventType::ShowDialogue {
                    text: dia.text.clone(),
                    speaker: dia.speaker.clone(),
                    portrait_id: dia.portrait_id,
                    auto_advance: dia.auto_advance,
                    advance_delay_ms: dia.advance_delay_ms,
                },
            });
        }
    }

    events
}

/// Optimize events by combining redundant ones
fn optimize_events(events: Vec<CutsceneEvent>) -> Vec<CutsceneEvent> {
    // For now, just return the events as-is
    // Future optimization: combine consecutive camera moves, etc.
    events
}

/// A cutscene event at a specific time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CutsceneEvent {
    /// Time in seconds when this event occurs
    pub time: f32,
    /// The type of event
    pub event_type: CutsceneEventType,
}

/// Types of cutscene events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CutsceneEventType {
    /// Set camera position/state immediately
    CameraSet {
        position: Vec3,
        zoom: f32,
        rotation: f32,
    },
    /// Move camera to position over time
    CameraMove {
        position: Vec3,
        duration: f32,
        easing: super::keyframes::EasingFunction,
    },
    /// Zoom camera over time
    CameraZoom {
        target: f32,
        duration: f32,
        easing: super::keyframes::EasingFunction,
    },
    /// Rotate camera over time
    CameraRotate {
        target: f32,
        duration: f32,
        easing: super::keyframes::EasingFunction,
    },
    /// Shake camera
    CameraShake {
        amount: f32,
        duration: f32,
    },
    /// Set entity state immediately
    EntitySetState {
        entity: Entity,
        position: Vec3,
        direction: Direction4,
        visible: bool,
    },
    /// Move entity to position over time
    EntityMove {
        entity: Entity,
        position: Vec3,
        duration: f32,
        easing: super::keyframes::EasingFunction,
    },
    /// Set entity direction
    EntitySetDirection {
        entity: Entity,
        direction: Direction4,
    },
    /// Set entity visibility
    EntitySetVisibility {
        entity: Entity,
        visible: bool,
    },
    /// Play animation on entity
    PlayAnimation {
        entity: Entity,
        anim_id: u32,
    },
    /// Play sound
    PlaySound {
        audio_id: u32,
        volume: f32,
        pitch: f32,
    },
    /// Show dialogue
    ShowDialogue {
        text: String,
        speaker: String,
        portrait_id: Option<u32>,
        auto_advance: bool,
        advance_delay_ms: u32,
    },
    /// Hide dialogue
    HideDialogue,
    /// Apply screen effect
    ScreenEffect {
        effect: EffectType,
        intensity: f32,
        color: [f32; 4],
    },
    /// Fade screen
    ScreenFade {
        target_alpha: f32,
        duration: f32,
        color: [f32; 3],
    },
    /// Wait for duration
    Wait {
        duration: f32,
    },
    /// Wait for user input
    WaitForInput,
}

/// Export format for game runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CutsceneData {
    pub name: String,
    pub duration: f32,
    pub events: Vec<CutsceneEvent>,
    pub loop_cutscene: bool,
}

impl CutsceneData {
    /// Create from timeline editor
    pub fn from_timeline(timeline: &TimelineEditor, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            duration: timeline.duration,
            events: export_to_events(timeline),
            loop_cutscene: timeline.loop_playback,
        }
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Import from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Get the event at or before a specific time
    pub fn events_at_time(&self, time: f32) -> Vec<&CutsceneEvent> {
        self.events.iter()
            .filter(|e| e.time <= time)
            .collect()
    }

    /// Get all events in a time range
    pub fn events_in_range(&self, start: f32, end: f32) -> Vec<&CutsceneEvent> {
        self.events.iter()
            .filter(|e| e.time >= start && e.time <= end)
            .collect()
    }

    /// Calculate total event count by type
    pub fn event_counts(&self) -> std::collections::HashMap<String, usize> {
        let mut counts = std::collections::HashMap::new();
        for event in &self.events {
            let name = format!("{:?}", std::mem::discriminant(&event.event_type));
            *counts.entry(name).or_insert(0) += 1;
        }
        counts
    }
}

/// Export options for fine-tuning the export
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Minimum duration for events (combines shorter events)
    pub min_event_duration: f32,
    /// Whether to optimize keyframes
    pub optimize_keyframes: bool,
    /// Maximum error tolerance for optimization
    pub max_optimization_error: f32,
    /// Include unused tracks
    pub include_muted_tracks: bool,
    /// Export format version
    pub format_version: u32,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            min_event_duration: 0.016, // ~1 frame at 60fps
            optimize_keyframes: true,
            max_optimization_error: 0.01,
            include_muted_tracks: false,
            format_version: 1,
        }
    }
}

/// Export timeline with custom options
pub fn export_to_events_with_options(
    timeline: &TimelineEditor,
    _options: &ExportOptions,
) -> Vec<CutsceneEvent> {
    // For now, just use the default export
    // Future: apply options for optimization
    export_to_events(timeline)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::keyframes::{CameraValue, EntityValue, DialogueValue, AudioValue, Vec3};
    use super::super::tracks::{Track, TrackType};
    use super::super::editor::TimelineEditor;

    #[test]
    fn test_export_camera_track() {
        let mut track = Track::new(TrackType::Camera, "Camera");
        
        track.add_keyframe(0.0, TrackValue::Camera(CameraValue {
            position: Vec3::ZERO,
            zoom: 1.0,
            ..Default::default()
        }));
        
        track.add_keyframe(2.0, TrackValue::Camera(CameraValue {
            position: Vec3::new(10.0, 0.0, 0.0),
            zoom: 2.0,
            ..Default::default()
        }));

        let events = export_camera_track(&track);
        assert!(!events.is_empty());
    }

    #[test]
    fn test_cutscene_data_serialization() {
        let mut timeline = TimelineEditor::new();
        timeline.duration = 10.0;
        
        let data = CutsceneData::from_timeline(&timeline, "Test Cutscene");
        let json = data.to_json().unwrap();
        let restored = CutsceneData::from_json(&json).unwrap();
        
        assert_eq!(data.name, restored.name);
        assert_eq!(data.duration, restored.duration);
    }
}
