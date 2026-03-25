//! Movement system

use glam::Vec2;

use crate::World;
use crate::components::{Position, SubPosition};
use crate::Direction4;
use crate::components::behavior::{MovementSpeed, CurrentPath, PathfindingTarget};

/// Movement system handles entity movement
pub struct MovementSystem;

impl MovementSystem {
    /// Update entity positions based on their movement
    pub fn update(world: &mut World, tilemap: &TileCollisionMap, dt: f32) {
        // Collect movement updates first to avoid borrow issues
        let mut updates: Vec<(crate::Entity, i32, i32, f32, f32, Direction4)> = Vec::new();
        
        // Update entities with pathfinding targets
        for (entity, (pos, sub_pos, speed, target)) in world
            .query_mut::<(&Position, &mut SubPosition, &MovementSpeed, &PathfindingTarget)>()
        {
            // Calculate direction to target
            let current_pos = Vec2::new(pos.x as f32 + sub_pos.px, pos.y as f32 + sub_pos.py);
            let target_pos = Vec2::new(target.x as f32, target.y as f32);
            let dir = target_pos - current_pos;
            let dist = dir.length();
            
            if dist > 0.1 {
                // Move towards target
                let move_dist = speed.speed * dt;
                let move_vec = dir.normalize() * move_dist.min(dist);
                
                // Calculate new position
                let mut new_px = sub_pos.px + move_vec.x;
                let mut new_py = sub_pos.py + move_vec.y;
                let mut new_x = pos.x;
                let mut new_y = pos.y;
                
                // Normalize sub-position
                while new_px >= 1.0 {
                    new_px -= 1.0;
                    new_x += 1;
                }
                while new_px < 0.0 {
                    new_px += 1.0;
                    new_x -= 1;
                }
                while new_py >= 1.0 {
                    new_py -= 1.0;
                    new_y += 1;
                }
                while new_py < 0.0 {
                    new_py += 1.0;
                    new_y -= 1;
                }
                
                // Check collision
                if tilemap.is_walkable(new_x, new_y) {
                    let direction = direction_from_vector(move_vec);
                    updates.push((entity, new_x, new_y, new_px, new_py, direction));
                }
            }
        }
        
        // Apply updates
        for (entity, new_x, new_y, new_px, new_py, direction) in updates {
            if let Ok((pos, sub_pos)) = world.query_one_mut::<(&mut Position, &mut SubPosition)>(entity) {
                pos.x = new_x;
                pos.y = new_y;
                sub_pos.px = new_px;
                sub_pos.py = new_py;
            }
            // Update direction separately
            if let Ok(dir_comp) = world.query_one_mut::<&mut Direction4>(entity) {
                *dir_comp = direction;
            }
        }
    }
    
    /// Move an entity with collision detection
    pub fn move_entity(
        world: &mut World,
        entity: crate::Entity,
        direction: Vec2,
        speed: f32,
        tilemap: &TileCollisionMap,
        dt: f32,
    ) -> bool {
        // Get current position
        let (pos, sub_pos) = {
            let mut query = match world.query_one::<(&Position, &SubPosition)>(entity) {
                Ok(q) => q,
                Err(_) => return false,
            };
            match query.get() {
                Some((p, s)) => (*p, *s),
                None => return false,
            }
        };
        
        // Calculate movement
        let move_vec = direction * speed * dt;
        let mut new_sub_x = sub_pos.px + move_vec.x / 32.0; // Convert from pixels to tiles
        let mut new_sub_y = sub_pos.py + move_vec.y / 32.0;
        let mut new_x = pos.x;
        let mut new_y = pos.y;
        
        // Handle sub-position overflow
        while new_sub_x >= 1.0 {
            new_sub_x -= 1.0;
            new_x += 1;
        }
        while new_sub_x < 0.0 {
            new_sub_x += 1.0;
            new_x -= 1;
        }
        while new_sub_y >= 1.0 {
            new_sub_y -= 1.0;
            new_y += 1;
        }
        while new_sub_y < 0.0 {
            new_sub_y += 1.0;
            new_y -= 1;
        }
        
        // Check collision at new position
        if tilemap.is_walkable(new_x, new_y) {
            // Apply movement
            if let Ok((pos, sub_pos)) = world.query_one_mut::<(&mut Position, &mut SubPosition)>(entity) {
                pos.x = new_x;
                pos.y = new_y;
                sub_pos.px = new_sub_x;
                sub_pos.py = new_sub_y;
            }
            
            // Update direction
            if let Ok(dir_comp) = world.query_one_mut::<&mut Direction4>(entity) {
                *dir_comp = direction_from_vector(direction);
            }
            
            true
        } else {
            false
        }
    }
}

/// Convert a vector to the closest cardinal direction
fn direction_from_vector(v: Vec2) -> Direction4 {
    if v.x.abs() > v.y.abs() {
        if v.x > 0.0 {
            Direction4::Right
        } else {
            Direction4::Left
        }
    } else {
        if v.y > 0.0 {
            Direction4::Down
        } else {
            Direction4::Up
        }
    }
}

/// Simple tile collision map
#[derive(Debug, Clone, Default)]
pub struct TileCollisionMap {
    pub width: i32,
    pub height: i32,
    pub walkable: Vec<bool>,
}

impl TileCollisionMap {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            walkable: vec![true; (width * height) as usize],
        }
    }
    
    /// Check if a tile is walkable
    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return false; // Out of bounds
        }
        let idx = (y * self.width + x) as usize;
        self.walkable.get(idx).copied().unwrap_or(false)
    }
    
    /// Set tile walkability
    pub fn set_walkable(&mut self, x: i32, y: i32, walkable: bool) {
        if x >= 0 && x < self.width && y >= 0 && y < self.height {
            let idx = (y * self.width + x) as usize;
            self.walkable[idx] = walkable;
        }
    }
    
    /// Block edges of the map
    pub fn block_edges(&mut self) {
        for x in 0..self.width {
            self.set_walkable(x, 0, false);
            self.set_walkable(x, self.height - 1, false);
        }
        for y in 0..self.height {
            self.set_walkable(0, y, false);
            self.set_walkable(self.width - 1, y, false);
        }
    }
}
