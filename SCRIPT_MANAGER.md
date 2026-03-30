# Script Manager Browser

A comprehensive Lua script management system for the DocDamage Engine editor.

## Overview

The Script Manager Browser provides a complete interface for managing all Lua scripts in your game project, including:

- **File browser** for all script types (NPC Behavior, Quests, Battle AI, Events, Utilities)
- **Script metadata** display (name, description, author, dependencies, last modified)
- **Actions**: Create, Edit, Duplicate, Delete, Organize in folders
- **Hot reload integration** with status display
- **Script validation** (syntax check, API usage check)

## UI Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│ Script Manager                                    [🔍 Search...]     │
├──────────┬────────────────────────────────┬─────────────────────────┤
│          │                                │                         │
│ 📁 Folders│  Scripts in: /npc              │  📜 Script Details      │
│          │  [Name ▲] [Type] [Status]      │                         │
│ 📂 npc   │  ──────────────────────────    │  👤 villager_behavior   │
│ 📂 quest │  👤 villager_behavior          │  ● Active               │
│ 📂 ai    │  👤 merchant_ai                │  Type: NPC Behavior     │
│ 📂 events│  🤖 goblin_ai                  │  Modified: 2024-01-15   │
│ 📂 util  │  📜 main_quest_01              │                         │
│          │                                │  [✏️ Edit] [🔄 Reload]  │
│          │                                │                         │
│          │                                │  Preview:               │
│          │                                │  ┌─────────────────────┐│
│          │                                │  │ -- NPC Behavior...  ││
│          │                                │  │ function npc.on_... ││
│          │                                │  └─────────────────────┘│
├──────────┴────────────────────────────────┴─────────────────────────┤
│ ⚠️ Error Log (3)                                    [Clear] [✕]     │
│ 🔴 villager_behavior: Syntax error at line 5                        │
│ 🟡 goblin_ai: API warning - unknown function                        │
└─────────────────────────────────────────────────────────────────────┘
```

## Integration

### 1. Add to Cargo.toml

Already done - `dde-lua` dependency added to `dde-editor/Cargo.toml`.

### 2. Implement ScriptManagerBackend

Create a backend that implements the `ScriptManagerBackend` trait:

```rust
use dde_editor::{ScriptManagerBackend, ValidationResult};
use dde_lua::scripts::{ScriptMetadata, ScriptTemplate, ScriptFolder, ScriptType};

pub struct MyScriptBackend {
    scripts: HashMap<i64, ScriptMetadata>,
    folders: Vec<ScriptFolder>,
    // ... other fields
}

impl ScriptManagerBackend for MyScriptBackend {
    fn get_scripts(&self) -> Vec<&ScriptMetadata> { ... }
    fn get_scripts_in_folder(&self, folder: &str) -> Vec<&ScriptMetadata> { ... }
    fn get_script(&self, id: i64) -> Option<&ScriptMetadata> { ... }
    fn create_script(&mut self, template: &ScriptTemplate, folder: &str) -> Result<ScriptMetadata, String> { ... }
    fn delete_script(&mut self, id: i64) -> Result<(), String> { ... }
    fn duplicate_script(&mut self, id: i64, new_name: &str) -> Result<ScriptMetadata, String> { ... }
    fn rename_script(&mut self, id: i64, new_name: &str) -> Result<(), String> { ... }
    fn move_script(&mut self, id: i64, folder: &str) -> Result<(), String> { ... }
    fn validate_script(&mut self, id: i64) -> ValidationResult { ... }
    fn reload_script(&mut self, id: i64) -> Result<(), String> { ... }
    fn reload_all(&mut self) -> Vec<(i64, Result<(), String>)> { ... }
    fn get_folders(&self) -> Vec<&ScriptFolder> { ... }
    fn create_folder(&mut self, name: &str, parent: &str) -> Result<String, String> { ... }
    fn delete_folder(&mut self, path: &str, move_scripts_to_parent: bool) -> Result<(), String> { ... }
    fn toggle_folder(&mut self, path: &str) { ... }
    fn get_error_log(&self) -> Vec<ScriptErrorEntry> { ... }
    fn clear_error_log(&mut self) { ... }
    fn open_in_external_editor(&self, id: i64) -> Result<(), String> { ... }
    fn get_script_source(&self, id: i64) -> Option<String> { ... }
}
```

### 3. Add Menu Item

In your main menu bar:

```rust
ui.menu_button("Tools", |ui| {
    editor.draw_tools_menu(ui);
});
```

This adds the "📜 Script Manager..." option to the Tools menu.

### 4. Draw the Panel

In your main draw loop:

```rust
// Update (call each frame)
editor.update_script_manager(dt, &mut script_backend);

// Draw (when visible)
if editor.is_script_manager_visible() {
    editor.draw_script_manager(ctx, &mut script_backend);
}
```

## Script Types

The Script Manager supports the following script types:

| Icon | Type | Description |
|------|------|-------------|
| 🤖 | NPC Behavior | Scripts for NPC AI and interactions |
| 📜 | Quest | Quest definitions and objectives |
| 🧠 | Battle AI | Enemy AI for combat encounters |
| ⚡ | Event | Map trigger and event scripts |
| 👤 | Entity | Scripts attached to specific entities |
| 🛠️ | Utility | Shared utility functions and libraries |
| 🌍 | Global | Global hook scripts (on_tick, on_map_enter, etc.) |
| ⚔️ | Battle | Battle-related scripts (environmental hazards) |

## Features

### Folder Management

- Create new folders to organize scripts
- Drag and drop scripts between folders
- Expand/collapse folder tree
- Delete folders (with option to move scripts to parent)

### Script Actions

- **Create**: Create new scripts from templates
- **Edit**: Open in external editor (VS Code, etc.)
- **Duplicate**: Create a copy of an existing script
- **Delete**: Remove scripts with confirmation
- **Rename**: Change script names
- **Validate**: Check syntax and API usage
- **Reload**: Hot reload scripts while game is running

### Search and Filter

- Search by script name or description
- Filter by script type
- Sort by name, type, modification date, or status

### Hot Reload Integration

- Visual indicators for script status:
  - 🟢 **Active** - Script is loaded and running
  - 🟡 **Modified** - Script has changes pending reload
  - 🔴 **Error** - Script has errors
  - 🔵 **Loading/Reloading** - Script is being processed
  - ⚪ **Unloaded** - Script is not loaded

### Validation

The Script Manager can validate scripts for:
- Syntax errors (unclosed blocks, malformed Lua)
- API usage (unknown functions, deprecated calls)
- Warnings (long functions, unused variables)

### Error Log

The bottom panel shows:
- Syntax errors with line numbers
- Runtime errors from hot reload
- API usage warnings
- Timestamps for each error

## Templates

New scripts can be created from templates:

```rust
let template = ScriptTemplate::npc_behavior();
// or
let template = ScriptTemplate::quest();
// or
let template = ScriptTemplate::battle_ai();
// etc.
```

Each template includes:
- Default code structure
- Appropriate comments
- Common function signatures
- Return statement

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Double-click | Edit script in external editor |
| Delete | Delete selected script (with confirmation) |
| Ctrl+Click | Multi-select scripts |
| F5 | Reload selected script |

## API Reference

### Editor Methods

```rust
// Show/hide
editor.show_script_manager();
editor.hide_script_manager();
editor.toggle_script_manager();

// Query state
if editor.is_script_manager_visible() { ... }

// Draw
editor.draw_script_manager(ctx, backend);

// Update (call each frame)
editor.update_script_manager(dt, backend);

// Menu
editor.draw_tools_menu(ui);

// Set external editor path
editor.set_script_editor(Some(PathBuf::from("code")));
```

### ScriptMetadata

```rust
pub struct ScriptMetadata {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub source: String,
    pub script_type: ScriptType,
    pub file_path: Option<PathBuf>,
    pub dependencies: Vec<String>,
    pub created_at: i64,
    pub modified_at: i64,
    pub compiled: bool,
    pub syntax_valid: bool,
    pub api_valid: bool,
    pub reload_status: ReloadStatus,
    pub folder_path: String,
}
```

## Example

See `dde-editor/examples/script_manager_example.rs` for a complete working example.

## Future Enhancements

Potential future features:
- Script diff viewer
- Git integration for version control
- Collaborative editing
- Script performance profiling
- Auto-completion in preview
- Integrated mini-editor
- Script dependencies graph
- Search within script content
