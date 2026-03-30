//! Event Bus System
//!
//! All subsystems communicate through typed events.
//! No subsubsystem holds a direct reference to another.
//!
//! This module provides two event bus implementations:
//!
//! 1. **`EventBus`** (new) - A typed, priority-based event bus with filtering support.
//!    Use this for new code that needs flexible event handling.
//!
//! 2. **`EngineEventBus`** (legacy) - A simple channel-based bus for `EngineEvent` enums.
//!    Maintained for backward compatibility.

pub mod battle;
pub mod bus;

// New typed event system
pub use bus::{
    downcast_event, Event, EventBus, EventFilter, EventPriority, EventType, SubscriptionId,
};

// Battle events
pub use battle::{BattleError, BattleEvent, BattleRewards};

use crate::{Element, Entity, GameState, WorldState};

/// Engine events - all inter-system communication goes through here
///
/// This is the legacy event enum used with `EngineEventBus`.
/// For new code, consider using the typed `Event` trait with `EventBus`.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    // ==================== World Events ====================
    /// Tile state changed
    TileStateChanged {
        tile_id: u64,
        old: WorldState,
        new: WorldState,
    },

    /// Entity spawned
    EntitySpawned { entity: Entity, kind: String },

    /// Entity destroyed
    EntityDestroyed { entity: Entity },

    /// Entity moved
    EntityMoved {
        entity: Entity,
        from: (i32, i32),
        to: (i32, i32),
    },

    // ==================== Sim Events ====================
    /// Simulation tick occurred
    SimulationTick { tick_count: u64 },

    /// Simulation stat changed
    SimStatChanged { key: String, old: f64, new: f64 },

    /// Calamity propagated
    CalamityPropagated { center: (i32, i32), radius: u32 },

    /// Era advanced
    EraAdvanced { new_era: u32 },

    // ==================== Interaction Events ====================
    /// Player interacted with something
    PlayerInteracted { target: Entity },

    /// Dialogue started
    DialogueStarted { npc: Entity, tree_id: Option<u32> },

    /// Dialogue ended
    DialogueEnded { npc: Entity },

    /// Dialogue choice selected
    DialogueChoiceSelected {
        choice_id: u32,
        next_node_id: Option<u32>,
    },

    // ==================== Battle Events ====================
    /// Battle triggered
    BattleTriggered {
        enemies: Vec<Entity>,
        terrain: String,
    },

    /// Battle ended
    BattleEnded { result: BattleResult },

    /// Damage dealt in battle
    DamageDealt {
        source: Entity,
        target: Entity,
        amount: i32,
        element: Element,
        is_crit: bool,
    },

    /// Entity defeated
    EntityDefeated {
        entity: Entity,
        killer: Option<Entity>,
    },

    /// Entity gained EXP
    ExpGained { entity: Entity, amount: u32 },

    /// Level up
    LevelUp { entity: Entity, new_level: i32 },

    // ==================== Audio Events ====================
    /// Stem crossfade
    StemCrossfade {
        track: String,
        target_volume: f32,
        duration_ms: u32,
    },

    /// Play SFX
    SfxPlay {
        sound_id: String,
        position: Option<(f32, f32)>,
    },

    /// Change BGM
    BgmChange { stem_set_id: String },

    // ==================== AI Events ====================
    /// AI request sent
    AiRequestSent {
        request_id: u64,
        task_type: AiTaskType,
    },

    /// AI response received
    AiResponseReceived { request_id: u64, response: String },

    /// AI sidecar unavailable
    AiSidecarUnavailable,

    // ==================== Editor Events ====================
    /// Brush applied
    BrushApplied {
        tiles: Vec<u64>,
        brush_type: BrushType,
    },

    /// Undo requested
    UndoRequested,

    /// Redo requested
    RedoRequested,

    /// Entity selected in editor
    EntitySelected { entity: Option<Entity> },

    // ==================== Scene Events ====================
    /// Scene transition
    SceneTransition {
        from: GameState,
        to: GameState,
        transition: String,
    },

    /// Sub-map entered
    SubMapEntered { entity: Entity, sub_map_id: u32 },

    /// Sub-map exited
    SubMapExited { entity: Entity },

    // ==================== Quest Events ====================
    /// Quest started
    QuestStarted { quest_id: u32 },

    /// Quest objective updated
    QuestObjectiveUpdated {
        quest_id: u32,
        objective_id: u32,
        progress: u32,
        required: u32,
    },

    /// Quest completed
    QuestCompleted { quest_id: u32 },

    /// Quest failed
    QuestFailed { quest_id: u32 },

    // ==================== Input Events ====================
    /// Input action pressed
    InputPressed { action: String },

    /// Input action released
    InputReleased { action: String },

    // ==================== System Events ====================
    /// Game saved
    GameSaved { slot: u32 },

    /// Game loaded
    GameLoaded { slot: u32 },

    /// Window resized
    WindowResized { width: u32, height: u32 },

    /// Request quit
    QuitRequested,
}

/// Battle result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleResult {
    Victory,
    Defeat,
    Flee,
}

/// AI task types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiTaskType {
    Dialogue,
    Bark,
    Narrative,
    Balancing,
    Shader,
    Music,
}

/// Brush types for editor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushType {
    Biome,
    Tile,
    Entity,
    Calamity,
}

/// Legacy event bus for publishing and subscribing to `EngineEvent`s
///
/// This is the original event bus implementation using a simple channel-based
/// approach. It is maintained for backward compatibility.
///
/// For new code, consider using the typed [`EventBus`] which provides:
/// - Type-safe events
/// - Priority-based dispatch
/// - Event filtering
/// - Subscription management
///
/// # Example
///
/// ```
/// use dde_core::events::{EngineEventBus, EngineEvent};
///
/// let bus = EngineEventBus::new();
/// bus.send(EngineEvent::UndoRequested);
///
/// let events = bus.drain();
/// ```
pub struct EngineEventBus {
    sender: crossbeam_channel::Sender<EngineEvent>,
    receiver: crossbeam_channel::Receiver<EngineEvent>,
}

impl Default for EngineEventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineEventBus {
    /// Create a new legacy event bus
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self { sender, receiver }
    }

    /// Send an event
    pub fn send(&self, event: EngineEvent) {
        if let Err(e) = self.sender.send(event) {
            tracing::error!("Failed to send event: {}", e);
        }
    }

    /// Receive all pending events
    pub fn drain(&self) -> Vec<EngineEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            events.push(event);
        }
        events
    }

    /// Get sender clone
    pub fn sender(&self) -> crossbeam_channel::Sender<EngineEvent> {
        self.sender.clone()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.receiver.is_empty()
    }

    /// Emit an event (alias for send)
    pub fn emit(&self, event: EngineEvent) {
        self.send(event);
    }
}

/// Event handler trait
///
/// This trait is used with the legacy `EngineEventBus` and `EngineEvent` enum.
pub trait EventHandler {
    fn handle_event(&mut self, event: &EngineEvent);
    fn handles_event(&self, event: &EngineEvent) -> bool;
}

/// Adapter to bridge `BattleEvent` from the new typed event system to `EngineEvent`
///
/// This allows publishing typed battle events that get converted to legacy engine events.
pub struct BattleEventAdapter;

impl BattleEventAdapter {
    /// Convert a `BattleEvent` to an `EngineEvent` if applicable
    pub fn convert(event: &BattleEvent) -> Option<EngineEvent> {
        match event {
            BattleEvent::BattleEnded { victory, .. } => {
                let result = if *victory {
                    BattleResult::Victory
                } else {
                    BattleResult::Defeat
                };
                Some(EngineEvent::BattleEnded { result })
            }
            BattleEvent::DamageDealt {
                attacker,
                defender,
                amount,
                ..
            } => Some(EngineEvent::DamageDealt {
                source: *attacker,
                target: *defender,
                amount: *amount as i32,
                element: Element::None,
                is_crit: false,
            }),
            BattleEvent::EnemyDefeated {
                entity,
                xp_gained: _,
            } => Some(EngineEvent::EntityDefeated {
                entity: *entity,
                killer: None,
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_event_bus() {
        let bus = EngineEventBus::new();
        bus.send(EngineEvent::UndoRequested);
        bus.send(EngineEvent::RedoRequested);

        let events = bus.drain();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], EngineEvent::UndoRequested));
        assert!(matches!(events[1], EngineEvent::RedoRequested));
    }

    #[test]
    fn test_battle_event_adapter() {
        use hecs::Entity;

        let entity = Entity::DANGLING;
        let battle_event = BattleEvent::BattleEnded {
            victory: true,
            rewards: BattleRewards::new().with_xp(100),
        };

        let converted = BattleEventAdapter::convert(&battle_event);
        assert!(converted.is_some());
        assert!(matches!(
            converted.unwrap(),
            EngineEvent::BattleEnded {
                result: BattleResult::Victory,
                ..
            }
        ));

        let damage_event = BattleEvent::DamageDealt {
            attacker: entity,
            defender: entity,
            amount: 50,
            critical: true,
        };

        let converted = BattleEventAdapter::convert(&damage_event);
        assert!(converted.is_some());
    }

    #[test]
    fn test_new_typed_event_bus() {
        use crate::events::{EventBus, EventFilter, EventPriority};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = Arc::clone(&counter);
        bus.subscribe(EventFilter::Type(EventType::Battle), move |event| {
            if downcast_event::<BattleEvent>(event).is_some() {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }
        });

        bus.publish(
            BattleEvent::BattleStarted { encounter_id: 1 },
            EventPriority::High,
        );

        bus.process_events();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
