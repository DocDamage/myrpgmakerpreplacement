//! Event Bus System
//! 
//! All subsystems communicate through typed events.
//! No subsystem holds a direct reference to another.

use crate::{Element, Entity, GameState, WorldState};

/// Engine events - all inter-system communication goes through here
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
    EntitySpawned {
        entity: Entity,
        kind: String,
    },
    
    /// Entity destroyed
    EntityDestroyed {
        entity: Entity,
    },
    
    /// Entity moved
    EntityMoved {
        entity: Entity,
        from: (i32, i32),
        to: (i32, i32),
    },
    
    // ==================== Sim Events ====================
    /// Simulation stat changed
    SimStatChanged {
        key: String,
        old: f64,
        new: f64,
    },
    
    /// Calamity propagated
    CalamityPropagated {
        center: (i32, i32),
        radius: u32,
    },
    
    /// Era advanced
    EraAdvanced {
        new_era: u32,
    },
    
    // ==================== Interaction Events ====================
    /// Player interacted with something
    PlayerInteracted {
        target: Entity,
    },
    
    /// Dialogue started
    DialogueStarted {
        npc: Entity,
        tree_id: Option<u32>,
    },
    
    /// Dialogue ended
    DialogueEnded {
        npc: Entity,
    },
    
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
    BattleEnded {
        result: BattleResult,
    },
    
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
    ExpGained {
        entity: Entity,
        amount: u32,
    },
    
    /// Level up
    LevelUp {
        entity: Entity,
        new_level: i32,
    },
    
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
    BgmChange {
        stem_set_id: String,
    },
    
    // ==================== AI Events ====================
    /// AI request sent
    AiRequestSent {
        request_id: u64,
        task_type: AiTaskType,
    },
    
    /// AI response received
    AiResponseReceived {
        request_id: u64,
        response: String,
    },
    
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
    EntitySelected {
        entity: Option<Entity>,
    },
    
    // ==================== Scene Events ====================
    /// Scene transition
    SceneTransition {
        from: GameState,
        to: GameState,
        transition: String,
    },
    
    /// Sub-map entered
    SubMapEntered {
        entity: Entity,
        sub_map_id: u32,
    },
    
    /// Sub-map exited
    SubMapExited {
        entity: Entity,
    },
    
    // ==================== Quest Events ====================
    /// Quest started
    QuestStarted {
        quest_id: u32,
    },
    
    /// Quest objective updated
    QuestObjectiveUpdated {
        quest_id: u32,
        objective_id: u32,
        progress: u32,
        required: u32,
    },
    
    /// Quest completed
    QuestCompleted {
        quest_id: u32,
    },
    
    /// Quest failed
    QuestFailed {
        quest_id: u32,
    },
    
    // ==================== Input Events ====================
    /// Input action pressed
    InputPressed {
        action: String,
    },
    
    /// Input action released
    InputReleased {
        action: String,
    },
    
    // ==================== System Events ====================
    /// Game saved
    GameSaved {
        slot: u32,
    },
    
    /// Game loaded
    GameLoaded {
        slot: u32,
    },
    
    /// Window resized
    WindowResized {
        width: u32,
        height: u32,
    },
    
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

/// Event bus for publishing and subscribing to events
pub struct EventBus {
    sender: crossbeam_channel::Sender<EngineEvent>,
    receiver: crossbeam_channel::Receiver<EngineEvent>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// Create a new event bus
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
}

/// Event handler trait
pub trait EventHandler {
    fn handle_event(&mut self, event: &EngineEvent);
    fn handles_event(&self, event: &EngineEvent) -> bool;
}
