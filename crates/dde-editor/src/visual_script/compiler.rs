//! Visual Script Compiler
//!
//! Compiles node graphs to event sequences for execution.
//! Traverses the graph from event nodes and generates corresponding GameEvents.

use super::canvas::NodeGraph;
use super::nodes::{
    CompareOp, MathOp, Node, NodeId, NodeType, PinId, PinType, StatType, ValueSource,
};
use dde_core::events::EngineEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Compilation error types
#[derive(thiserror::Error, Debug, Clone)]
pub enum CompileError {
    #[error("Orphaned node: {0:?}")]
    OrphanedNode(NodeId),

    #[error("Missing required input: {0:?}")]
    MissingInput(NodeId),

    #[error("Type mismatch at node {0:?}")]
    TypeMismatch(NodeId),

    #[error("Circular dependency detected involving node {0:?}")]
    CircularDependency(NodeId),

    #[error("Invalid connection at node {0:?}")]
    InvalidConnection(NodeId),

    #[error("Unsupported node type: {0}")]
    UnsupportedNodeType(String),
}

/// Result type for compilation
pub type CompileResult<T> = std::result::Result<T, CompileError>;

/// Compiled script with events and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledScript {
    pub events: Vec<GameEvent>,
    pub event_nodes: Vec<NodeId>,
    pub warnings: Vec<String>,
}

/// A compiled game event with execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    // World events
    MoveEntity {
        entity_ref: EntityRef,
        x: i32,
        y: i32,
        relative: bool,
    },
    PlayAnimation {
        anim_id: u32,
        target: AnimationTarget,
    },
    Teleport {
        map_id: u32,
        x: i32,
        y: i32,
    },
    SpawnEntity {
        template_id: u32,
        x: i32,
        y: i32,
    },
    DespawnEntity {
        entity_ref: EntityRef,
    },

    // Battle events
    StartBattle {
        encounter_id: u32,
        transition: String,
    },
    ModifyHealth {
        target: EntityRef,
        amount: i32,
    },
    GrantExp {
        target: EntityRef,
        amount: u32,
    },

    // UI events
    ShowDialogue {
        text: String,
        speaker: String,
        portrait: Option<u32>,
    },
    ShowNotification {
        text: String,
        duration_secs: f32,
    },
    PlaySfx {
        sound_id: String,
    },
    ChangeBgm {
        bgm_id: String,
        fade_ms: u32,
    },

    // Game state events
    GiveItem {
        item_id: u32,
        quantity: u32,
    },
    RemoveItem {
        item_id: u32,
        quantity: u32,
    },
    SetGameFlag {
        flag_key: String,
        value: bool,
    },
    ModifyVariable {
        name: String,
        operation: MathOp,
        value: i32,
    },

    // Quest events
    StartQuest {
        quest_id: u32,
    },
    UpdateQuest {
        quest_id: u32,
        objective_id: u32,
        progress: u32,
    },
    CompleteQuest {
        quest_id: u32,
    },

    // Control flow
    Delay {
        seconds: f32,
    },
    Branch {
        condition: Condition,
        true_branch: Vec<GameEvent>,
        false_branch: Vec<GameEvent>,
    },
    Loop {
        count: u32,
        body: Vec<GameEvent>,
    },
    WhileLoop {
        condition: Condition,
        body: Vec<GameEvent>,
    },
    Sequence {
        events: Vec<GameEvent>,
    },
    Parallel {
        branches: Vec<Vec<GameEvent>>,
    },
    Break,
    Continue,
}

/// Entity reference for compiled events
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EntityRef {
    SelfEntity,
    Player,
    Target,
    ById(u64),
}

/// Animation target types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AnimationTarget {
    SelfEntity,
    Player,
    Target,
}

/// Compiled condition for branches
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Condition {
    HasItem {
        item_id: u32,
        quantity: u32,
    },
    StatCheck {
        stat: StatType,
        operator: CompareOp,
        value: i32,
    },
    QuestStage {
        quest_id: u32,
        stage: u32,
    },
    TimeOfDay {
        min_hour: u8,
        max_hour: u8,
    },
    RandomChance {
        percent: u8,
    },
    GameFlag {
        flag_key: String,
        expected: bool,
    },
    Compare {
        left: ValueSource,
        operator: CompareOp,
        right: ValueSource,
    },
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
    Literal(bool),
}

/// Compile a node graph to event sequences
pub fn compile_to_events(graph: &NodeGraph) -> CompileResult<CompiledScript> {
    let mut compiler = Compiler::new(graph);
    compiler.compile()
}

/// Internal compiler state
struct Compiler<'a> {
    graph: &'a NodeGraph,
    visited: HashSet<NodeId>,
    visiting: HashSet<NodeId>,
    warnings: Vec<String>,
}

impl<'a> Compiler<'a> {
    fn new(graph: &'a NodeGraph) -> Self {
        Self {
            graph,
            visited: HashSet::new(),
            visiting: HashSet::new(),
            warnings: Vec::new(),
        }
    }

    fn compile(&mut self) -> CompileResult<CompiledScript> {
        let mut events = Vec::new();
        let mut event_nodes = Vec::new();

        // Find all event nodes (nodes without execution inputs)
        for (node_id, node) in &self.graph.nodes {
            if node.is_event_node() {
                event_nodes.push(*node_id);

                // Compile this event chain
                let event = self.compile_event_node(*node_id)?;
                events.push(event);
            }
        }

        // Check for orphaned nodes (not connected to any event)
        for node_id in self.graph.nodes.keys() {
            if !self.visited.contains(node_id) && !event_nodes.contains(node_id) {
                self.warnings.push(format!("Orphaned node: {:?}", node_id));
            }
        }

        Ok(CompiledScript {
            events,
            event_nodes,
            warnings: self.warnings.clone(),
        })
    }

    fn compile_event_node(&mut self, node_id: NodeId) -> CompileResult<GameEvent> {
        // Mark as visiting for cycle detection
        if self.visiting.contains(&node_id) {
            return Err(CompileError::CircularDependency(node_id));
        }
        self.visiting.insert(node_id);

        let node = self
            .graph
            .nodes
            .get(&node_id)
            .ok_or(CompileError::OrphanedNode(node_id))?
            .clone();

        // Mark as visited
        self.visited.insert(node_id);
        self.visiting.remove(&node_id);

        // Compile based on node type
        match &node.node_type {
            // Event nodes - compile their execution chain
            NodeType::OnInteract => {
                let chain = self.compile_execution_chain(node_id)?;
                Ok(GameEvent::Sequence { events: chain })
            }
            NodeType::OnEnterRegion { .. } => {
                let chain = self.compile_execution_chain(node_id)?;
                Ok(GameEvent::Sequence { events: chain })
            }
            NodeType::OnItemUse { .. } => {
                let chain = self.compile_execution_chain(node_id)?;
                Ok(GameEvent::Sequence { events: chain })
            }
            NodeType::OnBattleStart { .. } => {
                let chain = self.compile_execution_chain(node_id)?;
                Ok(GameEvent::Sequence { events: chain })
            }
            NodeType::OnTick => {
                let chain = self.compile_execution_chain(node_id)?;
                Ok(GameEvent::Sequence { events: chain })
            }
            NodeType::OnStep { .. } => {
                let chain = self.compile_execution_chain(node_id)?;
                Ok(GameEvent::Sequence { events: chain })
            }

            // Direct action nodes
            NodeType::MoveEntity { x, y, relative } => Ok(GameEvent::MoveEntity {
                entity_ref: EntityRef::SelfEntity,
                x: *x,
                y: *y,
                relative: *relative,
            }),
            NodeType::PlayAnimation { anim_id, target } => {
                let target = match target {
                    super::nodes::AnimationTarget::SelfEntity => AnimationTarget::SelfEntity,
                    super::nodes::AnimationTarget::Player => AnimationTarget::Player,
                    super::nodes::AnimationTarget::Target => AnimationTarget::Target,
                };
                Ok(GameEvent::PlayAnimation {
                    anim_id: *anim_id,
                    target,
                })
            }
            NodeType::StartBattle {
                encounter_id,
                transition,
            } => Ok(GameEvent::StartBattle {
                encounter_id: *encounter_id,
                transition: transition.clone(),
            }),
            NodeType::ShowDialogue {
                text,
                speaker,
                portrait,
            } => Ok(GameEvent::ShowDialogue {
                text: text.clone(),
                speaker: speaker.clone(),
                portrait: *portrait,
            }),
            NodeType::ModifyVariable {
                name,
                operation,
                value,
            } => Ok(GameEvent::ModifyVariable {
                name: name.clone(),
                operation: *operation,
                value: *value,
            }),
            NodeType::GiveItem { item_id, quantity } => Ok(GameEvent::GiveItem {
                item_id: *item_id,
                quantity: *quantity,
            }),
            NodeType::RemoveItem { item_id, quantity } => Ok(GameEvent::RemoveItem {
                item_id: *item_id,
                quantity: *quantity,
            }),
            NodeType::Teleport { map_id, x, y } => Ok(GameEvent::Teleport {
                map_id: *map_id,
                x: *x,
                y: *y,
            }),
            NodeType::PlaySfx { sound_id } => Ok(GameEvent::PlaySfx {
                sound_id: sound_id.clone(),
            }),
            NodeType::ChangeBgm { bgm_id, fade_ms } => Ok(GameEvent::ChangeBgm {
                bgm_id: bgm_id.clone(),
                fade_ms: *fade_ms,
            }),
            NodeType::SpawnEntity { template_id, x, y } => Ok(GameEvent::SpawnEntity {
                template_id: *template_id,
                x: *x,
                y: *y,
            }),
            NodeType::DespawnEntity { entity_ref } => {
                let entity_ref = match entity_ref {
                    super::nodes::EntityRef::SelfEntity => EntityRef::SelfEntity,
                    super::nodes::EntityRef::Player => EntityRef::Player,
                    super::nodes::EntityRef::Target => EntityRef::Target,
                    super::nodes::EntityRef::ById(id) => EntityRef::ById(*id),
                };
                Ok(GameEvent::DespawnEntity { entity_ref })
            }
            NodeType::SetGameFlag { flag_key, value } => Ok(GameEvent::SetGameFlag {
                flag_key: flag_key.clone(),
                value: *value,
            }),
            NodeType::StartQuest { quest_id } => Ok(GameEvent::StartQuest {
                quest_id: *quest_id,
            }),
            NodeType::UpdateQuest {
                quest_id,
                objective_id,
                progress,
            } => Ok(GameEvent::UpdateQuest {
                quest_id: *quest_id,
                objective_id: *objective_id,
                progress: *progress,
            }),
            NodeType::CompleteQuest { quest_id } => Ok(GameEvent::CompleteQuest {
                quest_id: *quest_id,
            }),
            NodeType::ShowNotification {
                text,
                duration_secs,
            } => Ok(GameEvent::ShowNotification {
                text: text.clone(),
                duration_secs: *duration_secs,
            }),
            NodeType::ModifyHealth { target, amount } => {
                let target = match target {
                    super::nodes::EntityRef::SelfEntity => EntityRef::SelfEntity,
                    super::nodes::EntityRef::Player => EntityRef::Player,
                    super::nodes::EntityRef::Target => EntityRef::Target,
                    super::nodes::EntityRef::ById(id) => EntityRef::ById(*id),
                };
                Ok(GameEvent::ModifyHealth {
                    target,
                    amount: *amount,
                })
            }
            NodeType::GrantExp { target, amount } => {
                let target = match target {
                    super::nodes::EntityRef::SelfEntity => EntityRef::SelfEntity,
                    super::nodes::EntityRef::Player => EntityRef::Player,
                    super::nodes::EntityRef::Target => EntityRef::Target,
                    super::nodes::EntityRef::ById(id) => EntityRef::ById(*id),
                };
                Ok(GameEvent::GrantExp {
                    target,
                    amount: *amount,
                })
            }
            NodeType::Delay { seconds } => Ok(GameEvent::Delay { seconds: *seconds }),

            // Flow control nodes
            NodeType::Branch => self.compile_branch_node(&node),
            NodeType::Loop { count } => {
                let body = self.compile_execution_chain(node_id)?;
                Ok(GameEvent::Loop {
                    count: *count,
                    body,
                })
            }
            NodeType::WhileLoop => self.compile_while_loop_node(&node),
            NodeType::Sequence => {
                let events = self.compile_execution_chain(node_id)?;
                Ok(GameEvent::Sequence { events })
            }
            NodeType::Parallel => self.compile_parallel_node(&node),
            NodeType::Break => Ok(GameEvent::Break),
            NodeType::Continue => Ok(GameEvent::Continue),

            // Condition nodes - compile as condition expressions
            NodeType::HasItem {
                item_id: _,
                quantity: _,
            } => {
                // This should be used in a Branch node context
                Err(CompileError::UnsupportedNodeType(
                    "HasItem must be connected to Branch condition pin".to_string(),
                ))
            }
            NodeType::StatCheck {
                stat: _,
                operator: _,
                value: _,
            } => Err(CompileError::UnsupportedNodeType(
                "StatCheck must be connected to Branch condition pin".to_string(),
            )),
            NodeType::QuestStage {
                quest_id: _,
                stage: _,
            } => Err(CompileError::UnsupportedNodeType(
                "QuestStage must be connected to Branch condition pin".to_string(),
            )),
            NodeType::TimeOfDay {
                min_hour: _,
                max_hour: _,
            } => Err(CompileError::UnsupportedNodeType(
                "TimeOfDay must be connected to Branch condition pin".to_string(),
            )),
            NodeType::RandomChance { percent: _ } => Err(CompileError::UnsupportedNodeType(
                "RandomChance must be connected to Branch condition pin".to_string(),
            )),
            NodeType::GameFlag {
                flag_key: _flag_key,
                expected: _expected,
            } => Err(CompileError::UnsupportedNodeType(
                "GameFlag must be connected to Branch condition pin".to_string(),
            )),
            NodeType::Compare {
                left: _,
                operator: _,
                right: _,
            } => Err(CompileError::UnsupportedNodeType(
                "Compare must be connected to Branch condition pin".to_string(),
            )),

            // Variable and math nodes - need to be used as inputs
            _ => Err(CompileError::UnsupportedNodeType(format!(
                "Node type {:?} cannot be compiled directly",
                node.node_type
            ))),
        }
    }

    fn compile_execution_chain(&mut self, start_node_id: NodeId) -> CompileResult<Vec<GameEvent>> {
        let mut events = Vec::new();
        let mut current_id = start_node_id;
        let mut visited_in_chain = HashSet::new();

        loop {
            if visited_in_chain.contains(&current_id) {
                return Err(CompileError::CircularDependency(current_id));
            }
            visited_in_chain.insert(current_id);

            // Get the execution output connections
            let connections: Vec<_> = self
                .graph
                .get_connections_from(current_id, self.get_execution_output_pin(current_id)?)
                .into_iter()
                .filter(|c| {
                    // Check if the target is an execution input
                    if let Some(target_node) = self.graph.nodes.get(&c.target_node) {
                        target_node
                            .inputs
                            .iter()
                            .any(|p| p.id == c.target_pin && p.pin_type == PinType::Execution)
                    } else {
                        false
                    }
                })
                .cloned()
                .collect();

            if connections.is_empty() {
                break;
            }

            // For a linear chain, follow the first execution output
            let conn = &connections[0];
            let next_node = self
                .graph
                .nodes
                .get(&conn.target_node)
                .ok_or(CompileError::InvalidConnection(conn.target_node))?;

            // Compile this node
            let event = self.compile_event_node(next_node.id)?;
            events.push(event);

            current_id = next_node.id;
        }

        Ok(events)
    }

    fn compile_branch_node(&mut self, node: &Node) -> CompileResult<GameEvent> {
        // Get the condition value
        let condition = self.extract_condition(node)?;

        // Find True and False branches
        let mut true_branch = Vec::new();
        let mut false_branch = Vec::new();

        // Find connections from True and False outputs
        for conn in &self.graph.connections {
            if conn.source_node == node.id {
                if let Some(pin) = node.get_pin(conn.source_pin) {
                    let branch_events = self.compile_execution_chain(conn.target_node)?;
                    if pin.name == "True" {
                        true_branch = branch_events;
                    } else if pin.name == "False" {
                        false_branch = branch_events;
                    }
                }
            }
        }

        Ok(GameEvent::Branch {
            condition,
            true_branch,
            false_branch,
        })
    }

    fn compile_while_loop_node(&mut self, node: &Node) -> CompileResult<GameEvent> {
        let condition = self.extract_condition(node)?;

        // Find Body output connection
        let mut body = Vec::new();

        for conn in &self.graph.connections {
            if conn.source_node == node.id {
                if let Some(pin) = node.get_pin(conn.source_pin) {
                    if pin.name == "Body" {
                        body = self.compile_execution_chain(conn.target_node)?;
                    }
                }
            }
        }

        Ok(GameEvent::WhileLoop { condition, body })
    }

    fn compile_parallel_node(&mut self, node: &Node) -> CompileResult<GameEvent> {
        let mut branches: Vec<Vec<GameEvent>> = Vec::new();

        // Find all output connections
        for output in &node.outputs {
            let output_connections: Vec<_> = self
                .graph
                .connections
                .iter()
                .filter(|c| c.source_node == node.id && c.source_pin == output.id)
                .collect();

            for conn in output_connections {
                let branch = self.compile_execution_chain(conn.target_node)?;
                if !branch.is_empty() {
                    branches.push(branch);
                }
            }
        }

        Ok(GameEvent::Parallel { branches })
    }

    fn extract_condition(&mut self, node: &Node) -> CompileResult<Condition> {
        // Find the Condition input pin connection
        for conn in &self.graph.connections {
            if conn.target_node == node.id {
                if let Some(pin) = node.get_pin(conn.target_pin) {
                    if pin.name == "Condition" || pin.pin_type == PinType::Boolean {
                        // Get the source node which should be a condition
                        if let Some(source_node) = self.graph.nodes.get(&conn.source_node) {
                            return self.compile_condition_node(source_node);
                        }
                    }
                }
            }
        }

        // Default to true if no condition connected
        Ok(Condition::Literal(true))
    }

    fn compile_condition_node(&mut self, node: &Node) -> CompileResult<Condition> {
        match &node.node_type {
            NodeType::HasItem { item_id, quantity } => Ok(Condition::HasItem {
                item_id: *item_id,
                quantity: *quantity,
            }),
            NodeType::StatCheck {
                stat,
                operator,
                value,
            } => Ok(Condition::StatCheck {
                stat: *stat,
                operator: *operator,
                value: *value,
            }),
            NodeType::QuestStage { quest_id, stage } => Ok(Condition::QuestStage {
                quest_id: *quest_id,
                stage: *stage,
            }),
            NodeType::TimeOfDay { min_hour, max_hour } => Ok(Condition::TimeOfDay {
                min_hour: *min_hour,
                max_hour: *max_hour,
            }),
            NodeType::RandomChance { percent } => Ok(Condition::RandomChance { percent: *percent }),
            NodeType::GameFlag { flag_key, expected } => Ok(Condition::GameFlag {
                flag_key: flag_key.clone(),
                expected: *expected,
            }),
            NodeType::Compare {
                left,
                operator,
                right,
            } => Ok(Condition::Compare {
                left: left.clone(),
                operator: *operator,
                right: right.clone(),
            }),
            _ => {
                self.warnings
                    .push(format!("Node {:?} is not a valid condition", node.id));
                Ok(Condition::Literal(true))
            }
        }
    }

    fn get_execution_output_pin(&self, node_id: NodeId) -> CompileResult<PinId> {
        let node = self
            .graph
            .nodes
            .get(&node_id)
            .ok_or(CompileError::OrphanedNode(node_id))?;

        node.exec_outputs()
            .next()
            .map(|p| p.id)
            .ok_or(CompileError::MissingInput(node_id))
    }
}

/// Convert compiled GameEvents to EngineEvents for the event bus
pub fn to_engine_events(events: &[GameEvent]) -> Vec<EngineEvent> {
    events.iter().filter_map(convert_to_engine_event).collect()
}

fn convert_to_engine_event(event: &GameEvent) -> Option<EngineEvent> {
    match event {
        GameEvent::ShowDialogue { .. } => {
            Some(EngineEvent::DialogueStarted {
                npc: hecs::Entity::DANGLING, // Placeholder
                tree_id: None,
            })
        }
        GameEvent::StartBattle { .. } => Some(EngineEvent::BattleTriggered {
            enemies: vec![],
            terrain: "default".to_string(),
        }),
        GameEvent::Teleport {
            map_id,
            x: _x,
            y: _y,
        } => Some(EngineEvent::SubMapEntered {
            entity: hecs::Entity::DANGLING,
            sub_map_id: *map_id,
        }),
        GameEvent::PlaySfx { sound_id } => Some(EngineEvent::SfxPlay {
            sound_id: sound_id.clone(),
            position: None,
        }),
        GameEvent::ChangeBgm { bgm_id, .. } => Some(EngineEvent::BgmChange {
            stem_set_id: bgm_id.clone(),
        }),
        GameEvent::ModifyHealth { amount, .. } => Some(EngineEvent::DamageDealt {
            source: hecs::Entity::DANGLING,
            target: hecs::Entity::DANGLING,
            amount: *amount,
            element: dde_core::Element::None,
            is_crit: false,
        }),
        GameEvent::StartQuest { quest_id } => Some(EngineEvent::QuestStarted {
            quest_id: *quest_id,
        }),
        GameEvent::CompleteQuest { quest_id } => Some(EngineEvent::QuestCompleted {
            quest_id: *quest_id,
        }),
        _ => None,
    }
}

/// Serialize a compiled script to JSON
pub fn script_to_json(script: &CompiledScript) -> serde_json::Result<String> {
    serde_json::to_string_pretty(script)
}

/// Deserialize a compiled script from JSON
pub fn script_from_json(json: &str) -> serde_json::Result<CompiledScript> {
    serde_json::from_str(json)
}

/// Serialize a node graph to JSON for storage
pub fn graph_to_json(graph: &NodeGraph) -> serde_json::Result<String> {
    serde_json::to_string_pretty(graph)
}

/// Deserialize a node graph from JSON
pub fn graph_from_json(json: &str) -> serde_json::Result<NodeGraph> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::super::canvas::Connection;
    use super::super::nodes::{Node, NodeType, PinType};
    use super::*;

    #[test]
    fn test_compile_simple_chain() {
        let mut graph = NodeGraph::new();

        // OnInteract -> ShowDialogue
        let event_node = Node::new(NodeType::OnInteract, [0.0, 0.0]);
        let action_node = Node::new(
            NodeType::ShowDialogue {
                text: "Hello!".to_string(),
                speaker: "NPC".to_string(),
                portrait: None,
            },
            [200.0, 0.0],
        );

        let event_id = graph.add_node(event_node);
        let action_id = graph.add_node(action_node);

        // Connect execution output to input
        let event_out = graph.nodes[&event_id].outputs[0].id;
        let action_in = graph.nodes[&action_id].inputs[0].id;
        graph.add_connection(Connection::new(event_id, event_out, action_id, action_in));

        let result = compile_to_events(&graph);
        assert!(result.is_ok());

        let script = result.unwrap();
        assert_eq!(script.events.len(), 1);
        assert_eq!(script.warnings.len(), 0);
    }

    #[test]
    fn test_compile_branch() {
        let mut graph = NodeGraph::new();

        // OnInteract -> Branch -> [ShowDialogue True, ShowDialogue False]
        let event_node = Node::new(NodeType::OnInteract, [0.0, 0.0]);
        let branch_node = Node::new(NodeType::Branch, [200.0, 0.0]);
        let true_node = Node::new(
            NodeType::ShowDialogue {
                text: "You have it!".to_string(),
                speaker: "NPC".to_string(),
                portrait: None,
            },
            [400.0, -100.0],
        );
        let false_node = Node::new(
            NodeType::ShowDialogue {
                text: "You don't have it.".to_string(),
                speaker: "NPC".to_string(),
                portrait: None,
            },
            [400.0, 100.0],
        );

        let event_id = graph.add_node(event_node);
        let branch_id = graph.add_node(branch_node);
        let true_id = graph.add_node(true_node);
        let false_id = graph.add_node(false_node);

        // Connect event to branch
        let event_out = graph.nodes[&event_id].outputs[0].id;
        let branch_in = graph.nodes[&branch_id]
            .inputs
            .iter()
            .find(|p| p.pin_type == PinType::Execution)
            .unwrap()
            .id;
        graph.add_connection(Connection::new(event_id, event_out, branch_id, branch_in));

        // Connect branch True to true_node
        let branch_true_out = graph.nodes[&branch_id]
            .outputs
            .iter()
            .find(|p| p.name == "True")
            .unwrap()
            .id;
        let true_in = graph.nodes[&true_id].inputs[0].id;
        graph.add_connection(Connection::new(
            branch_id,
            branch_true_out,
            true_id,
            true_in,
        ));

        // Connect branch False to false_node
        let branch_false_out = graph.nodes[&branch_id]
            .outputs
            .iter()
            .find(|p| p.name == "False")
            .unwrap()
            .id;
        let false_in = graph.nodes[&false_id].inputs[0].id;
        graph.add_connection(Connection::new(
            branch_id,
            branch_false_out,
            false_id,
            false_in,
        ));

        let result = compile_to_events(&graph);
        assert!(result.is_ok());

        let script = result.unwrap();
        assert_eq!(script.events.len(), 1);
    }

    #[test]
    fn test_circular_dependency() {
        let mut graph = NodeGraph::new();

        // Create a circular chain: A -> B -> A
        let node_a = Node::new(NodeType::Delay { seconds: 1.0 }, [0.0, 0.0]);
        let node_b = Node::new(NodeType::Delay { seconds: 1.0 }, [200.0, 0.0]);

        let id_a = graph.add_node(node_a);
        let id_b = graph.add_node(node_b);

        // Connect A -> B
        let out_a = graph.nodes[&id_a].outputs[0].id;
        let in_b = graph.nodes[&id_b].inputs[0].id;
        graph.add_connection(Connection::new(id_a, out_a, id_b, in_b));

        // Connect B -> A (creates cycle)
        let out_b = graph.nodes[&id_b].outputs[0].id;
        let in_a = graph.nodes[&id_a].inputs[0].id;
        graph.add_connection(Connection::new(id_b, out_b, id_a, in_a));

        // Add an event node to start from
        let event_node = Node::new(NodeType::OnInteract, [-200.0, 0.0]);
        let event_id = graph.add_node(event_node);
        let event_out = graph.nodes[&event_id].outputs[0].id;
        graph.add_connection(Connection::new(event_id, event_out, id_a, in_a));

        let result = compile_to_events(&graph);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialization() {
        let mut graph = NodeGraph::new();
        let node = Node::new(NodeType::OnInteract, [100.0, 200.0]);
        graph.add_node(node);

        let json = graph_to_json(&graph).unwrap();
        let restored = graph_from_json(&json).unwrap();

        assert_eq!(graph.nodes.len(), restored.nodes.len());
    }
}
