//! Visual Scripting System Example
//!
//! This example demonstrates creating a simple quest script using the visual scripting system.

use dde_editor::visual_script::{
    compile_to_events, graph_from_json, graph_to_json, AnimationTarget, CollectionType,
    CompareOp, Connection, EntityRef, MathOp, Node, NodeCanvas, NodeGraph, NodeType, PinType,
    ScriptExecutor, StatType,
};
use dde_editor::visual_script_editor::VisualScriptEditor;

fn main() {
    println!("DocDamage Engine - Visual Scripting Example\n");

    // Example 1: Simple interaction
    println!("=== Example 1: Simple NPC Dialogue ===");
    let simple_dialogue = create_simple_dialogue();
    println!("Nodes: {}", simple_dialogue.nodes.len());
    println!("Connections: {}", simple_dialogue.connections.len());

    // Compile and check for errors
    match compile_to_events(&simple_dialogue) {
        Ok(script) => {
            println!("✓ Compilation successful!");
            println!("  Events: {}", script.events.len());
            println!("  Warnings: {}", script.warnings.len());
        }
        Err(e) => {
            println!("✗ Compilation failed: {}", e);
        }
    }

    // Example 2: Quest with conditions
    println!("\n=== Example 2: Quest with Item Check ===");
    let quest_script = create_quest_with_condition();
    println!("Nodes: {}", quest_script.nodes.len());
    println!("Connections: {}", quest_script.connections.len());

    match compile_to_events(&quest_script) {
        Ok(script) => {
            println!("✓ Compilation successful!");
            println!("  Events: {}", script.events.len());
        }
        Err(e) => {
            println!("✗ Compilation failed: {}", e);
        }
    }

    // Example 3: Save/Load
    println!("\n=== Example 3: Serialization ===");
    let json = graph_to_json(&simple_dialogue).expect("Failed to serialize");
    println!("Serialized to {} bytes", json.len());

    let loaded = graph_from_json(&json).expect("Failed to deserialize");
    println!("Loaded graph: {} nodes, {} connections", 
        loaded.nodes.len(), loaded.connections.len());

    // Example 4: Export to Lua
    println!("\n=== Example 4: Lua Export ===");
    let mut editor = VisualScriptEditor::with_graph(simple_dialogue, "ExampleDialogue");
    match editor.export_to_lua() {
        Ok(lua) => {
            println!("Generated Lua ({} bytes):", lua.len());
            println!("---");
            println!("{}", lua.lines().take(10).collect::<Vec<_>>().join("\n"));
            if lua.lines().count() > 10 {
                println!("... ({} more lines)", lua.lines().count() - 10);
            }
            println!("---");
        }
        Err(e) => {
            println!("Export failed: {}", e);
        }
    }

    println!("\n=== Done! ===");
}

/// Creates a simple dialogue script: OnInteract -> ShowDialogue
fn create_simple_dialogue() -> NodeGraph {
    let mut graph = NodeGraph::new();

    // Event: OnInteract
    let event_node = Node::new(NodeType::OnInteract, [100.0, 100.0]);
    let event_id = graph.add_node(event_node);

    // Action: ShowDialogue
    let action_node = Node::new(NodeType::ShowDialogue {
        text: "Welcome to the village, traveler!".to_string(),
        speaker: "Village Elder".to_string(),
        portrait: Some(1),
    }, [350.0, 100.0]);
    let action_id = graph.add_node(action_node);

    // Connect: Event -> Action
    let event_out = graph.nodes[&event_id].outputs[0].id;
    let action_in = graph.nodes[&action_id].inputs[0].id;
    graph.add_connection(Connection::new(event_id, event_out, action_id, action_in));

    graph
}

/// Creates a quest script with branching based on item possession
fn create_quest_with_condition() -> NodeGraph {
    let mut graph = NodeGraph::new();

    // Event: OnInteract
    let event_node = Node::new(NodeType::OnInteract, [50.0, 200.0]);
    let event_id = graph.add_node(event_node);

    // Condition: HasItem (Dragon Scale)
    let condition_node = Node::new(NodeType::HasItem { item_id: 42, quantity: 1 }, [300.0, 200.0]);
    let condition_id = graph.add_node(condition_node);

    // Branch node for logic
    let branch_node = Node::new(NodeType::Branch, [550.0, 200.0]);
    let branch_id = graph.add_node(branch_node);

    // True branch: Complete quest
    let complete_node = Node::new(NodeType::ShowDialogue {
        text: "You found the Dragon Scale! Thank you!".to_string(),
        speaker: "Quest Giver".to_string(),
        portrait: None,
    }, [800.0, 100.0]);
    let complete_id = graph.add_node(complete_node);

    let quest_complete = Node::new(NodeType::CompleteQuest { quest_id: 1 }, [800.0, 200.0]);
    let quest_complete_id = graph.add_node(quest_complete);

    // False branch: Quest not ready
    let not_ready_node = Node::new(NodeType::ShowDialogue {
        text: "Please bring me the Dragon Scale from the mountain.".to_string(),
        speaker: "Quest Giver".to_string(),
        portrait: None,
    }, [800.0, 350.0]);
    let not_ready_id = graph.add_node(not_ready_node);

    // Connect execution flow
    let event_out = graph.nodes[&event_id].outputs[0].id;
    let condition_in = graph.nodes[&condition_id].inputs[0].id;
    graph.add_connection(Connection::new(event_id, event_out, condition_id, condition_in));

    let condition_out = graph.nodes[&condition_id].outputs[0].id;
    let branch_in = graph.nodes[&branch_id].inputs.iter().find(|p| p.pin_type == PinType::Execution).unwrap().id;
    graph.add_connection(Connection::new(condition_id, condition_out, branch_id, branch_in));

    // Connect data: Condition result to Branch condition
    let condition_result = graph.nodes[&condition_id].outputs.iter().find(|p| p.pin_type == PinType::Boolean).unwrap().id;
    let branch_condition = graph.nodes[&branch_id].inputs.iter().find(|p| p.pin_type == PinType::Boolean).unwrap().id;
    graph.add_connection(Connection::new(condition_id, condition_result, branch_id, branch_condition));

    // Connect True branch
    let branch_true = graph.nodes[&branch_id].outputs.iter().find(|p| p.name == "True").unwrap().id;
    let complete_in = graph.nodes[&complete_id].inputs[0].id;
    graph.add_connection(Connection::new(branch_id, branch_true, complete_id, complete_in));

    let complete_out = graph.nodes[&complete_id].outputs[0].id;
    let quest_complete_in = graph.nodes[&quest_complete_id].inputs[0].id;
    graph.add_connection(Connection::new(complete_id, complete_out, quest_complete_id, quest_complete_in));

    // Connect False branch
    let branch_false = graph.nodes[&branch_id].outputs.iter().find(|p| p.name == "False").unwrap().id;
    let not_ready_in = graph.nodes[&not_ready_id].inputs[0].id;
    graph.add_connection(Connection::new(branch_id, branch_false, not_ready_id, not_ready_in));

    graph
}
