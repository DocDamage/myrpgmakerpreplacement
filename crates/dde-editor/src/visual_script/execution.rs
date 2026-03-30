//! Visual Script Execution Engine
//!
//! Runtime execution of compiled visual scripts.
//! Manages script state, variable storage, and execution flow.

use super::compiler::{AnimationTarget, CompiledScript, Condition, EntityRef, GameEvent};
use super::nodes::{CompareOp, MathOp, StatType, ValueSource};
use dde_core::components::{Inventory, Name, Position, Stats};
use dde_core::events::{
    EngineEvent, EventBus,
};
use dde_core::{Entity, World};
use std::collections::HashMap;

/// Script execution error
#[derive(thiserror::Error, Debug, Clone)]
pub enum ExecutionError {
    #[error("Unknown variable: {0}")]
    UnknownVariable(String),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Entity not found: {0:?}")]
    EntityNotFound(EntityRef),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Execution timeout")]
    Timeout,

    #[error("Stack overflow")]
    StackOverflow,

    #[error("ECS error: {0}")]
    Ecs(String),

    #[error("Component not found: {0}")]
    ComponentNotFound(String),
}

/// Result type for execution
pub type ExecutionResult<T> = std::result::Result<T, ExecutionError>;

/// Values that can be stored in script variables
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ScriptValue {
    #[default]
    None,
    Bool(bool),
    Number(f64),
    String(String),
    Entity(Entity),
    List(Vec<ScriptValue>),
}

impl ScriptValue {
    /// Convert to boolean
    pub fn as_bool(&self) -> bool {
        match self {
            ScriptValue::Bool(b) => *b,
            ScriptValue::Number(n) => *n != 0.0,
            ScriptValue::String(s) => !s.is_empty(),
            ScriptValue::List(l) => !l.is_empty(),
            _ => false,
        }
    }

    /// Convert to number
    pub fn as_number(&self) -> f64 {
        match self {
            ScriptValue::Number(n) => *n,
            ScriptValue::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }

    /// Convert to string
    pub fn as_string(&self) -> String {
        match self {
            ScriptValue::String(s) => s.clone(),
            ScriptValue::Number(n) => n.to_string(),
            ScriptValue::Bool(b) => b.to_string(),
            ScriptValue::Entity(e) => format!("{:?}", e),
            ScriptValue::List(l) => format!("{:?}", l),
            ScriptValue::None => "none".to_string(),
        }
    }

    /// Get type name
    pub fn type_name(&self) -> &'static str {
        match self {
            ScriptValue::None => "none",
            ScriptValue::Bool(_) => "bool",
            ScriptValue::Number(_) => "number",
            ScriptValue::String(_) => "string",
            ScriptValue::Entity(_) => "entity",
            ScriptValue::List(_) => "list",
        }
    }

    /// Create from i32
    pub fn from_i32(value: i32) -> Self {
        ScriptValue::Number(value as f64)
    }
}

/// A stack frame for function call tracking
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub event_index: usize,
    pub variables: HashMap<String, ScriptValue>,
}

impl StackFrame {
    /// Create a new stack frame
    pub fn new(event_index: usize) -> Self {
        Self {
            event_index,
            variables: HashMap::new(),
        }
    }
}

/// Execution state for a running script
#[derive(Debug, Clone)]
pub enum ExecutionState {
    /// Script is ready to run
    Ready,
    /// Script is currently running
    Running,
    /// Script is paused (e.g., waiting for dialogue)
    Paused { resume_after: f32 },
    /// Script has completed
    Completed,
    /// Script encountered an error
    Error(ExecutionError),
}

/// Script executor that runs compiled visual scripts
pub struct ScriptExecutor {
    /// Global variables
    pub variables: HashMap<String, ScriptValue>,
    /// Call stack for nested execution
    pub call_stack: Vec<StackFrame>,
    /// Current execution state
    pub state: ExecutionState,
    /// Maximum stack depth
    max_stack_depth: usize,
    /// Execution timeout in seconds
    timeout_secs: f32,
    /// Time spent executing
    execution_time: f32,
    /// Break flag for loop control
    break_requested: bool,
    /// Continue flag for loop control
    continue_requested: bool,
    /// Event bus for sending events
    event_bus: Option<EventBus>,
    /// Self entity reference (the entity running this script)
    self_entity: Option<Entity>,
}

impl Default for ScriptExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptExecutor {
    /// Create a new script executor
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            call_stack: Vec::new(),
            state: ExecutionState::Ready,
            max_stack_depth: 100,
            timeout_secs: 30.0,
            execution_time: 0.0,
            break_requested: false,
            continue_requested: false,
            event_bus: None,
            self_entity: None,
        }
    }

    /// Set maximum stack depth
    pub fn with_max_stack_depth(mut self, depth: usize) -> Self {
        self.max_stack_depth = depth;
        self
    }

    /// Set execution timeout
    pub fn with_timeout(mut self, timeout_secs: f32) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Set the event bus for sending events
    pub fn with_event_bus(mut self, event_bus: EventBus) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    /// Set the self entity reference
    pub fn with_self_entity(mut self, entity: Entity) -> Self {
        self.self_entity = Some(entity);
        self
    }

    /// Get the event bus reference
    pub fn event_bus(&self) -> Option<&EventBus> {
        self.event_bus.as_ref()
    }

    /// Get the event bus mutable reference
    pub fn event_bus_mut(&mut self) -> Option<&mut EventBus> {
        self.event_bus.as_mut()
    }

    /// Send an EngineEvent through the event bus
    /// Note: This is a placeholder - the actual event bus integration would need
    /// to be implemented based on the specific event bus architecture
    fn send_engine_event(&self, _event: EngineEvent) {
        // TODO: Implement event bus integration
        // For now, events are logged via tracing
    }

    /// Get a variable value
    pub fn get_variable(&self, name: &str) -> Option<&ScriptValue> {
        // Check local variables first (top of call stack)
        for frame in self.call_stack.iter().rev() {
            if let Some(value) = frame.variables.get(name) {
                return Some(value);
            }
        }
        // Check global variables
        self.variables.get(name)
    }

    /// Set a variable value (global)
    pub fn set_variable(&mut self, name: impl Into<String>, value: ScriptValue) {
        self.variables.insert(name.into(), value);
    }

    /// Set a local variable (in current stack frame)
    pub fn set_local_variable(
        &mut self,
        name: impl Into<String>,
        value: ScriptValue,
    ) -> ExecutionResult<()> {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.variables.insert(name.into(), value);
            Ok(())
        } else {
            // No stack frame, set as global
            self.set_variable(name, value);
            Ok(())
        }
    }

    /// Resolve an EntityRef to an actual Entity
    fn resolve_entity(&self, entity_ref: EntityRef, world: &World) -> ExecutionResult<Entity> {
        match entity_ref {
            EntityRef::SelfEntity => self
                .self_entity
                .ok_or_else(|| ExecutionError::EntityNotFound(entity_ref)),
            EntityRef::Player => Self::find_player_entity(world)
                .ok_or_else(|| ExecutionError::EntityNotFound(entity_ref)),
            EntityRef::Target => Err(ExecutionError::EntityNotFound(entity_ref)),
            EntityRef::ById(_id) => {
                // Try to construct entity from bits - this is a best effort
                // In a real implementation, you'd have a proper entity lookup
                Err(ExecutionError::EntityNotFound(entity_ref))
            }
        }
    }

    /// Find the player entity in the world
    fn find_player_entity(world: &World) -> Option<Entity> {
        // Query for player entity - typically has a Player marker component
        // or is marked with EntityKind::Player
        use dde_core::components::EntityKindComp;
        use dde_core::EntityKind;

        // Use query to iterate over entities (immutable borrow of world)
        let mut query = world.query::<&EntityKindComp>();
        for (entity, kind) in query.iter() {
            if kind.kind == EntityKind::Player {
                return Some(entity);
            }
        }
        None
    }

    /// Execute a compiled script
    pub fn execute(&mut self, script: &CompiledScript, world: &mut World) -> ExecutionResult<()> {
        self.state = ExecutionState::Running;
        self.execution_time = 0.0;

        // Create initial stack frame
        self.call_stack.clear();
        self.call_stack.push(StackFrame::new(0));

        // Execute each event node
        for event in &script.events {
            if let Err(e) = self.execute_event(event, world) {
                self.state = ExecutionState::Error(e.clone());
                return Err(e);
            }

            // Check for timeout
            if self.execution_time > self.timeout_secs {
                self.state = ExecutionState::Error(ExecutionError::Timeout);
                return Err(ExecutionError::Timeout);
            }
        }

        self.state = ExecutionState::Completed;
        Ok(())
    }

    /// Execute a single event
    fn execute_event(&mut self, event: &GameEvent, world: &mut World) -> ExecutionResult<()> {
        match event {
            GameEvent::Sequence { events } => {
                for e in events {
                    self.execute_event(e, world)?;
                }
                Ok(())
            }

            GameEvent::MoveEntity {
                entity_ref,
                x,
                y,
                relative,
            } => self.execute_move_entity(*entity_ref, *x, *y, *relative, world),

            GameEvent::PlayAnimation { anim_id, target } => {
                self.execute_play_animation(*anim_id, *target, world)
            }

            GameEvent::Teleport { map_id, x, y } => self.execute_teleport(*map_id, *x, *y, world),

            GameEvent::SpawnEntity { template_id, x, y } => {
                self.execute_spawn_entity(*template_id, *x, *y, world)
            }

            GameEvent::DespawnEntity { entity_ref } => {
                self.execute_despawn_entity(*entity_ref, world)
            }

            GameEvent::StartBattle {
                encounter_id,
                transition,
            } => self.execute_start_battle(*encounter_id, transition, world),

            GameEvent::ModifyHealth { target, amount } => {
                self.execute_modify_health(*target, *amount, world)
            }

            GameEvent::GrantExp { target, amount } => {
                self.execute_grant_exp(*target, *amount, world)
            }

            GameEvent::ShowDialogue {
                text,
                speaker,
                portrait,
            } => self.execute_show_dialogue(text, speaker, *portrait, world),

            GameEvent::ShowNotification {
                text,
                duration_secs,
            } => self.execute_show_notification(text, *duration_secs, world),

            GameEvent::PlaySfx { sound_id } => self.execute_play_sfx(sound_id, world),

            GameEvent::ChangeBgm { bgm_id, fade_ms } => {
                self.execute_change_bgm(bgm_id, *fade_ms, world)
            }

            GameEvent::GiveItem { item_id, quantity } => {
                self.execute_give_item(*item_id, *quantity, world)
            }

            GameEvent::RemoveItem { item_id, quantity } => {
                self.execute_remove_item(*item_id, *quantity, world)
            }

            GameEvent::SetGameFlag { flag_key, value } => {
                self.execute_set_game_flag(flag_key, *value, world)
            }

            GameEvent::ModifyVariable {
                name,
                operation,
                value,
            } => self.execute_modify_variable(name, *operation, *value),

            GameEvent::StartQuest { quest_id } => self.execute_start_quest(*quest_id, world),

            GameEvent::UpdateQuest {
                quest_id,
                objective_id,
                progress,
            } => self.execute_update_quest(*quest_id, *objective_id, *progress, world),

            GameEvent::CompleteQuest { quest_id } => self.execute_complete_quest(*quest_id, world),

            GameEvent::Delay { seconds } => self.execute_delay(*seconds),

            GameEvent::Branch {
                condition,
                true_branch,
                false_branch,
            } => self.execute_branch(condition, true_branch, false_branch, world),

            GameEvent::Loop { count, body } => self.execute_loop(*count, body, world),

            GameEvent::WhileLoop { condition, body } => {
                self.execute_while_loop(condition, body, world)
            }

            GameEvent::Parallel { branches } => self.execute_parallel(branches, world),

            GameEvent::Break => {
                self.break_requested = true;
                Ok(())
            }

            GameEvent::Continue => {
                self.continue_requested = true;
                Ok(())
            }
        }
    }

    // ==================== Action Executors ====================

    fn execute_move_entity(
        &mut self,
        entity_ref: EntityRef,
        x: i32,
        y: i32,
        relative: bool,
        world: &mut World,
    ) -> ExecutionResult<()> {
        let entity = self.resolve_entity(entity_ref, world)?;

        // Get current position (in a scope to drop the immutable borrow before mutable borrow)
        let new_pos = {
            let mut query = world
                .query_one::<&Position>(entity)
                .map_err(|_| ExecutionError::ComponentNotFound("Position".to_string()))?;
            let current_pos = query
                .get()
                .ok_or_else(|| ExecutionError::ComponentNotFound("Position".to_string()))?;

            if relative {
                Position {
                    x: current_pos.x + x,
                    y: current_pos.y + y,
                    z: current_pos.z,
                }
            } else {
                Position {
                    x,
                    y,
                    z: current_pos.z,
                }
            }
        };

        // Update position using hecs query_one_mut
        if let Ok((pos,)) = world.query_one_mut::<(&mut Position,)>(entity) {
            let from = (pos.x, pos.y);
            pos.x = new_pos.x;
            pos.y = new_pos.y;

            // Entity moved event would be sent here via event bus
            tracing::debug!("Entity {:?} moved from {:?} to ({}, {})", entity, from, new_pos.x, new_pos.y);
        }

        tracing::debug!(
            "Move entity {:?} to ({}, {}) relative={}",
            entity_ref,
            x,
            y,
            relative
        );
        Ok(())
    }

    fn execute_play_animation(
        &mut self,
        anim_id: u32,
        target: AnimationTarget,
        world: &mut World,
    ) -> ExecutionResult<()> {
        let entity = match target {
            AnimationTarget::SelfEntity => self.self_entity,
            AnimationTarget::Player => Self::find_player_entity(world),
            AnimationTarget::Target => None,
        };

        if let Some(entity) = entity {
            // Start animation on the entity if it has animation components
            // Animation triggering would go here
            // For now just log it - real implementation would use event bus
            tracing::debug!("Starting animation {} on entity {:?}", anim_id, entity);
        }

        tracing::debug!("Play animation {} on {:?}", anim_id, target);
        Ok(())
    }

    fn execute_teleport(
        &mut self,
        map_id: u32,
        x: i32,
        y: i32,
        world: &mut World,
    ) -> ExecutionResult<()> {
        let player = Self::find_player_entity(world)
            .ok_or_else(|| ExecutionError::EntityNotFound(EntityRef::Player))?;

        // Update player position using hecs query_one_mut
        if let Ok((pos,)) = world.query_one_mut::<(&mut Position,)>(player) {
            pos.x = x;
            pos.y = y;
        }

        // Teleport event would be sent here via event bus
        tracing::debug!("Player teleported to map {} at ({}, {})", map_id, x, y);

        tracing::debug!("Teleport to map {} at ({}, {})", map_id, x, y);
        Ok(())
    }

    fn execute_spawn_entity(
        &mut self,
        template_id: u32,
        x: i32,
        y: i32,
        world: &mut World,
    ) -> ExecutionResult<()> {
        // Spawn a new entity with basic components
        let _entity = world.spawn((
            Position { x, y, z: 0 },
            Name {
                display: format!("Entity_{}", template_id),
                internal: format!("entity_{}", template_id),
            },
        ));

        // Entity spawned event would be sent here via event bus
        tracing::debug!("Entity spawned with template {} at ({}, {})", template_id, x, y);

        tracing::debug!("Spawn entity {} at ({}, {})", template_id, x, y);
        Ok(())
    }

    fn execute_despawn_entity(
        &mut self,
        entity_ref: EntityRef,
        world: &mut World,
    ) -> ExecutionResult<()> {
        let entity = self.resolve_entity(entity_ref, world)?;

        // Despawn the entity
        world.despawn(entity).map_err(|e| ExecutionError::Ecs(e.to_string()))?;

        // Entity destroyed event would be sent here via event bus
        tracing::debug!("Entity {:?} destroyed", entity);

        tracing::debug!("Despawn entity {:?}", entity_ref);
        Ok(())
    }

    fn execute_start_battle(
        &mut self,
        encounter_id: u32,
        transition: &str,
        _world: &mut World,
    ) -> ExecutionResult<()> {
        // Battle triggered event would be sent here via event bus
        tracing::debug!("Battle started with encounter {} (transition: {})", encounter_id, transition);

        tracing::debug!(
            "Start battle {} with transition {}",
            encounter_id,
            transition
        );
        Ok(())
    }

    fn execute_modify_health(
        &mut self,
        target: EntityRef,
        amount: i32,
        world: &mut World,
    ) -> ExecutionResult<()> {
        let entity = self.resolve_entity(target, world)?;

        // Get the entity's stats and modify health using hecs query_one_mut
        if let Ok((stats,)) = world.query_one_mut::<(&mut Stats,)>(entity) {
            let old_hp = stats.hp;

            if amount >= 0 {
                stats.heal(amount);
            } else {
                stats.take_damage(-amount);
            }

            // Damage dealt event would be sent here via event bus
            tracing::debug!("Entity {:?} health modified by {} ({} -> {})", entity, amount, old_hp, stats.hp);
        }

        tracing::debug!("Modify health of {:?} by {}", target, amount);
        Ok(())
    }

    fn execute_grant_exp(
        &mut self,
        target: EntityRef,
        amount: u32,
        world: &mut World,
    ) -> ExecutionResult<()> {
        let entity = self.resolve_entity(target, world)?;

        // Update entity's EXP using hecs query_one_mut
        if let Ok((stats,)) = world.query_one_mut::<(&mut Stats,)>(entity) {
            stats.exp += amount as i32;

            // EXP gained event would be sent here via event bus
            tracing::debug!("Entity {:?} gained {} EXP", entity, amount);
        }

        tracing::debug!("Grant {} EXP to {:?}", amount, target);
        Ok(())
    }

    fn execute_show_dialogue(
        &mut self,
        text: &str,
        speaker: &str,
        _portrait: Option<u32>,
        _world: &mut World,
    ) -> ExecutionResult<()> {
        // Dialogue started event would be sent here via event bus
        tracing::debug!("Dialogue started: {} - {}", speaker, text);

        tracing::debug!("Show dialogue from {}: {}", speaker, text);
        // Pause execution until dialogue is complete
        self.state = ExecutionState::Paused { resume_after: 0.0 };
        Ok(())
    }

    fn execute_show_notification(
        &mut self,
        text: &str,
        duration_secs: f32,
        _world: &mut World,
    ) -> ExecutionResult<()> {
        // Notification event would be sent here via event bus
        tracing::debug!("Notification: {} ({}s)", text, duration_secs);

        tracing::debug!("Show notification: {} ({}s)", text, duration_secs);
        Ok(())
    }

    fn execute_play_sfx(&mut self, sound_id: &str, _world: &mut World) -> ExecutionResult<()> {
        // Play SFX event would be sent here via event bus
        tracing::debug!("Playing SFX: {}", sound_id);

        tracing::debug!("Play SFX: {}", sound_id);
        Ok(())
    }

    fn execute_change_bgm(
        &mut self,
        bgm_id: &str,
        fade_ms: u32,
        _world: &mut World,
    ) -> ExecutionResult<()> {
        // BGM change event would be sent here via event bus
        tracing::debug!("Changing BGM to {} with {}ms fade", bgm_id, fade_ms);

        tracing::debug!("Change BGM to {} with {}ms fade", bgm_id, fade_ms);
        Ok(())
    }

    fn execute_give_item(
        &mut self,
        item_id: u32,
        quantity: u32,
        world: &mut World,
    ) -> ExecutionResult<()> {
        let player = Self::find_player_entity(world)
            .ok_or_else(|| ExecutionError::EntityNotFound(EntityRef::Player))?;

        // Add item to player's inventory using hecs query_one_mut
        if let Ok((inventory,)) = world.query_one_mut::<(&mut Inventory,)>(player) {
            const DEFAULT_MAX_STACK: u32 = 99;
            inventory.add_item(item_id, quantity, DEFAULT_MAX_STACK);

            // Item acquired - no specific EngineEvent for this, could add later
            tracing::debug!("Item acquired: {} x{}", item_id, quantity);
        }

        tracing::debug!("Give item {} x{}", item_id, quantity);
        Ok(())
    }

    fn execute_remove_item(
        &mut self,
        item_id: u32,
        quantity: u32,
        world: &mut World,
    ) -> ExecutionResult<()> {
        let player = Self::find_player_entity(world)
            .ok_or_else(|| ExecutionError::EntityNotFound(EntityRef::Player))?;

        // Remove item from player's inventory using hecs query_one_mut
        if let Ok((inventory,)) = world.query_one_mut::<(&mut Inventory,)>(player) {
            let success = inventory.remove_item(item_id, quantity);

            if success {
                // Item removed - no specific EngineEvent for this, could add later
                tracing::debug!("Item removed: {} x{}", item_id, quantity);
            }
        }

        tracing::debug!("Remove item {} x{}", item_id, quantity);
        Ok(())
    }

    fn execute_set_game_flag(
        &mut self,
        flag_key: &str,
        value: bool,
        _world: &mut World,
    ) -> ExecutionResult<()> {
        // Store the flag in a special variable
        let flag_var = format!("__flag_{}", flag_key);
        self.set_variable(flag_var, ScriptValue::Bool(value));

        // Game flag changed event would be sent here via event bus
        tracing::debug!("Game flag {} set to {}", flag_key, value);

        tracing::debug!("Set game flag {} = {}", flag_key, value);
        Ok(())
    }

    fn execute_modify_variable(
        &mut self,
        name: &str,
        operation: MathOp,
        value: i32,
    ) -> ExecutionResult<()> {
        let current = self
            .get_variable(name)
            .map(|v| v.as_number() as i32)
            .unwrap_or(0);

        let new_value = match operation {
            MathOp::Set => value,
            MathOp::Add => current + value,
            MathOp::Subtract => current - value,
            MathOp::Multiply => current * value,
            MathOp::Divide => {
                if value == 0 {
                    return Err(ExecutionError::InvalidOperation(
                        "Division by zero".to_string(),
                    ));
                }
                current / value
            }
            MathOp::Modulo => {
                if value == 0 {
                    return Err(ExecutionError::InvalidOperation(
                        "Modulo by zero".to_string(),
                    ));
                }
                current % value
            }
        };

        self.set_variable(name, ScriptValue::Number(new_value as f64));
        Ok(())
    }

    fn execute_start_quest(&mut self, quest_id: u32, _world: &mut World) -> ExecutionResult<()> {
        // Quest started event would be sent here via event bus
        tracing::debug!("Quest {} started", quest_id);

        tracing::debug!("Start quest {}", quest_id);
        Ok(())
    }

    fn execute_update_quest(
        &mut self,
        quest_id: u32,
        objective_id: u32,
        progress: u32,
        _world: &mut World,
    ) -> ExecutionResult<()> {
        // Quest updated event would be sent here via event bus
        tracing::debug!("Quest {} objective {} updated to progress {}", quest_id, objective_id, progress);

        tracing::debug!(
            "Update quest {} objective {} to progress {}",
            quest_id,
            objective_id,
            progress
        );
        Ok(())
    }

    fn execute_complete_quest(&mut self, quest_id: u32, _world: &mut World) -> ExecutionResult<()> {
        // Quest completed event would be sent here via event bus
        tracing::debug!("Quest {} completed", quest_id);

        tracing::debug!("Complete quest {}", quest_id);
        Ok(())
    }

    fn execute_delay(&mut self, seconds: f32) -> ExecutionResult<()> {
        tracing::debug!("Delay for {} seconds", seconds);
        self.state = ExecutionState::Paused {
            resume_after: seconds,
        };
        Ok(())
    }

    // ==================== Flow Control Executors ====================

    fn execute_branch(
        &mut self,
        condition: &Condition,
        true_branch: &[GameEvent],
        false_branch: &[GameEvent],
        world: &mut World,
    ) -> ExecutionResult<()> {
        let result = self.evaluate_condition(condition, world)?;

        let branch = if result { true_branch } else { false_branch };

        for event in branch {
            self.execute_event(event, world)?;
        }

        Ok(())
    }

    fn execute_loop(
        &mut self,
        count: u32,
        body: &[GameEvent],
        world: &mut World,
    ) -> ExecutionResult<()> {
        for i in 0..count {
            self.break_requested = false;
            self.continue_requested = false;

            // Set loop counter variable
            self.set_local_variable("loop_index", ScriptValue::Number(i as f64))?;

            for event in body {
                self.execute_event(event, world)?;

                if self.break_requested {
                    self.break_requested = false;
                    return Ok(());
                }

                if self.continue_requested {
                    self.continue_requested = false;
                    break;
                }
            }
        }

        Ok(())
    }

    fn execute_while_loop(
        &mut self,
        condition: &Condition,
        body: &[GameEvent],
        world: &mut World,
    ) -> ExecutionResult<()> {
        let mut iteration = 0;
        const MAX_ITERATIONS: u32 = 10000;

        while self.evaluate_condition(condition, world)? {
            if iteration >= MAX_ITERATIONS {
                return Err(ExecutionError::Timeout);
            }
            iteration += 1;

            self.break_requested = false;
            self.continue_requested = false;

            for event in body {
                self.execute_event(event, world)?;

                if self.break_requested {
                    self.break_requested = false;
                    return Ok(());
                }

                if self.continue_requested {
                    self.continue_requested = false;
                    break;
                }
            }
        }

        Ok(())
    }

    fn execute_parallel(
        &mut self,
        branches: &[Vec<GameEvent>],
        world: &mut World,
    ) -> ExecutionResult<()> {
        // In a real implementation, this would execute branches concurrently
        // For now, we execute them sequentially
        for branch in branches {
            for event in branch {
                self.execute_event(event, world)?;
            }
        }
        Ok(())
    }

    // ==================== Condition Evaluation ====================

    fn evaluate_condition(
        &self,
        condition: &Condition,
        world: &mut World,
    ) -> ExecutionResult<bool> {
        match condition {
            Condition::Literal(value) => Ok(*value),

            Condition::HasItem { item_id, quantity } => {
                // Query player's inventory for item using hecs query_one
                if let Some(player) = Self::find_player_entity(world) {
                    let mut query = world.query_one::<&Inventory>(player);
                    if let Ok(ref mut q) = query {
                        if let Some(inventory) = q.get() {
                            return Ok(inventory.has_item(*item_id, *quantity));
                        }
                    }
                }
                Ok(false)
            }

            Condition::StatCheck {
                stat,
                operator,
                value,
            } => {
                let actual_value = self.get_stat_value(*stat, world)?;
                Ok(compare_values(actual_value, *operator, *value))
            }

            Condition::QuestStage { quest_id, stage } => {
                // Check quest stage from game state
                let quest_var = format!("__quest_{}_stage", quest_id);
                let current_stage = self
                    .get_variable(&quest_var)
                    .map(|v| v.as_number() as u32)
                    .unwrap_or(0);
                Ok(current_stage >= *stage)
            }

            Condition::TimeOfDay { min_hour, max_hour } => {
                // Get current time from world/game state
                let current_hour = self
                    .get_variable("__time_hour")
                    .map(|v| v.as_number() as u8)
                    .unwrap_or(12);
                Ok(current_hour >= *min_hour && current_hour <= *max_hour)
            }

            Condition::RandomChance { percent } => {
                let roll = rand::random::<u8>() as f32 / 255.0 * 100.0;
                Ok(roll < *percent as f32)
            }

            Condition::GameFlag { flag_key, expected } => {
                // Check game flag from variables
                let flag_var = format!("__flag_{}", flag_key);
                let value = self
                    .get_variable(&flag_var)
                    .map(|v| v.as_bool())
                    .unwrap_or(false);
                Ok(value == *expected)
            }

            Condition::Compare {
                left,
                operator,
                right,
            } => {
                let left_val = self.evaluate_value_source(left, world)?;
                let right_val = self.evaluate_value_source(right, world)?;
                Ok(compare_values(left_val as i32, *operator, right_val as i32))
            }

            Condition::And(a, b) => {
                Ok(self.evaluate_condition(a, world)? && self.evaluate_condition(b, world)?)
            }

            Condition::Or(a, b) => {
                Ok(self.evaluate_condition(a, world)? || self.evaluate_condition(b, world)?)
            }

            Condition::Not(a) => Ok(!self.evaluate_condition(a, world)?),
        }
    }

    fn get_stat_value(&self, stat: StatType, world: &mut World) -> ExecutionResult<i32> {
        // Get the player entity and query its stats
        let player = Self::find_player_entity(world)
            .ok_or_else(|| ExecutionError::EntityNotFound(EntityRef::Player))?;

        // Query stats using hecs query_one
        let mut query = world
            .query_one::<&Stats>(player)
            .map_err(|e| ExecutionError::Ecs(e.to_string()))?;
        let stats = query.get()
            .ok_or_else(|| ExecutionError::ComponentNotFound("Stats".to_string()))?;

        let value = match stat {
            StatType::Health => stats.hp,
            StatType::MaxHealth => stats.max_hp,
            StatType::Mana => stats.mp,
            StatType::MaxMana => stats.max_mp,
            StatType::Strength => stats.str,
            StatType::Defense => stats.def,
            StatType::Speed => stats.spd,
            StatType::Level => stats.level,
            StatType::Exp => stats.exp,
            StatType::Gold => stats.luck, // Gold not in stats, use luck as placeholder
        };

        Ok(value)
    }

    fn evaluate_value_source(
        &self,
        source: &ValueSource,
        world: &mut World,
    ) -> ExecutionResult<f64> {
        match source {
            ValueSource::Literal(value) => Ok(*value),
            ValueSource::Variable(name) => self
                .get_variable(name)
                .map(|v| v.as_number())
                .ok_or_else(|| ExecutionError::UnknownVariable(name.clone())),
            ValueSource::Stat { entity, stat } => {
                // Query entity stat
                let _entity_ref = match entity {
                    super::nodes::EntityRef::SelfEntity => EntityRef::SelfEntity,
                    super::nodes::EntityRef::Player => EntityRef::Player,
                    super::nodes::EntityRef::Target => EntityRef::Target,
                    super::nodes::EntityRef::ById(id) => EntityRef::ById(*id),
                };

                // Map StatType from nodes to compiler
                let stat_type = match stat {
                    super::nodes::StatType::Health => StatType::Health,
                    super::nodes::StatType::MaxHealth => StatType::MaxHealth,
                    super::nodes::StatType::Mana => StatType::Mana,
                    super::nodes::StatType::MaxMana => StatType::MaxMana,
                    super::nodes::StatType::Strength => StatType::Strength,
                    super::nodes::StatType::Defense => StatType::Defense,
                    super::nodes::StatType::Speed => StatType::Speed,
                    super::nodes::StatType::Level => StatType::Level,
                    super::nodes::StatType::Exp => StatType::Exp,
                    super::nodes::StatType::Gold => StatType::Gold,
                };

                let value = self.get_stat_value(stat_type, world)?;
                Ok(value as f64)
            }
        }
    }

    /// Resume execution after a pause
    pub fn resume(&mut self, delta_time: f32, _world: &mut World) -> ExecutionResult<()> {
        if let ExecutionState::Paused { resume_after } = self.state {
            let new_time = resume_after - delta_time;
            if new_time <= 0.0 {
                self.state = ExecutionState::Running;
            } else {
                self.state = ExecutionState::Paused {
                    resume_after: new_time,
                };
            }
        }
        Ok(())
    }

    /// Check if execution is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.state, ExecutionState::Completed)
    }

    /// Check if execution has errored
    pub fn has_error(&self) -> Option<&ExecutionError> {
        match &self.state {
            ExecutionState::Error(e) => Some(e),
            _ => None,
        }
    }
}

/// Compare two values using the given operator
fn compare_values(left: i32, operator: CompareOp, right: i32) -> bool {
    match operator {
        CompareOp::Equal => left == right,
        CompareOp::NotEqual => left != right,
        CompareOp::LessThan => left < right,
        CompareOp::LessThanOrEqual => left <= right,
        CompareOp::GreaterThan => left > right,
        CompareOp::GreaterThanOrEqual => left >= right,
    }
}

/// Global script registry for managing active scripts
pub struct ScriptRegistry {
    scripts: HashMap<u64, ScriptExecutor>,
    next_id: u64,
    event_bus: Option<EventBus>,
}

impl Default for ScriptRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptRegistry {
    /// Create a new script registry
    pub fn new() -> Self {
        Self {
            scripts: HashMap::new(),
            next_id: 1,
            event_bus: None,
        }
    }

    /// Create a new script registry with an event bus
    pub fn with_event_bus(event_bus: EventBus) -> Self {
        Self {
            scripts: HashMap::new(),
            next_id: 1,
            event_bus: Some(event_bus),
        }
    }

    /// Set the event bus for all scripts in this registry
    pub fn set_event_bus(&mut self, event_bus: EventBus) {
        self.event_bus = Some(event_bus);
    }

    /// Register a new script execution
    pub fn register(&mut self, executor: ScriptExecutor) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.scripts.insert(id, executor);
        id
    }

    /// Get a mutable reference to a script executor
    pub fn get_mut(&mut self, id: u64) -> Option<&mut ScriptExecutor> {
        self.scripts.get_mut(&id)
    }

    /// Remove a completed script
    pub fn remove(&mut self, id: u64) -> Option<ScriptExecutor> {
        self.scripts.remove(&id)
    }

    /// Update all running scripts
    pub fn update(&mut self, delta_time: f32, world: &mut World) {
        let completed: Vec<u64> = self
            .scripts
            .iter_mut()
            .filter_map(|(id, executor)| {
                if let Err(e) = executor.resume(delta_time, world) {
                    tracing::error!("Script execution error: {}", e);
                    return Some(*id);
                }

                if executor.is_complete() || executor.has_error().is_some() {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();

        for id in completed {
            self.scripts.remove(&id);
        }
    }

    /// Get count of active scripts
    pub fn active_count(&self) -> usize {
        self.scripts.len()
    }

    /// Process pending events from all script event buses
    pub fn process_events(&self) -> usize {
        if let Some(ref bus) = self.event_bus {
            bus.process_events()
        } else {
            0
        }
    }
}

// ==================== Event Helper Functions ====================

/// Helper function to create a notification event
pub fn create_notification_event(text: String, duration_secs: f32) -> EngineEvent {
    EngineEvent::NotificationRequested {
        text,
        duration_secs,
    }
}

/// Helper function to create a game flag changed event
pub fn create_game_flag_event(flag_key: String, value: bool) -> EngineEvent {
    EngineEvent::GameFlagChanged { flag_key, value }
}

#[cfg(test)]
mod tests {
    use super::super::compiler::CompiledScript;
    use super::*;

    #[test]
    fn test_script_value_conversions() {
        assert_eq!(ScriptValue::Bool(true).as_bool(), true);
        assert_eq!(ScriptValue::Bool(false).as_bool(), false);
        assert_eq!(ScriptValue::Number(5.0).as_bool(), true);
        assert_eq!(ScriptValue::Number(0.0).as_bool(), false);
        assert_eq!(ScriptValue::String("hello".to_string()).as_bool(), true);
        assert_eq!(ScriptValue::String("".to_string()).as_bool(), false);

        assert_eq!(ScriptValue::Number(42.5).as_number(), 42.5);
        assert_eq!(ScriptValue::Bool(true).as_number(), 1.0);
        assert_eq!(ScriptValue::Bool(false).as_number(), 0.0);

        assert_eq!(ScriptValue::Number(123.0).as_string(), "123");
        assert_eq!(ScriptValue::Bool(true).as_string(), "true");
    }

    #[test]
    fn test_variable_operations() {
        let mut executor = ScriptExecutor::new();

        executor.set_variable("test_var", ScriptValue::Number(10.0));
        assert_eq!(
            executor.get_variable("test_var"),
            Some(&ScriptValue::Number(10.0))
        );

        // Modify variable
        executor
            .execute_modify_variable("test_var", MathOp::Add, 5)
            .unwrap();
        assert_eq!(
            executor.get_variable("test_var"),
            Some(&ScriptValue::Number(15.0))
        );
    }

    #[test]
    fn test_script_executor_creation() {
        let executor = ScriptExecutor::new();
        assert!(matches!(executor.state, ExecutionState::Ready));
    }

    #[test]
    fn test_script_registry() {
        let mut registry = ScriptRegistry::new();
        let executor = ScriptExecutor::new();

        let id = registry.register(executor);
        assert_eq!(registry.active_count(), 1);

        registry.remove(id);
        assert_eq!(registry.active_count(), 0);
    }

    #[test]
    fn test_compare_values() {
        assert!(compare_values(5, CompareOp::Equal, 5));
        assert!(!compare_values(5, CompareOp::Equal, 3));
        assert!(compare_values(5, CompareOp::GreaterThan, 3));
        assert!(!compare_values(3, CompareOp::GreaterThan, 5));
        assert!(compare_values(3, CompareOp::LessThan, 5));
        assert!(compare_values(5, CompareOp::GreaterThanOrEqual, 5));
        assert!(compare_values(5, CompareOp::LessThanOrEqual, 5));
        assert!(compare_values(5, CompareOp::NotEqual, 3));
    }

    #[test]
    fn test_event_bus_integration() {
        let event_bus = EventBus::new();
        let executor = ScriptExecutor::new().with_event_bus(event_bus);

        assert!(executor.event_bus().is_some());
    }

    #[test]
    fn test_condition_evaluation_literal() {
        let executor = ScriptExecutor::new();
        let mut world = World::new();

        assert_eq!(
            executor.evaluate_condition(&Condition::Literal(true), &mut world),
            Ok(true)
        );
        assert_eq!(
            executor.evaluate_condition(&Condition::Literal(false), &mut world),
            Ok(false)
        );
    }

    #[test]
    fn test_condition_evaluation_game_flag() {
        let mut executor = ScriptExecutor::new();
        let mut world = World::new();

        // Set a flag
        executor
            .execute_set_game_flag("test_flag", true, &mut world)
            .unwrap();

        // Check the flag
        assert_eq!(
            executor.evaluate_condition(
                &Condition::GameFlag {
                    flag_key: "test_flag".to_string(),
                    expected: true,
                },
                &mut world
            ),
            Ok(true)
        );

        assert_eq!(
            executor.evaluate_condition(
                &Condition::GameFlag {
                    flag_key: "test_flag".to_string(),
                    expected: false,
                },
                &mut world
            ),
            Ok(false)
        );
    }

    #[test]
    fn test_math_operations() {
        let mut executor = ScriptExecutor::new();

        // Test Set
        executor.set_variable("num", ScriptValue::Number(10.0));
        executor.execute_modify_variable("num", MathOp::Set, 5).unwrap();
        assert_eq!(
            executor.get_variable("num"),
            Some(&ScriptValue::Number(5.0))
        );

        // Test Add
        executor.execute_modify_variable("num", MathOp::Add, 3).unwrap();
        assert_eq!(
            executor.get_variable("num"),
            Some(&ScriptValue::Number(8.0))
        );

        // Test Subtract
        executor.execute_modify_variable("num", MathOp::Subtract, 2).unwrap();
        assert_eq!(
            executor.get_variable("num"),
            Some(&ScriptValue::Number(6.0))
        );

        // Test Multiply
        executor.execute_modify_variable("num", MathOp::Multiply, 2).unwrap();
        assert_eq!(
            executor.get_variable("num"),
            Some(&ScriptValue::Number(12.0))
        );

        // Test Divide
        executor.execute_modify_variable("num", MathOp::Divide, 3).unwrap();
        assert_eq!(
            executor.get_variable("num"),
            Some(&ScriptValue::Number(4.0))
        );

        // Test Modulo
        executor.set_variable("num", ScriptValue::Number(10.0));
        executor.execute_modify_variable("num", MathOp::Modulo, 3).unwrap();
        assert_eq!(
            executor.get_variable("num"),
            Some(&ScriptValue::Number(1.0))
        );
    }

    #[test]
    fn test_division_by_zero() {
        let mut executor = ScriptExecutor::new();
        executor.set_variable("num", ScriptValue::Number(10.0));

        let result = executor.execute_modify_variable("num", MathOp::Divide, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_value_source_evaluation() {
        let executor = ScriptExecutor::new();
        let mut world = World::new();

        // Test literal
        assert_eq!(
            executor.evaluate_value_source(&ValueSource::Literal(42.0), &mut world),
            Ok(42.0)
        );
    }
}

// ==================== EngineEvent Extensions for Script System ====================

/// Extension trait for EngineEvent to add script-related events
pub trait EngineEventExt {
    /// Create a notification requested event
    fn notification_requested(text: String, duration_secs: f32) -> EngineEvent;
    /// Create a game flag changed event
    fn game_flag_changed(flag_key: String, value: bool) -> EngineEvent;
}

impl EngineEventExt for EngineEvent {
    fn notification_requested(text: String, _duration_secs: f32) -> EngineEvent {
        // Use SimStatChanged as a workaround for missing variant
        EngineEvent::SimStatChanged {
            key: format!("notification_{}", text),
            old: 0.0,
            new: 1.0,
        }
    }

    fn game_flag_changed(flag_key: String, value: bool) -> EngineEvent {
        EngineEvent::SimStatChanged {
            key: format!("flag_{}", flag_key),
            old: if value { 0.0 } else { 1.0 },
            new: if value { 1.0 } else { 0.0 },
        }
    }
}
