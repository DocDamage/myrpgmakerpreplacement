//! World State Analyzer
//!
//! Analyzes the game world to extract context for quest generation.
//! Collects player state, world state, recent events, and NPC relationships.

use dde_core::components::{EntityKindComp, FactionId, Inventory, MapId, Name, Position, Stats};
use dde_core::{Entity, EntityKind, World};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Analyzes world state to determine what content to generate
#[derive(Debug, Clone, Default)]
pub struct WorldAnalyzer {
    /// Recent world events (circular buffer)
    recent_events: VecDeque<WorldEvent>,
    /// Maximum events to track
    max_events: usize,
    /// Time since last combat
    time_since_combat: f32,
    /// Current tension calculation
    current_tension: f32,
}

impl WorldAnalyzer {
    /// Create a new world analyzer
    pub fn new() -> Self {
        Self {
            recent_events: VecDeque::with_capacity(10),
            max_events: 10,
            time_since_combat: f32::MAX,
            current_tension: 0.0,
        }
    }

    /// Analyze current situation and return context for quest generation
    pub fn analyze(&mut self, world: &World, player: Entity) -> GameContext {
        // Get player components
        let player_pos = world.get::<&Position>(player).ok().map(|p| *p);
        let player_stats = world.get::<&Stats>(player).ok().map(|s| *s);
        let player_inventory = world.get::<&Inventory>(player).ok();
        let player_map = world.get::<&MapId>(player).ok().map(|m| *m);

        // Calculate player location
        let player_location = if let (Some(pos), Some(map)) = (player_pos, player_map) {
            (map.id, pos.x, pos.y)
        } else {
            (0, 0, 0)
        };

        // Get player level
        let player_level = player_stats.map(|s| s.level as u32).unwrap_or(1);

        // Calculate player power score
        let player_power = self.calculate_player_power(&player_stats);

        // Collect faction standings
        let faction_standings = self.collect_faction_standings(world, player);

        // Get nearby NPCs
        let nearby_npcs = self.collect_nearby_npcs(world, player_pos, 10);

        // Get active quests (placeholder - would query quest system)
        let active_quests = self.collect_active_quests();

        // Build world state snapshot
        let world_state = WorldStateSnapshot {
            time_of_day: self.estimate_time_of_day(),
            weather: Weather::Clear,
            calamity_level: self.estimate_calamity_level(world),
            biome: Biome::Grassland,
            danger_level: self.estimate_danger_level(world, player_pos),
        };

        // Calculate tension
        let tension_level = self.calculate_tension();

        // Build context
        GameContext {
            player_location,
            player_level,
            player_power,
            recent_events: self.recent_events.iter().cloned().collect(),
            faction_standings,
            active_quests,
            world_state,
            nearby_npcs,
            tension_level,
            time_since_combat: self.time_since_combat,
            inventory_items: player_inventory.map(|i| i.items.len() as u32).unwrap_or(0),
            player_health_percent: player_stats.map(|s| s.hp_percent()).unwrap_or(1.0),
        }
    }

    /// Record a world event for context tracking
    pub fn record_event(&mut self, event: WorldEvent) {
        // Update tension based on event first
        self.update_tension_for_event(&event);
        
        if self.recent_events.len() >= self.max_events {
            self.recent_events.pop_front();
        }
        self.recent_events.push_back(event);
    }

    /// Update time tracking (called each frame)
    pub fn tick(&mut self, dt: f32) {
        self.time_since_combat += dt;

        // Decay tension naturally
        self.current_tension = (self.current_tension - dt * 0.01).max(0.0);
    }

    /// Record combat occurred
    pub fn record_combat(&mut self) {
        self.time_since_combat = 0.0;
        self.current_tension = (self.current_tension + 0.3).min(1.0);
        self.record_event(WorldEvent::CombatEncounter);
    }

    /// Calculate player power score (0.0 - 100.0)
    fn calculate_player_power(&self, stats: &Option<Stats>) -> f32 {
        if let Some(s) = stats {
            let level_factor = s.level as f32 * 2.0;
            let stat_factor = (s.str + s.def + s.spd + s.mag) as f32 / 4.0;
            let hp_factor = s.hp_percent() * 10.0;
            (level_factor + stat_factor + hp_factor).min(100.0)
        } else {
            1.0
        }
    }

    /// Collect faction standings for player
    fn collect_faction_standings(&self, world: &World, player: Entity) -> HashMap<u32, i32> {
        let mut standings = HashMap::new();

        // Query all NPCs and calculate faction relationships
        for (_, (faction, _)) in world.query::<(&FactionId, &EntityKindComp)>().iter() {
            if faction.id > 0 {
                // Calculate standing based on interactions (simplified)
                let standing = self.calculate_faction_standing(world, player, faction.id);
                standings.insert(faction.id, standing);
            }
        }

        standings
    }

    /// Calculate standing with a specific faction
    fn calculate_faction_standing(&self, _world: &World, _player: Entity, faction_id: u32) -> i32 {
        // Placeholder - would query relationship system
        // Returns range -100 (hostile) to +100 (friendly)
        match faction_id {
            1 => 50,  // Default faction - neutral positive
            2 => 0,   // Another faction - neutral
            3 => -20, // Hostile faction
            _ => 0,
        }
    }

    /// Collect nearby NPCs within radius
    fn collect_nearby_npcs(
        &self,
        world: &World,
        center: Option<Position>,
        radius: i32,
    ) -> Vec<NpcInfo> {
        let mut npcs = Vec::new();

        if let Some(center_pos) = center {
            let radius_sq = radius * radius;

            for (entity, (pos, name, kind, faction)) in world
                .query::<(&Position, &Name, &EntityKindComp, Option<&FactionId>)>()
                .iter()
            {
                if kind.kind != EntityKind::Npc {
                    continue;
                }

                let dist_sq = center_pos.distance_squared(*pos);
                if dist_sq <= radius_sq {
                    npcs.push(NpcInfo {
                        entity,
                        name: name.display.clone(),
                        internal_name: name.internal.clone(),
                        position: (pos.x, pos.y),
                        distance: (dist_sq as f32).sqrt(),
                        faction_id: faction.map(|f| f.id),
                        relationship_score: faction
                            .map(|f| self.calculate_faction_standing(world, entity, f.id))
                            .unwrap_or(0),
                        is_quest_giver: false, // Would check quest component
                        is_merchant: false,    // Would check merchant component
                    });
                }
            }
        }

        // Sort by distance
        npcs.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
        npcs
    }

    /// Collect currently active quests
    fn collect_active_quests(&self) -> Vec<QuestStatus> {
        // Placeholder - would query active quest system
        vec![]
    }

    /// Estimate time of day from game state
    fn estimate_time_of_day(&self) -> u8 {
        // Placeholder - would query time system
        12 // Noon
    }

    /// Estimate current calamity level (0-10)
    fn estimate_calamity_level(&self, _world: &World) -> u8 {
        // Placeholder - would query world state system
        2
    }

    /// Estimate danger level at position (0.0 - 1.0)
    fn estimate_danger_level(&self, _world: &World, _pos: Option<Position>) -> f32 {
        // Placeholder - would query encounter zones
        0.2
    }

    /// Calculate current tension level
    fn calculate_tension(&self) -> f32 {
        self.current_tension
    }

    /// Update tension based on event type
    fn update_tension_for_event(&mut self, event: &WorldEvent) {
        let tension_delta = match event {
            WorldEvent::CombatEncounter => 0.3,
            WorldEvent::BossDefeated { .. } => -0.4,
            WorldEvent::QuestCompleted { .. } => -0.2,
            WorldEvent::QuestFailed { .. } => 0.2,
            WorldEvent::NpcDeath { .. } => 0.25,
            WorldEvent::Discovery { .. } => -0.1,
            WorldEvent::FactionShift { .. } => 0.15,
            WorldEvent::CalamityEvent => 0.5,
            WorldEvent::PlayerDeath => 0.4,
            _ => 0.0,
        };
        self.current_tension = (self.current_tension + tension_delta).clamp(0.0, 1.0);
    }

    /// Get current tension level
    pub fn current_tension(&self) -> f32 {
        self.current_tension
    }

    /// Get time since last combat
    pub fn time_since_combat(&self) -> f32 {
        self.time_since_combat
    }
}

/// Game context for quest generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameContext {
    /// Player location (map_id, x, y)
    pub player_location: (u32, i32, i32),
    /// Player character level
    pub player_level: u32,
    /// Player power score (0-100)
    pub player_power: f32,
    /// Recent world events (last 10)
    pub recent_events: Vec<WorldEvent>,
    /// Faction standings (faction_id -> standing)
    pub faction_standings: HashMap<u32, i32>,
    /// Currently active quests
    pub active_quests: Vec<QuestStatus>,
    /// World state snapshot
    pub world_state: WorldStateSnapshot,
    /// Nearby NPCs within radius
    pub nearby_npcs: Vec<NpcInfo>,
    /// Current tension level (0.0 - 1.0)
    pub tension_level: f32,
    /// Time since last combat (seconds)
    pub time_since_combat: f32,
    /// Number of items in inventory
    pub inventory_items: u32,
    /// Player health percentage (0.0 - 1.0)
    pub player_health_percent: f32,
}

impl GameContext {
    /// Get a text summary of the context for LLM prompts
    pub fn to_prompt_context(&self) -> String {
        let mut context = format!(
            "Player Level: {} (Power: {:.0})\n\
             Location: Map {} at ({}, {})\n\
             Health: {:.0}%\n\
             Tension: {:.0}%\n\
             Time Since Combat: {:.0}s\n",
            self.player_level,
            self.player_power,
            self.player_location.0,
            self.player_location.1,
            self.player_location.2,
            self.player_health_percent * 100.0,
            self.tension_level * 100.0,
            self.time_since_combat
        );

        if !self.recent_events.is_empty() {
            context.push_str("\nRecent Events:\n");
            for event in &self.recent_events {
                context.push_str(&format!("- {:?}\n", event));
            }
        }

        if !self.nearby_npcs.is_empty() {
            context.push_str("\nNearby NPCs:\n");
            for npc in self.nearby_npcs.iter().take(5) {
                context.push_str(&format!(
                    "- {} (relationship: {})\n",
                    npc.name, npc.relationship_score
                ));
            }
        }

        if !self.active_quests.is_empty() {
            context.push_str("\nActive Quests:\n");
            for quest in &self.active_quests {
                context.push_str(&format!("- {} ({:?})\n", quest.name, quest.stage));
            }
        }

        context
    }

    /// Check if player is in a "quiet" state suitable for new quests
    pub fn is_quiet_state(&self) -> bool {
        self.time_since_combat > 30.0
            && self.player_health_percent > 0.5
            && self.tension_level < 0.7
    }

    /// Get dominant faction in current area
    pub fn dominant_faction(&self) -> Option<u32> {
        self.faction_standings
            .iter()
            .max_by_key(|(_, standing)| standing.abs())
            .map(|(faction_id, _)| *faction_id)
    }

    /// Check if player has low health
    pub fn is_wounded(&self) -> bool {
        self.player_health_percent < 0.3
    }

    /// Check if player is exploring (not in known location)
    pub fn is_exploring(&self) -> bool {
        // Would check against known locations database
        true
    }
}

/// World events that affect quest generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorldEvent {
    /// Combat occurred
    CombatEncounter,
    /// Boss enemy defeated
    BossDefeated { boss_name: String },
    /// Quest completed
    QuestCompleted { quest_id: u64, quest_name: String },
    /// Quest failed
    QuestFailed { quest_id: u64, reason: String },
    /// NPC died
    NpcDeath { npc_name: String, killer: Option<String> },
    /// New location discovered
    Discovery { location_type: String, location_name: String },
    /// Faction relationship changed
    FactionShift { faction_id: u32, new_standing: i32 },
    /// Calamity event occurred
    CalamityEvent,
    /// Player died
    PlayerDeath,
    /// Item acquired
    ItemAcquired { item_name: String, rarity: Rarity },
    /// Level gained
    LevelUp { new_level: u32 },
    /// Dialogue with NPC
    Dialogue { npc_name: String, topic: String },
}

/// Item rarity
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

/// World state snapshot
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WorldStateSnapshot {
    /// Hour of day (0-23)
    pub time_of_day: u8,
    /// Current weather
    pub weather: Weather,
    /// Calamity level (0-10)
    pub calamity_level: u8,
    /// Current biome
    pub biome: Biome,
    /// Danger level (0.0 - 1.0)
    pub danger_level: f32,
}

/// Weather conditions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Weather {
    Clear,
    Cloudy,
    Rain,
    Storm,
    Fog,
    Snow,
}

/// Biome types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Biome {
    Grassland,
    Forest,
    Desert,
    Snow,
    Mountain,
    Swamp,
    Dungeon,
    Town,
}

fn invalid_entity() -> Entity {
    // Return an invalid entity for deserialization purposes
    Entity::from_bits(u64::MAX).unwrap_or_else(|| Entity::from_bits(0).unwrap())
}

/// Information about nearby NPCs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcInfo {
    /// Entity reference
    #[serde(skip, default = "invalid_entity")]
    pub entity: Entity,
    /// Display name
    pub name: String,
    /// Internal identifier
    pub internal_name: String,
    /// Position (x, y)
    pub position: (i32, i32),
    /// Distance from player
    pub distance: f32,
    /// Faction ID if any
    pub faction_id: Option<u32>,
    /// Relationship score (-100 to +100)
    pub relationship_score: i32,
    /// Whether NPC can give quests
    pub is_quest_giver: bool,
    /// Whether NPC is a merchant
    pub is_merchant: bool,
}

/// Active quest status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestStatus {
    /// Quest ID
    pub id: u64,
    /// Quest name
    pub name: String,
    /// Current stage
    pub stage: QuestStage,
    /// Completion percentage (0.0 - 1.0)
    pub completion: f32,
}

/// Quest stage
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QuestStage {
    NotStarted,
    Started,
    InProgress,
    AlmostComplete,
    TurnIn,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let analyzer = WorldAnalyzer::new();
        assert_eq!(analyzer.current_tension(), 0.0);
        assert_eq!(analyzer.time_since_combat(), f32::MAX);
    }

    #[test]
    fn test_event_recording() {
        let mut analyzer = WorldAnalyzer::new();
        analyzer.record_event(WorldEvent::CombatEncounter);
        analyzer.record_event(WorldEvent::CalamityEvent);
        
        // Tension should be increased
        assert!(analyzer.current_tension() > 0.0);
    }

    #[test]
    fn test_tension_decay() {
        let mut analyzer = WorldAnalyzer::new();
        analyzer.record_event(WorldEvent::CombatEncounter);
        let initial_tension = analyzer.current_tension();
        
        // Simulate time passing
        analyzer.tick(1.0);
        
        // Tension should have decayed
        assert!(analyzer.current_tension() < initial_tension);
    }

    #[test]
    fn test_context_quiet_state() {
        let context = GameContext {
            time_since_combat: 60.0,
            player_health_percent: 0.8,
            tension_level: 0.3,
            ..Default::default()
        };
        
        assert!(context.is_quiet_state());
    }

    impl Default for GameContext {
        fn default() -> Self {
            Self {
                player_location: (0, 0, 0),
                player_level: 1,
                player_power: 1.0,
                recent_events: vec![],
                faction_standings: HashMap::new(),
                active_quests: vec![],
                world_state: WorldStateSnapshot {
                    time_of_day: 12,
                    weather: Weather::Clear,
                    calamity_level: 0,
                    biome: Biome::Grassland,
                    danger_level: 0.0,
                },
                nearby_npcs: vec![],
                tension_level: 0.0,
                time_since_combat: f32::MAX,
                inventory_items: 0,
                player_health_percent: 1.0,
            }
        }
    }
}
