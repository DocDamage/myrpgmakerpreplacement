//! Movement system

use crate::World;
use crate::components::{Position, SubPosition};
use crate::components::behavior::{CurrentPath, MovementSpeed, PathfindingTarget};

/// Movement system handles entity movement
pub struct MovementSystem;

impl MovementSystem {
    pub fn update(world: &mut World, dt: f32) {
        // Update sub-positions based on speed
        for (_entity, (pos, sub_pos, speed, target)) in world
            .query_mut::<(&mut Position, &mut SubPosition, &MovementSpeed, &PathfindingTarget)>()
        {
            // TODO: Implement movement towards target
            let _ = (pos, sub_pos, speed, target, dt);
        }
        
        // Process path following
        for (_entity, (pos, path, speed)) in world
            .query_mut::<(&mut Position, &mut CurrentPath, &MovementSpeed)>()
        {
            if !path.is_complete() {
                // TODO: Move along path
                let _ = (pos, path, speed);
            }
        }
    }
}
