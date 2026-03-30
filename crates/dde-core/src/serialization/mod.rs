//! Serialization system for save/load
//!
//! Provides JSON serialization for entities, world state, and game data.

use hecs::World;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::components::animation::{AnimationState, RenderLayer, Sprite};
use crate::components::battle::{AtbGauge, Combatant};
use crate::components::behavior::{LogicPrompt, MovementSpeed, PatrolPath, Schedule};
use crate::components::render::ColorTint;
use crate::components::*;
use crate::vibecode::Vibecode;
use crate::{Direction4, Entity, EntityKind};

/// Serializable entity data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedEntity {
    /// Entity ID (stable across saves)
    pub id: u64,
    /// Entity kind
    pub kind: EntityKind,
    /// Position
    pub position: Option<Position>,
    /// Sub-position (for smooth movement)
    pub sub_position: Option<SubPosition>,
    /// Name
    pub name: Option<Name>,
    /// Stats
    pub stats: Option<Stats>,
    /// Movement speed
    pub movement_speed: Option<MovementSpeed>,
    /// Direction
    pub direction: Option<Direction4>,
    /// Sprite
    pub sprite: Option<Sprite>,
    /// Animation state
    pub animation: Option<AnimationState>,
    /// Vibecode (serialized as TOML)
    pub vibecode: Option<String>,
    /// Render layer
    pub render_layer: Option<RenderLayer>,
    /// Color tint
    pub color_tint: Option<ColorTint>,
    /// Dialogue tree ID
    pub dialogue_tree_id: Option<String>,
    /// Patrol path
    pub patrol_path: Option<PatrolPath>,
    /// Schedule
    pub schedule: Option<Schedule>,
    /// NPC bark settings
    pub npc_bark: Option<SerializedNpcBark>,
    /// Battle components
    pub atb_gauge: Option<AtbGauge>,
    pub combatant: Option<Combatant>,
}

/// Serializable NPC bark data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedNpcBark {
    pub cooldown_secs: f32,
    pub proximity_radius: f32,
    pub vibecode: Option<String>,
}

/// World state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    /// Version for migration
    pub version: u32,
    /// Timestamp
    pub timestamp: i64,
    /// Seed for RNG
    pub seed: u64,
    /// Tick count
    pub tick_count: u64,
    /// Entities
    pub entities: Vec<SerializedEntity>,
    /// Player entity ID
    pub player_entity_id: Option<u64>,
    /// World state flags
    pub flags: HashMap<String, bool>,
    /// Game variables
    pub variables: HashMap<String, i32>,
    /// Switches (game flags)
    pub switches: HashMap<String, bool>,
}

impl WorldSnapshot {
    /// Current save format version
    pub const CURRENT_VERSION: u32 = 1;

    /// Create a new empty snapshot
    pub fn new(seed: u64) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            timestamp: chrono::Utc::now().timestamp_millis(),
            seed,
            tick_count: 0,
            entities: Vec::new(),
            player_entity_id: None,
            flags: HashMap::new(),
            variables: HashMap::new(),
            switches: HashMap::new(),
        }
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Entity serializer
pub struct EntitySerializer;

impl EntitySerializer {
    /// Serialize an entity from the world
    pub fn serialize(world: &World, entity: Entity, id: u64) -> Option<SerializedEntity> {
        let kind = world.get::<&EntityKindComp>(entity).ok()?;
        let kind_value = kind.kind;

        let position = world.get::<&Position>(entity).ok().map(|p| *p);
        let sub_position = world.get::<&SubPosition>(entity).ok().map(|p| *p);
        let name = world.get::<&Name>(entity).ok().map(|n| (*n).clone());
        let stats = world.get::<&Stats>(entity).ok().map(|s| *s);
        let movement_speed = world.get::<&MovementSpeed>(entity).ok().map(|m| *m);
        let direction = world.get::<&Direction4>(entity).ok().map(|d| *d);
        let sprite = world.get::<&Sprite>(entity).ok().map(|s| *s);
        let animation = world.get::<&AnimationState>(entity).ok().map(|a| *a);
        let render_layer = world.get::<&RenderLayer>(entity).ok().map(|r| *r);
        let color_tint = world.get::<&ColorTint>(entity).ok().map(|c| *c);
        let patrol_path = world.get::<&PatrolPath>(entity).ok().map(|p| (*p).clone());
        let schedule = world.get::<&Schedule>(entity).ok().map(|s| (*s).clone());
        let atb_gauge = world.get::<&AtbGauge>(entity).ok().map(|a| *a);
        let combatant = world.get::<&Combatant>(entity).ok().map(|c| *c);

        // Serialize Vibecode to TOML string
        let vibecode = world.get::<&LogicPrompt>(entity).ok().and_then(|lp| {
            Vibecode::from_toml(&lp.directives)
                .ok()
                .and_then(|v| v.to_toml().ok())
        });

        // Serialize NPC bark
        let npc_bark = world
            .get::<&crate::systems::NpcBark>(entity)
            .ok()
            .map(|bark| SerializedNpcBark {
                cooldown_secs: bark.cooldown_secs,
                proximity_radius: bark.proximity_radius,
                vibecode: bark.vibecode.as_ref().and_then(|v| v.to_toml().ok()),
            });

        Some(SerializedEntity {
            id,
            kind: kind_value,
            position,
            sub_position,
            name,
            stats,
            movement_speed,
            direction,
            sprite,
            animation,
            vibecode,
            render_layer,
            color_tint,
            dialogue_tree_id: None, // TODO: Add dialogue tree component
            patrol_path,
            schedule,
            npc_bark,
            atb_gauge,
            combatant,
        })
    }

    /// Deserialize an entity into the world
    pub fn deserialize(world: &mut World, data: &SerializedEntity) -> Entity {
        let mut builder = hecs::EntityBuilder::new();

        // Add kind
        builder.add(EntityKindComp { kind: data.kind });

        // Add optional components
        if let Some(pos) = data.position {
            builder.add(pos);
        }
        if let Some(sub_pos) = data.sub_position {
            builder.add(sub_pos);
        }
        if let Some(name) = &data.name {
            builder.add(name.clone());
        }
        if let Some(stats) = data.stats {
            builder.add(stats);
        }
        if let Some(speed) = data.movement_speed {
            builder.add(speed);
        }
        if let Some(dir) = data.direction {
            builder.add(dir);
        }
        if let Some(sprite) = &data.sprite {
            builder.add(*sprite);
        }
        if let Some(anim) = data.animation {
            builder.add(anim);
        }
        if let Some(layer) = data.render_layer {
            builder.add(layer);
        }
        if let Some(tint) = data.color_tint {
            builder.add(tint);
        }
        if let Some(patrol) = &data.patrol_path {
            builder.add(patrol.clone());
        }
        if let Some(schedule) = &data.schedule {
            builder.add(schedule.clone());
        }
        if let Some(atb) = data.atb_gauge {
            builder.add(atb);
        }
        if let Some(combatant) = &data.combatant {
            builder.add(*combatant);
        }

        // Add Vibecode as LogicPrompt
        if let Some(vibe_toml) = &data.vibecode {
            builder.add(LogicPrompt {
                directives: vibe_toml.clone(),
            });
        }

        // Add NPC bark
        if let Some(bark_data) = &data.npc_bark {
            use crate::systems::NpcBark;
            let mut bark = NpcBark::new()
                .with_cooldown(bark_data.cooldown_secs)
                .with_proximity(bark_data.proximity_radius);

            if let Some(vibe_toml) = &bark_data.vibecode {
                if let Ok(vibecode) = Vibecode::from_toml(vibe_toml) {
                    bark = bark.with_vibecode(vibecode);
                }
            }

            builder.add(bark);
        }

        world.spawn(builder.build())
    }
}

/// World serializer
pub struct WorldSerializer;

impl WorldSerializer {
    /// Serialize the entire world
    pub fn serialize(world: &World, seed: u64, tick_count: u64) -> WorldSnapshot {
        let mut snapshot = WorldSnapshot::new(seed);
        snapshot.tick_count = tick_count;

        // Serialize all entities
        let mut id_counter: u64 = 1;

        for (entity, _kind) in world.query::<&EntityKindComp>().iter() {
            if let Some(serialized) = EntitySerializer::serialize(world, entity, id_counter) {
                snapshot.entities.push(serialized);
                id_counter += 1;
            }
        }

        snapshot
    }

    /// Deserialize world from snapshot
    pub fn deserialize(world: &mut World, snapshot: &WorldSnapshot) {
        // Clear existing entities
        world.clear();

        // Spawn all entities
        for entity_data in &snapshot.entities {
            EntitySerializer::deserialize(world, entity_data);
        }
    }

    /// Serialize to JSON file
    pub fn save_to_file(
        world: &World,
        seed: u64,
        tick_count: u64,
        path: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let snapshot = Self::serialize(world, seed, tick_count);
        let json = snapshot.to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Deserialize from JSON file
    pub fn load_from_file(
        world: &mut World,
        path: &std::path::Path,
    ) -> Result<WorldSnapshot, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let snapshot = WorldSnapshot::from_json(&json)?;
        Self::deserialize(world, &snapshot);
        Ok(snapshot)
    }
}

/// Component for stable entity IDs across saves
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StableId(pub u64);

/// Game save data (includes world + metadata)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSave {
    /// Save format version
    pub version: u32,
    /// Save timestamp
    pub timestamp: i64,
    /// Save slot number
    pub slot: u32,
    /// Play time in seconds
    pub play_time_secs: u64,
    /// Player name
    pub player_name: String,
    /// Current map
    pub current_map: String,
    /// World snapshot
    pub world: WorldSnapshot,
    /// Screenshot data (optional)
    pub screenshot: Option<Vec<u8>>,
}

impl GameSave {
    /// Current save format version
    pub const CURRENT_VERSION: u32 = 1;

    /// Create a new game save
    pub fn new(slot: u32, player_name: impl Into<String>, current_map: impl Into<String>) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            timestamp: chrono::Utc::now().timestamp_millis(),
            slot,
            play_time_secs: 0,
            player_name: player_name.into(),
            current_map: current_map.into(),
            world: WorldSnapshot::new(0),
            screenshot: None,
        }
    }

    /// Get formatted timestamp
    pub fn formatted_time(&self) -> String {
        let dt = chrono::DateTime::from_timestamp_millis(self.timestamp)
            .unwrap_or_else(chrono::Utc::now);
        dt.format("%Y-%m-%d %H:%M").to_string()
    }

    /// Get formatted play time
    pub fn formatted_play_time(&self) -> String {
        let hours = self.play_time_secs / 3600;
        let minutes = (self.play_time_secs % 3600) / 60;
        let secs = self.play_time_secs % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_serialization() {
        let mut world = World::new();

        // Spawn test entity
        let entity = world.spawn((
            EntityKindComp {
                kind: crate::EntityKind::Player,
            },
            Position::new(10, 20, 0),
            Name::new("Test Player", "player"),
            Stats::default(),
            Direction4::Down,
        ));

        // Serialize
        let serialized = EntitySerializer::serialize(&world, entity, 1).unwrap();
        assert_eq!(serialized.id, 1);
        assert_eq!(serialized.kind, crate::EntityKind::Player);
        assert!(serialized.position.is_some());

        // Deserialize into new world
        let mut new_world = World::new();
        let new_entity = EntitySerializer::deserialize(&mut new_world, &serialized);

        // Verify
        let pos = new_world.get::<&Position>(new_entity).unwrap();
        assert_eq!(pos.x, 10);
        assert_eq!(pos.y, 20);
    }

    #[test]
    fn test_world_snapshot() {
        let mut world = World::new();

        world.spawn((
            EntityKindComp {
                kind: crate::EntityKind::Player,
            },
            Position::new(0, 0, 0),
            Name::new("Player", "player"),
        ));

        world.spawn((
            EntityKindComp {
                kind: crate::EntityKind::Npc,
            },
            Position::new(5, 5, 0),
            Name::new("NPC", "npc"),
        ));

        let snapshot = WorldSerializer::serialize(&world, 12345, 100);
        assert_eq!(snapshot.entities.len(), 2);
        assert_eq!(snapshot.seed, 12345);
        assert_eq!(snapshot.tick_count, 100);

        // Test JSON roundtrip
        let json = snapshot.to_json().unwrap();
        let restored = WorldSnapshot::from_json(&json).unwrap();
        assert_eq!(restored.entities.len(), 2);
    }

    #[test]
    fn test_game_save() {
        let save = GameSave::new(1, "Test Player", "map_001");
        assert_eq!(save.slot, 1);
        assert_eq!(save.player_name, "Test Player");
        assert_eq!(save.current_map, "map_001");

        // Test JSON
        let json = save.to_json().unwrap();
        let restored = GameSave::from_json(&json).unwrap();
        assert_eq!(restored.player_name, "Test Player");
    }
}
