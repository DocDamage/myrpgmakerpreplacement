//! Behavior tree node types
//!
//! This module defines all node types for the visual behavior tree editor,
//! including composites, decorators, conditions, and actions.

use dde_core::ai::NodeId;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Behavior tree node types (composite, decorator, leaf)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BtNodeType {
    // Composites (control flow)
    /// Try children until one succeeds
    Selector {
        children: Vec<BtNode>,
    },
    /// Execute children in order until one fails
    Sequence {
        children: Vec<BtNode>,
    },
    /// Execute all children simultaneously
    Parallel {
        children: Vec<BtNode>,
        success_policy: ParallelPolicy,
        failure_policy: ParallelPolicy,
    },

    // Decorators (modify child behavior)
    /// Invert child result
    Inverter {
        child: Box<BtNode>,
    },
    /// Repeat child N times or forever
    Repeater {
        child: Box<BtNode>,
        count: Option<u32>, // None = forever
    },
    /// Repeat until child succeeds
    UntilSuccess {
        child: Box<BtNode>,
    },
    /// Repeat until child fails
    UntilFailure {
        child: Box<BtNode>,
    },
    /// Prevent execution for N seconds
    Cooldown {
        child: Box<BtNode>,
        seconds: f32,
    },

    // Conditions (return Success or Failure)
    IsPlayerNearby {
        radius: f32,
    },
    HasLineOfSight {
        target: Target,
    },
    HealthBelow {
        percent: f32,
    },
    InCombat,
    TimeOfDay {
        min: u8,
        max: u8,
    },
    HasItem {
        item_id: u32,
    },
    QuestActive {
        quest_id: u32,
    },
    RandomChance {
        percent: u8,
    },
    CustomCondition {
        script: String,
    },

    // Actions (perform game action)
    MoveTo {
        target: MoveTarget, // Position, Entity, or PatrolPoint
        speed: MoveSpeed,
    },
    Follow {
        target: Target,
        distance: f32,
    },
    Attack {
        target: Target,
    },
    UseSkill {
        skill_id: u32,
        target: Target,
    },
    UseItem {
        item_id: u32,
    },
    Flee,
    Wait {
        seconds: f32,
    },
    PlayAnimation {
        anim_id: u32,
    },
    Speak {
        dialogue_id: u32,
    },
    SetVariable {
        name: String,
        value: VariableValue,
    },
    CustomAction {
        script: String,
    },
}

impl Default for BtNodeType {
    fn default() -> Self {
        Self::Sequence {
            children: Vec::new(),
        }
    }
}

impl BtNodeType {
    /// Get the display name for this node type
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Selector { .. } => "Selector",
            Self::Sequence { .. } => "Sequence",
            Self::Parallel { .. } => "Parallel",
            Self::Inverter { .. } => "Inverter",
            Self::Repeater { .. } => "Repeater",
            Self::UntilSuccess { .. } => "Until Success",
            Self::UntilFailure { .. } => "Until Failure",
            Self::Cooldown { .. } => "Cooldown",
            Self::IsPlayerNearby { .. } => "Is Player Nearby?",
            Self::HasLineOfSight { .. } => "Has Line of Sight?",
            Self::HealthBelow { .. } => "Health Below?",
            Self::InCombat => "In Combat?",
            Self::TimeOfDay { .. } => "Time of Day?",
            Self::HasItem { .. } => "Has Item?",
            Self::QuestActive { .. } => "Quest Active?",
            Self::RandomChance { .. } => "Random Chance?",
            Self::CustomCondition { .. } => "Custom Condition",
            Self::MoveTo { .. } => "Move To",
            Self::Follow { .. } => "Follow",
            Self::Attack { .. } => "Attack",
            Self::UseSkill { .. } => "Use Skill",
            Self::UseItem { .. } => "Use Item",
            Self::Flee => "Flee",
            Self::Wait { .. } => "Wait",
            Self::PlayAnimation { .. } => "Play Animation",
            Self::Speak { .. } => "Speak",
            Self::SetVariable { .. } => "Set Variable",
            Self::CustomAction { .. } => "Custom Action",
        }
    }

    /// Get the category for this node type
    pub fn category(&self) -> NodeCategory {
        match self {
            Self::Selector { .. } | Self::Sequence { .. } | Self::Parallel { .. } => {
                NodeCategory::Composite
            }
            Self::Inverter { .. }
            | Self::Repeater { .. }
            | Self::UntilSuccess { .. }
            | Self::UntilFailure { .. }
            | Self::Cooldown { .. } => NodeCategory::Decorator,
            Self::IsPlayerNearby { .. }
            | Self::HasLineOfSight { .. }
            | Self::HealthBelow { .. }
            | Self::InCombat
            | Self::TimeOfDay { .. }
            | Self::HasItem { .. }
            | Self::QuestActive { .. }
            | Self::RandomChance { .. }
            | Self::CustomCondition { .. } => NodeCategory::Condition,
            Self::MoveTo { .. }
            | Self::Follow { .. }
            | Self::Attack { .. }
            | Self::UseSkill { .. }
            | Self::UseItem { .. }
            | Self::Flee
            | Self::Wait { .. }
            | Self::PlayAnimation { .. }
            | Self::Speak { .. }
            | Self::SetVariable { .. }
            | Self::CustomAction { .. } => NodeCategory::Action,
        }
    }

    /// Get the color for this node type (for visual editor)
    pub fn color(&self) -> [u8; 3] {
        match self.category() {
            NodeCategory::Composite => [100, 149, 237], // Cornflower blue
            NodeCategory::Decorator => [255, 165, 0],   // Orange
            NodeCategory::Condition => [144, 238, 144], // Light green
            NodeCategory::Action => [255, 182, 193],    // Light pink
        }
    }

    /// Get the icon character for this node type
    pub fn icon(&self) -> char {
        match self {
            Self::Selector { .. } => '?',
            Self::Sequence { .. } => '→',
            Self::Parallel { .. } => '∥',
            Self::Inverter { .. } => '!',
            Self::Repeater { .. } => '↻',
            Self::UntilSuccess { .. } => '✓',
            Self::UntilFailure { .. } => '✗',
            Self::Cooldown { .. } => '⏱',
            Self::IsPlayerNearby { .. } => '👁',
            Self::HasLineOfSight { .. } => '👀',
            Self::HealthBelow { .. } => '♥',
            Self::InCombat => '⚔',
            Self::TimeOfDay { .. } => '☀',
            Self::HasItem { .. } => '🎒',
            Self::QuestActive { .. } => '📜',
            Self::RandomChance { .. } => '🎲',
            Self::CustomCondition { .. } => '🔍',
            Self::MoveTo { .. } => '🚶',
            Self::Follow { .. } => '→',
            Self::Attack { .. } => '⚔',
            Self::UseSkill { .. } => '✨',
            Self::UseItem { .. } => '📦',
            Self::Flee => '🏃',
            Self::Wait { .. } => '⏸',
            Self::PlayAnimation { .. } => '🎬',
            Self::Speak { .. } => '💬',
            Self::SetVariable { .. } => '⚙',
            Self::CustomAction { .. } => '🔧',
        }
    }

    /// Check if this node type can have children
    pub fn can_have_children(&self) -> bool {
        matches!(
            self,
            Self::Selector { .. } | Self::Sequence { .. } | Self::Parallel { .. }
        )
    }

    /// Check if this node type wraps a single child
    pub fn has_single_child(&self) -> bool {
        matches!(
            self,
            Self::Inverter { .. }
                | Self::Repeater { .. }
                | Self::UntilSuccess { .. }
                | Self::UntilFailure { .. }
                | Self::Cooldown { .. }
        )
    }

    /// Get children if this node has them
    pub fn children(&self) -> Option<&Vec<BtNode>> {
        match self {
            Self::Selector { children } | Self::Sequence { children } | Self::Parallel { children, .. } => {
                Some(children)
            }
            _ => None,
        }
    }

    /// Get mutable children if this node has them
    pub fn children_mut(&mut self) -> Option<&mut Vec<BtNode>> {
        match self {
            Self::Selector { children } | Self::Sequence { children } | Self::Parallel { children, .. } => {
                Some(children)
            }
            _ => None,
        }
    }

    /// Get child if this node has a single child
    pub fn child(&self) -> Option<&BtNode> {
        match self {
            Self::Inverter { child }
            | Self::Repeater { child, .. }
            | Self::UntilSuccess { child }
            | Self::UntilFailure { child }
            | Self::Cooldown { child, .. } => Some(child),
            _ => None,
        }
    }

    /// Get mutable child if this node has a single child
    pub fn child_mut(&mut self) -> Option<&mut BtNode> {
        match self {
            Self::Inverter { child }
            | Self::Repeater { child, .. }
            | Self::UntilSuccess { child }
            | Self::UntilFailure { child }
            | Self::Cooldown { child, .. } => Some(child),
            _ => None,
        }
    }
}

/// Node categories for organizing the palette
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeCategory {
    Composite,
    Decorator,
    Condition,
    Action,
}

impl NodeCategory {
    /// Get all node categories
    pub fn all() -> &'static [NodeCategory] {
        &[
            NodeCategory::Composite,
            NodeCategory::Decorator,
            NodeCategory::Condition,
            NodeCategory::Action,
        ]
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Composite => "Composites",
            Self::Decorator => "Decorators",
            Self::Condition => "Conditions",
            Self::Action => "Actions",
        }
    }

    /// Get all node templates for this category
    pub fn templates(&self) -> Vec<BtNodeType> {
        match self {
            Self::Composite => vec![
                BtNodeType::Selector { children: Vec::new() },
                BtNodeType::Sequence { children: Vec::new() },
                BtNodeType::Parallel {
                    children: Vec::new(),
                    success_policy: ParallelPolicy::RequireAll,
                    failure_policy: ParallelPolicy::RequireOne,
                },
            ],
            Self::Decorator => vec![
                BtNodeType::Inverter {
                    child: Box::new(BtNode::default()),
                },
                BtNodeType::Repeater {
                    child: Box::new(BtNode::default()),
                    count: Some(3),
                },
                BtNodeType::UntilSuccess {
                    child: Box::new(BtNode::default()),
                },
                BtNodeType::UntilFailure {
                    child: Box::new(BtNode::default()),
                },
                BtNodeType::Cooldown {
                    child: Box::new(BtNode::default()),
                    seconds: 5.0,
                },
            ],
            Self::Condition => vec![
                BtNodeType::IsPlayerNearby { radius: 10.0 },
                BtNodeType::HasLineOfSight { target: Target::Player },
                BtNodeType::HealthBelow { percent: 0.25 },
                BtNodeType::InCombat,
                BtNodeType::TimeOfDay { min: 6, max: 18 },
                BtNodeType::HasItem { item_id: 0 },
                BtNodeType::QuestActive { quest_id: 0 },
                BtNodeType::RandomChance { percent: 50 },
                BtNodeType::CustomCondition {
                    script: String::new(),
                },
            ],
            Self::Action => vec![
                BtNodeType::MoveTo {
                    target: MoveTarget::Player,
                    speed: MoveSpeed::Walk,
                },
                BtNodeType::Follow {
                    target: Target::Player,
                    distance: 3.0,
                },
                BtNodeType::Attack { target: Target::Player },
                BtNodeType::UseSkill {
                    skill_id: 0,
                    target: Target::Player,
                },
                BtNodeType::UseItem { item_id: 0 },
                BtNodeType::Flee,
                BtNodeType::Wait { seconds: 1.0 },
                BtNodeType::PlayAnimation { anim_id: 0 },
                BtNodeType::Speak { dialogue_id: 0 },
                BtNodeType::SetVariable {
                    name: String::new(),
                    value: VariableValue::Bool(true),
                },
                BtNodeType::CustomAction {
                    script: String::new(),
                },
            ],
        }
    }
}

/// Move target options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MoveTarget {
    Position(MoveTargetPosition),
    Entity(dde_core::Entity),
    PatrolPoint(usize),
    Player,
}

/// Serializable position wrapper
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveTargetPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl From<Vec3> for MoveTargetPosition {
    fn from(v: Vec3) -> Self {
        Self { x: v.x, y: v.y, z: v.z }
    }
}

impl From<MoveTargetPosition> for Vec3 {
    fn from(p: MoveTargetPosition) -> Self {
        Vec3::new(p.x, p.y, p.z)
    }
}

impl serde::Serialize for MoveTargetPosition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        [self.x, self.y, self.z].serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for MoveTargetPosition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let arr: [f32; 3] = serde::Deserialize::deserialize(deserializer)?;
        Ok(Self { x: arr[0], y: arr[1], z: arr[2] })
    }
}

/// Target options for actions/conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    Player,
    SelfEntity,
    NearestEnemy,
    NearestAlly,
    Entity(dde_core::Entity),
}

impl Default for Target {
    fn default() -> Self {
        Self::Player
    }
}

/// Movement speed options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MoveSpeed {
    #[default]
    Walk,
    Run,
    Sprint,
}

impl MoveSpeed {
    /// Get speed multiplier
    pub fn multiplier(&self) -> f32 {
        match self {
            Self::Walk => 1.0,
            Self::Run => 2.0,
            Self::Sprint => 3.0,
        }
    }
}

/// Parallel execution policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ParallelPolicy {
    /// All must meet condition
    #[default]
    RequireAll,
    /// One must meet condition
    RequireOne,
}

/// Variable value types for SetVariable action
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VariableValue {
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
    Entity(dde_core::Entity),
}

/// Behavior tree node
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BtNode {
    pub id: NodeId,
    pub node_type: BtNodeType,
    pub position: [f32; 2], // For editor layout
    pub comment: Option<String>,
}

impl Default for BtNode {
    fn default() -> Self {
        Self {
            id: NodeId::new(),
            node_type: BtNodeType::default(),
            position: [0.0, 0.0],
            comment: None,
        }
    }
}

impl BtNode {
    /// Create a new node
    pub fn new(node_type: BtNodeType, position: [f32; 2]) -> Self {
        Self {
            id: NodeId::new(),
            node_type,
            position,
            comment: None,
        }
    }

    /// Create a new node with comment
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Get the display name
    pub fn display_name(&self) -> &'static str {
        self.node_type.display_name()
    }

    /// Get the color
    pub fn color(&self) -> [u8; 3] {
        self.node_type.color()
    }

    /// Get the icon
    pub fn icon(&self) -> char {
        self.node_type.icon()
    }

    /// Check if can have children
    pub fn can_have_children(&self) -> bool {
        self.node_type.can_have_children()
    }

    /// Get children
    pub fn children(&self) -> Option<&Vec<BtNode>> {
        self.node_type.children()
    }

    /// Get mutable children
    pub fn children_mut(&mut self) -> Option<&mut Vec<BtNode>> {
        self.node_type.children_mut()
    }

    /// Find a node by ID (recursive)
    pub fn find_node(&self, id: NodeId) -> Option<&BtNode> {
        if self.id == id {
            return Some(self);
        }
        
        if let Some(children) = self.children() {
            for child in children {
                if let Some(found) = child.find_node(id) {
                    return Some(found);
                }
            }
        }
        
        if let Some(child) = self.node_type.child() {
            if let Some(found) = child.find_node(id) {
                return Some(found);
            }
        }
        
        None
    }

    /// Find a node by ID mutable (recursive)
    pub fn find_node_mut(&mut self, id: NodeId) -> Option<&mut BtNode> {
        if self.id == id {
            return Some(self);
        }
        
        // Use a raw pointer approach to avoid borrow checker issues
        // First, find which path to take without keeping borrows
        enum ChildLocation {
            Multiple(*mut Vec<BtNode>),
            Single(*mut BtNode),
            None,
        }
        
        let location = match &mut self.node_type {
            BtNodeType::Selector { children } 
            | BtNodeType::Sequence { children } 
            | BtNodeType::Parallel { children, .. } => {
                ChildLocation::Multiple(children as *mut _)
            }
            BtNodeType::Inverter { child }
            | BtNodeType::Repeater { child, .. }
            | BtNodeType::UntilSuccess { child }
            | BtNodeType::UntilFailure { child }
            | BtNodeType::Cooldown { child, .. } => {
                ChildLocation::Single(child.as_mut() as *mut _)
            }
            _ => ChildLocation::None,
        };
        
        // Now search without any active borrows of self.node_type
        match location {
            ChildLocation::Multiple(children_ptr) => {
                let children = unsafe { &mut *children_ptr };
                for child in children.iter_mut() {
                    if let Some(found) = child.find_node_mut(id) {
                        return Some(found);
                    }
                }
            }
            ChildLocation::Single(child_ptr) => {
                let child = unsafe { &mut *child_ptr };
                if let Some(found) = child.find_node_mut(id) {
                    return Some(found);
                }
            }
            ChildLocation::None => {}
        }
        
        None
    }

    /// Get all node IDs in this subtree
    pub fn collect_ids(&self, ids: &mut Vec<NodeId>) {
        ids.push(self.id);
        
        if let Some(children) = self.children() {
            for child in children {
                child.collect_ids(ids);
            }
        }
        
        if let Some(child) = self.node_type.child() {
            child.collect_ids(ids);
        }
    }

    /// Remove a child node
    pub fn remove_child(&mut self, child_id: NodeId) -> bool {
        if let Some(children) = self.children_mut() {
            if let Some(pos) = children.iter().position(|c| c.id == child_id) {
                children.remove(pos);
                return true;
            }
            
            // Try recursively
            for child in children.iter_mut() {
                if child.remove_child(child_id) {
                    return true;
                }
            }
        }
        
        false
    }

    /// Add a child node
    pub fn add_child(&mut self, child: BtNode) -> Result<(), BtNodeError> {
        if let Some(children) = self.children_mut() {
            children.push(child);
            Ok(())
        } else if self.node_type.has_single_child() {
            // For decorators, replace the child
            match &mut self.node_type {
                BtNodeType::Inverter { child: c }
                | BtNodeType::Repeater { child: c, .. }
                | BtNodeType::UntilSuccess { child: c }
                | BtNodeType::UntilFailure { child: c }
                | BtNodeType::Cooldown { child: c, .. } => {
                    *c = Box::new(child);
                    Ok(())
                }
                _ => Err(BtNodeError::CannotHaveChildren),
            }
        } else {
            Err(BtNodeError::CannotHaveChildren)
        }
    }
}

/// Errors that can occur when working with behavior tree nodes
#[derive(Debug, Clone, thiserror::Error)]
pub enum BtNodeError {
    #[error("Node cannot have children")]
    CannotHaveChildren,
    
    #[error("Node not found: {0:?}")]
    NodeNotFound(NodeId),
    
    #[error("Invalid node connection")]
    InvalidConnection,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_categories() {
        let selector = BtNodeType::Selector { children: Vec::new() };
        assert_eq!(selector.category(), NodeCategory::Composite);
        
        let inverter = BtNodeType::Inverter {
            child: Box::new(BtNode::default()),
        };
        assert_eq!(inverter.category(), NodeCategory::Decorator);
        
        let condition = BtNodeType::InCombat;
        assert_eq!(condition.category(), NodeCategory::Condition);
        
        let action = BtNodeType::Flee;
        assert_eq!(action.category(), NodeCategory::Action);
    }

    #[test]
    fn test_node_creation() {
        let node = BtNode::new(BtNodeType::InCombat, [100.0, 200.0]);
        assert_eq!(node.position, [100.0, 200.0]);
        assert_eq!(node.node_type.category(), NodeCategory::Condition);
    }

    #[test]
    fn test_node_find() {
        let mut root = BtNode::new(
            BtNodeType::Sequence { children: Vec::new() },
            [0.0, 0.0],
        );
        
        let child1 = BtNode::new(BtNodeType::InCombat, [100.0, 0.0]);
        let child1_id = child1.id;
        let child2 = BtNode::new(BtNodeType::Flee, [200.0, 0.0]);
        let child2_id = child2.id;
        
        root.add_child(child1).unwrap();
        root.add_child(child2).unwrap();
        
        assert!(root.find_node(child1_id).is_some());
        assert!(root.find_node(child2_id).is_some());
        assert!(root.find_node(NodeId::new()).is_none());
    }

    #[test]
    fn test_collect_ids() {
        let mut root = BtNode::new(
            BtNodeType::Sequence { children: Vec::new() },
            [0.0, 0.0],
        );
        
        root.add_child(BtNode::new(BtNodeType::InCombat, [100.0, 0.0])).unwrap();
        root.add_child(BtNode::new(BtNodeType::Flee, [200.0, 0.0])).unwrap();
        
        let mut ids = Vec::new();
        root.collect_ids(&mut ids);
        assert_eq!(ids.len(), 3);
    }
}
