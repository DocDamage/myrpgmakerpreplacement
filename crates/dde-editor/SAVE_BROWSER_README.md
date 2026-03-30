# Save/Backup Browser UI Panel

A comprehensive save management interface for the DocDamage Engine editor.

## Features

### Save Slot Browser
- **Grid view** of save slots (1-99)
- **Screenshot thumbnails** (when available)
- **Metadata display**:
  - Play time (HH:MM:SS format)
  - Save timestamp
  - Location/map name
  - Party leader name and level
  - File size
  - Encryption status
- **Empty slot** placeholders
- **Current loaded slot** highlighting (green border)
- **Search/filter** by player name or map
- **Configurable grid columns** (1-5)

### Backup Management
- **Automatic backups** listing with:
  - Timestamp
  - Source slot
  - File size
  - Backup reason (Auto-Save, Manual, Pre-Update)
- **Backup actions**:
  - Restore from backup
  - Delete backup
  - Export backup
- **Backup settings**:
  - Auto-backup interval (minutes)
  - Max backups to keep per slot
  - Backup on save toggle
  - Backup before update toggle

### Save Import/Export
- **Export Save** button (exports .dde slot file)
- **Import Save** button (imports external save)
- **Drag-and-drop support** for save files (.dde, .json, .dat)
- Password support for encrypted saves

### Quick Actions
- **Save Now** - Quick save to loaded or selected slot
- **Load Selected** - Load the currently selected slot
- **Delete Save** - Delete with confirmation dialog
- **Create New Save** - Save to next available slot

## Integration

### 1. Add to Editor

The `SaveBrowser` is already integrated into the `Editor` struct:

```rust
use dde_editor::Editor;

let mut editor = Editor::new();
```

### 2. Add File Menu Item

Add the Save Browser to your File menu:

```rust
// In your UI drawing code
egui::menu::bar(ui, |ui| {
    ui.menu_button("File", |ui| {
        // Add Save Browser menu item
        SaveBrowser::draw_menu_item(ui, &mut editor.save_browser);
        
        ui.separator();
        
        if ui.button("Save").clicked() {
            // Your save logic
        }
        if ui.button("Load").clicked() {
            // Your load logic
        }
    });
});
```

### 3. Draw the Window

Draw the save browser window when visible:

```rust
// In your main draw loop
if editor.is_save_browser_visible() {
    editor.draw_save_browser(ctx);
}
```

Or use the direct method:

```rust
// Handle save/load requests from the browser
if editor.save_browser.is_visible() {
    editor.save_browser.draw_window(ctx);
}
```

### 4. Handle Save/Load Requests

After drawing, check for pending requests:

```rust
// Check for load requests
if let Some(slot) = editor.save_browser.take_load_request() {
    // Load the save from slot
    match game_state.load_from_slot(slot) {
        Ok(()) => {
            editor.save_browser.set_loaded_slot(Some(slot));
            editor.save_browser.set_status("Loaded successfully", false);
        }
        Err(e) => {
            editor.save_browser.set_status(format!("Load failed: {}", e), true);
        }
    }
}

// Check for save requests
if let Some(slot) = editor.save_browser.take_save_request() {
    // Create save data from current game state
    let save_data = game_state.create_save_data(slot);
    
    // Execute the save
    match editor.save_browser.execute_save(slot, &save_data) {
        Ok(()) => {
            editor.save_browser.set_loaded_slot(Some(slot));
        }
        Err(e) => {
            editor.save_browser.set_status(format!("Save failed: {}", e), true);
        }
    }
}
```

## API Reference

### SaveBrowser Methods

#### Window Management
- `show()` - Show the save browser window
- `hide()` - Hide the save browser window
- `toggle()` - Toggle visibility
- `is_visible()` - Check if visible
- `draw_window(ctx)` - Draw as a floating window
- `draw(ctx, ui)` - Draw the browser UI into an existing UI

#### Menu Integration
- `draw_menu_item(ui, browser)` - Draw the File menu item (static method)

#### Slot Management
- `select_slot(slot)` - Select a slot programmatically
- `selected_slot()` - Get the currently selected slot
- `set_loaded_slot(slot)` - Set the currently loaded slot (for highlighting)
- `loaded_slot()` - Get the currently loaded slot

#### Requests (check after drawing)
- `take_load_request()` - Get pending load request (if user clicked "Load")
- `take_save_request()` - Get pending save request (if user clicked "Save")

#### Operations
- `execute_save(slot, save_data)` - Execute a save operation
- `refresh()` - Refresh the save cache from disk

#### Status
- `set_status(message, is_error)` - Set a status message
- `password()` - Get the password input (for encrypted saves)

## Example: Complete Integration

```rust
use dde_editor::{Editor, SaveBrowser};
use egui;

struct MyApp {
    editor: Editor,
    game_state: GameState,
}

impl MyApp {
    fn draw_ui(&mut self, ctx: &egui::Context) {
        // Draw menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    SaveBrowser::draw_menu_item(ui, &mut self.editor.save_browser);
                });
            });
        });

        // Draw save browser window if visible
        if self.editor.is_save_browser_visible() {
            self.editor.draw_save_browser(ctx);
        }

        // Handle save/load requests
        self.handle_save_browser_requests();
    }

    fn handle_save_browser_requests(&mut self) {
        // Handle load requests
        if let Some(slot) = self.editor.save_browser.take_load_request() {
            match self.game_state.load_from_slot(slot) {
                Ok(save_data) => {
                    self.editor.save_browser.set_loaded_slot(Some(slot));
                    self.editor.save_browser.set_status(
                        format!("Loaded from slot {}", slot), 
                        false
                    );
                    // Apply save data to game world
                    self.apply_save_data(save_data);
                }
                Err(e) => {
                    self.editor.save_browser.set_status(
                        format!("Load failed: {}", e), 
                        true
                    );
                }
            }
        }

        // Handle save requests
        if let Some(slot) = self.editor.save_browser.take_save_request() {
            let save_data = self.game_state.create_save_data(slot);
            match self.editor.save_browser.execute_save(slot, &save_data) {
                Ok(()) => {
                    self.editor.save_browser.set_loaded_slot(Some(slot));
                    self.editor.save_browser.set_status(
                        format!("Saved to slot {}", slot), 
                        false
                    );
                }
                Err(e) => {
                    self.editor.save_browser.set_status(
                        format!("Save failed: {}", e), 
                        true
                    );
                }
            }
        }
    }
}
```

## File Structure

```
dde-engine/crates/dde-editor/
├── src/
│   ├── lib.rs           # Editor struct with save_browser field
│   ├── save_browser.rs  # Save/Backup Browser implementation
│   └── save_panel.rs    # Original save panel (still available)
└── SAVE_BROWSER_README.md  # This file
```

## Backend Integration

The Save Browser connects to the existing save manager in:

```
dde-engine/crates/dde-core/src/save/manager.rs
```

Key APIs used:
- `SaveManager::list_backups()` → `get_backups(slot)`
- `SaveManager::restore_backup(slot, backup_num)`
- `SaveManager::export(slot, path, password)`
- `SaveManager::import(slot, path, password)`
- `SaveManager::delete(slot)`
- `SaveManager::save(slot, data, password)`
- `SaveManager::load(slot, password)`

## Customization

### Backup Settings

Modify `BackupSettings` in `save_browser.rs`:

```rust
pub struct BackupSettings {
    /// Auto-backup interval in minutes
    pub auto_backup_interval: u32,
    /// Maximum backups to keep per slot
    pub max_backups: u32,
    /// Enable backup on save
    pub backup_on_save: bool,
    /// Enable backup before game updates
    pub backup_before_update: bool,
}
```

### Slot Display

The `SlotDisplayInfo` struct can be extended to show additional game-specific data:

```rust
pub struct SlotDisplayInfo {
    pub slot: u32,
    pub has_data: bool,
    pub metadata: Option<SaveMetadata>,
    pub screenshot: Option<TextureHandle>,
    pub party_leader: Option<String>,
    pub party_level: Option<u32>,
    // Add your custom fields here
    pub custom_data: Option<MyCustomData>,
}
```

## License

Part of the DocDamage Engine project.
