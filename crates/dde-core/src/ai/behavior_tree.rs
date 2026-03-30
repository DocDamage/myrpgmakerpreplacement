//! Runtime behavior tree execution
//!
//! This module provides the runtime execution engine for behavior trees,
//! responsible for ticking nodes and managing execution state.

use std::collections::HashMap;

use crate::Entity;
use crate::World;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Serializable wrapper for Vec3
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SerializableVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl From<Vec3> for SerializableVec3 {
    fn from(v: Vec3) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }
}

impl From<SerializableVec3> for Vec3 {
    fn from(v: SerializableVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

/// Unique identifier for behavior tree nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct NodeId(pub u64);

impl NodeId {
    /// Generate a new unique node ID
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Execution status for behavior trees
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BtStatus {
    #[default]
    Success,
    Failure,
    Running,
}

/// Behavior tree runner - executes compiled behavior trees
#[derive(Debug, Clone)]
pub struct BehaviorTreeRunner {
    /// The compiled tree to execute
    tree: CompiledBehaviorTree,
    /// Blackboard for data sharing between nodes
    blackboard: Blackboard,
    /// Current execution path (stack of node IDs)
    current_path: Vec<NodeId>,
    /// Currently running node (for resuming Running status)
    running_node: Option<NodeId>,
}

impl BehaviorTreeRunner {
    /// Create a new behavior tree runner
    pub fn new(tree: CompiledBehaviorTree) -> Self {
        Self {
            tree,
            blackboard: Blackboard::new(),
            current_path: Vec::new(),
            running_node: None,
        }
    }

    /// Create a new runner with an initialized blackboard
    pub fn with_blackboard(tree: CompiledBehaviorTree, blackboard: Blackboard) -> Self {
        Self {
            tree,
            blackboard,
            current_path: Vec::new(),
            running_node: None,
        }
    }

    /// Get reference to the blackboard
    pub fn blackboard(&self) -> &Blackboard {
        &self.blackboard
    }

    /// Get mutable reference to the blackboard
    pub fn blackboard_mut(&mut self) -> &mut Blackboard {
        &mut self.blackboard
    }

    /// Tick the behavior tree
    pub fn tick(&mut self, entity: Entity, world: &mut World) -> BtStatus {
        self.current_path.clear();

        // If we have a running node from previous tick, resume from there
        if let Some(running_id) = self.running_node {
            self.current_path.push(running_id);
        }

        // Clone the root to avoid borrow issues
        let root = self.tree.root.clone();
        let result = self.execute_node(&root, entity, world);

        // Update running node for next tick
        self.running_node = if result == BtStatus::Running {
            self.current_path.last().copied()
        } else {
            None
        };

        result
    }

    /// Get the current execution path
    pub fn current_path(&self) -> &[NodeId] {
        &self.current_path
    }

    /// Get the currently running node ID
    pub fn running_node(&self) -> Option<NodeId> {
        self.running_node
    }

    /// Execute a specific node
    fn execute_node(&mut self, node: &RuntimeNode, entity: Entity, world: &mut World) -> BtStatus {
        let node_id = node.id();
        self.current_path.push(node_id);

        let result = match node {
            RuntimeNode::Selector { children, .. } => {
                // Try each child until one succeeds
                for child in children {
                    match self.execute_node(child, entity, world) {
                        BtStatus::Success => return BtStatus::Success,
                        BtStatus::Running => return BtStatus::Running,
                        BtStatus::Failure => continue,
                    }
                }
                BtStatus::Failure
            }
            RuntimeNode::Sequence { children, .. } => {
                // Execute each child until one fails
                for child in children {
                    match self.execute_node(child, entity, world) {
                        BtStatus::Success => continue,
                        BtStatus::Running => return BtStatus::Running,
                        BtStatus::Failure => return BtStatus::Failure,
                    }
                }
                BtStatus::Success
            }
            RuntimeNode::Parallel {
                children,
                success_policy,
                failure_policy,
                ..
            } => {
                // Execute all children and apply policies
                let mut success_count = 0;
                let mut failure_count = 0;
                let mut running_count = 0;

                for child in children {
                    match self.execute_node(child, entity, world) {
                        BtStatus::Success => success_count += 1,
                        BtStatus::Failure => failure_count += 1,
                        BtStatus::Running => running_count += 1,
                    }
                }

                let total = children.len();

                // Check success policy
                match success_policy {
                    ParallelPolicy::RequireAll if success_count == total => BtStatus::Success,
                    ParallelPolicy::RequireOne if success_count >= 1 => BtStatus::Success,
                    _ => {
                        // Check failure policy
                        match failure_policy {
                            ParallelPolicy::RequireAll if failure_count == total => {
                                BtStatus::Failure
                            }
                            ParallelPolicy::RequireOne if failure_count >= 1 => BtStatus::Failure,
                            _ => {
                                if running_count > 0 {
                                    BtStatus::Running
                                } else {
                                    BtStatus::Failure
                                }
                            }
                        }
                    }
                }
            }
            RuntimeNode::Inverter { child, .. } => match self.execute_node(child, entity, world) {
                BtStatus::Success => BtStatus::Failure,
                BtStatus::Failure => BtStatus::Success,
                BtStatus::Running => BtStatus::Running,
            },
            RuntimeNode::Repeater { child, count, .. } => {
                let current_count =
                    self.blackboard.get::<i32>("repeater_count").unwrap_or(0) as u32;

                if let Some(max) = count {
                    if current_count >= *max {
                        self.blackboard.set("repeater_count", 0u32);
                        return BtStatus::Success;
                    }
                }

                match self.execute_node(child, entity, world) {
                    BtStatus::Success | BtStatus::Failure => {
                        self.blackboard.set("repeater_count", current_count + 1);
                        BtStatus::Running // Continue repeating
                    }
                    BtStatus::Running => BtStatus::Running,
                }
            }
            RuntimeNode::UntilSuccess { child, .. } => loop {
                match self.execute_node(child, entity, world) {
                    BtStatus::Success => return BtStatus::Success,
                    BtStatus::Running => return BtStatus::Running,
                    BtStatus::Failure => continue,
                }
            },
            RuntimeNode::UntilFailure { child, .. } => loop {
                match self.execute_node(child, entity, world) {
                    BtStatus::Success => continue,
                    BtStatus::Running => return BtStatus::Running,
                    BtStatus::Failure => return BtStatus::Failure,
                }
            },
            RuntimeNode::Cooldown {
                child,
                seconds,
                last_execution,
                ..
            } => {
                let now = std::time::Instant::now();
                if let Some(last) = last_execution {
                    if now.duration_since(*last).as_secs_f32() < *seconds {
                        return BtStatus::Failure;
                    }
                }
                self.execute_node(child, entity, world)
            }
            RuntimeNode::Condition { condition, .. } => {
                condition.evaluate(entity, world, &self.blackboard)
            }
            RuntimeNode::Action { action, .. } => {
                action.execute(entity, world, &mut self.blackboard)
            }
        };

        self.current_path.pop();
        result
    }

    /// Reset the behavior tree runner
    pub fn reset(&mut self) {
        self.current_path.clear();
        self.running_node = None;
        self.blackboard.clear();
    }
}

/// Policy for parallel node execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParallelPolicy {
    /// All children must meet the condition
    RequireAll,
    /// At least one child must meet the condition
    RequireOne,
}

/// Compiled behavior tree ready for runtime execution
#[derive(Debug, Clone)]
pub struct CompiledBehaviorTree {
    /// Root node of the tree
    pub root: RuntimeNode,
    /// Blackboard keys used by this tree
    pub blackboard_keys: Vec<String>,
}

impl CompiledBehaviorTree {
    /// Create a new compiled behavior tree
    pub fn new(root: RuntimeNode) -> Self {
        Self {
            root,
            blackboard_keys: Vec::new(),
        }
    }

    /// Create a new tree with specified blackboard keys
    pub fn with_keys(root: RuntimeNode, keys: Vec<String>) -> Self {
        Self {
            root,
            blackboard_keys: keys,
        }
    }
}

/// Runtime node representation
#[derive(Debug, Clone)]
pub enum RuntimeNode {
    /// Selector - tries children until one succeeds
    Selector {
        id: NodeId,
        children: Vec<RuntimeNode>,
    },
    /// Sequence - executes children in order until one fails
    Sequence {
        id: NodeId,
        children: Vec<RuntimeNode>,
    },
    /// Parallel - executes all children simultaneously
    Parallel {
        id: NodeId,
        children: Vec<RuntimeNode>,
        success_policy: ParallelPolicy,
        failure_policy: ParallelPolicy,
    },
    /// Inverter - inverts child result
    Inverter { id: NodeId, child: Box<RuntimeNode> },
    /// Repeater - repeats child N times or forever
    Repeater {
        id: NodeId,
        child: Box<RuntimeNode>,
        count: Option<u32>,
        current: u32,
    },
    /// UntilSuccess - repeats until child succeeds
    UntilSuccess { id: NodeId, child: Box<RuntimeNode> },
    /// UntilFailure - repeats until child fails
    UntilFailure { id: NodeId, child: Box<RuntimeNode> },
    /// Cooldown - prevents execution for N seconds
    Cooldown {
        id: NodeId,
        child: Box<RuntimeNode>,
        seconds: f32,
        last_execution: Option<std::time::Instant>,
    },
    /// Condition - evaluates a condition
    Condition {
        id: NodeId,
        condition: Box<dyn Condition>,
    },
    /// Action - performs an action
    Action { id: NodeId, action: Box<dyn Action> },
}

impl RuntimeNode {
    /// Get the node ID
    pub fn id(&self) -> NodeId {
        match self {
            Self::Selector { id, .. } => *id,
            Self::Sequence { id, .. } => *id,
            Self::Parallel { id, .. } => *id,
            Self::Inverter { id, .. } => *id,
            Self::Repeater { id, .. } => *id,
            Self::UntilSuccess { id, .. } => *id,
            Self::UntilFailure { id, .. } => *id,
            Self::Cooldown { id, .. } => *id,
            Self::Condition { id, .. } => *id,
            Self::Action { id, .. } => *id,
        }
    }
}

/// Trait for condition nodes
pub trait Condition: std::fmt::Debug + Send + Sync {
    /// Evaluate the condition
    fn evaluate(&self, entity: Entity, world: &World, blackboard: &Blackboard) -> BtStatus;

    /// Clone into a boxed trait object
    fn box_clone(&self) -> Box<dyn Condition>;
}

impl Clone for Box<dyn Condition> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

/// Trait for action nodes
pub trait Action: std::fmt::Debug + Send + Sync {
    /// Execute the action
    fn execute(&self, entity: Entity, world: &mut World, blackboard: &mut Blackboard) -> BtStatus;

    /// Clone into a boxed trait object
    fn box_clone(&self) -> Box<dyn Action>;
}

impl Clone for Box<dyn Action> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

/// Blackboard for sharing data between nodes
#[derive(Debug, Clone, Default)]
pub struct Blackboard {
    values: HashMap<String, BlackboardValue>,
}

impl Blackboard {
    /// Create a new empty blackboard
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    /// Set a value in the blackboard
    pub fn set<T: Into<BlackboardValue>>(&mut self, key: impl Into<String>, value: T) {
        self.values.insert(key.into(), value.into());
    }

    /// Get a value from the blackboard
    pub fn get<T>(&self, key: &str) -> Option<T>
    where
        T: TryFrom<BlackboardValue>,
    {
        self.values
            .get(key)
            .cloned()
            .and_then(|v| T::try_from(v).ok())
    }

    /// Check if a key exists
    pub fn has(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Remove a key
    pub fn remove(&mut self, key: &str) -> Option<BlackboardValue> {
        self.values.remove(key)
    }

    /// Clear all values
    pub fn clear(&mut self) {
        self.values.clear();
    }

    /// Get all keys
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.values.keys()
    }
}

/// Value types that can be stored in the blackboard
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlackboardValue {
    Bool(bool),
    Int(i32),
    Float(f32),
    Vec3(SerializableVec3),
    Entity(u64), // Store as u64 for serialization
    String(String),
}

impl From<bool> for BlackboardValue {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl From<i32> for BlackboardValue {
    fn from(v: i32) -> Self {
        Self::Int(v)
    }
}

impl From<u32> for BlackboardValue {
    fn from(v: u32) -> Self {
        Self::Int(v as i32)
    }
}

impl From<f32> for BlackboardValue {
    fn from(v: f32) -> Self {
        Self::Float(v)
    }
}

impl From<Vec3> for BlackboardValue {
    fn from(v: Vec3) -> Self {
        Self::Vec3(v.into())
    }
}

impl From<BlackboardValue> for Vec3 {
    fn from(v: BlackboardValue) -> Self {
        match v {
            BlackboardValue::Vec3(v) => v.into(),
            _ => Self::ZERO,
        }
    }
}

impl From<Entity> for BlackboardValue {
    fn from(v: Entity) -> Self {
        Self::Entity(v.to_bits().get())
    }
}

impl From<String> for BlackboardValue {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<&str> for BlackboardValue {
    fn from(v: &str) -> Self {
        Self::String(v.to_string())
    }
}

impl TryFrom<BlackboardValue> for bool {
    type Error = ();
    fn try_from(v: BlackboardValue) -> Result<Self, Self::Error> {
        match v {
            BlackboardValue::Bool(b) => Ok(b),
            _ => Err(()),
        }
    }
}

impl From<BlackboardValue> for Option<bool> {
    fn from(v: BlackboardValue) -> Self {
        v.try_into().ok()
    }
}

impl TryFrom<BlackboardValue> for i32 {
    type Error = ();
    fn try_from(v: BlackboardValue) -> Result<Self, Self::Error> {
        match v {
            BlackboardValue::Int(i) => Ok(i),
            BlackboardValue::Float(f) => Ok(f as i32),
            _ => Err(()),
        }
    }
}

impl From<BlackboardValue> for Option<i32> {
    fn from(v: BlackboardValue) -> Self {
        v.try_into().ok()
    }
}

impl TryFrom<BlackboardValue> for u32 {
    type Error = ();
    fn try_from(v: BlackboardValue) -> Result<Self, Self::Error> {
        match v {
            BlackboardValue::Int(i) if i >= 0 => Ok(i as u32),
            _ => Err(()),
        }
    }
}

impl From<BlackboardValue> for Option<u32> {
    fn from(v: BlackboardValue) -> Self {
        v.try_into().ok()
    }
}

impl TryFrom<BlackboardValue> for f32 {
    type Error = ();
    fn try_from(v: BlackboardValue) -> Result<Self, Self::Error> {
        match v {
            BlackboardValue::Float(f) => Ok(f),
            BlackboardValue::Int(i) => Ok(i as f32),
            _ => Err(()),
        }
    }
}

impl From<BlackboardValue> for Option<f32> {
    fn from(v: BlackboardValue) -> Self {
        v.try_into().ok()
    }
}

impl From<BlackboardValue> for Option<Vec3> {
    fn from(v: BlackboardValue) -> Self {
        Some(v.into())
    }
}

impl TryFrom<BlackboardValue> for Entity {
    type Error = ();
    fn try_from(v: BlackboardValue) -> Result<Self, Self::Error> {
        match v {
            BlackboardValue::Entity(e) => Entity::from_bits(e).ok_or(()),
            _ => Err(()),
        }
    }
}

impl From<BlackboardValue> for Option<Entity> {
    fn from(v: BlackboardValue) -> Self {
        v.try_into().ok()
    }
}

impl TryFrom<BlackboardValue> for String {
    type Error = ();
    fn try_from(v: BlackboardValue) -> Result<Self, Self::Error> {
        match v {
            BlackboardValue::String(s) => Ok(s),
            _ => Err(()),
        }
    }
}

impl From<BlackboardValue> for Option<String> {
    fn from(v: BlackboardValue) -> Self {
        v.try_into().ok()
    }
}

/// Component to attach a behavior tree to an entity
#[derive(Debug, Clone)]
pub struct BehaviorTreeComponent {
    /// The compiled behavior tree
    pub tree: CompiledBehaviorTree,
    /// The runner instance
    pub runner: BehaviorTreeRunner,
    /// Whether the tree is currently active
    pub active: bool,
}

impl BehaviorTreeComponent {
    /// Create a new behavior tree component
    pub fn new(tree: CompiledBehaviorTree) -> Self {
        let runner = BehaviorTreeRunner::new(tree.clone());
        Self {
            tree,
            runner,
            active: true,
        }
    }

    /// Tick the behavior tree
    pub fn tick(&mut self, entity: Entity, world: &mut World) -> BtStatus {
        if self.active {
            self.runner.tick(entity, world)
        } else {
            BtStatus::Failure
        }
    }

    /// Enable the behavior tree
    pub fn enable(&mut self) {
        self.active = true;
    }

    /// Disable the behavior tree
    pub fn disable(&mut self) {
        self.active = false;
    }

    /// Reset the behavior tree
    pub fn reset(&mut self) {
        self.runner.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_generation() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_blackboard_operations() {
        let mut bb = Blackboard::new();

        bb.set("health", 100i32);
        bb.set("position", Vec3::new(1.0, 2.0, 3.0));
        bb.set("alive", true);

        assert_eq!(bb.get::<i32>("health"), Some(100));
        assert_eq!(bb.get::<Vec3>("position"), Some(Vec3::new(1.0, 2.0, 3.0)));
        assert_eq!(bb.get::<bool>("alive"), Some(true));
        assert_eq!(bb.get::<i32>("missing"), None);
    }

    #[test]
    fn test_bt_status_equality() {
        assert_eq!(BtStatus::Success, BtStatus::Success);
        assert_eq!(BtStatus::Failure, BtStatus::Failure);
        assert_eq!(BtStatus::Running, BtStatus::Running);
        assert_ne!(BtStatus::Success, BtStatus::Failure);
    }
}
