//! Animation system

use crate::World;
use crate::components::animation::{AnimationDef, AnimationPlayback};

/// Animation system updates sprite animations
pub struct AnimationSystem;

impl AnimationSystem {
    pub fn update(world: &mut World, dt_ms: f32, _defs: &[AnimationDef]) {
        for (_entity, playback) in world.query_mut::<&mut AnimationPlayback>() {
            // TODO: Update animation playback
            let _ = (playback, dt_ms);
        }
    }
}
