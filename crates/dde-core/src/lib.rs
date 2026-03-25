//! DocDamage Engine - Core Library
//! 
//! This crate provides the foundational ECS architecture, simulation loop,
//! event bus, and core game logic for the DocDamage Engine.

pub mod components;
pub mod events;
pub mod resources;
pub mod systems;

use std::time::Duration;

pub use glam;
pub use hecs;

/// Fixed simulation tick rate: 20 ticks per second (50ms per tick)
pub const TICK_RATE: Duration = Duration::from_millis(50);

/// Maximum catch-up ticks to prevent death spiral
pub const MAX_CATCH_UP_TICKS: u32 = 10;

/// ECS World type alias for convenience
pub type World = hecs::World;

/// Entity type alias
pub type Entity = hecs::Entity;

/// Result type for core operations
pub type Result<T> = std::result::Result<T, CoreError>;

/// Core error types
#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("ECS error: {0}")]
    Ecs(#[from] hecs::ComponentError),
    
    #[error("Entity not found: {0:?}")]
    EntityNotFound(Entity),
    
    #[error("Invalid state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },
    
    #[error("Simulation error: {0}")]
    Simulation(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Direction enum for 4-way movement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum Direction4 {
    #[default]
    Down = 0,
    Left = 1,
    Right = 2,
    Up = 3,
}

impl Direction4 {
    /// Get the opposite direction
    pub fn opposite(self) -> Self {
        match self {
            Direction4::Down => Direction4::Up,
            Direction4::Up => Direction4::Down,
            Direction4::Left => Direction4::Right,
            Direction4::Right => Direction4::Left,
        }
    }
    
    /// Convert to a vector
    pub fn to_vec2(self) -> glam::Vec2 {
        match self {
            Direction4::Down => glam::Vec2::new(0.0, 1.0),
            Direction4::Up => glam::Vec2::new(0.0, -1.0),
            Direction4::Left => glam::Vec2::new(-1.0, 0.0),
            Direction4::Right => glam::Vec2::new(1.0, 0.0),
        }
    }
}

/// World state for tiles/areas
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum WorldState {
    #[default]
    Balance = 0,
    Ruin = 1,
    Reclaimed = 2,
}

impl WorldState {
    /// Get display name
    pub fn name(self) -> &'static str {
        match self {
            WorldState::Balance => "Balance",
            WorldState::Ruin => "Ruin",
            WorldState::Reclaimed => "Reclaimed",
        }
    }
}

/// Biome types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BiomeKind {
    Desert,
    Snow,
    Forest,
    Ocean,
    Void,
    Town,
    Dungeon,
    Grassland,
    Mountain,
}

impl BiomeKind {
    /// Get display name
    pub fn name(self) -> &'static str {
        match self {
            BiomeKind::Desert => "Desert",
            BiomeKind::Snow => "Snow",
            BiomeKind::Forest => "Forest",
            BiomeKind::Ocean => "Ocean",
            BiomeKind::Void => "Void",
            BiomeKind::Town => "Town",
            BiomeKind::Dungeon => "Dungeon",
            BiomeKind::Grassland => "Grassland",
            BiomeKind::Mountain => "Mountain",
        }
    }
}

/// Map type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MapType {
    Overworld,
    Town,
    Dungeon,
    Interior,
    BattleArena,
}

/// Entity kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EntityKind {
    Player,
    Npc,
    Object,
    Town,
    Boss,
    Enemy,
    Chest,
    Door,
    Sign,
    Projectile,
}

/// Element types for combat
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Element {
    Fire,
    Ice,
    Lightning,
    Holy,
    Dark,
    None,
}

/// Game state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameState {
    Title,
    Overworld,
    Battle,
    Dialogue,
    Menu,
    Cutscene,
    Editor,
}

/// Input action types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputAction {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Confirm,
    Cancel,
    Menu,
    Interact,
    Run,
}

/// Transition kinds between game states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionKind {
    Fade { color: [f32; 3], duration_ms: u32 },
    Wipe { direction: WipeDirection, duration_ms: u32 },
    BattleSwirl { duration_ms: u32 },
    Instant,
}

/// Wipe directions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WipeDirection {
    Left,
    Right,
    Up,
    Down,
}
