//! Visual Script Execution Engine
//!
//! Runtime execution of compiled visual scripts.
//! Manages script state, variable storage, and execution flow.

use super::compiler::{AnimationTarget, CompiledScript, Condition, EntityRef, GameEvent};
use super::nodes::{CompareOp, MathOp, StatType, ValueSource};
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
}

/// Result type for execution
pub type ExecutionResult<T> = std::result::Result<T, ExecutionError>;

/// Values that can be stored in script variables
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptValue {
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
            ScriptValue::Bool(b) => if *b { 1.0 } else { 0.0 },
            _ => 0.0,
        }
    }

    /// Convert to string
    pub fn as_string(&self) -> String {
        match self {
            ScriptValue::String(s) => s.clone(),
            ScriptValue::Number(n) => n.to_string(),
            ScriptValue::Bool(b) => b.to_string(),
            _ => String::new(),
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
}

impl Default for ScriptValue {
    fn default() -> Self {
        ScriptValue::None
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
#[derive(Debug)]
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
    pub fn set_local_variable(&mut self, name: impl Into<String>, value: ScriptValue) -> ExecutionResult<()> {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.variables.insert(name.into(), value);
            Ok(())
        } else {
            // No stack frame, set as global
            self.set_variable(name, value);
            Ok(())
        }
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

            GameEvent::MoveEntity { entity_ref, x, y, relative } => {
                self.execute_move_entity(*entity_ref, *x, *y, *relative, world)
            }

            GameEvent::PlayAnimation { anim_id, target } => {
                self.execute_play_animation(*anim_id, *target, world)
            }

            GameEvent::Teleport { map_id, x, y } => {
                self.execute_teleport(*map_id, *x, *y, world)
            }

            GameEvent::SpawnEntity { template_id, x, y } => {
                self.execute_spawn_entity(*template_id, *x, *y, world)
            }

            GameEvent::DespawnEntity { entity_ref } => {
                self.execute_despawn_entity(*entity_ref, world)
            }

            GameEvent::StartBattle { encounter_id, transition } => {
                self.execute_start_battle(*encounter_id, transition, world)
            }

            GameEvent::ModifyHealth { target, amount } => {
                self.execute_modify_health(*target, *amount, world)
            }

            GameEvent::GrantExp { target, amount } => {
                self.execute_grant_exp(*target, *amount, world)
            }

            GameEvent::ShowDialogue { text, speaker, portrait } => {
                self.execute_show_dialogue(text, speaker, *portrait, world)
            }

            GameEvent::ShowNotification { text, duration_secs } => {
                self.execute_show_notification(text, *duration_secs, world)
            }

            GameEvent::PlaySfx { sound_id } => {
                self.execute_play_sfx(sound_id, world)
            }

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

            GameEvent::ModifyVariable { name, operation, value } => {
                self.execute_modify_variable(name, *operation, *value)
            }

            GameEvent::StartQuest { quest_id } => {
                self.execute_start_quest(*quest_id, world)
            }

            GameEvent::UpdateQuest { quest_id, objective_id, progress } => {
                self.execute_update_quest(*quest_id, *objective_id, *progress, world)
            }

            GameEvent::CompleteQuest { quest_id } => {
                self.execute_complete_quest(*quest_id, world)
            }

            GameEvent::Delay { seconds } => {
                self.execute_delay(*seconds)
            }

            GameEvent::Branch { condition, true_branch, false_branch } => {
                self.execute_branch(condition, true_branch, false_branch, world)
            }

            GameEvent::Loop { count, body } => {
                self.execute_loop(*count, body, world)
            }

            GameEvent::WhileLoop { condition, body } => {
                self.execute_while_loop(condition, body, world)
            }

            GameEvent::Parallel { branches } => {
                self.execute_parallel(branches, world)
            }

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
        _world: &mut World,
    ) -> ExecutionResult<()> {
        // In a real implementation, this would query the world for the entity
        // and update its position component
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
        _world: &mut World,
    ) -> ExecutionResult<()> {
        tracing::debug!("Play animation {} on {:?}", anim_id, target);
        Ok(())
    }

    fn execute_teleport(
        &mut self,
        map_id: u32,
        x: i32,
        y: i32,
        _world: &mut World,
    ) -> ExecutionResult<()> {
        tracing::debug!("Teleport to map {} at ({}, {})", map_id, x, y);
        Ok(())
    }

    fn execute_spawn_entity(
        &mut self,
        template_id: u32,
        x: i32,
        y: i32,
        _world: &mut World,
    ) -> ExecutionResult<()> {
        tracing::debug!("Spawn entity {} at ({}, {})", template_id, x, y);
        Ok(())
    }

    fn execute_despawn_entity(
        &mut self,
        entity_ref: EntityRef,
        _world: &mut World,
    ) -> ExecutionResult<()> {
        tracing::debug!("Despawn entity {:?}", entity_ref);
        Ok(())
    }

    fn execute_start_battle(
        &mut self,
        encounter_id: u32,
        transition: &str,
        _world: &mut World,
    ) -> ExecutionResult<()> {
        tracing::debug!("Start battle {} with transition {}", encounter_id, transition);
        Ok(())
    }

    fn execute_modify_health(
        &mut self,
        target: EntityRef,
        amount: i32,
        _world: &mut World,
    ) -> ExecutionResult<()> {
        tracing::debug!("Modify health of {:?} by {}", target, amount);
        Ok(())
    }

    fn execute_grant_exp(
        &mut self,
        target: EntityRef,
        amount: u32,
        _world: &mut World,
    ) -> ExecutionResult<()> {
        tracing::debug!("Grant {} EXP to {:?}", amount, target);
        Ok(())
    }

    fn execute_show_dialogue(
        &mut self,
        text: &str,
        speaker: &str,
        portrait: Option<u32>,
        _world: &mut World,
    ) -> ExecutionResult<()> {
        tracing::debug!("Show dialogue from {}: {}", speaker, text);
        // In a real implementation, this would trigger the dialogue system
        // and pause execution until the dialogue is complete
        self.state = ExecutionState::Paused { resume_after: 0.0 };
        Ok(())
    }

    fn execute_show_notification(
        &mut self,
        text: &str,
        duration_secs: f32,
        _world: &mut World,
    ) -> ExecutionResult<()> {
        tracing::debug!("Show notification: {} ({}s)", text, duration_secs);
        Ok(())
    }

    fn execute_play_sfx(&mut self, sound_id: &str, _world: &mut World) -> ExecutionResult<()> {
        tracing::debug!("Play SFX: {}", sound_id);
        Ok(())
    }

    fn execute_change_bgm(
        &mut self,
        bgm_id: &str,
        fade_ms: u32,
        _world: &mut World,
    ) -> ExecutionResult<()> {
        tracing::debug!("Change BGM to {} with {}ms fade", bgm_id, fade_ms);
        Ok(())
    }

    fn execute_give_item(
        &mut self,
        item_id: u32,
        quantity: u32,
        _world: &mut World,
    ) -> ExecutionResult<()> {
        tracing::debug!("Give item {} x{}", item_id, quantity);
        Ok(())
    }

    fn execute_remove_item(
        &mut self,
        item_id: u32,
        quantity: u32,
        _world: &mut World,
    ) -> ExecutionResult<()> {
        tracing::debug!("Remove item {} x{}", item_id, quantity);
        Ok(())
    }

    fn execute_set_game_flag(
        &mut self,
        flag_key: &str,
        value: bool,
        _world: &mut World,
    ) -> ExecutionResult<()> {
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
        tracing::debug!(
            "Update quest {} objective {} to progress {}",
            quest_id,
            objective_id,
            progress
        );
        Ok(())
    }

    fn execute_complete_quest(&mut self, quest_id: u32, _world: &mut World) -> ExecutionResult<()> {
        tracing::debug!("Complete quest {}", quest_id);
        Ok(())
    }

    fn execute_delay(&mut self, seconds: f32) -> ExecutionResult<()> {
        tracing::debug!("Delay for {} seconds", seconds);
        self.state = ExecutionState::Paused { resume_after: seconds };
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

    fn evaluate_condition(&self, condition: &Condition, world: &mut World) -> ExecutionResult<bool> {
        match condition {
            Condition::Literal(value) => Ok(*value),

            Condition::HasItem { item_id, quantity } => {
                // Query world for inventory
                tracing::debug!("Check has item {} x{}", item_id, quantity);
                Ok(true) // Placeholder
            }

            Condition::StatCheck { stat, operator, value } => {
                let actual_value = self.get_stat_value(*stat, world)?;
                Ok(compare_values(actual_value, *operator, *value))
            }

            Condition::QuestStage { quest_id, stage } => {
                tracing::debug!("Check quest {} at stage {}", quest_id, stage);
                Ok(true) // Placeholder
            }

            Condition::TimeOfDay { min_hour, max_hour } => {
                // Query world for current time
                tracing::debug!("Check time between {} and {}", min_hour, max_hour);
                Ok(true) // Placeholder
            }

            Condition::RandomChance { percent } => {
                let roll = rand::random::<u8>() as f32 / 255.0 * 100.0;
                Ok(roll < *percent as f32)
            }

            Condition::GameFlag { flag_key, expected } => {
                // Query world for game flag
                tracing::debug!("Check flag {} = {}", flag_key, expected);
                Ok(true) // Placeholder
            }

            Condition::Compare { left, operator, right } => {
                let left_val = self.evaluate_value_source(left, world)?;
                let right_val = self.evaluate_value_source(right, world)?;
                Ok(compare_values(
                    left_val as i32,
                    *operator,
                    right_val as i32,
                ))
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

    fn get_stat_value(&self, stat: StatType, _world: &mut World) -> ExecutionResult<i32> {
        // In a real implementation, query the entity's stats component
        tracing::debug!("Get stat {:?}", stat);
        Ok(100) // Placeholder
    }

    fn evaluate_value_source(&self, source: &ValueSource, _world: &mut World) -> ExecutionResult<f64> {
        match source {
            ValueSource::Literal(value) => Ok(*value),
            ValueSource::Variable(name) => {
                self.get_variable(name)
                    .map(|v| v.as_number())
                    .ok_or_else(|| ExecutionError::UnknownVariable(name.clone()))
            }
            ValueSource::Stat { entity, stat } => {
                // Query entity stat
                tracing::debug!("Get stat {:?} for entity {:?}", stat, entity);
                Ok(100.0) // Placeholder
            }
        }
    }

    /// Resume execution after a pause
    pub fn resume(&mut self, delta_time: f32, world: &mut World) -> ExecutionResult<()> {
        if let ExecutionState::Paused { resume_after } = self.state {
            let new_time = resume_after - delta_time;
            if new_time <= 0.0 {
                self.state = ExecutionState::Running;
            } else {
                self.state = ExecutionState::Paused { resume_after: new_time };
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
#[derive(Debug, Default)]
pub struct ScriptRegistry {
    scripts: HashMap<u64, ScriptExecutor>,
    next_id: u64,
}

impl ScriptRegistry {
    /// Create a new script registry
    pub fn new() -> Self {
        Self {
            scripts: HashMap::new(),
            next_id: 1,
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::compiler::CompiledScript;

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
        executor.execute_modify_variable("test_var", MathOp::Add, 5).unwrap();
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
}
