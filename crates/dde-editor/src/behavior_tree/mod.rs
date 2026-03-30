//! Behavior Tree Visual Editor
//!
//! This module provides a complete visual editor for creating and editing
//! behavior trees for NPC AI, with runtime debugging capabilities.
//!
//! ## Architecture
//!
//! - `nodes.rs` - Node type definitions (composites, decorators, conditions, actions)
//! - `editor.rs` - Visual node editor with canvas and properties panel
//! - `debugger.rs` - Runtime debugging and visualization
//! - `compiler.rs` - Compile editor trees to runtime format
//!
//! ## Usage
//!
//! ```rust,ignore
//! use dde_editor::behavior_tree::{BehaviorTreeEditor, BtDebugger};
//!
//! // Create editor
//! let mut editor = BehaviorTreeEditor::new();
//! editor.new_tree();
//!
//! // Create debugger
//! let mut debugger = BtDebugger::new();
//! let entity = dde_core::Entity::DANGLING;
//! debugger.set_target(entity);
//!
//! // In your UI loop (ui is an egui::Ui instance)
//! // editor.draw_ui(ui, Some(&debugger));
//! ```

pub mod compiler;
pub mod debugger;
pub mod editor;
pub mod nodes;

// Re-export the enhanced visual editor from behavior_tree_editor module
pub use crate::behavior_tree_editor::{
    BehaviorTreeVisualEditor, ConnectionSource, EditorConfig, EditorTheme, ExecutionMode,
    NodeTheme,
};
pub use compiler::{compile, CompileError};
pub use dde_core::ai::NodeId;
pub use debugger::{draw_entity_debug, status_color, status_color_dark, BtDebugStats, BtDebugger};
pub use editor::BehaviorTreeEditor;
pub use nodes::{
    BtNode, BtNodeError, BtNodeType, MoveSpeed, MoveTarget, NodeCategory, ParallelPolicy, Target,
    VariableValue,
};

use serde::{Deserialize, Serialize};

/// Behavior tree asset format for saving/loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorTreeAsset {
    /// Asset version
    pub version: u32,
    /// Tree name
    pub name: String,
    /// Tree description
    pub description: String,
    /// Root node
    pub root: BtNode,
    /// Editor state (optional)
    pub editor_state: Option<EditorState>,
}

impl BehaviorTreeAsset {
    /// Current asset version
    pub const CURRENT_VERSION: u32 = 1;

    /// Create a new behavior tree asset
    pub fn new(name: impl Into<String>, root: BtNode) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            name: name.into(),
            description: String::new(),
            root,
            editor_state: None,
        }
    }

    /// Create with description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Save to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Load from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Save to file
    pub fn save_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = self
            .to_json()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// Load from file
    pub fn load_from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        Self::from_json(&json).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Editor state for saving/loading
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorState {
    /// Canvas offset
    pub canvas_offset: [f32; 2],
    /// Canvas zoom
    pub canvas_zoom: f32,
    /// Selected node
    pub selected_node: Option<NodeId>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            canvas_offset: [100.0, 50.0],
            canvas_zoom: 1.0,
            selected_node: None,
        }
    }
}

/// Example AI behavior presets
pub mod presets {
    use super::*;

    /// Basic patrol behavior
    pub fn patrol_behavior() -> BtNode {
        BtNode::new(
            BtNodeType::Sequence {
                children: vec![
                    BtNode::new(
                        BtNodeType::MoveTo {
                            target: MoveTarget::PatrolPoint(0),
                            speed: MoveSpeed::Walk,
                        },
                        [0.0, 100.0],
                    ),
                    BtNode::new(BtNodeType::Wait { seconds: 2.0 }, [150.0, 100.0]),
                ],
            },
            [0.0, 0.0],
        )
    }

    /// Basic combat behavior
    pub fn combat_behavior() -> BtNode {
        BtNode::new(
            BtNodeType::Selector {
                children: vec![
                    BtNode::new(
                        BtNodeType::Sequence {
                            children: vec![
                                BtNode::new(
                                    BtNodeType::HealthBelow { percent: 0.25 },
                                    [0.0, 150.0],
                                ),
                                BtNode::new(BtNodeType::Flee, [0.0, 250.0]),
                            ],
                        },
                        [0.0, 100.0],
                    ),
                    BtNode::new(
                        BtNodeType::Sequence {
                            children: vec![
                                BtNode::new(
                                    BtNodeType::IsPlayerNearby { radius: 5.0 },
                                    [200.0, 150.0],
                                ),
                                BtNode::new(
                                    BtNodeType::Attack {
                                        target: Target::Player,
                                    },
                                    [200.0, 250.0],
                                ),
                            ],
                        },
                        [200.0, 100.0],
                    ),
                    BtNode::new(
                        BtNodeType::MoveTo {
                            target: MoveTarget::Player,
                            speed: MoveSpeed::Run,
                        },
                        [400.0, 100.0],
                    ),
                ],
            },
            [0.0, 0.0],
        )
    }

    /// Smart guard behavior with multiple states
    pub fn guard_behavior() -> BtNode {
        BtNode::new(
            BtNodeType::Selector {
                children: vec![
                    // Combat mode
                    BtNode::new(
                        BtNodeType::Sequence {
                            children: vec![
                                BtNode::new(BtNodeType::InCombat, [0.0, 150.0]),
                                BtNode::new(
                                    BtNodeType::Cooldown {
                                        child: Box::new(BtNode::new(
                                            BtNodeType::Attack {
                                                target: Target::Player,
                                            },
                                            [150.0, 250.0],
                                        )),
                                        seconds: 1.0,
                                    },
                                    [150.0, 200.0],
                                ),
                            ],
                        },
                        [0.0, 100.0],
                    ),
                    // Alert mode - player nearby
                    BtNode::new(
                        BtNodeType::Sequence {
                            children: vec![
                                BtNode::new(
                                    BtNodeType::IsPlayerNearby { radius: 10.0 },
                                    [300.0, 150.0],
                                ),
                                BtNode::new(
                                    BtNodeType::Follow {
                                        target: Target::Player,
                                        distance: 3.0,
                                    },
                                    [300.0, 250.0],
                                ),
                            ],
                        },
                        [300.0, 100.0],
                    ),
                    // Idle patrol
                    BtNode::new(
                        BtNodeType::Repeater {
                            child: Box::new(BtNode::new(
                                BtNodeType::Sequence {
                                    children: vec![
                                        BtNode::new(
                                            BtNodeType::MoveTo {
                                                target: MoveTarget::PatrolPoint(0),
                                                speed: MoveSpeed::Walk,
                                            },
                                            [500.0, 250.0],
                                        ),
                                        BtNode::new(
                                            BtNodeType::Wait { seconds: 3.0 },
                                            [650.0, 250.0],
                                        ),
                                    ],
                                },
                                [500.0, 200.0],
                            )),
                            count: None, // Forever
                        },
                        [500.0, 100.0],
                    ),
                ],
            },
            [0.0, 0.0],
        )
    }

    /// Merchant NPC behavior
    pub fn merchant_behavior() -> BtNode {
        BtNode::new(
            BtNodeType::Selector {
                children: vec![
                    // Interact with player
                    BtNode::new(
                        BtNodeType::Sequence {
                            children: vec![
                                BtNode::new(
                                    BtNodeType::IsPlayerNearby { radius: 2.0 },
                                    [0.0, 150.0],
                                ),
                                BtNode::new(BtNodeType::Speak { dialogue_id: 100 }, [0.0, 250.0]),
                            ],
                        },
                        [0.0, 100.0],
                    ),
                    // Work schedule
                    BtNode::new(
                        BtNodeType::Sequence {
                            children: vec![
                                BtNode::new(
                                    BtNodeType::TimeOfDay { min: 8, max: 18 },
                                    [200.0, 150.0],
                                ),
                                BtNode::new(
                                    BtNodeType::PlayAnimation { anim_id: 1 },
                                    [200.0, 250.0],
                                ),
                            ],
                        },
                        [200.0, 100.0],
                    ),
                    // Rest
                    BtNode::new(BtNodeType::PlayAnimation { anim_id: 2 }, [400.0, 100.0]),
                ],
            },
            [0.0, 0.0],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::nodes::BtNodeType;
    use super::*;

    #[test]
    fn test_asset_serialization() {
        let root = BtNode::new(BtNodeType::InCombat, [0.0, 0.0]);
        let asset = BehaviorTreeAsset::new("Test Tree", root);

        let json = asset.to_json().unwrap();
        let loaded = BehaviorTreeAsset::from_json(&json).unwrap();

        assert_eq!(asset.name, loaded.name);
        assert_eq!(asset.version, loaded.version);
    }

    #[test]
    fn test_preset_patrol() {
        let patrol = presets::patrol_behavior();
        assert!(matches!(patrol.node_type, BtNodeType::Sequence { .. }));
    }

    #[test]
    fn test_preset_combat() {
        let combat = presets::combat_behavior();
        assert!(matches!(combat.node_type, BtNodeType::Selector { .. }));
    }

    #[test]
    fn test_preset_guard() {
        let guard = presets::guard_behavior();
        assert!(matches!(guard.node_type, BtNodeType::Selector { .. }));
    }
}
