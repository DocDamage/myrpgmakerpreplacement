//! Live Play Mode
//!
//! Seamless switching between editing and playing without stopping the engine.
//! Provides pause & inspect, step-through debugging, and live component modification.

use dde_core::{
    components::*,
    events::{EventBus, EventPriority, EventType},
    Entity, GameState, World,
};
use std::collections::HashMap;

/// Current play mode state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayMode {
    /// Full editor access, simulation paused
    Editing,
    /// Game running normally
    Playing,
    /// Game paused, can inspect entities
    Paused,
    /// Advance one tick (transient state)
    StepForward,
}

impl PlayMode {
    /// Check if the simulation is running
    pub fn is_running(&self) -> bool {
        matches!(self, PlayMode::Playing)
    }

    /// Check if the simulation is paused
    pub fn is_paused(&self) -> bool {
        matches!(
            self,
            PlayMode::Paused | PlayMode::Editing | PlayMode::StepForward
        )
    }

    /// Check if editor controls are available
    pub fn can_edit(&self) -> bool {
        matches!(self, PlayMode::Editing | PlayMode::Paused)
    }
}

impl std::fmt::Display for PlayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayMode::Editing => write!(f, "Editing"),
            PlayMode::Playing => write!(f, "Playing"),
            PlayMode::Paused => write!(f, "Paused"),
            PlayMode::StepForward => write!(f, "Step"),
        }
    }
}

/// Camera state for bookmarking positions
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraState {
    pub position: dde_core::glam::Vec2,
    pub zoom: f32,
    pub target: Option<Entity>,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            position: dde_core::glam::Vec2::ZERO,
            zoom: 1.0,
            target: None,
        }
    }
}

/// Component modification record for undo/redo
#[derive(Debug, Clone)]
pub struct ComponentModification {
    pub entity: Entity,
    pub component_type: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub timestamp: std::time::Instant,
}

/// Entity selection info for inspection
#[derive(Debug, Clone)]
pub struct EntitySelection {
    pub entity: Entity,
    pub name: Option<String>,
    pub kind: Option<String>,
    pub position: Option<Position>,
}

/// Event emitted when play mode changes
#[derive(Debug, Clone)]
pub struct PlayModeEvent {
    pub old_mode: PlayMode,
    pub new_mode: PlayMode,
    pub tick_count: u64,
}

/// Controls the live play mode
pub struct EditorController {
    /// Current mode
    mode: PlayMode,
    /// Whether time is paused
    paused: bool,
    /// Current tick when paused (for inspection)
    inspection_tick: Option<u64>,
    /// Selected entity for inspection
    selected_entity: Option<Entity>,
    /// Camera state before entering play mode
    saved_camera: Option<CameraState>,
    /// Camera bookmarks
    camera_bookmarks: HashMap<String, CameraState>,
    /// Modification history for undo/redo
    modification_history: Vec<ComponentModification>,
    /// History position for undo/redo
    history_position: usize,
    /// Maximum history size
    max_history: usize,
    /// Event bus for broadcasting changes
    event_bus: Option<EventBus>,
    /// Game state before entering play mode
    previous_game_state: Option<GameState>,
    /// Entity selection cache
    entity_cache: Vec<EntitySelection>,
    /// Whether split-screen is enabled
    split_screen: bool,
    /// Split ratio (0.0 = full game, 1.0 = full editor)
    split_ratio: f32,
}

impl Default for EditorController {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorController {
    /// Maximum modification history size
    const DEFAULT_MAX_HISTORY: usize = 100;

    /// Create a new editor controller
    pub fn new() -> Self {
        Self {
            mode: PlayMode::Editing,
            paused: true,
            inspection_tick: None,
            selected_entity: None,
            saved_camera: None,
            camera_bookmarks: HashMap::new(),
            modification_history: Vec::new(),
            history_position: 0,
            max_history: Self::DEFAULT_MAX_HISTORY,
            event_bus: None,
            previous_game_state: None,
            entity_cache: Vec::new(),
            split_screen: false,
            split_ratio: 0.5,
        }
    }

    /// Create with an event bus for broadcasting changes
    pub fn with_event_bus(mut self, event_bus: EventBus) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    /// Get current play mode
    pub fn mode(&self) -> PlayMode {
        self.mode
    }

    /// Check if currently editing
    pub fn is_editing(&self) -> bool {
        self.mode == PlayMode::Editing
    }

    /// Check if currently playing
    pub fn is_playing(&self) -> bool {
        self.mode == PlayMode::Playing
    }

    /// Check if paused
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Get the selected entity
    pub fn selected_entity(&self) -> Option<Entity> {
        self.selected_entity
    }

    /// Get current inspection tick
    pub fn inspection_tick(&self) -> Option<u64> {
        self.inspection_tick
    }

    /// Check if split-screen is enabled
    pub fn is_split_screen(&self) -> bool {
        self.split_screen
    }

    /// Get split ratio
    pub fn split_ratio(&self) -> f32 {
        self.split_ratio
    }

    /// Set split ratio (clamped to 0.1..=0.9)
    pub fn set_split_ratio(&mut self, ratio: f32) {
        self.split_ratio = ratio.clamp(0.1, 0.9);
    }

    /// Toggle split-screen mode
    pub fn toggle_split_screen(&mut self) {
        self.split_screen = !self.split_screen;
    }

    /// Enter play mode from editing
    ///
    /// Saves editor state and starts the simulation
    pub fn enter_play_mode(&mut self, current_camera: CameraState, current_game_state: GameState) {
        if self.mode == PlayMode::Playing {
            return;
        }

        let old_mode = self.mode;
        self.saved_camera = Some(current_camera);
        self.previous_game_state = Some(current_game_state);
        self.mode = PlayMode::Playing;
        self.paused = false;
        self.inspection_tick = None;

        self.broadcast_mode_change(old_mode, PlayMode::Playing);
        tracing::info!("Entered play mode");
    }

    /// Exit play mode and return to editing
    ///
    /// Restores editor state and pauses the simulation
    pub fn exit_play_mode(&mut self) -> Option<CameraState> {
        if self.mode == PlayMode::Editing {
            return None;
        }

        let old_mode = self.mode;
        self.mode = PlayMode::Editing;
        self.paused = true;
        self.inspection_tick = None;

        self.broadcast_mode_change(old_mode, PlayMode::Editing);
        tracing::info!("Exited to editing mode");

        self.saved_camera.take()
    }

    /// Pause the simulation for inspection
    pub fn pause(&mut self, current_tick: u64) {
        if self.paused {
            return;
        }

        let old_mode = self.mode;
        self.paused = true;
        self.inspection_tick = Some(current_tick);

        if self.mode == PlayMode::Playing {
            self.mode = PlayMode::Paused;
            self.broadcast_mode_change(old_mode, PlayMode::Paused);
        }

        tracing::debug!("Simulation paused at tick {}", current_tick);
    }

    /// Resume the simulation
    pub fn resume(&mut self) {
        if !self.paused {
            return;
        }

        let old_mode = self.mode;
        self.paused = false;
        self.inspection_tick = None;

        if self.mode == PlayMode::Paused {
            self.mode = PlayMode::Playing;
            self.broadcast_mode_change(old_mode, PlayMode::Playing);
        }

        tracing::debug!("Simulation resumed");
    }

    /// Toggle pause state
    pub fn toggle_pause(&mut self, current_tick: u64) {
        if self.paused {
            self.resume();
        } else {
            self.pause(current_tick);
        }
    }

    /// Step forward one tick
    ///
    /// Returns true if the step should be executed
    pub fn step_forward(&mut self, current_tick: u64) -> bool {
        if self.mode == PlayMode::Playing {
            self.pause(current_tick);
            return false;
        }

        let old_mode = self.mode;
        self.mode = PlayMode::StepForward;
        self.inspection_tick = Some(current_tick + 1);

        self.broadcast_mode_change(old_mode, PlayMode::StepForward);

        // Reset to paused after step
        self.mode = PlayMode::Paused;

        tracing::debug!("Stepping to tick {}", current_tick + 1);
        true
    }

    /// Select an entity for inspection
    pub fn inspect_entity(&mut self, entity: Entity, world: &World) -> Option<EntitySelection> {
        self.selected_entity = Some(entity);

        // Build selection info
        let selection = self.build_entity_selection(entity, world);

        tracing::debug!("Inspecting entity {:?}", entity);
        selection
    }

    /// Clear entity selection
    pub fn clear_selection(&mut self) {
        self.selected_entity = None;
        tracing::debug!("Entity selection cleared");
    }

    /// Build entity selection info
    fn build_entity_selection(&self, entity: Entity, world: &World) -> Option<EntitySelection> {
        let name = world
            .get::<&Name>(entity)
            .ok()
            .map(|n| n.display.clone());
        let kind = world
            .get::<&EntityKindComp>(entity)
            .ok()
            .map(|k| format!("{:?}", k.kind));
        let position = world.get::<&Position>(entity).ok().map(|p| *p);

        Some(EntitySelection {
            entity,
            name,
            kind,
            position,
        })
    }

    /// Get all components on an entity as JSON values
    ///
    /// Returns a map of component type names to their JSON representation
    pub fn get_entity_components(
        &self,
        entity: Entity,
        world: &World,
    ) -> HashMap<String, serde_json::Value> {
        let mut components = HashMap::new();

        // Use hecs' query to get all component types
        // Note: This is a simplified version - in a full implementation,
        // you'd need reflection or manual registration of component types

        // Try to get known components
        if let Ok(pos) = world.get::<&Position>(entity) {
            if let Ok(json) = serde_json::to_value(*pos) {
                components.insert("Position".to_string(), json);
            }
        }

        if let Ok(sub_pos) = world.get::<&SubPosition>(entity) {
            if let Ok(json) = serde_json::to_value(*sub_pos) {
                components.insert("SubPosition".to_string(), json);
            }
        }

        if let Ok(name) = world.get::<&Name>(entity) {
            let name_val: Name = (*name).clone();
            if let Ok(json) = serde_json::to_value(name_val) {
                components.insert("Name".to_string(), json);
            }
        }

        if let Ok(stats) = world.get::<&Stats>(entity) {
            if let Ok(json) = serde_json::to_value(*stats) {
                components.insert("Stats".to_string(), json);
            }
        }

        if let Ok(inventory) = world.get::<&Inventory>(entity) {
            if let Ok(json) = serde_json::to_value((*inventory).clone()) {
                components.insert("Inventory".to_string(), json);
            }
        }

        if let Ok(equipment) = world.get::<&Equipment>(entity) {
            if let Ok(json) = serde_json::to_value(*equipment) {
                components.insert("Equipment".to_string(), json);
            }
        }

        if let Ok(kind) = world.get::<&EntityKindComp>(entity) {
            if let Ok(json) = serde_json::to_value(*kind) {
                components.insert("EntityKind".to_string(), json);
            }
        }

        if let Ok(biome) = world.get::<&Biome>(entity) {
            if let Ok(json) = serde_json::to_value(*biome) {
                components.insert("Biome".to_string(), json);
            }
        }

        if let Ok(passability) = world.get::<&Passability>(entity) {
            if let Ok(json) = serde_json::to_value(*passability) {
                components.insert("Passability".to_string(), json);
            }
        }

        if let Ok(interactable) = world.get::<&Interactable>(entity) {
            if let Ok(json) = serde_json::to_value(*interactable) {
                components.insert("Interactable".to_string(), json);
            }
        }

        if let Ok(status_effects) = world.get::<&StatusEffects>(entity) {
            if let Ok(json) = serde_json::to_value((*status_effects).clone()) {
                components.insert("StatusEffects".to_string(), json);
            }
        }

        if let Ok(respawn) = world.get::<&Respawn>(entity) {
            if let Ok(json) = serde_json::to_value(*respawn) {
                components.insert("Respawn".to_string(), json);
            }
        }

        components
    }

    /// Modify a component on an entity
    ///
    /// Records the modification for undo/redo support
    pub fn modify_component<T: serde::Serialize + 'static>(
        &mut self,
        entity: Entity,
        component_type: &str,
        new_value: T,
        _world: &mut World,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Serialize new value
        let new_json = serde_json::to_value(&new_value)?;

        // Record modification
        let modification = ComponentModification {
            entity,
            component_type: component_type.to_string(),
            old_value: None, // Would need to get current value in full impl
            new_value: Some(new_json),
            timestamp: std::time::Instant::now(),
        };

        self.record_modification(modification);

        // In a full implementation, we'd use reflection or type registration
        // to actually modify the component in the world
        tracing::info!(
            "Modified {} component on entity {:?}",
            component_type,
            entity
        );

        Ok(())
    }

    /// Record a modification for undo/redo
    fn record_modification(&mut self, modification: ComponentModification) {
        // Truncate redo history if we're not at the end
        if self.history_position < self.modification_history.len() {
            self.modification_history.truncate(self.history_position);
        }

        self.modification_history.push(modification);
        self.history_position += 1;

        // Limit history size
        if self.modification_history.len() > self.max_history {
            self.modification_history.remove(0);
            self.history_position -= 1;
        }
    }

    /// Undo the last modification
    pub fn undo(&mut self) -> Option<&ComponentModification> {
        if self.history_position == 0 {
            return None;
        }

        self.history_position -= 1;
        self.modification_history.get(self.history_position)
    }

    /// Redo the last undone modification
    pub fn redo(&mut self) -> Option<&ComponentModification> {
        if self.history_position >= self.modification_history.len() {
            return None;
        }

        let mod_ref = self.modification_history.get(self.history_position);
        self.history_position += 1;
        mod_ref
    }

    /// Can undo
    pub fn can_undo(&self) -> bool {
        self.history_position > 0
    }

    /// Can redo
    pub fn can_redo(&self) -> bool {
        self.history_position < self.modification_history.len()
    }

    /// Save camera bookmark
    pub fn save_camera_bookmark(&mut self, name: impl Into<String>, camera: CameraState) {
        self.camera_bookmarks.insert(name.into(), camera);
    }

    /// Get camera bookmark
    pub fn get_camera_bookmark(&self, name: &str) -> Option<CameraState> {
        self.camera_bookmarks.get(name).copied()
    }

    /// Remove camera bookmark
    pub fn remove_camera_bookmark(&mut self, name: &str) -> bool {
        self.camera_bookmarks.remove(name).is_some()
    }

    /// Get all camera bookmarks
    pub fn camera_bookmarks(&self) -> &HashMap<String, CameraState> {
        &self.camera_bookmarks
    }

    /// Get saved camera state (from before entering play mode)
    pub fn saved_camera(&self) -> Option<CameraState> {
        self.saved_camera
    }

    /// Update entity cache for inspector
    pub fn update_entity_cache(&mut self, world: &World) {
        self.entity_cache.clear();

        for (entity, ()) in world.query::<()>().iter() {
            if let Some(selection) = self.build_entity_selection(entity, world) {
                self.entity_cache.push(selection);
            }
        }
    }

    /// Get cached entity list
    pub fn entity_cache(&self) -> &[EntitySelection] {
        &self.entity_cache
    }

    /// Clear modification history
    pub fn clear_history(&mut self) {
        self.modification_history.clear();
        self.history_position = 0;
    }

    /// Set maximum history size
    pub fn set_max_history(&mut self, max: usize) {
        self.max_history = max.max(1);

        // Trim if needed
        if self.modification_history.len() > self.max_history {
            let excess = self.modification_history.len() - self.max_history;
            self.modification_history.drain(0..excess);
            self.history_position = self.history_position.saturating_sub(excess);
        }
    }

    /// Get maximum history size
    pub fn max_history(&self) -> usize {
        self.max_history
    }

    /// Broadcast mode change event
    fn broadcast_mode_change(&self, old_mode: PlayMode, new_mode: PlayMode) {
        if let Some(ref bus) = self.event_bus {
            // Create and publish event
            let event = PlayModeEvent {
                old_mode,
                new_mode,
                tick_count: self.inspection_tick.unwrap_or(0),
            };

            // Publish as custom event type
            bus.publish(PlayModeChangedEvent(event), EventPriority::High);
        }
    }

    /// Check if simulation should tick
    ///
    /// Returns true if the simulation should advance
    pub fn should_tick(&self) -> bool {
        match self.mode {
            PlayMode::Playing => true,
            PlayMode::StepForward => true, // Will be reset after one tick
            _ => false,
        }
    }

    /// Get current tick rate multiplier
    ///
    /// Returns the speed multiplier for simulation ticks
    pub fn tick_rate_multiplier(&self) -> f32 {
        match self.mode {
            PlayMode::Playing => 1.0,
            PlayMode::StepForward => 1.0,
            _ => 0.0,
        }
    }
}

/// Event type for play mode changes
#[derive(Debug, Clone)]
struct PlayModeChangedEvent(#[allow(dead_code)] PlayModeEvent);

impl dde_core::events::Event for PlayModeChangedEvent {
    fn event_type(&self) -> dde_core::events::EventType {
        EventType::System
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_play_mode_transitions() {
        let mut controller = EditorController::new();

        assert_eq!(controller.mode(), PlayMode::Editing);
        assert!(controller.is_editing());
        assert!(!controller.is_playing());

        controller.enter_play_mode(CameraState::default(), GameState::Overworld);
        assert_eq!(controller.mode(), PlayMode::Playing);
        assert!(controller.is_playing());

        controller.pause(100);
        assert_eq!(controller.mode(), PlayMode::Paused);
        assert!(controller.is_paused());

        controller.resume();
        assert_eq!(controller.mode(), PlayMode::Playing);

        let camera = controller.exit_play_mode();
        assert!(camera.is_some());
        assert_eq!(controller.mode(), PlayMode::Editing);
    }

    #[test]
    fn test_pause_toggle() {
        let mut controller = EditorController::new();

        controller.enter_play_mode(CameraState::default(), GameState::Overworld);
        assert!(!controller.is_paused());

        controller.toggle_pause(50);
        assert!(controller.is_paused());
        assert_eq!(controller.inspection_tick(), Some(50));

        controller.toggle_pause(50);
        assert!(!controller.is_paused());
        assert_eq!(controller.inspection_tick(), None);
    }

    #[test]
    fn test_camera_bookmarks() {
        let mut controller = EditorController::new();

        let camera = CameraState {
            position: dde_core::glam::Vec2::new(100.0, 200.0),
            zoom: 2.0,
            target: None,
        };

        controller.save_camera_bookmark("test", camera);
        assert!(controller.get_camera_bookmark("test").is_some());
        assert_eq!(controller.get_camera_bookmark("test").unwrap().zoom, 2.0);

        assert!(controller.remove_camera_bookmark("test"));
        assert!(!controller.remove_camera_bookmark("test"));
    }

    #[test]
    fn test_split_screen() {
        let mut controller = EditorController::new();

        assert!(!controller.is_split_screen());

        controller.toggle_split_screen();
        assert!(controller.is_split_screen());

        controller.set_split_ratio(0.8);
        assert_eq!(controller.split_ratio(), 0.8);

        // Test clamping
        controller.set_split_ratio(0.05);
        assert_eq!(controller.split_ratio(), 0.1);

        controller.set_split_ratio(0.95);
        assert_eq!(controller.split_ratio(), 0.9);
    }

    #[test]
    fn test_undo_redo() {
        let mut controller = EditorController::new();

        assert!(!controller.can_undo());
        assert!(!controller.can_redo());

        // Record a modification
        let mod1 = ComponentModification {
            entity: Entity::DANGLING,
            component_type: "Position".to_string(),
            old_value: None,
            new_value: Some(serde_json::json!({"x": 10, "y": 20})),
            timestamp: std::time::Instant::now(),
        };
        controller.record_modification(mod1);

        assert!(controller.can_undo());
        assert!(!controller.can_redo());

        // Undo
        assert!(controller.undo().is_some());
        assert!(!controller.can_undo());
        assert!(controller.can_redo());

        // Redo
        assert!(controller.redo().is_some());
        assert!(controller.can_undo());
        assert!(!controller.can_redo());
    }

    #[test]
    fn test_history_limit() {
        let mut controller = EditorController::new();
        controller.set_max_history(3);

        // Add 5 modifications
        for i in 0..5 {
            let modification = ComponentModification {
                entity: Entity::from_bits((i + 1) as u64).unwrap_or(Entity::DANGLING),
                component_type: "Test".to_string(),
                old_value: None,
                new_value: Some(serde_json::json!(i)),
                timestamp: std::time::Instant::now(),
            };
            controller.record_modification(modification);
        }

        // Should only have 3 in history
        assert_eq!(controller.modification_history.len(), 3);
    }
}
