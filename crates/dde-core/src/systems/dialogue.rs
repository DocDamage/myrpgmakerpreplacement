//! Dialogue system - Manages dialogue trees and conversations

use std::collections::HashMap;

use crate::Entity;

/// Dialogue node types
#[derive(Debug, Clone, PartialEq)]
pub enum DialogueNodeType {
    /// NPC speaks text
    NpcText,
    /// Player makes a choice
    PlayerChoice,
    /// Condition check
    Condition,
    /// Action/effect
    Action,
    /// End of dialogue
    End,
}

/// Dialogue node
#[derive(Debug, Clone)]
pub struct DialogueNode {
    pub id: String,
    pub node_type: DialogueNodeType,
    pub speaker: Option<String>,
    pub text: String,
    pub choices: Vec<DialogueChoice>,
    pub next_node: Option<String>,
    pub conditions: Vec<DialogueCondition>,
    pub effects: Vec<DialogueEffect>,
    pub emotion: String,
}

/// Dialogue choice
#[derive(Debug, Clone)]
pub struct DialogueChoice {
    pub id: String,
    pub text: String,
    pub next_node: Option<String>,
    pub conditions: Vec<DialogueCondition>,
    pub effects: Vec<DialogueEffect>,
}

/// Dialogue condition
#[derive(Debug, Clone)]
pub struct DialogueCondition {
    pub condition_type: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Dialogue effect
#[derive(Debug, Clone)]
pub struct DialogueEffect {
    pub effect_type: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Dialogue tree
#[derive(Debug, Clone)]
pub struct DialogueTree {
    pub id: String,
    pub name: String,
    pub root_node: String,
    pub nodes: HashMap<String, DialogueNode>,
}

impl DialogueTree {
    /// Create a new empty dialogue tree
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            root_node: String::new(),
            nodes: HashMap::new(),
        }
    }

    /// Add a node to the tree
    pub fn add_node(&mut self, node: DialogueNode) {
        if self.root_node.is_empty() {
            self.root_node = node.id.clone();
        }
        self.nodes.insert(node.id.clone(), node);
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: &str) -> Option<&DialogueNode> {
        self.nodes.get(node_id)
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
                next_node: if is_last {
                    None
                } else {
                    Some(format!("node_{}", i + 1))
                },
                conditions: vec![],
                effects: vec![],
                emotion: "neutral".to_string(),
            };

            tree.add_node(node);

            // Link previous node
            if let Some(ref prev) = prev_node_id {
                if let Some(prev_node) = tree.nodes.get_mut(prev) {
                    prev_node.next_node = Some(node_id.clone());
                }
            }

            prev_node_id = Some(node_id);
        }

        tree
    }
}

/// Active dialogue session
#[derive(Debug)]
pub struct DialogueSession {
    pub npc_entity: Entity,
    pub tree: DialogueTree,
    pub current_node: String,
    pub history: Vec<(String, String)>, // (speaker, text)
    pub player_choices: Vec<DialogueChoice>,
    pub completed: bool,
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
        };

        // Process initial node
        session.process_current_node();

        session
    }

    /// Process the current node
    fn process_current_node(&mut self) {
        if let Some(node) = self.tree.get_node(&self.current_node).cloned() {
            match node.node_type {
                DialogueNodeType::NpcText => {
                    if let Some(ref speaker) = node.speaker {
                        self.history.push((speaker.clone(), node.text.clone()));
                    }

                    // Auto-advance if there's a next node
                    if let Some(ref next) = node.next_node {
                        self.current_node = next.clone();
                        self.process_current_node();
                    }
                }
                DialogueNodeType::PlayerChoice => {
                    self.player_choices = node.choices.clone();
                }
                DialogueNodeType::End => {
                    self.completed = true;
                }
                _ => {}
            }
        }
    }

    /// Select a choice
    pub fn select_choice(&mut self, choice_index: usize) -> bool {
        if choice_index >= self.player_choices.len() {
            return false;
        }

        let choice = &self.player_choices[choice_index];

        // Add player choice to history
        self.history
            .push(("Player".to_string(), choice.text.clone()));

        // Move to next node
        if let Some(ref next) = choice.next_node {
            self.current_node = next.clone();
            self.player_choices.clear();
            self.process_current_node();
            true
        } else {
            self.completed = true;
            false
        }
    }

    /// Advance to the next node (for simple dialogues)
    pub fn advance(&mut self) -> bool {
        if !self.player_choices.is_empty() {
            // Can't advance if there are choices
            return false;
        }

        if self.completed {
            return false;
        }

        if let Some(node) = self.tree.get_node(&self.current_node) {
            if let Some(ref next) = node.next_node {
                self.current_node = next.clone();
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
        if let Some(node) = self.tree.get_node(&self.current_node) {
            if node.node_type == DialogueNodeType::NpcText {
                return Some(&node.text);
            }
        }
        None
    }

    /// Get current speaker
    pub fn current_speaker(&self) -> Option<&str> {
        if let Some(node) = self.tree.get_node(&self.current_node) {
            node.speaker.as_deref()
        } else {
            None
        }
    }
}

/// Dialogue manager
pub struct DialogueManager {
    /// Active dialogue session
    pub active_session: Option<DialogueSession>,
    /// Loaded dialogue trees
    trees: HashMap<String, DialogueTree>,
}

impl DialogueManager {
    /// Create a new dialogue manager
    pub fn new() -> Self {
        Self {
            active_session: None,
            trees: HashMap::new(),
        }
    }

    /// Register a dialogue tree
    pub fn register_tree(&mut self, tree: DialogueTree) {
        self.trees.insert(tree.id.clone(), tree);
    }

    /// Start dialogue with an NPC
    pub fn start_dialogue(&mut self, npc_entity: Entity, tree_id: &str) -> bool {
        if let Some(tree) = self.trees.get(tree_id).cloned() {
            self.active_session = Some(DialogueSession::new(npc_entity, tree));
            true
        } else {
            false
        }
    }

    /// Start dialogue with a simple generated tree
    pub fn start_simple_dialogue(&mut self, npc_entity: Entity, npc_name: &str, text: &str) {
        let tree = DialogueTree::simple_linear("simple", vec![(npc_name, text)]);
        self.active_session = Some(DialogueSession::new(npc_entity, tree));
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
}

impl Default for DialogueManager {
    fn default() -> Self {
        Self::new()
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
            next_node: Some("end".to_string()),
            conditions: vec![],
            effects: vec![],
            emotion: "happy".to_string(),
        };

        tree.add_node(node1);

        assert!(tree.get_node("start").is_some());
        assert_eq!(tree.root_node, "start");
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
}
