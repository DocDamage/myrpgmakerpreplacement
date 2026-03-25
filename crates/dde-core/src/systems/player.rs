//! Player system

use glam::Vec2;

use crate::{Entity, World};
use crate::components::{EntityKindComp, Name, Position, Stats, SubPosition};
use crate::Direction4;
use crate::components::behavior::MovementSpeed;

/// Player marker component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Player;

/// Player controller
pub struct PlayerController {
    pub entity: Option<Entity>,
    pub move_speed: f32,
    pub run_multiplier: f32,
}

impl PlayerController {
    pub fn new() -> Self {
        Self {
            entity: None,
            move_speed: 4.0, // tiles per second
            run_multiplier: 2.0,
        }
    }
    
    /// Spawn player entity
    pub fn spawn_player(&mut self, world: &mut World, x: i32, y: i32) -> Entity {
        let entity = world.spawn((
            Player,
            EntityKindComp { kind: crate::EntityKind::Player },
            Name::new("Player", "player"),
            Position::new(x, y, 0),
            SubPosition::default(),
            Stats {
                hp: 100,
                max_hp: 100,
                mp: 50,
                max_mp: 50,
                str: 10,
                def: 10,
                spd: 10,
                mag: 10,
                luck: 10,
                level: 1,
                exp: 0,
            },
            MovementSpeed::from_spd_stat(10),
            Direction4::Down,
        ));
        
        self.entity = Some(entity);
        entity
    }
    
    /// Get player position if exists
    pub fn position(&self, world: &World) -> Option<Position> {
        self.entity.and_then(|e| world.query_one::<&Position>(e).ok()?.get().copied())
    }
    
    /// Get player world position (including sub-position)
    pub fn world_position(&self, world: &World) -> Option<Vec2> {
        self.entity.and_then(|e| {
            let mut query = world.query_one::<(&Position, &SubPosition)>(e).ok()?;
            let (pos, sub) = query.get()?;
            Some(Vec2::new(
                pos.x as f32 + sub.px,
                pos.y as f32 + sub.py,
            ))
        })
    }
    
    /// Check if player exists
    pub fn exists(&self) -> bool {
        self.entity.is_some()
    }
    
    /// Get player entity
    pub fn entity(&self) -> Option<Entity> {
        self.entity
    }
}

impl Default for PlayerController {
    fn default() -> Self {
        Self::new()
    }
}
