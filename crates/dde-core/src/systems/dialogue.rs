//! Dialogue System - Manages dialogue trees and NPC conversations
//!
//! This module provides a complete dialogue tree system with:
//! - Multiple node types (NPC text, player choices, conditions, actions, branches)
//! - Variable interpolation and condition evaluation
//! - Audio/visual metadata support
//! - Serialization for editor integration
//!
//! ## Example
//!
//! ```rust,ignore
//! use dde_core::systems::dialogue::{DialogueTree, DialogueNode, DialogueNodeType};
//!
//! let mut tree = DialogueTree::new("greeting", "NPC Greeting");
//!
//! // Add a text node
//! tree.add_node(DialogueNode::new_text("hello", "Hello there!", "Shopkeeper"));
//!
//! // Add a choice node
//! tree.add_node(DialogueNode::new_choice("choices", vec![
//!     ("buy", "I'd like to buy something", Some("shop")),
//!     ("sell", "I have items to sell", Some("sell_menu")),
//!     ("leave", "Goodbye", None),
//! ]));
//! ```

use std::collections::HashMap;

use crate::Entity;
use serde::{Deserialize, Serialize};

/// Unique identifier for dialogue nodes
pub type NodeId = String;

/// Dialogue node types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogueNodeType {
    /// NPC speaks text
    NpcText,
    /// Player makes a choice
    PlayerChoice,
    /// Condition check (branches based on game state)
    Condition,
    /// Action/effect (modifies game state)
    Action,
    /// Branch node (random or sequential selection)
    Branch {
        mode: BranchMode,
    },
    /// End of dialogue
    End,
}

/// Branch selection mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchMode {
    /// Select randomly with equal probability
    Random,
    /// Select based on weights
    Weighted,
    /// Select sequentially (round-robin)
    Sequential,
    /// Select first valid path
    FirstValid,
}

impl Default for BranchMode {
    fn default() -> Self {
        BranchMode::FirstValid
    }
}

/// Condition operator for comparisons
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionOp {
    /// Equal
    Eq,
    /// Not equal
    Neq,
    /// Greater than
    Gt,
    /// Greater than or equal
    Gte,
    /// Less than
    Lt,
    /// Less than or equal
    Lte,
    /// Contains (for strings/arrays)
    Contains,
    /// Starts with (for strings)
    StartsWith,
}

impl Default for ConditionOp {
    fn default() -> Self {
        ConditionOp::Eq
    }
}

/// A single dialogue condition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueCondition {
    /// Variable name or key to check
    pub variable: String,
    /// Comparison operator
    pub operator: ConditionOp,
    /// Value to compare against
    pub value: serde_json::Value,
    /// Whether to negate the condition
    pub negate: bool,
}

impl DialogueCondition {
    /// Create a new condition
    pub fn new(
        variable: impl Into<String>,
        operator: ConditionOp,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        Self {
            variable: variable.into(),
            operator,
            value: value.into(),
            negate: false,
        }
    }

    /// Create an equals condition
    pub fn eq(variable: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        Self::new(variable, ConditionOp::Eq, value)
    }

    /// Create a greater-than condition
    pub fn gt(variable: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        Self::new(variable, ConditionOp::Gt, value)
    }

    /// Negate this condition
    pub fn negate(mut self) -> Self {
        self.negate = true;
        self
    }

    /// Evaluate the condition against a variable store
    pub fn evaluate(&self, variables: &HashMap<String, serde_json::Value>) -> bool {
        let var_value = variables.get(&self.variable);
        let result = match &self.operator {
            ConditionOp::Eq => Self::compare_eq(var_value, &self.value),
            ConditionOp::Neq => !Self::compare_eq(var_value, &self.value),
            ConditionOp::Gt => Self::compare_gt(var_value, &self.value),
            ConditionOp::Gte => {
                Self::compare_gt(var_value, &self.value) || Self::compare_eq(var_value, &self.value)
            }
            ConditionOp::Lt => Self::compare_lt(var_value, &self.value),
            ConditionOp::Lte => {
                Self::compare_lt(var_value, &self.value) || Self::compare_eq(var_value, &self.value)
            }
            ConditionOp::Contains => Self::compare_contains(var_value, &self.value),
            ConditionOp::StartsWith => Self::compare_starts_with(var_value, &self.value),
        };

        if self.negate {
            !result
        } else {
            result
        }
    }

    fn compare_eq(a: Option<&serde_json::Value>, b: &serde_json::Value) -> bool {
        match a {
            Some(a) => a == b,
            None => matches!(b, serde_json::Value::Null),
        }
    }

    fn compare_gt(a: Option<&serde_json::Value>, b: &serde_json::Value) -> bool {
        match (a, b) {
            (Some(serde_json::Value::Number(a)), serde_json::Value::Number(b)) => {
                if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
                    a > b
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn compare_lt(a: Option<&serde_json::Value>, b: &serde_json::Value) -> bool {
        match (a, b) {
            (Some(serde_json::Value::Number(a)), serde_json::Value::Number(b)) => {
                if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
                    a < b
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn compare_contains(a: Option<&serde_json::Value>, b: &serde_json::Value) -> bool {
        match (a, b) {
            (Some(serde_json::Value::String(a)), serde_json::Value::String(b)) => {
                a.contains(b)
            }
            (Some(serde_json::Value::Array(arr)), b) => arr.contains(b),
            _ => false,
        }
    }

    fn compare_starts_with(a: Option<&serde_json::Value>, b: &serde_json::Value) -> bool {
        match (a, b) {
            (Some(serde_json::Value::String(a)), serde_json::Value::String(b)) => {
                a.starts_with(b)
            }
            _ => false,
        }
    }
}

impl Default for DialogueCondition {
    fn default() -> Self {
        Self {
            variable: String::new(),
            operator: ConditionOp::Eq,
            value: serde_json::Value::Null,
            negate: false,
        }
    }
}

/// Action types for dialogue effects
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    /// Set a variable/flag
    SetVariable,
    /// Add to a numeric variable
    AddToVariable,
    /// Give item to player
    GiveItem,
    /// Remove item from player
    RemoveItem,
    /// Start a quest
    StartQuest,
    /// Complete a quest objective
    CompleteObjective,
    /// Trigger an animation
    TriggerAnimation,
    /// Play a sound effect
    PlaySound,
    /// Change scene/map
    ChangeScene,
    /// Call custom script/function
    Custom,
}

/// A single dialogue effect/action
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueEffect {
    /// Type of action
    pub action_type: ActionType,
    /// Target/variable name
    pub target: String,
    /// Action parameters
    pub parameters: HashMap<String, serde_json::Value>,
}

impl DialogueEffect {
    /// Create a new effect
    pub fn new(action_type: ActionType, target: impl Into<String>) -> Self {
        Self {
            action_type,
            target: target.into(),
            parameters: HashMap::new(),
        }
    }

    /// Add a parameter
    pub fn with_param(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.parameters.insert(key.into(), value.into());
        self
    }

    /// Create a set variable effect
    pub fn set_variable(name: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        Self::new(ActionType::SetVariable, name).with_param("value", value)
    }

    /// Create a give item effect
    pub fn give_item(item_id: impl Into<String>, quantity: u32) -> Self {
        Self::new(ActionType::GiveItem, item_id).with_param("quantity", quantity)
    }
}

impl Default for DialogueEffect {
    fn default() -> Self {
        Self {
            action_type: ActionType::SetVariable,
            target: String::new(),
            parameters: HashMap::new(),
        }
    }
}

/// Connection to another node
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeConnection {
    /// Target node ID
    pub target_node: NodeId,
    /// Optional condition for this connection
    pub condition: Option<DialogueCondition>,
    /// Weight for random/weighted branches
    pub weight: f32,
    /// Connection label (shown on the line)
    pub label: Option<String>,
}

impl NodeConnection {
    /// Create a simple connection
    pub fn to(target: impl Into<NodeId>) -> Self {
        Self {
            target_node: target.into(),
            condition: None,
            weight: 1.0,
            label: None,
        }
    }

    /// Create a conditional connection
    pub fn conditional(
        target: impl Into<NodeId>,
        condition: DialogueCondition,
    ) -> Self {
        Self {
            target_node: target.into(),
            condition: Some(condition),
            weight: 1.0,
            label: None,
        }
    }

    /// Set the weight
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    /// Set the label
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl Default for NodeConnection {
    fn default() -> Self {
        Self {
            target_node: String::new(),
            condition: None,
            weight: 1.0,
            label: None,
        }
    }
}

/// Dialogue choice for player selection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueChoice {
    /// Unique ID for this choice
    pub id: String,
    /// Display text
    pub text: String,
    /// Tooltip/hover text
    pub tooltip: Option<String>,
    /// Conditions for showing this choice
    pub conditions: Vec<DialogueCondition>,
    /// Effects when selecting this choice
    pub effects: Vec<DialogueEffect>,
    /// Target node to go to
    pub next_node: Option<NodeId>,
    /// Icon for this choice
    pub icon: Option<String>,
}

impl DialogueChoice {
    /// Create a new choice
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            tooltip: None,
            conditions: Vec::new(),
            effects: Vec::new(),
            next_node: None,
            icon: None,
        }
    }

    /// Set the target node
    pub fn leads_to(mut self, node: impl Into<NodeId>) -> Self {
        self.next_node = Some(node.into());
        self
    }

    /// Add a condition
    pub fn with_condition(mut self, condition: DialogueCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Add an effect
    pub fn with_effect(mut self, effect: DialogueEffect) -> Self {
        self.effects.push(effect);
        self
    }

    /// Set tooltip
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Set icon
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

/// Node position in the editor canvas
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct NodePosition {
    pub x: f32,
    pub y: f32,
}

impl NodePosition {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Dialogue node
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialogueNode {
    /// Unique node ID
    pub id: NodeId,
    /// Node type
    pub node_type: DialogueNodeType,
    /// Speaker name (for NPC text)
    pub speaker: Option<String>,
    /// Dialogue text content
    pub text: String,
    /// Speaker portrait image path
    pub portrait: Option<String>,
    /// Portrait position
    pub portrait_position: PortraitPosition,
    /// Voiceover audio path
    pub voiceover: Option<String>,
    /// Sound effect to play
    pub sound_effect: Option<String>,
    /// Animation trigger name
    pub animation_trigger: Option<String>,
    /// Animation target (speaker, player, etc.)
    pub animation_target: Option<String>,
    /// Emotion/mood for this line
    pub emotion: String,
    /// Player choices (for choice nodes)
    pub choices: Vec<DialogueChoice>,
    /// Outgoing connections
    pub connections: Vec<NodeConnection>,
    /// Conditions for this node to be shown
    pub conditions: Vec<DialogueCondition>,
    /// Effects when this node is entered
    pub on_enter_effects: Vec<DialogueEffect>,
    /// Effects when this node is exited
    pub on_exit_effects: Vec<DialogueEffect>,
    /// Editor canvas position
    pub position: NodePosition,
    /// Editor node size (for layout)
    pub size: Option<[f32; 2]>,
    /// Comments/notes for this node
    pub comments: Option<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Portrait display position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PortraitPosition {
    /// No portrait
    #[default]
    None,
    /// Left side
    Left,
    /// Right side
    Right,
    /// Full screen background
    Fullscreen,
}

impl DialogueNode {
    /// Create a new node with basic properties
    pub fn new(id: impl Into<NodeId>, node_type: DialogueNodeType) -> Self {
        Self {
            id: id.into(),
            node_type,
            speaker: None,
            text: String::new(),
            portrait: None,
            portrait_position: PortraitPosition::None,
            voiceover: None,
            sound_effect: None,
            animation_trigger: None,
            animation_target: None,
            emotion: "neutral".to_string(),
            choices: Vec::new(),
            connections: Vec::new(),
            conditions: Vec::new(),
            on_enter_effects: Vec::new(),
            on_exit_effects: Vec::new(),
            position: NodePosition::default(),
            size: None,
            comments: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a text node
    pub fn new_text(
        id: impl Into<NodeId>,
        text: impl Into<String>,
        speaker: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            node_type: DialogueNodeType::NpcText,
            speaker: Some(speaker.into()),
            text: text.into(),
            ..Default::default()
        }
    }

    /// Create a choice node
    pub fn new_choice(id: impl Into<NodeId>, choices: Vec<DialogueChoice>) -> Self {
        Self {
            id: id.into(),
            node_type: DialogueNodeType::PlayerChoice,
            choices,
            ..Default::default()
        }
    }

    /// Create a condition node
    pub fn new_condition(id: impl Into<NodeId>, conditions: Vec<DialogueCondition>) -> Self {
        Self {
            id: id.into(),
            node_type: DialogueNodeType::Condition,
            conditions,
            ..Default::default()
        }
    }

    /// Create an action node
    pub fn new_action(id: impl Into<NodeId>, effects: Vec<DialogueEffect>) -> Self {
        Self {
            id: id.into(),
            node_type: DialogueNodeType::Action,
            on_enter_effects: effects,
            ..Default::default()
        }
    }

    /// Create a branch node
    pub fn new_branch(id: impl Into<NodeId>, mode: BranchMode) -> Self {
        Self {
            id: id.into(),
            node_type: DialogueNodeType::Branch { mode },
            ..Default::default()
        }
    }

    /// Create an end node
    pub fn new_end(id: impl Into<NodeId>) -> Self {
        Self::new(id, DialogueNodeType::End)
    }

    /// Set the speaker
    pub fn with_speaker(mut self, speaker: impl Into<String>) -> Self {
        self.speaker = Some(speaker.into());
        self
    }

    /// Set the text
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    /// Set the portrait
    pub fn with_portrait(mut self, portrait: impl Into<String>, position: PortraitPosition) -> Self {
        self.portrait = Some(portrait.into());
        self.portrait_position = position;
        self
    }

    /// Set the voiceover
    pub fn with_voiceover(mut self, voiceover: impl Into<String>) -> Self {
        self.voiceover = Some(voiceover.into());
        self
    }

    /// Set the animation trigger
    pub fn with_animation(mut self, trigger: impl Into<String>, target: impl Into<String>) -> Self {
        self.animation_trigger = Some(trigger.into());
        self.animation_target = Some(target.into());
        self
    }

    /// Set the emotion
    pub fn with_emotion(mut self, emotion: impl Into<String>) -> Self {
        self.emotion = emotion.into();
        self
    }

    /// Set the position
    pub fn at_position(mut self, x: f32, y: f32) -> Self {
        self.position = NodePosition::new(x, y);
        self
    }

    /// Add a connection
    pub fn connect_to(mut self, connection: NodeConnection) -> Self {
        self.connections.push(connection);
        self
    }

    /// Add a condition
    pub fn with_condition(mut self, condition: DialogueCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Add an enter effect
    pub fn on_enter(mut self, effect: DialogueEffect) -> Self {
        self.on_enter_effects.push(effect);
        self
    }

    /// Add an exit effect
    pub fn on_exit(mut self, effect: DialogueEffect) -> Self {
        self.on_exit_effects.push(effect);
        self
    }

    /// Add a choice
    pub fn with_choice(mut self, choice: DialogueChoice) -> Self {
        self.choices.push(choice);
        self
    }

    /// Get display name for this node
    pub fn display_name(&self) -> String {
        match &self.node_type {
            DialogueNodeType::NpcText => {
                let speaker = self.speaker.as_deref().unwrap_or("NPC");
                let preview = if self.text.len() > 20 {
                    format!("{}...", &self.text[..20])
                } else {
                    self.text.clone()
                };
                format!("{}: {}", speaker, preview)
            }
            DialogueNodeType::PlayerChoice => {
                format!("Choice ({} options)", self.choices.len())
            }
            DialogueNodeType::Condition => {
                if let Some(cond) = self.conditions.first() {
                    format!("If: {}", cond.variable)
                } else {
                    "Condition".to_string()
                }
            }
            DialogueNodeType::Action => {
                if let Some(effect) = self.on_enter_effects.first() {
                    format!("Action: {:?}", effect.action_type)
                } else {
                    "Action".to_string()
                }
            }
            DialogueNodeType::Branch { mode } => {
                format!("Branch ({:?})", mode)
            }
            DialogueNodeType::End => "End".to_string(),
        }
    }

    /// Get the color for this node type
    pub fn color(&self) -> [u8; 3] {
        match &self.node_type {
            DialogueNodeType::NpcText => [100, 150, 255],       // Blue
            DialogueNodeType::PlayerChoice => [100, 200, 100], // Green
            DialogueNodeType::Condition => [255, 200, 100],    // Orange
            DialogueNodeType::Action => [200, 100, 200],       // Purple
            DialogueNodeType::Branch { .. } => [255, 150, 100], // Coral
            DialogueNodeType::End => [150, 150, 150],          // Gray
        }
    }

    /// Check if all conditions are met
    pub fn check_conditions(&self, variables: &HashMap<String, serde_json::Value>) -> bool {
        self.conditions.iter().all(|c| c.evaluate(variables))
    }
}

impl Default for DialogueNode {
    fn default() -> Self {
        Self::new("", DialogueNodeType::NpcText)
    }
}

/// Dialogue tree metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DialogueMetadata {
    /// Author/creator
    pub author: Option<String>,
    /// Creation timestamp
    pub created_at: Option<String>,
    /// Last modified timestamp
    pub modified_at: Option<String>,
    /// Version number
    pub version: String,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Description
    pub description: Option<String>,
}

/// Dialogue tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueTree {
    /// Unique tree ID
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Root/start node ID
    pub root_node: NodeId,
    /// All nodes in the tree
    pub nodes: HashMap<NodeId, DialogueNode>,
    /// Tree metadata
    pub metadata: DialogueMetadata,
    /// Global variables used in this tree
    pub variables: Vec<String>,
}

impl DialogueTree {
    /// Create a new empty dialogue tree
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        let id_str: String = id.into();
        let root_id = format!("{}_start", id_str);

        let mut tree = Self {
            id: id_str,
            name: name.into(),
            root_node: root_id.clone(),
            nodes: HashMap::new(),
            metadata: DialogueMetadata {
                version: "1.0".to_string(),
                ..Default::default()
            },
            variables: Vec::new(),
        };

        // Add a default start node
        let start_node = DialogueNode::new_text(&root_id, "Hello!", "NPC")
            .at_position(0.0, 0.0);
        tree.add_node(start_node);

        tree
    }

    /// Add a node to the tree
    pub fn add_node(&mut self, node: DialogueNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: &str) -> Option<&DialogueNode> {
        self.nodes.get(node_id)
    }

    /// Get a mutable node by ID
    pub fn get_node_mut(&mut self, node_id: &str) -> Option<&mut DialogueNode> {
        self.nodes.get_mut(node_id)
    }

    /// Remove a node from the tree
    pub fn remove_node(&mut self, node_id: &str) -> Option<DialogueNode> {
        let removed = self.nodes.remove(node_id);

        // Update connections from other nodes
        if removed.is_some() {
            for node in self.nodes.values_mut() {
                node.connections.retain(|c| c.target_node != node_id);
            }

            // Update root if needed
            if self.root_node == node_id {
                self.root_node = self.nodes.keys().next().cloned().unwrap_or_default();
            }
        }

        removed
    }

    /// Set the root node
    pub fn set_root_node(&mut self, node_id: impl Into<NodeId>) {
        self.root_node = node_id.into();
    }

    /// Get all connections targeting a specific node
    pub fn get_incoming_connections(&self, node_id: &str) -> Vec<(&NodeId, &NodeConnection)> {
        let mut result = Vec::new();
        for (id, node) in &self.nodes {
            for conn in &node.connections {
                if conn.target_node == node_id {
                    result.push((id, conn));
                }
            }
        }
        result
    }

    /// Validate the tree for errors
    pub fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Check root node exists
        if !self.nodes.contains_key(&self.root_node) {
            errors.push(ValidationError::MissingRootNode);
        }

        // Check for orphaned nodes
        for node_id in self.nodes.keys() {
            if node_id == &self.root_node {
                continue;
            }

            let has_incoming = self.get_incoming_connections(node_id).len() > 0;
            if !has_incoming {
                errors.push(ValidationError::OrphanedNode(node_id.clone()));
            }
        }

        // Check for invalid connections
        for (id, node) in &self.nodes {
            for conn in &node.connections {
                if !self.nodes.contains_key(&conn.target_node) {
                    errors.push(ValidationError::InvalidConnection {
                        from: id.clone(),
                        to: conn.target_node.clone(),
                    });
                }
            }
        }

        // Check for choice nodes without choices
        for (id, node) in &self.nodes {
            if matches!(node.node_type, DialogueNodeType::PlayerChoice) && node.choices.is_empty() {
                errors.push(ValidationError::EmptyChoiceNode(id.clone()));
            }
        }

        errors
    }

    /// Create a simple linear dialogue
    pub fn simple_linear(id: impl Into<String>, lines: Vec<(&str, &str)>) -> Self {
        let id_str = id.into();
        let mut tree = Self::new(&id_str, &id_str);

        let mut prev_node_id: Option<String> = None;

        for (i, (speaker, text)) in lines.iter().enumerate() {
            let node_id = format!("node_{}", i);
            let is_last = i == lines.len() - 1;

            let node = DialogueNode {
                id: node_id.clone(),
                node_type: if is_last {
                    DialogueNodeType::End
                } else {
                    DialogueNodeType::NpcText
                },
                speaker: Some(speaker.to_string()),
                text: text.to_string(),
                choices: vec![],
                connections: if is_last {
                    vec![]
                } else {
                    vec![NodeConnection::to(format!("node_{}", i + 1))]
                },
                position: NodePosition::new(i as f32 * 250.0, 0.0),
                ..Default::default()
            };

            tree.add_node(node);

            // Link previous node
            if let Some(ref prev) = prev_node_id {
                if let Some(prev_node) = tree.nodes.get_mut(prev) {
                    if prev_node.connections.is_empty() {
                        prev_node.connections.push(NodeConnection::to(node_id.clone()));
                    }
                }
            }

            prev_node_id = Some(node_id);
        }

        // Update root
        if !tree.nodes.is_empty() {
            tree.root_node = "node_0".to_string();
        }

        tree
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Import from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Get all node positions for editor bounds calculation
    pub fn get_bounds(&self) -> Option<(f32, f32, f32, f32)> {
        if self.nodes.is_empty() {
            return None;
        }

        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for node in self.nodes.values() {
            min_x = min_x.min(node.position.x);
            max_x = max_x.max(node.position.x);
            min_y = min_y.min(node.position.y);
            max_y = max_y.max(node.position.y);
        }

        Some((min_x, max_x, min_y, max_y))
    }
}

impl Default for DialogueTree {
    fn default() -> Self {
        Self::new("default", "Default Dialogue")
    }
}

/// Validation error types
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    MissingRootNode,
    OrphanedNode(NodeId),
    InvalidConnection { from: NodeId, to: NodeId },
    EmptyChoiceNode(NodeId),
    MissingPortrait { node: NodeId, path: String },
    MissingAudio { node: NodeId, path: String },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::MissingRootNode => write!(f, "Root node is missing"),
            ValidationError::OrphanedNode(id) => write!(f, "Node '{}' has no incoming connections", id),
            ValidationError::InvalidConnection { from, to } => {
                write!(f, "Invalid connection from '{}' to '{}'", from, to)
            }
            ValidationError::EmptyChoiceNode(id) => write!(f, "Choice node '{}' has no choices", id),
            ValidationError::MissingPortrait { node, path } => {
                write!(f, "Missing portrait '{}' in node '{}'", path, node)
            }
            ValidationError::MissingAudio { node, path } => {
                write!(f, "Missing audio '{}' in node '{}'", path, node)
            }
        }
    }
}

/// Active dialogue session
#[derive(Debug)]
pub struct DialogueSession {
    /// NPC entity
    pub npc_entity: Entity,
    /// Dialogue tree
    pub tree: DialogueTree,
    /// Current node ID
    pub current_node: NodeId,
    /// Dialogue history
    pub history: Vec<DialogueEntry>,
    /// Current available choices
    pub player_choices: Vec<DialogueChoice>,
    /// Whether dialogue is complete
    pub completed: bool,
    /// Active typewriter text
    pub typewriter: Option<TypewriterText>,
    /// Variable store for conditions
    pub variables: HashMap<String, serde_json::Value>,
    /// Current speaker portrait
    pub current_portrait: Option<String>,
    /// Portrait position
    pub portrait_position: PortraitPosition,
}

/// A single entry in dialogue history
#[derive(Debug, Clone)]
pub struct DialogueEntry {
    pub speaker: String,
    pub text: String,
    pub is_player: bool,
    pub emotion: Option<String>,
}

impl DialogueSession {
    /// Start a new dialogue session
    pub fn new(npc_entity: Entity, tree: DialogueTree) -> Self {
        let root = tree.root_node.clone();
        let mut session = Self {
            npc_entity,
            tree,
            current_node: root.clone(),
            history: vec![],
            player_choices: vec![],
            completed: false,
            typewriter: None,
            variables: HashMap::new(),
            current_portrait: None,
            portrait_position: PortraitPosition::None,
        };

        // Process initial node
        session.process_current_node();

        session
    }

    /// Set a variable for condition evaluation
    pub fn set_variable(&mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) {
        self.variables.insert(key.into(), value.into());
    }

    /// Get a variable
    pub fn get_variable(&self, key: &str) -> Option<&serde_json::Value> {
        self.variables.get(key)
    }

    /// Process the current node
    fn process_current_node(&mut self) {
        let node = match self.tree.get_node(&self.current_node).cloned() {
            Some(n) => n,
            None => {
                self.completed = true;
                return;
            }
        };

        // Check conditions
        if !node.check_conditions(&self.variables) {
            // Try to find alternative path
            if let Some(conn) = node.connections.iter().find(|c| {
                c.condition
                    .as_ref()
                    .map(|cond| cond.evaluate(&self.variables))
                    .unwrap_or(true)
            }) {
                self.current_node = conn.target_node.clone();
                self.process_current_node();
                return;
            } else {
                self.completed = true;
                return;
            }
        }

        // Apply on-enter effects
        for effect in &node.on_enter_effects {
            self.apply_effect(effect);
        }

        // Update portrait
        if let Some(ref portrait) = node.portrait {
            self.current_portrait = Some(portrait.clone());
            self.portrait_position = node.portrait_position;
        }

        match &node.node_type {
            DialogueNodeType::NpcText => {
                // Add to history
                let speaker = node.speaker.clone().unwrap_or_else(|| "NPC".to_string());
                self.history.push(DialogueEntry {
                    speaker: speaker.clone(),
                    text: node.text.clone(),
                    is_player: false,
                    emotion: Some(node.emotion.clone()),
                });

                // Start typewriter effect
                self.typewriter = Some(TypewriterText::new(&node.text, 30.0));

                // Auto-advance if there's only one connection
                if node.connections.len() == 1 && node.connections[0].condition.is_none() {
                    self.current_node = node.connections[0].target_node.clone();
                    // Don't auto-process, wait for advance()
                }
            }
            DialogueNodeType::PlayerChoice => {
                // Filter available choices
                self.player_choices = node
                    .choices
                    .iter()
                    .filter(|c| c.conditions.iter().all(|cond| cond.evaluate(&self.variables)))
                    .cloned()
                    .collect();
            }
            DialogueNodeType::Condition => {
                // Find first matching connection
                for conn in &node.connections {
                    let matches = conn
                        .condition
                        .as_ref()
                        .map(|c| c.evaluate(&self.variables))
                        .unwrap_or(true);

                    if matches {
                        self.current_node = conn.target_node.clone();
                        self.process_current_node();
                        return;
                    }
                }

                // No matching path
                self.completed = true;
            }
            DialogueNodeType::Action => {
                // Apply effects and move to next
                if let Some(conn) = node.connections.first() {
                    self.current_node = conn.target_node.clone();
                    self.process_current_node();
                } else {
                    self.completed = true;
                }
            }
            DialogueNodeType::Branch { mode } => {
                let target = match mode {
                    BranchMode::FirstValid => node
                        .connections
                        .iter()
                        .find(|c| {
                            c.condition
                                .as_ref()
                                .map(|cond| cond.evaluate(&self.variables))
                                .unwrap_or(true)
                        })
                        .map(|c| c.target_node.clone()),
                    BranchMode::Random => {
                        use rand::seq::SliceRandom;
                        node.connections.choose(&mut rand::thread_rng()).map(|c| c.target_node.clone())
                    }
                    BranchMode::Weighted => {
                        use rand::distributions::WeightedIndex;
                        use rand::prelude::*;
                        
                        if !node.connections.is_empty() {
                            let weights: Vec<f32> = node.connections.iter().map(|c| c.weight).collect();
                            if let Ok(dist) = WeightedIndex::new(&weights) {
                                let mut rng = thread_rng();
                                Some(node.connections[dist.sample(&mut rng)].target_node.clone())
                            } else {
                                node.connections.first().map(|c| c.target_node.clone())
                            }
                        } else {
                            None
                        }
                    }
                    BranchMode::Sequential => node.connections.first().map(|c| c.target_node.clone()),
                };

                if let Some(target) = target {
                    self.current_node = target;
                    self.process_current_node();
                } else {
                    self.completed = true;
                }
            }
            DialogueNodeType::End => {
                self.completed = true;
            }
        }

        // Apply on-exit effects for certain node types
        if !matches!(node.node_type, DialogueNodeType::NpcText | DialogueNodeType::PlayerChoice) {
            for effect in &node.on_exit_effects {
                self.apply_effect(effect);
            }
        }
    }

    /// Apply an effect
    fn apply_effect(&mut self, effect: &DialogueEffect) {
        match effect.action_type {
            ActionType::SetVariable => {
                if let Some(value) = effect.parameters.get("value") {
                    self.variables.insert(effect.target.clone(), value.clone());
                }
            }
            ActionType::AddToVariable => {
                if let Some(serde_json::Value::Number(add)) = effect.parameters.get("value") {
                    let current = self
                        .variables
                        .get(&effect.target)
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    if let Some(add_val) = add.as_f64() {
                        self.variables.insert(
                            effect.target.clone(),
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(current + add_val)
                                    .unwrap_or_else(|| serde_json::Number::from(0)),
                            ),
                        );
                    }
                }
            }
            _ => {
                // Other effects handled by game systems
            }
        }
    }

    /// Select a choice
    pub fn select_choice(&mut self, choice_index: usize) -> bool {
        if choice_index >= self.player_choices.len() {
            return false;
        }

        let choice = self.player_choices[choice_index].clone();

        // Add player choice to history
        self.history.push(DialogueEntry {
            speaker: "Player".to_string(),
            text: choice.text.clone(),
            is_player: true,
            emotion: None,
        });

        // Apply choice effects
        for effect in &choice.effects {
            self.apply_effect(effect);
        }

        // Move to next node
        if let Some(ref next) = choice.next_node {
            self.current_node = next.clone();
        } else if let Some(ref next) = self.tree.get_node(&self.current_node).and_then(|n| {
            n.connections.first().map(|c| c.target_node.clone())
        }) {
            self.current_node = next.clone();
        } else {
            self.completed = true;
            return false;
        }

        self.player_choices.clear();
        self.process_current_node();
        true
    }

    /// Advance to the next node (for simple dialogues)
    pub fn advance(&mut self) -> bool {
        // Skip typewriter if active
        if let Some(ref mut tw) = self.typewriter {
            if !tw.is_complete() {
                tw.skip();
                return true;
            }
        }

        if !self.player_choices.is_empty() {
            // Can't advance if there are choices
            return false;
        }

        if self.completed {
            return false;
        }

        // Get node info first to avoid borrow issues
        let node_info = self.tree.get_node(&self.current_node).map(|node| {
            (
                node.on_exit_effects.clone(),
                node.connections.first().map(|c| c.target_node.clone()),
            )
        });

        if let Some((exit_effects, next_node)) = node_info {
            // Apply on-exit effects
            for effect in &exit_effects {
                self.apply_effect(effect);
            }

            if let Some(next) = next_node {
                self.current_node = next;
                self.process_current_node();
                return true;
            }
        }

        self.completed = true;
        false
    }

    /// Check if dialogue is complete
    pub fn is_complete(&self) -> bool {
        self.completed
    }

    /// Get current text to display
    pub fn current_text(&self) -> Option<&str> {
        self.tree.get_node(&self.current_node).map(|n| n.text.as_str())
    }

    /// Get current speaker
    pub fn current_speaker(&self) -> Option<&str> {
        self.tree
            .get_node(&self.current_node)
            .and_then(|n| n.speaker.as_deref())
    }

    /// Update the session
    pub fn update(&mut self, dt: f32) {
        if let Some(ref mut tw) = self.typewriter {
            tw.update(dt);
        }
    }

    /// Get displayed text (with typewriter effect)
    pub fn displayed_text(&self) -> String {
        if let Some(ref tw) = self.typewriter {
            tw.displayed_text().to_string()
        } else if let Some(text) = self.current_text() {
            text.to_string()
        } else {
            String::new()
        }
    }

    /// Check if typewriter is complete
    pub fn is_text_complete(&self) -> bool {
        self.typewriter.as_ref().map(|t| t.is_complete()).unwrap_or(true)
    }
}

/// Dialogue manager
#[derive(Debug, Default)]
pub struct DialogueManager {
    /// Active dialogue session
    pub active_session: Option<DialogueSession>,
    /// Loaded dialogue trees
    trees: HashMap<String, DialogueTree>,
    /// Global variables shared across all dialogues
    pub global_variables: HashMap<String, serde_json::Value>,
}

impl DialogueManager {
    /// Create a new dialogue manager
    pub fn new() -> Self {
        Self {
            active_session: None,
            trees: HashMap::new(),
            global_variables: HashMap::new(),
        }
    }

    /// Register a dialogue tree
    pub fn register_tree(&mut self, tree: DialogueTree) {
        self.trees.insert(tree.id.clone(), tree);
    }

    /// Get a tree
    pub fn get_tree(&self, tree_id: &str) -> Option<&DialogueTree> {
        self.trees.get(tree_id)
    }

    /// Remove a tree
    pub fn remove_tree(&mut self, tree_id: &str) -> Option<DialogueTree> {
        self.trees.remove(tree_id)
    }

    /// Start dialogue with an NPC
    pub fn start_dialogue(&mut self, npc_entity: Entity, tree_id: &str) -> bool {
        if let Some(tree) = self.trees.get(tree_id).cloned() {
            let mut session = DialogueSession::new(npc_entity, tree);
            // Copy global variables
            for (k, v) in &self.global_variables {
                session.set_variable(k.clone(), v.clone());
            }
            self.active_session = Some(session);
            true
        } else {
            false
        }
    }

    /// Start dialogue with a simple generated tree
    pub fn start_simple_dialogue(&mut self, npc_entity: Entity, npc_name: &str, text: &str) {
        let tree = DialogueTree::simple_linear("simple", vec![(npc_name, text)]);
        let mut session = DialogueSession::new(npc_entity, tree);
        for (k, v) in &self.global_variables {
            session.set_variable(k.clone(), v.clone());
        }
        self.active_session = Some(session);
    }

    /// End the active dialogue
    pub fn end_dialogue(&mut self) {
        self.active_session = None;
    }

    /// Check if dialogue is active
    pub fn is_active(&self) -> bool {
        self.active_session.is_some()
    }

    /// Get mutable session
    pub fn session_mut(&mut self) -> Option<&mut DialogueSession> {
        self.active_session.as_mut()
    }

    /// Get session
    pub fn session(&self) -> Option<&DialogueSession> {
        self.active_session.as_ref()
    }

    /// Update the dialogue manager
    pub fn update(&mut self, dt: f32) {
        if let Some(ref mut session) = self.active_session {
            session.update(dt);
        }
    }

    /// Set a global variable
    pub fn set_global(&mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) {
        self.global_variables.insert(key.into(), value.into());
    }

    /// Get a global variable
    pub fn get_global(&self, key: &str) -> Option<&serde_json::Value> {
        self.global_variables.get(key)
    }

    /// Advance dialogue
    pub fn advance(&mut self) -> bool {
        if let Some(ref mut session) = self.active_session {
            session.advance()
        } else {
            false
        }
    }

    /// Select a choice
    pub fn select_choice(&mut self, index: usize) -> bool {
        if let Some(ref mut session) = self.active_session {
            session.select_choice(index)
        } else {
            false
        }
    }
}

/// Typewriter text effect
#[derive(Debug, Clone)]
pub struct TypewriterText {
    pub full_text: String,
    pub displayed_chars: usize,
    pub chars_per_second: f32,
    pub completed: bool,
    pub elapsed: f32,
}

impl TypewriterText {
    /// Create a new typewriter effect
    pub fn new(text: impl Into<String>, chars_per_second: f32) -> Self {
        Self {
            full_text: text.into(),
            displayed_chars: 0,
            chars_per_second,
            completed: false,
            elapsed: 0.0,
        }
    }

    /// Update the typewriter effect
    pub fn update(&mut self, dt: f32) {
        if self.completed {
            return;
        }

        self.elapsed += dt;

        let target_chars = (self.elapsed * self.chars_per_second) as usize;
        self.displayed_chars = target_chars.min(self.full_text.len());

        if self.displayed_chars >= self.full_text.len() {
            self.completed = true;
        }
    }

    /// Skip to the end
    pub fn skip(&mut self) {
        self.displayed_chars = self.full_text.len();
        self.completed = true;
    }

    /// Get the displayed text
    pub fn displayed_text(&self) -> &str {
        &self.full_text[..self.displayed_chars]
    }

    /// Check if complete
    pub fn is_complete(&self) -> bool {
        self.completed
    }

    /// Get progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        if self.full_text.is_empty() {
            1.0
        } else {
            self.displayed_chars as f32 / self.full_text.len() as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialogue_tree() {
        let mut tree = DialogueTree::new("test", "Test Dialogue");

        let node1 = DialogueNode {
            id: "start".to_string(),
            node_type: DialogueNodeType::NpcText,
            speaker: Some("NPC".to_string()),
            text: "Hello!".to_string(),
            choices: vec![],
            connections: vec![NodeConnection::to("end")],
            conditions: vec![],
            emotion: "happy".to_string(),
            ..Default::default()
        };

        tree.add_node(node1);

        assert!(tree.get_node("start").is_some());
    }

    #[test]
    fn test_simple_linear() {
        let tree = DialogueTree::simple_linear(
            "greeting",
            vec![
                ("NPC", "Hello there!"),
                ("NPC", "How are you?"),
                ("NPC", "Goodbye!"),
            ],
        );

        assert_eq!(tree.nodes.len(), 3);
    }

    #[test]
    fn test_typewriter() {
        let mut tw = TypewriterText::new("Hello, World!", 10.0);

        assert_eq!(tw.displayed_text(), "");

        tw.update(0.5); // 0.5 seconds, 5 chars at 10/s
        assert_eq!(tw.displayed_text(), "Hello");

        tw.skip();
        assert!(tw.is_complete());
        assert_eq!(tw.displayed_text(), "Hello, World!");
    }

    #[test]
    fn test_condition_evaluation() {
        let mut vars = HashMap::new();
        vars.insert("gold".to_string(), serde_json::json!(100));
        vars.insert("name".to_string(), serde_json::json!("Player"));
        vars.insert("has_key".to_string(), serde_json::json!(true));

        let cond1 = DialogueCondition::eq("gold", 100);
        assert!(cond1.evaluate(&vars));

        let cond2 = DialogueCondition::gt("gold", 50);
        assert!(cond2.evaluate(&vars));

        let cond3 = DialogueCondition::eq("gold", 200);
        assert!(!cond3.evaluate(&vars));

        let cond4 = DialogueCondition::eq("has_key", true);
        assert!(cond4.evaluate(&vars));
    }

    #[test]
    fn test_choice_builder() {
        let choice = DialogueChoice::new("buy", "I want to buy")
            .leads_to("shop_menu")
            .with_tooltip("Open the shop")
            .with_condition(DialogueCondition::eq("shop_open", true));

        assert_eq!(choice.id, "buy");
        assert_eq!(choice.text, "I want to buy");
        assert_eq!(choice.next_node, Some("shop_menu".to_string()));
        assert_eq!(choice.conditions.len(), 1);
    }

    #[test]
    fn test_tree_validation() {
        let mut tree = DialogueTree::new("test", "Test");
        
        // Add an orphaned node
        tree.add_node(DialogueNode::new_text("orphan", "I'm lost!", "NPC").at_position(500.0, 500.0));
        
        let errors = tree.validate();
        assert!(errors.iter().any(|e| matches!(e, ValidationError::OrphanedNode(id) if id == "orphan")));
    }

    #[test]
    fn test_json_serialization() {
        let tree = DialogueTree::simple_linear("test", vec![("NPC", "Hello!")]);
        let json = tree.to_json().unwrap();
        let tree2 = DialogueTree::from_json(&json).unwrap();
        
        assert_eq!(tree.id, tree2.id);
        assert_eq!(tree.nodes.len(), tree2.nodes.len());
    }
}
