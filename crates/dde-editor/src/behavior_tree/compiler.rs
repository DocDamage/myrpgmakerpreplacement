//! Behavior tree compiler
//!
//! This module compiles editor behavior trees to the runtime format
//! used by the game engine.

use dde_core::ai::behavior_tree::{
    BtStatus, CompiledBehaviorTree, NodeId, ParallelPolicy, RuntimeNode,
};
use dde_core::components::battle::Combatant;
use dde_core::components::{Position, Stats};
use dde_core::systems::player::Player;
use dde_core::resources::SimTime;
use dde_core::{Entity, World};

use super::nodes::{BtNode, BtNodeType, MoveSpeed, MoveTarget, Target, VariableValue};

/// Compile an editor behavior tree to runtime format
pub fn compile(tree: &BtNode) -> Result<CompiledBehaviorTree, CompileError> {
    let validator = Validator::new();
    validator.validate(tree)?;

    let optimizer = Optimizer::new();
    let optimized = optimizer.optimize(tree);

    let compiler = Compiler::new();
    let root = compiler.compile_node(&optimized)?;

    let mut blackboard_keys = Vec::new();
    collect_blackboard_keys(&optimized, &mut blackboard_keys);

    Ok(CompiledBehaviorTree::with_keys(root, blackboard_keys))
}

/// Compile a behavior tree and register it with a runner for the given entity
pub fn compile_and_attach(
    tree: &BtNode,
    entity: Entity,
    world: &mut World,
) -> Result<(), CompileError> {
    let compiled = compile(tree)?;
    
    // Create the behavior tree component and attach to entity
    let bt_component = dde_core::ai::behavior_tree::BehaviorTreeComponent::new(compiled);
    
    // Insert or replace the component on the entity
    if let Err(e) = world.insert(entity, (bt_component,)) {
        return Err(CompileError::Validation(format!(
            "Failed to attach behavior tree to entity {:?}: {}",
            entity, e
        )));
    }
    
    Ok(())
}

/// Compile error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum CompileError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Empty composite node: {0:?}")]
    EmptyComposite(NodeId),

    #[error("Missing child in decorator: {0:?}")]
    MissingChild(NodeId),

    #[error("Invalid node configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Unsupported node type at runtime: {0}")]
    UnsupportedNodeType(String),
}

/// Validator for checking tree integrity
struct Validator;

impl Validator {
    fn new() -> Self {
        Self
    }

    fn validate(&self, root: &BtNode) -> Result<(), CompileError> {
        self.validate_node(root)
    }

    #[allow(clippy::only_used_in_recursion)]
    fn validate_node(&self, node: &BtNode) -> Result<(), CompileError> {
        match &node.node_type {
            BtNodeType::Selector { children } | BtNodeType::Sequence { children } => {
                if children.is_empty() {
                    return Err(CompileError::EmptyComposite(node.id));
                }
                for child in children {
                    self.validate_node(child)?;
                }
            }
            BtNodeType::Parallel { children, .. } => {
                if children.is_empty() {
                    return Err(CompileError::EmptyComposite(node.id));
                }
                if children.len() < 2 {
                    return Err(CompileError::Validation(format!(
                        "Parallel node {:?} should have at least 2 children",
                        node.id
                    )));
                }
                for child in children {
                    self.validate_node(child)?;
                }
            }
            BtNodeType::Inverter { child }
            | BtNodeType::Repeater { child, .. }
            | BtNodeType::UntilSuccess { child }
            | BtNodeType::UntilFailure { child }
            | BtNodeType::Cooldown { child, .. } => {
                // Box::new(BtNode::default()) is a placeholder, check if it's been replaced
                if matches!(child.node_type, BtNodeType::Sequence { ref children } if children.is_empty())
                {
                    // It's a default/placeholder node
                    return Err(CompileError::MissingChild(node.id));
                }
                self.validate_node(child)?;
            }
            BtNodeType::HealthBelow { percent } => {
                if !(0.0..=1.0).contains(percent) {
                    return Err(CompileError::InvalidConfiguration(format!(
                        "HealthBelow percent must be between 0.0 and 1.0, got {}",
                        percent
                    )));
                }
            }
            BtNodeType::RandomChance { percent } => {
                if *percent > 100 {
                    return Err(CompileError::InvalidConfiguration(format!(
                        "RandomChance percent must be <= 100, got {}",
                        percent
                    )));
                }
            }
            BtNodeType::TimeOfDay { min, max } => {
                if min >= max || *max > 24 {
                    return Err(CompileError::InvalidConfiguration(format!(
                        "Invalid TimeOfDay range: {} to {}",
                        min, max
                    )));
                }
            }
            _ => {}
        }

        Ok(())
    }
}

/// Optimizer for simplifying trees
struct Optimizer;

impl Optimizer {
    fn new() -> Self {
        Self
    }

    fn optimize(&self, node: &BtNode) -> BtNode {
        let mut optimized = node.clone();

        // Optimize children recursively
        match &mut optimized.node_type {
            BtNodeType::Selector { children }
            | BtNodeType::Sequence { children }
            | BtNodeType::Parallel { children, .. } => {
                for child in children.iter_mut() {
                    *child = self.optimize(child);
                }
            }
            BtNodeType::Inverter { child }
            | BtNodeType::Repeater { child, .. }
            | BtNodeType::UntilSuccess { child }
            | BtNodeType::UntilFailure { child }
            | BtNodeType::Cooldown { child, .. } => {
                let optimized_child = self.optimize(child);
                *child = Box::new(optimized_child);
            }
            _ => {}
        }

        // Apply optimizations
        self.collapse_single_child_composites(&mut optimized);
        self.remove_redundant_inverters(&mut optimized);

        optimized
    }

    /// Collapse composites with single child (useless wrapper)
    fn collapse_single_child_composites(&self, node: &mut BtNode) {
        if let BtNodeType::Selector { children } | BtNodeType::Sequence { children } =
            &mut node.node_type
        {
            if children.len() == 1 {
                // Replace with the single child (keeping the composite's position)
                let child = children.remove(0);
                let old_position = node.position;
                *node = child;
                node.position = old_position;
            }
        }
    }

    /// Remove redundant double inverters
    fn remove_redundant_inverters(&self, node: &mut BtNode) {
        if let BtNodeType::Inverter { child } = &mut node.node_type {
            if let BtNodeType::Inverter { child: inner } = &child.node_type {
                // Double inverter - replace with inner child
                let inner_child = inner.as_ref().clone();
                let old_position = node.position;
                *node = inner_child;
                node.position = old_position;
            }
        }
    }
}

/// Compiler from editor nodes to runtime nodes
struct Compiler;

impl Compiler {
    fn new() -> Self {
        Self
    }

    #[allow(clippy::only_used_in_recursion)]
    fn compile_node(&self, node: &BtNode) -> Result<RuntimeNode, CompileError> {
        match &node.node_type {
            BtNodeType::Selector { children } => {
                let compiled_children: Result<Vec<_>, _> =
                    children.iter().map(|c| self.compile_node(c)).collect();
                Ok(RuntimeNode::Selector {
                    id: node.id,
                    children: compiled_children?,
                })
            }
            BtNodeType::Sequence { children } => {
                let compiled_children: Result<Vec<_>, _> =
                    children.iter().map(|c| self.compile_node(c)).collect();
                Ok(RuntimeNode::Sequence {
                    id: node.id,
                    children: compiled_children?,
                })
            }
            BtNodeType::Parallel {
                children,
                success_policy,
                failure_policy,
            } => {
                let compiled_children: Result<Vec<_>, _> =
                    children.iter().map(|c| self.compile_node(c)).collect();
                Ok(RuntimeNode::Parallel {
                    id: node.id,
                    children: compiled_children?,
                    success_policy: convert_policy(*success_policy),
                    failure_policy: convert_policy(*failure_policy),
                })
            }
            BtNodeType::Inverter { child } => {
                let compiled = self.compile_node(child)?;
                Ok(RuntimeNode::Inverter {
                    id: node.id,
                    child: Box::new(compiled),
                })
            }
            BtNodeType::Repeater { child, count } => {
                let compiled = self.compile_node(child)?;
                Ok(RuntimeNode::Repeater {
                    id: node.id,
                    child: Box::new(compiled),
                    count: *count,
                    current: 0,
                })
            }
            BtNodeType::UntilSuccess { child } => {
                let compiled = self.compile_node(child)?;
                Ok(RuntimeNode::UntilSuccess {
                    id: node.id,
                    child: Box::new(compiled),
                })
            }
            BtNodeType::UntilFailure { child } => {
                let compiled = self.compile_node(child)?;
                Ok(RuntimeNode::UntilFailure {
                    id: node.id,
                    child: Box::new(compiled),
                })
            }
            BtNodeType::Cooldown { child, seconds } => {
                let compiled = self.compile_node(child)?;
                Ok(RuntimeNode::Cooldown {
                    id: node.id,
                    child: Box::new(compiled),
                    seconds: *seconds,
                    last_execution: None,
                })
            }

            // Conditions
            BtNodeType::IsPlayerNearby { radius } => Ok(RuntimeNode::Condition {
                id: node.id,
                condition: Box::new(IsPlayerNearbyCondition { radius: *radius }),
            }),
            BtNodeType::HealthBelow { percent } => Ok(RuntimeNode::Condition {
                id: node.id,
                condition: Box::new(HealthBelowCondition { percent: *percent }),
            }),
            BtNodeType::RandomChance { percent } => Ok(RuntimeNode::Condition {
                id: node.id,
                condition: Box::new(RandomChanceCondition { percent: *percent }),
            }),
            BtNodeType::InCombat => Ok(RuntimeNode::Condition {
                id: node.id,
                condition: Box::new(InCombatCondition),
            }),
            BtNodeType::HasLineOfSight { target } => Ok(RuntimeNode::Condition {
                id: node.id,
                condition: Box::new(HasLineOfSightCondition { target: *target }),
            }),
            BtNodeType::TimeOfDay { min, max } => Ok(RuntimeNode::Condition {
                id: node.id,
                condition: Box::new(TimeOfDayCondition { min: *min, max: *max }),
            }),

            // Actions
            BtNodeType::MoveTo { target, speed } => Ok(RuntimeNode::Action {
                id: node.id,
                action: Box::new(MoveToAction {
                    target: target.clone(),
                    speed: *speed,
                }),
            }),
            BtNodeType::Wait { seconds } => Ok(RuntimeNode::Action {
                id: node.id,
                action: Box::new(WaitAction { seconds: *seconds }),
            }),
            BtNodeType::Flee => Ok(RuntimeNode::Action {
                id: node.id,
                action: Box::new(FleeAction),
            }),
            BtNodeType::Follow { target, distance } => Ok(RuntimeNode::Action {
                id: node.id,
                action: Box::new(FollowAction {
                    target: *target,
                    distance: *distance,
                }),
            }),
            BtNodeType::Attack { target } => Ok(RuntimeNode::Action {
                id: node.id,
                action: Box::new(AttackAction { target: *target }),
            }),
            BtNodeType::SetVariable { name, value } => Ok(RuntimeNode::Action {
                id: node.id,
                action: Box::new(SetVariableAction {
                    name: name.clone(),
                    value: value.clone(),
                }),
            }),

            // For custom/script nodes, we'll need runtime script execution
            BtNodeType::CustomCondition { script } => Ok(RuntimeNode::Condition {
                id: node.id,
                condition: Box::new(ScriptCondition {
                    script: script.clone(),
                }),
            }),
            BtNodeType::CustomAction { script } => Ok(RuntimeNode::Action {
                id: node.id,
                action: Box::new(ScriptAction {
                    script: script.clone(),
                }),
            }),

            // Other conditions/actions - placeholder implementations
            _ => {
                // For unimplemented nodes, create a placeholder that always fails
                Ok(RuntimeNode::Condition {
                    id: node.id,
                    condition: Box::new(UnimplementedCondition {
                        name: format!("{:?}", node.node_type),
                    }),
                })
            }
        }
    }
}

fn convert_policy(
    policy: super::nodes::ParallelPolicy,
) -> dde_core::ai::behavior_tree::ParallelPolicy {
    match policy {
        super::nodes::ParallelPolicy::RequireAll => ParallelPolicy::RequireAll,
        super::nodes::ParallelPolicy::RequireOne => ParallelPolicy::RequireOne,
    }
}

/// Collect all blackboard keys used in the tree
fn collect_blackboard_keys(node: &BtNode, keys: &mut Vec<String>) {
    if let BtNodeType::SetVariable { name, .. } = &node.node_type {
        if !keys.contains(name) {
            keys.push(name.clone());
        }
    }

    // Recursively collect from children
    if let Some(children) = node.children() {
        for child in children {
            collect_blackboard_keys(child, keys);
        }
    }
    if let Some(child) = node.node_type.child() {
        collect_blackboard_keys(child, keys);
    }
}

/// Interpolate variables in a string, replacing {var_name} with blackboard values
fn interpolate_variables(input: &str, blackboard: &dde_core::ai::Blackboard) -> String {
    let mut result = input.to_string();
    
    // Simple variable interpolation: replace {key} with blackboard value
    let mut start = 0;
    while let Some(open) = result[start..].find('{') {
        let open = start + open;
        if let Some(close) = result[open..].find('}') {
            let close = open + close;
            let key = &result[open + 1..close];
            
            // Try to get value from blackboard
            let replacement = if let Some(val) = blackboard.get::<String>(key) {
                val
            } else if let Some(val) = blackboard.get::<f32>(key) {
                format!("{:.2}", val)
            } else if let Some(val) = blackboard.get::<i32>(key) {
                val.to_string()
            } else if let Some(val) = blackboard.get::<bool>(key) {
                val.to_string()
            } else {
                // Keep original if not found
                format!("{{{}}}", key)
            };
            
            result.replace_range(open..=close, &replacement);
            start = open + replacement.len();
        } else {
            break;
        }
    }
    
    result
}

/// Get player entity from the world
fn find_player_entity(world: &World) -> Option<Entity> {
    world.query::<&Player>().iter().next().map(|(entity, _)| entity)
}

/// Get simulation time from world resources (via query)
fn get_simulation_time(world: &World) -> Option<SimTime> {
    // SimTime is stored as a resource in the world
    // In hecs, resources can be queried as singleton entities
    for (_, time) in world.query::<&SimTime>().iter() {
        return Some(*time);
    }
    None
}

/// Get position of a target
fn get_target_position(target: Target, entity: Entity, world: &World) -> Option<Position> {
    match target {
        Target::Player => {
            let player = find_player_entity(world)?;
            world.query_one::<&Position>(player)
                .ok()
                .and_then(|mut q| q.get().copied())
        }
        Target::SelfEntity => {
            world.query_one::<&Position>(entity)
                .ok()
                .and_then(|mut q| q.get().copied())
        }
        Target::Entity(e) => {
            world.query_one::<&Position>(e)
                .ok()
                .and_then(|mut q| q.get().copied())
        }
        _ => None, // NearestEnemy and NearestAlly would need more complex queries
    }
}

// Runtime condition implementations

#[derive(Debug, Clone)]
struct IsPlayerNearbyCondition {
    radius: f32,
}

impl dde_core::ai::behavior_tree::Condition for IsPlayerNearbyCondition {
    fn evaluate(
        &self,
        entity: Entity,
        world: &World,
        _blackboard: &dde_core::ai::Blackboard,
    ) -> BtStatus {
        // Get the entity's position
        let entity_pos = {
            let query_result = world.query_one::<&Position>(entity);
            match query_result {
                Ok(mut query) => match query.get() {
                    Some(pos) => *pos,
                    None => return BtStatus::Failure,
                },
                Err(_) => return BtStatus::Failure,
            }
        };

        // Find the player entity and get its position
        let player_pos = match find_player_entity(world) {
            Some(player) => {
                let query_result = world.query_one::<&Position>(player);
                match query_result {
                    Ok(mut query) => match query.get() {
                        Some(pos) => *pos,
                        None => return BtStatus::Failure,
                    },
                    Err(_) => return BtStatus::Failure,
                }
            }
            None => return BtStatus::Failure,
        };

        // Calculate distance (in tiles)
        let dx = (entity_pos.x - player_pos.x) as f32;
        let dy = (entity_pos.y - player_pos.y) as f32;
        let distance_sq = dx * dx + dy * dy;
        let radius_sq = self.radius * self.radius;

        if distance_sq <= radius_sq {
            BtStatus::Success
        } else {
            BtStatus::Failure
        }
    }

    fn box_clone(&self) -> Box<dyn dde_core::ai::behavior_tree::Condition> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct HealthBelowCondition {
    percent: f32,
}

impl dde_core::ai::behavior_tree::Condition for HealthBelowCondition {
    fn evaluate(
        &self,
        entity: Entity,
        world: &World,
        _blackboard: &dde_core::ai::Blackboard,
    ) -> BtStatus {
        // Get the entity's stats and check HP percentage
        let query_result = world.query_one::<&Stats>(entity);
        match query_result {
            Ok(mut query) => match query.get() {
                Some(stats) => {
                    let hp_percent = stats.hp_percent();
                    if hp_percent <= self.percent {
                        BtStatus::Success
                    } else {
                        BtStatus::Failure
                    }
                }
                None => BtStatus::Failure,
            },
            Err(_) => BtStatus::Failure,
        }
    }

    fn box_clone(&self) -> Box<dyn dde_core::ai::behavior_tree::Condition> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct RandomChanceCondition {
    percent: u8,
}

impl dde_core::ai::behavior_tree::Condition for RandomChanceCondition {
    fn evaluate(
        &self,
        _entity: Entity,
        _world: &World,
        _blackboard: &dde_core::ai::Blackboard,
    ) -> BtStatus {
        // Use the rand crate's thread_rng for random number generation
        // In a deterministic simulation context, this should be replaced with RngPool
        let roll: f32 = rand::random();
        let threshold = self.percent as f32 / 100.0;
        
        if roll < threshold {
            BtStatus::Success
        } else {
            BtStatus::Failure
        }
    }

    fn box_clone(&self) -> Box<dyn dde_core::ai::behavior_tree::Condition> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct InCombatCondition;

impl dde_core::ai::behavior_tree::Condition for InCombatCondition {
    fn evaluate(
        &self,
        entity: Entity,
        world: &World,
        _blackboard: &dde_core::ai::Blackboard,
    ) -> BtStatus {
        // Check if the entity has a Combatant component
        match world.query_one::<&Combatant>(entity) {
            Ok(mut query) => {
                if query.get().is_some() {
                    BtStatus::Success
                } else {
                    BtStatus::Failure
                }
            }
            Err(_) => BtStatus::Failure,
        }
    }

    fn box_clone(&self) -> Box<dyn dde_core::ai::behavior_tree::Condition> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct HasLineOfSightCondition {
    target: Target,
}

impl dde_core::ai::behavior_tree::Condition for HasLineOfSightCondition {
    fn evaluate(
        &self,
        entity: Entity,
        world: &World,
        _blackboard: &dde_core::ai::Blackboard,
    ) -> BtStatus {
        // Get entity position
        let entity_pos = {
            let query_result = world.query_one::<&Position>(entity);
            match query_result {
                Ok(mut query) => match query.get() {
                    Some(pos) => *pos,
                    None => return BtStatus::Failure,
                },
                Err(_) => return BtStatus::Failure,
            }
        };

        // Get target position
        let target_pos = match get_target_position(self.target, entity, world) {
            Some(pos) => pos,
            None => return BtStatus::Failure,
        };

        // Simple line of sight check - same Z level and within reasonable distance
        if entity_pos.z != target_pos.z {
            return BtStatus::Failure;
        }

        // Check if within line of sight distance (e.g., 20 tiles)
        let dx = entity_pos.x - target_pos.x;
        let dy = entity_pos.y - target_pos.y;
        let distance_sq = dx * dx + dy * dy;

        if distance_sq <= 400 { // 20 tiles squared
            BtStatus::Success
        } else {
            BtStatus::Failure
        }
    }

    fn box_clone(&self) -> Box<dyn dde_core::ai::behavior_tree::Condition> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct TimeOfDayCondition {
    min: u8,
    max: u8,
}

impl dde_core::ai::behavior_tree::Condition for TimeOfDayCondition {
    fn evaluate(
        &self,
        _entity: Entity,
        world: &World,
        _blackboard: &dde_core::ai::Blackboard,
    ) -> BtStatus {
        // Get the current simulation time from world resources
        match get_simulation_time(world) {
            Some(sim_time) => {
                let hour = sim_time.hour;
                if hour >= self.min && hour < self.max {
                    BtStatus::Success
                } else {
                    BtStatus::Failure
                }
            }
            None => BtStatus::Failure, // No time resource available
        }
    }

    fn box_clone(&self) -> Box<dyn dde_core::ai::behavior_tree::Condition> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct ScriptCondition {
    script: String,
}

impl dde_core::ai::behavior_tree::Condition for ScriptCondition {
    fn evaluate(
        &self,
        _entity: Entity,
        _world: &World,
        blackboard: &dde_core::ai::Blackboard,
    ) -> BtStatus {
        if self.script.is_empty() {
            return BtStatus::Failure;
        }

        // Interpolate variables in the script
        let script = interpolate_variables(&self.script, blackboard);

        // For now, succeed if script is non-empty and starts with "return true"
        // In a full implementation, this would execute the script via LuaEngine
        if script.trim().starts_with("return true") || script.trim() == "true" {
            BtStatus::Success
        } else if script.trim().starts_with("return false") || script.trim() == "false" {
            BtStatus::Failure
        } else {
            // Script exists but doesn't explicitly return - default to success
            BtStatus::Success
        }
    }

    fn box_clone(&self) -> Box<dyn dde_core::ai::behavior_tree::Condition> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct UnimplementedCondition {
    name: String,
}

impl dde_core::ai::behavior_tree::Condition for UnimplementedCondition {
    fn evaluate(
        &self,
        _entity: Entity,
        _world: &World,
        _blackboard: &dde_core::ai::Blackboard,
    ) -> BtStatus {
        tracing::warn!("Unimplemented condition: {}", self.name);
        BtStatus::Failure
    }

    fn box_clone(&self) -> Box<dyn dde_core::ai::behavior_tree::Condition> {
        Box::new(self.clone())
    }
}

// Runtime action implementations

#[derive(Debug, Clone)]
struct MoveToAction {
    target: MoveTarget,
    speed: MoveSpeed,
}

impl dde_core::ai::behavior_tree::Action for MoveToAction {
    fn execute(
        &self,
        entity: Entity,
        world: &mut World,
        blackboard: &mut dde_core::ai::Blackboard,
    ) -> BtStatus {
        // Get target position based on MoveTarget type
        let target_pos = match &self.target {
            MoveTarget::Position(pos) => Position::new(pos.x as i32, pos.y as i32, pos.z as i32),
            MoveTarget::Entity(target_entity) => {
                let query_result = world.query_one::<&Position>(*target_entity);
                match query_result {
                    Ok(mut query) => match query.get() {
                        Some(pos) => *pos,
                        None => return BtStatus::Failure,
                    },
                    Err(_) => return BtStatus::Failure,
                }
            }
            MoveTarget::Player => {
                match find_player_entity(world) {
                    Some(player) => {
                        let query_result = world.query_one::<&Position>(player);
                        match query_result {
                            Ok(mut query) => match query.get() {
                                Some(pos) => *pos,
                                None => return BtStatus::Failure,
                            },
                            Err(_) => return BtStatus::Failure,
                        }
                    }
                    None => return BtStatus::Failure,
                }
            }
            MoveTarget::PatrolPoint(index) => {
                // Default patrol points arranged in a square pattern
                let default_positions = [
                    Position::new(5, 5, 0),
                    Position::new(10, 5, 0),
                    Position::new(10, 10, 0),
                    Position::new(5, 10, 0),
                ];
                *default_positions.get(*index % default_positions.len()).unwrap_or(&default_positions[0])
            }
        };

        // Get current position
        let current_pos = {
            let query_result = world.query_one::<&Position>(entity);
            match query_result {
                Ok(mut query) => match query.get() {
                    Some(pos) => *pos,
                    None => return BtStatus::Failure,
                },
                Err(_) => return BtStatus::Failure,
            }
        };

        // Check if already at target
        if current_pos.x == target_pos.x && current_pos.y == target_pos.y {
            blackboard.remove("move_target_x");
            blackboard.remove("move_target_y");
            blackboard.remove("move_target_z");
            return BtStatus::Success;
        }

        // Calculate movement direction
        let dx = (target_pos.x - current_pos.x).signum();
        let dy = (target_pos.y - current_pos.y).signum();

        // Store target in blackboard for multi-tick movement
        blackboard.set("move_target_x", target_pos.x);
        blackboard.set("move_target_y", target_pos.y);
        blackboard.set("move_target_z", target_pos.z);
        
        // Move one step closer (simplified - full implementation would use pathfinding)
        let speed_mult = self.speed.multiplier();
        let step_size = (speed_mult as i32).max(1);
        
        let new_x = current_pos.x + dx * step_size.min((target_pos.x - current_pos.x).abs());
        let new_y = current_pos.y + dy * step_size.min((target_pos.y - current_pos.y).abs());

        // Update position
        if let Ok(mut query) = world.query_one::<&mut Position>(entity) {
            if let Some(pos) = query.get() {
                pos.x = new_x;
                pos.y = new_y;
                pos.z = target_pos.z;
            }
        }

        // Return Running if still moving, Success if arrived
        if new_x == target_pos.x && new_y == target_pos.y {
            blackboard.remove("move_target_x");
            blackboard.remove("move_target_y");
            blackboard.remove("move_target_z");
            BtStatus::Success
        } else {
            BtStatus::Running
        }
    }

    fn box_clone(&self) -> Box<dyn dde_core::ai::behavior_tree::Action> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct WaitAction {
    seconds: f32,
}

impl dde_core::ai::behavior_tree::Action for WaitAction {
    fn execute(
        &self,
        _entity: Entity,
        _world: &mut World,
        blackboard: &mut dde_core::ai::Blackboard,
    ) -> BtStatus {
        // Check if we're already waiting
        let key = "wait_remaining";
        let remaining: f32 = blackboard.get(key).unwrap_or(self.seconds);

        // Simulate tick (0.05s = 1 tick at 20 TPS)
        let new_remaining = remaining - 0.05;

        if new_remaining <= 0.0 {
            blackboard.remove(key);
            BtStatus::Success
        } else {
            blackboard.set(key, new_remaining);
            BtStatus::Running
        }
    }

    fn box_clone(&self) -> Box<dyn dde_core::ai::behavior_tree::Action> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct FleeAction;

impl dde_core::ai::behavior_tree::Action for FleeAction {
    fn execute(
        &self,
        entity: Entity,
        world: &mut World,
        blackboard: &mut dde_core::ai::Blackboard,
    ) -> BtStatus {
        // Get entity position
        let entity_pos = {
            let query_result = world.query_one::<&Position>(entity);
            match query_result {
                Ok(mut query) => match query.get() {
                    Some(pos) => *pos,
                    None => return BtStatus::Failure,
                },
                Err(_) => return BtStatus::Failure,
            }
        };

        // Get player position (flee away from player)
        let player_pos = match find_player_entity(world) {
            Some(player) => {
                let query_result = world.query_one::<&Position>(player);
                match query_result {
                    Ok(mut query) => match query.get() {
                        Some(pos) => *pos,
                        None => return BtStatus::Failure,
                    },
                    Err(_) => return BtStatus::Failure,
                }
            }
            None => return BtStatus::Failure,
        };

        // Calculate flee direction (opposite of player)
        let dx = entity_pos.x - player_pos.x;
        let dy = entity_pos.y - player_pos.y;

        // Normalize and scale
        let dist_sq = dx * dx + dy * dy;
        if dist_sq == 0 {
            // Entity is on top of player, pick random direction
            let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
            let idx = (rand::random::<u32>() % 4) as usize;
            let (dx, dy) = dirs[idx];
            
            if let Ok(mut query) = world.query_one::<&mut Position>(entity) {
                if let Some(pos) = query.get() {
                    pos.x += dx * 3;
                    pos.y += dy * 3;
                }
            }
        } else {
            let dist = (dist_sq as f32).sqrt();
            let flee_dist: f32 = 5.0; // Flee 5 tiles away
            let new_x = entity_pos.x + ((dx as f32 / dist) * flee_dist) as i32;
            let new_y = entity_pos.y + ((dy as f32 / dist) * flee_dist) as i32;

            if let Ok(mut query) = world.query_one::<&mut Position>(entity) {
                if let Some(pos) = query.get() {
                    pos.x = new_x;
                    pos.y = new_y;
                }
            }
        }

        // Store flee state in blackboard
        blackboard.set("is_fleeing", true);
        blackboard.set("flee_cooldown", 5.0f32); // Can't flee again for 5 seconds

        BtStatus::Success
    }

    fn box_clone(&self) -> Box<dyn dde_core::ai::behavior_tree::Action> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct FollowAction {
    target: Target,
    distance: f32,
}

impl dde_core::ai::behavior_tree::Action for FollowAction {
    fn execute(
        &self,
        entity: Entity,
        world: &mut World,
        _blackboard: &mut dde_core::ai::Blackboard,
    ) -> BtStatus {
        // Get entity position
        let entity_pos = {
            let query_result = world.query_one::<&Position>(entity);
            match query_result {
                Ok(mut query) => match query.get() {
                    Some(pos) => *pos,
                    None => return BtStatus::Failure,
                },
                Err(_) => return BtStatus::Failure,
            }
        };

        // Get target position
        let target_pos = match get_target_position(self.target, entity, world) {
            Some(pos) => pos,
            None => return BtStatus::Failure,
        };

        // Calculate distance
        let dx = target_pos.x - entity_pos.x;
        let dy = target_pos.y - entity_pos.y;
        let distance_sq = dx * dx + dy * dy;
        let target_dist_sq = (self.distance * self.distance) as i32;

        // If already at desired distance, succeed
        if distance_sq <= target_dist_sq {
            return BtStatus::Success;
        }

        // Move one step closer
        let step_x = dx.signum();
        let step_y = dy.signum();

        if let Ok(mut query) = world.query_one::<&mut Position>(entity) {
            if let Some(pos) = query.get() {
                pos.x += step_x;
                pos.y += step_y;
            }
        }

        // Check if now at desired distance
        let new_dx = target_pos.x - (entity_pos.x + step_x);
        let new_dy = target_pos.y - (entity_pos.y + step_y);
        let new_dist_sq = new_dx * new_dx + new_dy * new_dy;

        if new_dist_sq <= target_dist_sq {
            BtStatus::Success
        } else {
            BtStatus::Running
        }
    }

    fn box_clone(&self) -> Box<dyn dde_core::ai::behavior_tree::Action> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct AttackAction {
    target: Target,
}

impl dde_core::ai::behavior_tree::Action for AttackAction {
    fn execute(
        &self,
        entity: Entity,
        world: &mut World,
        blackboard: &mut dde_core::ai::Blackboard,
    ) -> BtStatus {
        // Get target entity
        let target_entity = match self.target {
            Target::Player => match find_player_entity(world) {
                Some(e) => e,
                None => return BtStatus::Failure,
            },
            Target::Entity(e) => e,
            _ => return BtStatus::Failure, // Other targets not yet supported
        };

        // Check if in range (adjacent)
        let entity_pos = {
            let query_result = world.query_one::<&Position>(entity);
            match query_result {
                Ok(mut query) => match query.get() {
                    Some(pos) => *pos,
                    None => return BtStatus::Failure,
                },
                Err(_) => return BtStatus::Failure,
            }
        };

        let target_pos = {
            let query_result = world.query_one::<&Position>(target_entity);
            match query_result {
                Ok(mut query) => match query.get() {
                    Some(pos) => *pos,
                    None => return BtStatus::Failure,
                },
                Err(_) => return BtStatus::Failure,
            }
        };

        let dx = (entity_pos.x - target_pos.x).abs();
        let dy = (entity_pos.y - target_pos.y).abs();

        // Must be adjacent to attack
        if dx > 1 || dy > 1 {
            return BtStatus::Failure;
        }

        // Get entity's stats for damage calculation
        let attacker_stats = world.query_one::<&Stats>(entity)
            .ok()
            .and_then(|mut q| q.get().copied())
            .unwrap_or_default();

        // Apply damage to target
        let damage = attacker_stats.str.max(1);
        if let Ok(mut query) = world.query_one::<&mut Stats>(target_entity) {
            if let Some(stats) = query.get() {
                stats.take_damage(damage);
            }
        }

        // Store attack info in blackboard
        blackboard.set("last_attack_target", target_entity);
        blackboard.set("last_attack_damage", damage as i32);

        BtStatus::Success
    }

    fn box_clone(&self) -> Box<dyn dde_core::ai::behavior_tree::Action> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct SetVariableAction {
    name: String,
    value: VariableValue,
}

impl dde_core::ai::behavior_tree::Action for SetVariableAction {
    fn execute(
        &self,
        _entity: Entity,
        _world: &mut World,
        blackboard: &mut dde_core::ai::Blackboard,
    ) -> BtStatus {
        match &self.value {
            VariableValue::Bool(v) => blackboard.set(&self.name, *v),
            VariableValue::Int(v) => blackboard.set(&self.name, *v),
            VariableValue::Float(v) => blackboard.set(&self.name, *v),
            VariableValue::String(v) => blackboard.set(&self.name, v.clone()),
            VariableValue::Entity(e) => blackboard.set(&self.name, *e),
        }
        BtStatus::Success
    }

    fn box_clone(&self) -> Box<dyn dde_core::ai::behavior_tree::Action> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone)]
struct ScriptAction {
    script: String,
}

impl dde_core::ai::behavior_tree::Action for ScriptAction {
    fn execute(
        &self,
        entity: Entity,
        _world: &mut World,
        blackboard: &mut dde_core::ai::Blackboard,
    ) -> BtStatus {
        if self.script.is_empty() {
            return BtStatus::Failure;
        }

        // Interpolate variables in the script
        let script = interpolate_variables(&self.script, blackboard);

        // Store the script result and entity info in blackboard for debugging
        blackboard.set("last_script_entity", entity);
        blackboard.set("last_script", script.clone());

        // In a full implementation, this would execute via LuaEngine
        // For now, we simulate script execution:
        // - Scripts starting with "success" or "return true" succeed
        // - Scripts starting with "fail" or "return false" fail
        // - Other scripts succeed by default
        let trimmed = script.trim().to_lowercase();
        if trimmed.starts_with("success") 
            || trimmed.starts_with("return true") 
            || trimmed == "true" {
            blackboard.set("last_script_result", "success");
            BtStatus::Success
        } else if trimmed.starts_with("fail") 
            || trimmed.starts_with("return false") 
            || trimmed == "false" {
            blackboard.set("last_script_result", "failure");
            BtStatus::Failure
        } else {
            // Default: script executes successfully
            blackboard.set("last_script_result", "success");
            BtStatus::Success
        }
    }

    fn box_clone(&self) -> Box<dyn dde_core::ai::behavior_tree::Action> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::super::nodes::{BtNode, BtNodeType};
    use super::*;

    #[test]
    fn test_compile_simple_sequence() {
        let root = BtNode::new(
            BtNodeType::Sequence {
                children: vec![
                    BtNode::new(BtNodeType::InCombat, [0.0, 0.0]),
                    BtNode::new(BtNodeType::Flee, [100.0, 0.0]),
                ],
            },
            [0.0, 0.0],
        );

        let result = compile(&root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_empty_composite_fails() {
        let root = BtNode::new(BtNodeType::Sequence { children: vec![] }, [0.0, 0.0]);

        let result = compile(&root);
        assert!(matches!(result, Err(CompileError::EmptyComposite(_))));
    }

    #[test]
    fn test_validate_health_percent() {
        let root = BtNode::new(BtNodeType::HealthBelow { percent: 1.5 }, [0.0, 0.0]);

        let result = compile(&root);
        assert!(matches!(result, Err(CompileError::InvalidConfiguration(_))));
    }

    #[test]
    fn test_variable_interpolation() {
        let mut blackboard = dde_core::ai::Blackboard::new();
        blackboard.set("player_name", "Alice");
        blackboard.set("health", 50i32);
        blackboard.set("is_alive", true);

        let result = interpolate_variables("Hello {player_name}, HP: {health}, Alive: {is_alive}", &blackboard);
        assert_eq!(result, "Hello Alice, HP: 50, Alive: true");
    }

    #[test]
    fn test_variable_interpolation_missing() {
        let blackboard = dde_core::ai::Blackboard::new();

        let result = interpolate_variables("Missing: {unknown}", &blackboard);
        assert_eq!(result, "Missing: {unknown}");
    }

    #[test]
    fn test_compile_and_attach() {
        let root = BtNode::new(
            BtNodeType::Sequence {
                children: vec![
                    BtNode::new(BtNodeType::InCombat, [0.0, 0.0]),
                    BtNode::new(BtNodeType::Flee, [100.0, 0.0]),
                ],
            },
            [0.0, 0.0],
        );

        let mut world = World::new();
        let entity = world.spawn((Position::new(0, 0, 0),));

        let result = compile_and_attach(&root, entity, &mut world);
        assert!(result.is_ok());

        // Verify the component was attached
        let has_component = world.query_one::<&dde_core::ai::behavior_tree::BehaviorTreeComponent>(entity).is_ok();
        assert!(has_component);
    }
}
