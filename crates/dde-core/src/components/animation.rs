//! Animation components

use serde::{Deserialize, Serialize};

use crate::Direction4;

/// Sprite component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Sprite {
    pub sheet_id: u32,
    pub frame: u32,
    pub direction: Direction4,
}

/// Animation state component
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct AnimationState {
    pub anim_id: u32,
    pub frame: u32,
    pub elapsed_ms: f32,
}

/// Visibility component
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Visible {
    pub layer: RenderLayer,
    pub opacity: f32,
}

/// Render layers (bottom to top)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RenderLayer {
    Ground = 0,
    Terrain = 1,
    BelowEntity = 2,
    Entity = 3,
    AboveEntity = 4,
    WeatherFx = 5,
    UI = 6,
    #[default]
    Default = 7,
}

/// Animation priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AnimationPriority {
    Idle = 0,
    Walk = 1,
    Cast = 2,
    Attack = 3,
    Hurt = 4,
    Death = 5,
}

/// Current animation playback
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnimationPlayback {
    pub anim_id: u32,
    pub priority: AnimationPriority,
    pub frame_index: usize,
    pub frame_timer_ms: f32,
    pub is_looping: bool,
    pub is_finished: bool,
}

impl Default for AnimationPlayback {
    fn default() -> Self {
        Self {
            anim_id: 0,
            priority: AnimationPriority::Idle,
            frame_index: 0,
            frame_timer_ms: 0.0,
            is_looping: true,
            is_finished: false,
        }
    }
}

/// Animation definition (stored in resources, not on entities)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationDef {
    pub id: u32,
    pub name: String,
    pub sheet_id: u32,
    pub frames: Vec<u32>,
    pub durations_ms: Vec<u32>,
    pub is_looping: bool,
    pub priority: AnimationPriority,
}

impl AnimationDef {
    /// Get total duration of one loop
    #[must_use]
    pub fn total_duration_ms(&self) -> u32 {
        self.durations_ms.iter().sum()
    }

    /// Get duration for a specific frame
    #[must_use]
    pub fn frame_duration_ms(&self, frame_index: usize) -> u32 {
        self.durations_ms.get(frame_index).copied().unwrap_or(100)
    }
}

/// Animation transition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationTransition {
    pub from_anim: u32,
    pub to_anim: u32,
    pub condition: AnimationCondition,
    pub blend_ms: u32,
}

/// Condition for animation transition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnimationCondition {
    Always,
    EndOfAnimation,
    Input { action: String },
    Grounded,
    InAir,
    Expression(String),
}

/// Animation state machine
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AnimationStateMachine {
    pub initial_anim: u32,
    pub transitions: Vec<AnimationTransition>,
}
