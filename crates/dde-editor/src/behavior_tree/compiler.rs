//! Behavior tree compiler
//!
//! This module compiles editor behavior trees to the runtime format
//! used by the game engine.

use dde_core::ai::behavior_tree::{
    BtStatus, CompiledBehaviorTree, NodeId, ParallelPolicy, RuntimeNode,
};
use dde_core::{Entity, World};

use super::nodes::{BtNode, BtNodeType, MoveSpeed, MoveTarget, Target};

/// Compile an editor behavior tree to runtime format
pub fn compile(tree: &BtNode) -> Result<CompiledBehaviorTree, CompileError> {
    let mut validator = Validator::new();
    validator.validate(tree)?;
    
    let optimizer = Optimizer::new();
    let optimized = optimizer.optimize(tree);
    
    let compiler = Compiler::new();
    let root = compiler.compile_node(&optimized)?;
    
    let mut blackboard_keys = Vec::new();
    collect_blackboard_keys(&optimized, &mut blackboard_keys);
    
    Ok(CompiledBehaviorTree::with_keys(root, blackboard_keys))
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
                    return Err(CompileError::Validation(
                        format!("Parallel node {:?} should have at least 2 children", node.id)
                    ));
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
                if matches!(child.node_type, BtNodeType::Sequence { ref children } if children.is_empty()) {
                    // It's a default/placeholder node
                    return Err(CompileError::MissingChild(node.id));
                }
                self.validate_node(child)?;
            }
            BtNodeType::HealthBelow { percent } => {
                if !(0.0..=1.0).contains(percent) {
                    return Err(CompileError::InvalidConfiguration(
                        format!("HealthBelow percent must be between 0.0 and 1.0, got {}", percent)
                    ));
                }
            }
            BtNodeType::RandomChance { percent } => {
                if *percent > 100 {
                    return Err(CompileError::InvalidConfiguration(
                        format!("RandomChance percent must be <= 100, got {}", percent)
                    ));
                }
            }
            BtNodeType::TimeOfDay { min, max } => {
                if min >= max || *max > 24 {
                    return Err(CompileError::InvalidConfiguration(
                        format!("Invalid TimeOfDay range: {} to {}", min, max)
                    ));
                }
            }
            BtNodeType::Cooldown { seconds, .. } => {
                if *seconds <= 0.0 {
                    return Err(CompileError::InvalidConfiguration(
                        "Cooldown seconds must be positive".to_string()
                    ));
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
        if let BtNodeType::Selector { children } | BtNodeType::Sequence { children } = &mut node.node_type {
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
    
    fn compile_node(&self, node: &BtNode) -> Result<RuntimeNode, CompileError> {
        match &node.node_type {
            BtNodeType::Selector { children } => {
                let compiled_children: Result<Vec<_>, _> = children
                    .iter()
                    .map(|c| self.compile_node(c))
                    .collect();
                Ok(RuntimeNode::Selector {
                    id: node.id,
                    children: compiled_children?,
                })
            }
            BtNodeType::Sequence { children } => {
                let compiled_children: Result<Vec<_>, _> = children
                    .iter()
                    .map(|c| self.compile_node(c))
                    .collect();
                Ok(RuntimeNode::Sequence {
                    id: node.id,
                    children: compiled_children?,
                })
            }
            BtNodeType::Parallel { children, success_policy, failure_policy } => {
                let compiled_children: Result<Vec<_>, _> = children
                    .iter()
                    .map(|c| self.compile_node(c))
                    .collect();
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
            BtNodeType::IsPlayerNearby { radius } => {
                Ok(RuntimeNode::Condition {
                    id: node.id,
                    condition: Box::new(IsPlayerNearbyCondition { radius: *radius }),
                })
            }
            BtNodeType::HealthBelow { percent } => {
                Ok(RuntimeNode::Condition {
                    id: node.id,
                    condition: Box::new(HealthBelowCondition { percent: *percent }),
                })
            }
            BtNodeType::RandomChance { percent } => {
                Ok(RuntimeNode::Condition {
                    id: node.id,
                    condition: Box::new(RandomChanceCondition { percent: *percent }),
                })
            }
            BtNodeType::InCombat => {
                Ok(RuntimeNode::Condition {
                    id: node.id,
                    condition: Box::new(InCombatCondition),
                })
            }
            
            // Actions
            BtNodeType::MoveTo { target, speed } => {
                Ok(RuntimeNode::Action {
                    id: node.id,
                    action: Box::new(MoveToAction {
                        target: target.clone(),
                        speed: *speed,
                    }),
                })
            }
            BtNodeType::Wait { seconds } => {
                Ok(RuntimeNode::Action {
                    id: node.id,
                    action: Box::new(WaitAction { seconds: *seconds }),
                })
            }
            BtNodeType::Flee => {
                Ok(RuntimeNode::Action {
                    id: node.id,
                    action: Box::new(FleeAction),
                })
            }
            
            // For custom/script nodes, we'll need runtime script execution
            BtNodeType::CustomCondition { script } => {
                Ok(RuntimeNode::Condition {
                    id: node.id,
                    condition: Box::new(ScriptCondition { script: script.clone() }),
                })
            }
            BtNodeType::CustomAction { script } => {
                Ok(RuntimeNode::Action {
                    id: node.id,
                    action: Box::new(ScriptAction { script: script.clone() }),
                })
            }
            
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

fn convert_policy(policy: super::nodes::ParallelPolicy) -> dde_core::ai::behavior_tree::ParallelPolicy {
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

// Runtime condition implementations

#[derive(Debug, Clone)]
struct IsPlayerNearbyCondition {
    radius: f32,
}

impl dde_core::ai::behavior_tree::Condition for IsPlayerNearbyCondition {
    fn evaluate(&self, _entity: Entity, _world: &World, _blackboard: &dde_core::ai::Blackboard) -> BtStatus {
        // TODO: Implement actual player distance check
        // For now, return failure for testing
        BtStatus::Failure
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
    fn evaluate(&self, _entity: Entity, _world: &World, _blackboard: &dde_core::ai::Blackboard) -> BtStatus {
        // TODO: Implement actual health check
        BtStatus::Failure
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
    fn evaluate(&self, _entity: Entity, _world: &World, _blackboard: &dde_core::ai::Blackboard) -> BtStatus {
        // TODO: Use proper RNG
        // For now, succeed if percent >= 50
        if self.percent >= 50 {
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
    fn evaluate(&self, _entity: Entity, _world: &World, _blackboard: &dde_core::ai::Blackboard) -> BtStatus {
        // TODO: Check combat state
        BtStatus::Failure
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
    fn evaluate(&self, _entity: Entity, _world: &World, _blackboard: &dde_core::ai::Blackboard) -> BtStatus {
        // TODO: Execute script and return result
        // For now, succeed if script is non-empty
        if self.script.is_empty() {
            BtStatus::Failure
        } else {
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
    fn evaluate(&self, _entity: Entity, _world: &World, _blackboard: &dde_core::ai::Blackboard) -> BtStatus {
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
    fn execute(&self, _entity: Entity, _world: &mut World, _blackboard: &mut dde_core::ai::Blackboard) -> BtStatus {
        // TODO: Implement actual movement
        // For now, return running to simulate ongoing action
        BtStatus::Running
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
    fn execute(&self, _entity: Entity, _world: &mut World, blackboard: &mut dde_core::ai::Blackboard) -> BtStatus {
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
    fn execute(&self, _entity: Entity, _world: &mut World, _blackboard: &mut dde_core::ai::Blackboard) -> BtStatus {
        // TODO: Implement flee behavior
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
    fn execute(&self, _entity: Entity, _world: &mut World, _blackboard: &mut dde_core::ai::Blackboard) -> BtStatus {
        // TODO: Execute script
        if self.script.is_empty() {
            BtStatus::Failure
        } else {
            BtStatus::Success
        }
    }
    
    fn box_clone(&self) -> Box<dyn dde_core::ai::behavior_tree::Action> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::nodes::{BtNode, BtNodeType};

    #[test]
    fn test_compile_simple_sequence() {
        let root = BtNode::new(
            BtNodeType::Sequence { children: vec![
                BtNode::new(BtNodeType::InCombat, [0.0, 0.0]),
                BtNode::new(BtNodeType::Flee, [100.0, 0.0]),
            ]},
            [0.0, 0.0],
        );
        
        let result = compile(&root);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_empty_composite_fails() {
        let root = BtNode::new(
            BtNodeType::Sequence { children: vec![] },
            [0.0, 0.0],
        );
        
        let result = compile(&root);
        assert!(matches!(result, Err(CompileError::EmptyComposite(_))));
    }

    #[test]
    fn test_validate_health_percent() {
        let root = BtNode::new(
            BtNodeType::HealthBelow { percent: 1.5 },
            [0.0, 0.0],
        );
        
        let result = compile(&root);
        assert!(matches!(result, Err(CompileError::InvalidConfiguration(_))));
    }
}
