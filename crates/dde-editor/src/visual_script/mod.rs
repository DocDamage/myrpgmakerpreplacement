//! Visual Scripting System (Blueprints)
//!
//! Provides node-based visual scripting for creating game logic without code.
//!
//! ## Architecture
//!
//! The visual scripting system consists of:
//!
//! - **Canvas**: Visual node editor with pan/zoom, node rendering, and connection handling
//! - **Nodes**: Type definitions for all available node types (Events, Actions, Conditions, etc.)
//! - **Compiler**: Compiles node graphs to executable event sequences
//! - **Execution**: Runtime engine for executing compiled scripts
//!
//! ## Usage
//!
//! ```rust,ignore
//! use dde_editor::visual_script::{NodeCanvas, Node, NodeType, compile_to_events, ScriptExecutor};
//!
//! // Create a canvas and add nodes
//! let mut canvas = NodeCanvas::new();
//! let event_node = Node::new(NodeType::OnInteract, [100.0, 100.0]);
//! let action_node = Node::new(NodeType::ShowDialogue {
//!     text: "Hello!".to_string(),
//!     speaker: "NPC".to_string(),
//!     portrait: None,
//! }, [300.0, 100.0]);
//!
//! // Connect and compile
//! // ... (see canvas module for connection API)
//!
//! // Compile to events
//! let script = compile_to_events(canvas.graph()).unwrap();
//!
//! // Execute (world is a dde_core::World instance)
//! let mut executor = ScriptExecutor::new();
//! // executor.execute(&script, &mut world).unwrap();
//! ```

pub mod canvas;
pub mod compiler;
pub mod execution;
pub mod nodes;

// Re-export core types
pub use canvas::{Connection, NodeCanvas, NodeGraph};
pub use compiler::{
    compile_to_events, graph_from_json, graph_to_json, AnimationTarget, CompileError,
    CompiledScript, Condition, EntityRef, GameEvent,
};
pub use execution::{ExecutionError, ExecutionState, ScriptExecutor, ScriptRegistry, ScriptValue};
pub use nodes::{
    get_node_categories, AnimationTarget as NodeAnimationTarget, CollectionType, CompareOp,
    EntityRef as NodeEntityRef, MathOp, Node, NodeCategory, NodeId, NodeProperty, NodeType,
    NodeTypeTemplate, Pin, PinId, PinType, PinValue, StatType, ValueSource,
};

/// Current version of the visual scripting system
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Feature flags for the visual scripting system
#[derive(Debug, Clone, Copy)]
pub struct Features {
    /// Enable experimental nodes
    pub experimental_nodes: bool,
    /// Enable live reload
    pub live_reload: bool,
    /// Enable debug visualization
    pub debug_viz: bool,
}

impl Default for Features {
    fn default() -> Self {
        Self {
            experimental_nodes: false,
            live_reload: true,
            debug_viz: false,
        }
    }
}

/// Initialize the visual scripting system with the given features
pub fn init(features: Features) {
    tracing::info!("Initializing Visual Scripting System v{}", VERSION);

    if features.experimental_nodes {
        tracing::info!("Experimental nodes enabled");
    }

    if features.live_reload {
        tracing::info!("Live reload enabled");
    }

    if features.debug_viz {
        tracing::info!("Debug visualization enabled");
    }
}

/// Check if the visual scripting system is available
pub fn is_available() -> bool {
    true
}
