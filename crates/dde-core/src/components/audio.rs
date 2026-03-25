//! Audio components

use serde::{Deserialize, Serialize};

/// Audio emitter component for spatial audio
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AudioEmitter {
    pub stem_group: &'static str,
    pub radius: f32,
    pub volume: f32,
}

impl Default for AudioEmitter {
    fn default() -> Self {
        Self {
            stem_group: "",
            radius: 10.0,
            volume: 1.0,
        }
    }
}

/// SFX trigger component (one-shot sound)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SfxTrigger {
    pub sound_id: &'static str,
    pub volume: f32,
    pub pitch_variation: f32,
}

impl Default for SfxTrigger {
    fn default() -> Self {
        Self {
            sound_id: "",
            volume: 1.0,
            pitch_variation: 0.0,
        }
    }
}

/// BGM track reference
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BgmTrack {
    pub stem_set_id: String,
}

/// Ambient audio reference
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AmbientTrack {
    pub stem_set_id: String,
}
