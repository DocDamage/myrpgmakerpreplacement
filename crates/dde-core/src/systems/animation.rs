//! Animation system

use crate::components::animation::{AnimationDef, AnimationPlayback, Sprite};
use crate::World;

/// Animation system updates sprite animations
pub struct AnimationSystem;

impl AnimationSystem {
    pub fn update(world: &mut World, dt_ms: f32, defs: &[AnimationDef]) {
        for (_, (playback, sprite)) in world.query_mut::<(&mut AnimationPlayback, &mut Sprite)>() {
            // Skip if animation is already finished and not looping
            if playback.is_finished && !playback.is_looping {
                continue;
            }

            // Find the AnimationDef matching playback.anim_id
            let Some(def) = defs.iter().find(|d| d.id == playback.anim_id) else {
                continue;
            };

            // Update frame timer
            playback.frame_timer_ms += dt_ms;

            // Get current frame duration from definition
            let frame_duration = def.frame_duration_ms(playback.frame_index);

            // Check if we've exceeded the current frame's duration
            if playback.frame_timer_ms >= frame_duration as f32 {
                // Advance to next frame
                playback.frame_index += 1;

                // Check if we've reached the end of the animation
                if playback.frame_index >= def.frames.len() {
                    if playback.is_looping {
                        // Wrap to first frame
                        playback.frame_index = 0;
                    } else {
                        // Clamp to last frame and mark as finished
                        playback.frame_index = def.frames.len().saturating_sub(1);
                        playback.is_finished = true;
                    }
                }

                // Reset frame timer
                playback.frame_timer_ms = 0.0;

                // Update Sprite's frame to match the animation frame
                if let Some(&frame_id) = def.frames.get(playback.frame_index) {
                    sprite.frame = frame_id;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::animation::{AnimationPriority, Sprite};

    fn create_test_animation_def() -> AnimationDef {
        AnimationDef {
            id: 1,
            name: "test_anim".to_string(),
            sheet_id: 1,
            frames: vec![10, 11, 12, 13],
            durations_ms: vec![100, 100, 100, 100],
            is_looping: true,
            priority: AnimationPriority::Idle,
        }
    }

    #[test]
    fn test_animation_advances_frame() {
        let mut world = World::new();
        let def = create_test_animation_def();

        let entity = world.spawn((
            AnimationPlayback {
                anim_id: 1,
                priority: AnimationPriority::Idle,
                frame_index: 0,
                frame_timer_ms: 0.0,
                is_looping: true,
                is_finished: false,
            },
            Sprite {
                sheet_id: 1,
                frame: 10,
                direction: crate::Direction4::Down,
            },
        ));

        // Update with enough time to advance one frame
        AnimationSystem::update(&mut world, 150.0, &[def]);

        let playback = world.get::<&AnimationPlayback>(entity).unwrap();
        assert_eq!(playback.frame_index, 1);
        assert_eq!(playback.frame_timer_ms, 0.0);

        let sprite = world.get::<&Sprite>(entity).unwrap();
        assert_eq!(sprite.frame, 11);
    }

    #[test]
    fn test_animation_loops() {
        let mut world = World::new();
        let def = create_test_animation_def();

        let entity = world.spawn((
            AnimationPlayback {
                anim_id: 1,
                priority: AnimationPriority::Idle,
                frame_index: 3, // Last frame
                frame_timer_ms: 0.0,
                is_looping: true,
                is_finished: false,
            },
            Sprite {
                sheet_id: 1,
                frame: 13,
                direction: crate::Direction4::Down,
            },
        ));

        // Update with enough time to advance past last frame
        AnimationSystem::update(&mut world, 150.0, &[def]);

        let playback = world.get::<&AnimationPlayback>(entity).unwrap();
        assert_eq!(playback.frame_index, 0);
        assert!(!playback.is_finished);

        let sprite = world.get::<&Sprite>(entity).unwrap();
        assert_eq!(sprite.frame, 10);
    }

    #[test]
    fn test_animation_finishes_when_not_looping() {
        let mut world = World::new();
        let mut def = create_test_animation_def();
        def.is_looping = false;

        let entity = world.spawn((
            AnimationPlayback {
                anim_id: 1,
                priority: AnimationPriority::Idle,
                frame_index: 3, // Last frame
                frame_timer_ms: 0.0,
                is_looping: false,
                is_finished: false,
            },
            Sprite {
                sheet_id: 1,
                frame: 13,
                direction: crate::Direction4::Down,
            },
        ));

        // Update with enough time to advance past last frame
        AnimationSystem::update(&mut world, 150.0, &[def]);

        let playback = world.get::<&AnimationPlayback>(entity).unwrap();
        assert_eq!(playback.frame_index, 3); // Stays on last frame
        assert!(playback.is_finished);

        let sprite = world.get::<&Sprite>(entity).unwrap();
        assert_eq!(sprite.frame, 13); // Frame stays the same
    }

    #[test]
    fn test_finished_animation_skips_update() {
        let mut world = World::new();
        let def = create_test_animation_def();

        let entity = world.spawn((
            AnimationPlayback {
                anim_id: 1,
                priority: AnimationPriority::Idle,
                frame_index: 2,
                frame_timer_ms: 50.0,
                is_looping: false,
                is_finished: true,
            },
            Sprite {
                sheet_id: 1,
                frame: 12,
                direction: crate::Direction4::Down,
            },
        ));

        // Update should not change anything since animation is finished
        AnimationSystem::update(&mut world, 1000.0, &[def]);

        let playback = world.get::<&AnimationPlayback>(entity).unwrap();
        assert_eq!(playback.frame_index, 2);
        assert_eq!(playback.frame_timer_ms, 50.0);
    }
}
