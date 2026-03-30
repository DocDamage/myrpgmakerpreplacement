# Visual Scripting System (Blueprints) for DocDamage Engine

## Overview

The Visual Scripting System provides node-based visual scripting for creating game logic without writing code. Inspired by Unreal Engine Blueprints, it allows game designers to create complex behaviors by connecting nodes in a graph.

## Architecture

```
dde-editor/src/visual_script/
├── mod.rs              # Module exports and initialization
├── nodes.rs            # Node type definitions and pin system
├── canvas.rs           # Visual node editor with egui
├── compiler.rs         # Compile node graphs to events
└── execution.rs        # Runtime execution engine

dde-editor/src/
└── visual_script_editor.rs  # Main editor window

dde-db/src/migrations/
└── v4_visual_scripts.rs     # Database schema for scripts
```

## Features

### Node Types

1. **Events** - Trigger nodes (yellow)
   - `OnInteract` - Player interaction
   - `OnEnterRegion` - Region trigger
   - `OnItemUse` - Item usage
   - `OnBattleStart` - Battle trigger
   - `OnTick` - Game tick
   - `OnStep` - Tile step

2. **Conditions** - Branch nodes (purple)
   - `HasItem` - Inventory check
   - `StatCheck` - Character stat comparison
   - `QuestStage` - Quest progress check
   - `TimeOfDay` - Time range check
   - `RandomChance` - Probability check
   - `GameFlag` - Game state flag
   - `Compare` - Value comparison

3. **Actions** - Game effect nodes (blue)
   - `MoveEntity` - Move character/object
   - `PlayAnimation` - Play animation
   - `StartBattle` - Initiate combat
   - `ShowDialogue` - Display dialogue
   - `ModifyVariable` - Change game variables
   - `GiveItem` / `RemoveItem` - Inventory management
   - `Teleport` - Map transition
   - `PlaySfx` / `ChangeBgm` - Audio control
   - `SpawnEntity` / `DespawnEntity` - Entity lifecycle
   - `StartQuest` / `CompleteQuest` - Quest management
   - `ModifyHealth` / `GrantExp` - Character stats

4. **Flow Control** - Logic nodes (green)
   - `Branch` - If/else condition
   - `Loop` - For loop with count
   - `WhileLoop` - Conditional loop
   - `ForEach` - Collection iteration
   - `Delay` - Time delay
   - `Parallel` - Concurrent execution
   - `Sequence` - Ordered execution
   - `Break` / `Continue` - Loop control

5. **Variables** - Data nodes (teal)
   - `GetVariable` / `SetVariable` - Variable access
   - `BoolLiteral` / `NumberLiteral` / `StringLiteral` - Constants

6. **Math** - Arithmetic nodes (orange)
   - `Add`, `Subtract`, `Multiply`, `Divide`, `Modulo`
   - `Clamp` - Value limiting
   - `RandomRange` - Random number generation

7. **Entity** - Game object nodes (red)
   - `GetPlayer` - Get player entity
   - `GetPosition` / `GetStat` / `SetStat` - Entity queries
   - `FindNearest` - Proximity search
   - `GetEntitiesInRegion` - Region query

### Editor Features

- **Canvas**: Pan/zoom navigation with grid
- **Node Rendering**: Color-coded by category
- **Connection System**: Drag connections between pins
- **Selection**: Box selection, multi-select with Shift
- **Context Menu**: Right-click to add nodes
- **Minimap**: Overview of entire graph
- **Property Panel**: Edit node properties
- **Node Palette**: Searchable node categories

### Pin System

Pins have types with color coding:
- **Execution** (White) - Flow control
- **Boolean** (Red) - True/False
- **Number** (Blue) - Integer/Float
- **String** (Yellow) - Text
- **Entity** (Green) - Game objects
- **Item** (Magenta) - Inventory items
- **Vector** (Cyan) - Position/coordinates

Pins enforce type safety - only compatible types can connect.

## Usage Example

```rust
use dde_editor::visual_script::{
    NodeCanvas, Node, NodeType, 
    compile_to_events, ScriptExecutor
};
use dde_editor::visual_script_editor::VisualScriptEditor;

// Create editor
let mut editor = VisualScriptEditor::new();

// Or create canvas directly
let mut canvas = NodeCanvas::new();

// Add event node
let event_node = Node::new(NodeType::OnInteract, [100.0, 100.0]);
let event_id = canvas.graph_mut().add_node(event_node);

// Add action node
let action_node = Node::new(NodeType::ShowDialogue {
    text: "Hello, adventurer!".to_string(),
    speaker: "NPC".to_string(),
    portrait: None,
}, [300.0, 100.0]);
let action_id = canvas.graph_mut().add_node(action_node);

// Connect nodes (execution flow)
use dde_editor::visual_script::Connection;

let event_out = canvas.graph().nodes[&event_id].outputs[0].id;
let action_in = canvas.graph().nodes[&action_id].inputs[0].id;
canvas.graph_mut().add_connection(Connection::new(
    event_id, event_out, action_id, action_in
));

// Compile to events
let script = compile_to_events(canvas.graph()).unwrap();

// Execute in game
let mut executor = ScriptExecutor::new();
executor.execute(&script, &mut world).unwrap();
```

## Database Schema

```sql
CREATE TABLE visual_scripts (
    script_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    graph_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    modified_at INTEGER NOT NULL
);

CREATE TABLE visual_script_triggers (
    trigger_id INTEGER PRIMARY KEY,
    script_id INTEGER NOT NULL REFERENCES visual_scripts(script_id),
    trigger_type TEXT NOT NULL,
    target_id INTEGER,
    target_type TEXT,
    enabled BOOLEAN NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 0,
    one_shot BOOLEAN NOT NULL DEFAULT 0,
    cooldown_ms INTEGER NOT NULL DEFAULT 0
);
```

## Integration

### Editor Integration

The visual script editor is integrated into the main Editor:

```rust
// In Editor struct
pub visual_script_editor: VisualScriptEditor,

// Draw in editor
self.visual_script_editor.draw(ctx);
```

### Runtime Integration

Scripts can be triggered from:
- Map events (tile triggers)
- Entity interactions
- Dialogue choices
- Item usage
- Battle events

### Export Options

- **JSON**: Serialized NodeGraph for storage
- **Lua**: Export to Lua scripts for RPG Maker MZ compatibility
- **Events**: Direct compilation to EngineEvents

## File Locations

| Component | Path |
|-----------|------|
| Core Module | `crates/dde-editor/src/visual_script/` |
| Editor Window | `crates/dde-editor/src/visual_script_editor.rs` |
| Database Migration | `crates/dde-db/src/migrations/v4_visual_scripts.rs` |
| Tests | Embedded in each module (`#[cfg(test)]`) |

## Future Enhancements

- [ ] Custom node types (user-defined)
- [ ] Live debugging with breakpoints
- [ ] Variable watch window
- [ ] Performance profiling
- [ ] Collaborative editing
- [ ] Version control diff view
- [ ] Lua import (reverse engineer existing scripts)

## Dependencies

- `egui` - UI framework
- `serde` / `serde_json` - Serialization
- `dde-core` - Core types and events
- `rand` - Random number generation
- `thiserror` - Error handling
- `tracing` - Logging

## Testing

Run tests with:
```bash
cargo test -p dde-editor visual_script
```

Each module contains unit tests covering:
- Node creation and configuration
- Graph operations (add/remove/connect)
- Compilation pipeline
- Execution engine
- Serialization round-trips
