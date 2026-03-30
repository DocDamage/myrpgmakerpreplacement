//! Keyframe interpolation system for timeline animations
//!
//! Provides smooth transitions between values using various interpolation methods
//! and easing functions.

use dde_core::{Direction4, Entity};
use serde::{Deserialize, Serialize};

/// A single keyframe at a specific time point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyframe {
    /// Time in seconds
    pub time: f32,
    /// Value at this keyframe
    pub value: TrackValue,
    /// Interpolation method to next keyframe
    pub interpolation: Interpolation,
    /// Easing function for the transition
    pub easing: EasingFunction,
}

impl Keyframe {
    /// Create a new keyframe
    pub fn new(time: f32, value: TrackValue) -> Self {
        Self {
            time,
            value,
            interpolation: Interpolation::Linear,
            easing: EasingFunction::Linear,
        }
    }

    /// Set interpolation method
    pub fn with_interpolation(mut self, interpolation: Interpolation) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Set easing function
    pub fn with_easing(mut self, easing: EasingFunction) -> Self {
        self.easing = easing;
        self
    }

    /// Interpolate between this keyframe and the next
    pub fn interpolate(&self, next: &Keyframe, t: f32) -> TrackValue {
        let eased_t = self.easing.apply(t);
        self.interpolation.interpolate(&self.value, &next.value, eased_t)
    }
}

/// Interpolation methods between keyframes
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Interpolation {
    /// Hold value until next keyframe (no interpolation)
    Step,
    /// Linear interpolation between values
    Linear,
    /// Cubic bezier with control points
    Bezier {
        /// Control point before this keyframe (0.0-1.0)
        control_in: f32,
        /// Control point after this keyframe (0.0-1.0)
        control_out: f32,
    },
}

impl Interpolation {
    /// Interpolate between two values using this interpolation method
    fn interpolate(&self, from: &TrackValue, to: &TrackValue, t: f32) -> TrackValue {
        match self {
            Interpolation::Step => from.clone(),
            Interpolation::Linear => TrackValue::lerp(from, to, t),
            Interpolation::Bezier { control_in, control_out } => {
                let bezier_t = Self::cubic_bezier(t, *control_in, *control_out);
                TrackValue::lerp(from, to, bezier_t)
            }
        }
    }

    /// Cubic bezier evaluation
    fn cubic_bezier(t: f32, p1: f32, p2: f32) -> f32 {
        // Simplified cubic bezier with control points (0, p1) and (1, p2)
        let p0 = 0.0f32;
        let p3 = 1.0f32;
        
        let one_minus_t = 1.0 - t;
        let one_minus_t2 = one_minus_t * one_minus_t;
        let one_minus_t3 = one_minus_t2 * one_minus_t;
        let t2 = t * t;
        let t3 = t2 * t;
        
        one_minus_t3 * p0 + 3.0 * one_minus_t2 * t * p1 + 3.0 * one_minus_t * t2 * p2 + t3 * p3
    }
}

/// Easing functions for smooth transitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EasingFunction {
    Linear,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInQuart,
    EaseOutQuart,
    EaseInOutQuart,
    EaseInElastic,
    EaseOutElastic,
    EaseInOutElastic,
    EaseInBack,
    EaseOutBack,
    EaseInOutBack,
    EaseInBounce,
    EaseOutBounce,
    EaseInOutBounce,
}

impl EasingFunction {
    /// Apply easing function to t (0.0-1.0)
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            EasingFunction::Linear => t,
            EasingFunction::EaseInQuad => t * t,
            EasingFunction::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            EasingFunction::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            EasingFunction::EaseInCubic => t * t * t,
            EasingFunction::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            EasingFunction::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            EasingFunction::EaseInQuart => t * t * t * t,
            EasingFunction::EaseOutQuart => 1.0 - (1.0 - t).powi(4),
            EasingFunction::EaseInOutQuart => {
                if t < 0.5 {
                    8.0 * t * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
                }
            }
            EasingFunction::EaseInElastic => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    let c4 = (2.0 * std::f32::consts::PI) / 3.0;
                    -(2.0f32.powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * c4).sin()
                }
            }
            EasingFunction::EaseOutElastic => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    let c4 = (2.0 * std::f32::consts::PI) / 3.0;
                    2.0f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
                }
            }
            EasingFunction::EaseInOutElastic => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    let c5 = (2.0 * std::f32::consts::PI) / 4.5;
                    if t < 0.5 {
                        -(2.0f32.powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * c5).sin()) / 2.0
                    } else {
                        2.0f32.powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * c5).sin() / 2.0 + 1.0
                    }
                }
            }
            EasingFunction::EaseInBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                c3 * t * t * t - c1 * t * t
            }
            EasingFunction::EaseOutBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
            }
            EasingFunction::EaseInOutBack => {
                let c1 = 1.70158;
                let c2 = c1 * 1.525;
                if t < 0.5 {
                    ((2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2)) / 2.0
                } else {
                    ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (t * 2.0 - 2.0) + c2) + 2.0) / 2.0
                }
            }
            EasingFunction::EaseInBounce => 1.0 - Self::ease_out_bounce(1.0 - t),
            EasingFunction::EaseOutBounce => Self::ease_out_bounce(t),
            EasingFunction::EaseInOutBounce => {
                if t < 0.5 {
                    (1.0 - Self::ease_out_bounce(1.0 - 2.0 * t)) / 2.0
                } else {
                    (1.0 + Self::ease_out_bounce(2.0 * t - 1.0)) / 2.0
                }
            }
        }
    }

    fn ease_out_bounce(t: f32) -> f32 {
        let n1 = 7.5625;
        let d1 = 2.75;
        
        if t < 1.0 / d1 {
            n1 * t * t
        } else if t < 2.0 / d1 {
            let t = t - 1.5 / d1;
            n1 * t * t + 0.75
        } else if t < 2.5 / d1 {
            let t = t - 2.25 / d1;
            n1 * t * t + 0.9375
        } else {
            let t = t - 2.625 / d1;
            n1 * t * t + 0.984375
        }
    }

    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            EasingFunction::Linear => "Linear",
            EasingFunction::EaseInQuad => "Ease In Quad",
            EasingFunction::EaseOutQuad => "Ease Out Quad",
            EasingFunction::EaseInOutQuad => "Ease In-Out Quad",
            EasingFunction::EaseInCubic => "Ease In Cubic",
            EasingFunction::EaseOutCubic => "Ease Out Cubic",
            EasingFunction::EaseInOutCubic => "Ease In-Out Cubic",
            EasingFunction::EaseInQuart => "Ease In Quart",
            EasingFunction::EaseOutQuart => "Ease Out Quart",
            EasingFunction::EaseInOutQuart => "Ease In-Out Quart",
            EasingFunction::EaseInElastic => "Ease In Elastic",
            EasingFunction::EaseOutElastic => "Ease Out Elastic",
            EasingFunction::EaseInOutElastic => "Ease In-Out Elastic",
            EasingFunction::EaseInBack => "Ease In Back",
            EasingFunction::EaseOutBack => "Ease Out Back",
            EasingFunction::EaseInOutBack => "Ease In-Out Back",
            EasingFunction::EaseInBounce => "Ease In Bounce",
            EasingFunction::EaseOutBounce => "Ease Out Bounce",
            EasingFunction::EaseInOutBounce => "Ease In-Out Bounce",
        }
    }
}

/// All possible values that can be keyed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrackValue {
    Camera(CameraValue),
    Entity(EntityValue),
    Audio(AudioValue),
    Effect(EffectValue),
    Dialogue(DialogueValue),
}

impl TrackValue {
    /// Linear interpolation between two track values
    fn lerp(from: &TrackValue, to: &TrackValue, t: f32) -> TrackValue {
        match (from, to) {
            (TrackValue::Camera(from_cam), TrackValue::Camera(to_cam)) => {
                TrackValue::Camera(CameraValue::lerp(from_cam, to_cam, t))
            }
            (TrackValue::Entity(from_ent), TrackValue::Entity(to_ent)) => {
                TrackValue::Entity(EntityValue::lerp(from_ent, to_ent, t))
            }
            (TrackValue::Audio(from_aud), TrackValue::Audio(to_aud)) => {
                TrackValue::Audio(AudioValue::lerp(from_aud, to_aud, t))
            }
            (TrackValue::Effect(from_eff), TrackValue::Effect(to_eff)) => {
                TrackValue::Effect(EffectValue::lerp(from_eff, to_eff, t))
            }
            // Dialogue doesn't interpolate - it's a discrete event
            (TrackValue::Dialogue(from_dia), _) => TrackValue::Dialogue(from_dia.clone()),
            _ => from.clone(), // Mismatched types - return from value
        }
    }
}

/// Serializable Vec3 wrapper
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
        }
    }
}

impl From<glam::Vec3> for Vec3 {
    fn from(v: glam::Vec3) -> Self {
        Self { x: v.x, y: v.y, z: v.z }
    }
}

impl From<Vec3> for glam::Vec3 {
    fn from(v: Vec3) -> Self {
        glam::Vec3::new(v.x, v.y, v.z)
    }
}

impl serde::Serialize for Vec3 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        [self.x, self.y, self.z].serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Vec3 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let arr: [f32; 3] = serde::Deserialize::deserialize(deserializer)?;
        Ok(Self { x: arr[0], y: arr[1], z: arr[2] })
    }
}

/// Camera track values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraValue {
    pub position: Vec3,
    pub zoom: f32,
    pub rotation: f32,
    pub shake_amount: f32,
    pub fade_alpha: f32,
}

impl Default for CameraValue {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            zoom: 1.0,
            rotation: 0.0,
            shake_amount: 0.0,
            fade_alpha: 0.0,
        }
    }
}

impl CameraValue {
    fn lerp(from: &CameraValue, to: &CameraValue, t: f32) -> CameraValue {
        CameraValue {
            position: from.position.lerp(to.position, t),
            zoom: from.zoom + (to.zoom - from.zoom) * t,
            rotation: from.rotation + (to.rotation - from.rotation) * t,
            shake_amount: from.shake_amount + (to.shake_amount - from.shake_amount) * t,
            fade_alpha: from.fade_alpha + (to.fade_alpha - from.fade_alpha) * t,
        }
    }
}

/// Entity track values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityValue {
    pub position: Vec3,
    pub animation_id: Option<u32>,
    pub visible: bool,
    pub direction: Direction4,
}

impl Default for EntityValue {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            animation_id: None,
            visible: true,
            direction: Direction4::default(),
        }
    }
}

impl EntityValue {
    fn lerp(from: &EntityValue, to: &EntityValue, t: f32) -> EntityValue {
        EntityValue {
            position: from.position.lerp(to.position, t),
            animation_id: if t >= 1.0 { to.animation_id } else { from.animation_id },
            visible: if t >= 0.5 { to.visible } else { from.visible },
            direction: if t >= 0.5 { to.direction } else { from.direction },
        }
    }
}

/// Audio track values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioValue {
    pub audio_id: u32,
    pub volume: f32,
    pub pitch: f32,
}

impl Default for AudioValue {
    fn default() -> Self {
        Self {
            audio_id: 0,
            volume: 1.0,
            pitch: 1.0,
        }
    }
}

impl AudioValue {
    fn lerp(from: &AudioValue, to: &AudioValue, t: f32) -> AudioValue {
        // Audio IDs don't interpolate - trigger at start
        AudioValue {
            audio_id: if t < 0.5 { from.audio_id } else { to.audio_id },
            volume: from.volume + (to.volume - from.volume) * t,
            pitch: from.pitch + (to.pitch - from.pitch) * t,
        }
    }
}

/// Screen effect types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectType {
    None,
    Flash,
    Shake,
    Blur,
    Bloom,
    ChromaticAberration,
    Vignette,
    BlackAndWhite,
    Sepia,
    Invert,
    Pixelate,
}

/// Effect track values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectValue {
    pub effect_type: EffectType,
    pub intensity: f32,
    pub color: [f32; 4],
}

impl Default for EffectValue {
    fn default() -> Self {
        Self {
            effect_type: EffectType::None,
            intensity: 0.0,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

impl EffectValue {
    fn lerp(from: &EffectValue, to: &EffectValue, t: f32) -> EffectValue {
        EffectValue {
            effect_type: if t >= 0.5 { to.effect_type } else { from.effect_type },
            intensity: from.intensity + (to.intensity - from.intensity) * t,
            color: [
                from.color[0] + (to.color[0] - from.color[0]) * t,
                from.color[1] + (to.color[1] - from.color[1]) * t,
                from.color[2] + (to.color[2] - from.color[2]) * t,
                from.color[3] + (to.color[3] - from.color[3]) * t,
            ],
        }
    }
}

/// Dialogue track values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueValue {
    pub text: String,
    pub speaker: String,
    pub portrait_id: Option<u32>,
    pub auto_advance: bool,
    pub advance_delay_ms: u32,
}

impl Default for DialogueValue {
    fn default() -> Self {
        Self {
            text: String::new(),
            speaker: String::new(),
            portrait_id: None,
            auto_advance: false,
            advance_delay_ms: 3000,
        }
    }
}

/// Track ID for identifying tracks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrackId(pub uuid::Uuid);

impl TrackId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for TrackId {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_linear() {
        let easing = EasingFunction::Linear;
        assert_eq!(easing.apply(0.0), 0.0);
        assert_eq!(easing.apply(0.5), 0.5);
        assert_eq!(easing.apply(1.0), 1.0);
    }

    #[test]
    fn test_easing_quad() {
        let easing = EasingFunction::EaseInQuad;
        assert_eq!(easing.apply(0.0), 0.0);
        assert_eq!(easing.apply(1.0), 1.0);
        // 0.5^2 = 0.25
        assert!((easing.apply(0.5) - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_camera_lerp() {
        let from = CameraValue {
            position: Vec3::ZERO,
            zoom: 1.0,
            rotation: 0.0,
            shake_amount: 0.0,
            fade_alpha: 0.0,
        };
        let to = CameraValue {
            position: Vec3::new(10.0, 0.0, 0.0),
            zoom: 2.0,
            rotation: 90.0,
            shake_amount: 0.5,
            fade_alpha: 1.0,
        };

        let result = CameraValue::lerp(&from, &to, 0.5);
        assert_eq!(result.position.x, 5.0);
        assert_eq!(result.zoom, 1.5);
        assert_eq!(result.rotation, 45.0);
    }

    #[test]
    fn test_keyframe_interpolation() {
        let kf1 = Keyframe::new(
            0.0,
            TrackValue::Camera(CameraValue {
                position: Vec3::ZERO,
                zoom: 1.0,
                rotation: 0.0,
                shake_amount: 0.0,
                fade_alpha: 0.0,
            }),
        );
        
        let kf2 = Keyframe::new(
            1.0,
            TrackValue::Camera(CameraValue {
                position: Vec3::new(10.0, 0.0, 0.0),
                zoom: 2.0,
                rotation: 0.0,
                shake_amount: 0.0,
                fade_alpha: 0.0,
            }),
        );

        let result = kf1.interpolate(&kf2, 0.5);
        match result {
            TrackValue::Camera(cam) => {
                assert_eq!(cam.position.x, 5.0);
                assert_eq!(cam.zoom, 1.5);
            }
            _ => panic!("Expected Camera value"),
        }
    }
}
