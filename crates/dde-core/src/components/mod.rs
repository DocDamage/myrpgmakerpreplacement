//! ECS Components
//!
//! Components are plain data structs attached to entities.
//! No methods, no behavior - just data.

use glam::{IVec3, Vec2};
use serde::{Deserialize, Serialize};

use crate::{BiomeKind, EntityKind, WorldState};

// Re-export all component modules
pub mod animation;
pub mod audio;
pub mod battle;
pub mod behavior;
pub mod render;

/// Position component - tile coordinates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub z: i32, // Layer: -1=underground, 0=surface, 1=overhead
}

impl Position {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn to_ivec3(self) -> IVec3 {
        IVec3::new(self.x, self.y, self.z)
    }

    pub fn distance_squared(self, other: Position) -> i32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }
}

/// Sub-position for smooth pixel-level movement
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct SubPosition {
    pub px: f32,
    pub py: f32,
}

impl SubPosition {
    pub fn new(px: f32, py: f32) -> Self {
        Self { px, py }
    }

    pub fn to_vec2(self) -> Vec2 {
        Vec2::new(self.px, self.py)
    }
}

/// World state component for tiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WorldStateComp {
    pub state: WorldState,
}

/// Biome component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Biome {
    pub kind: BiomeKind,
}

impl Default for Biome {
    fn default() -> Self {
        Self {
            kind: BiomeKind::Grassland,
        }
    }
}

/// Passability flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Passability {
    pub walkable: bool,
    pub swimable: bool,
    pub flyable: bool,
}

/// Tileset reference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TilesetRef {
    pub tileset_id: u32,
    pub tile_index: u32,
}

/// Entity kind component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityKindComp {
    pub kind: EntityKind,
}

impl Default for EntityKindComp {
    fn default() -> Self {
        Self {
            kind: EntityKind::Npc,
        }
    }
}

/// Name component
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Name {
    pub display: String,
    pub internal: String,
}

impl Name {
    pub fn new(display: impl Into<String>, internal: impl Into<String>) -> Self {
        Self {
            display: display.into(),
            internal: internal.into(),
        }
    }
}

/// Stats component for combat
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Stats {
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
    pub str: i32,
    pub def: i32,
    pub spd: i32,
    pub mag: i32,
    pub luck: i32,
    pub level: i32,
    pub exp: i32,
}

impl Stats {
    /// Calculate HP percentage
    pub fn hp_percent(&self) -> f32 {
        if self.max_hp == 0 {
            0.0
        } else {
            self.hp as f32 / self.max_hp as f32
        }
    }

    /// Calculate MP percentage
    pub fn mp_percent(&self) -> f32 {
        if self.max_mp == 0 {
            0.0
        } else {
            self.mp as f32 / self.max_mp as f32
        }
    }

    /// Check if alive
    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    /// Apply damage
    pub fn take_damage(&mut self, amount: i32) {
        self.hp = (self.hp - amount).max(0);
    }

    /// Heal
    pub fn heal(&mut self, amount: i32) {
        self.hp = (self.hp + amount).min(self.max_hp);
    }
}

/// Item slot in inventory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemSlot {
    pub item_id: u32,
    pub quantity: u32,
}

/// Inventory component
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Inventory {
    pub items: Vec<ItemSlot>,
}

impl Inventory {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Add item to inventory
    pub fn add_item(&mut self, item_id: u32, quantity: u32, max_stack: u32) {
        if let Some(slot) = self.items.iter_mut().find(|s| s.item_id == item_id) {
            let new_qty = (slot.quantity + quantity).min(max_stack);
            slot.quantity = new_qty;
        } else {
            self.items.push(ItemSlot { item_id, quantity });
        }
    }

    /// Remove item from inventory
    pub fn remove_item(&mut self, item_id: u32, quantity: u32) -> bool {
        if let Some(pos) = self.items.iter().position(|s| s.item_id == item_id) {
            if self.items[pos].quantity > quantity {
                self.items[pos].quantity -= quantity;
                true
            } else if self.items[pos].quantity == quantity {
                self.items.remove(pos);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Get item quantity
    pub fn get_quantity(&self, item_id: u32) -> u32 {
        self.items
            .iter()
            .find(|s| s.item_id == item_id)
            .map(|s| s.quantity)
            .unwrap_or(0)
    }

    /// Check if has item
    pub fn has_item(&self, item_id: u32, min_qty: u32) -> bool {
        self.get_quantity(item_id) >= min_qty
    }
}

/// Equipment slots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Equipment {
    pub weapon: Option<u32>,
    pub armor: Option<u32>,
    pub accessory: Option<u32>,
}

/// Status effect instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusInstance {
    pub effect_id: u32,
    pub duration_turns: u32,
    pub source_entity: u64, // Store as u64 for serialization
}

/// Status effects component
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StatusEffects {
    pub active: Vec<StatusInstance>,
}

/// Camera target component
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraTarget {
    pub entity: crate::Entity,
    pub offset: Vec2,
}

/// Camera configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraConfig {
    pub follow_speed: f32,
    pub deadzone: Vec2,
    pub zoom: f32,
    pub bounds: Option<CameraBounds>,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            follow_speed: 5.0,
            deadzone: Vec2::new(32.0, 32.0),
            zoom: 1.0,
            bounds: None,
        }
    }
}

/// Camera bounds
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraBounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

/// Map ID component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MapId {
    pub id: u32,
}

/// Faction ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FactionId {
    pub id: u32,
}

/// Interaction flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Interactable {
    pub is_interactable: bool,
    pub is_collidable: bool,
}

/// Respawn configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Respawn {
    /// Ticks until respawn after defeat. 0 = no respawn, -1 = permanent death
    pub respawn_ticks: i32,
}
