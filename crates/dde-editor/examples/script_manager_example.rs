//! Script Manager Browser Example
//!
//! This example demonstrates how to integrate the Script Manager Browser
//! into your application with the ScriptManagerBackend trait.

use dde_editor::{Editor, ScriptManagerBackend, ValidationResult, ValidationError};
use dde_lua::scripts::{
    ErrorType, ReloadStatus, ScriptFolder, ScriptMetadata, ScriptTemplate, ScriptType,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// Example backend implementation for the Script Manager
pub struct ExampleScriptBackend {
    scripts: HashMap<i64, ScriptMetadata>,
    folders: Vec<ScriptFolder>,
    error_log: Vec<dde_editor::script_manager::ScriptErrorEntry>,
    next_id: i64,
}

impl ExampleScriptBackend {
    pub fn new() -> Self {
        let mut backend = Self {
            scripts: HashMap::new(),
            folders: Vec::new(),
            error_log: Vec::new(),
            next_id: 1,
        };
        backend.init_sample_data();
        backend
    }

    fn init_sample_data(&mut self) {
        // Add default folders
        self.folders = vec![
            ScriptFolder { path: "/".to_string(), name: "Scripts".to_string(), parent: None, expanded: true },
            ScriptFolder { path: "/npc".to_string(), name: "NPC Behavior".to_string(), parent: Some("/".to_string()), expanded: true },
            ScriptFolder { path: "/quest".to_string(), name: "Quests".to_string(), parent: Some("/".to_string()), expanded: true },
            ScriptFolder { path: "/ai".to_string(), name: "Battle AI".to_string(), parent: Some("/".to_string()), expanded: true },
            ScriptFolder { path: "/events".to_string(), name: "Events".to_string(), parent: Some("/".to_string()), expanded: true },
            ScriptFolder { path: "/utility".to_string(), name: "Utilities".to_string(), parent: Some("/".to_string()), expanded: true },
        ];

        // Add sample scripts
        self.add_sample_script("villager_behavior", ScriptType::NpcBehavior, "/npc", "-- NPC Behavior for villagers\nlocal npc = {}\n\nfunction npc.on_interact(entity_id, player_id)\n    dde.log_info('Hello, traveler!')\nend\n\nreturn npc");
        
        self.add_sample_script("merchant_ai", ScriptType::NpcBehavior, "/npc", 
            "-- Merchant behavior\nlocal merchant = {}\n\nfunction npc.on_interact(entity_id, player_id)\n    dde.log_info('Would you like to trade?')\nend\n\nreturn merchant");
        
        self.add_sample_script("main_quest_01", ScriptType::Quest, "/quest",
            "-- Main Quest: The Beginning\nlocal quest = {\n    id = 'quest_001',\n    name = 'The Beginning',\n    objectives = {}\n}\n\nfunction quest.on_start(player_id)\n    dde.log_info('Quest started!')\nend\n\nreturn quest");
        
        self.add_sample_script("goblin_ai", ScriptType::BattleAi, "/ai",
            "-- Goblin Battle AI\nlocal ai = {}\n\nfunction ai.select_action(battle_state, entity_id)\n    return { type = 'attack', target = battle_state.enemies[1] }\nend\n\nreturn ai");
        
        self.add_sample_script("door_trigger", ScriptType::Event, "/events",
            "-- Door trigger event\nlocal event = {}\n\nfunction event.on_trigger(trigger_id, entity_id)\n    dde.log_info('Door opened!')\n    return true\nend\n\nreturn event");
        
        self.add_sample_script("math_utils", ScriptType::Utility, "/utility",
            "-- Math utilities\nlocal utils = {}\n\nfunction utils.clamp(value, min, max)\n    return math.max(min, math.min(max, value))\nend\n\nreturn utils");
    }

    fn add_sample_script(&mut self, name: &str, script_type: ScriptType, folder: &str, source: &str) {
        let id = self.next_id;
        self.next_id += 1;

        let now = chrono::Utc::now().timestamp();
        
        self.scripts.insert(id, ScriptMetadata {
            id,
            name: name.to_string(),
            description: Some(format!("{} script", script_type.display_name())),
            author: Some("Developer".to_string()),
            source: source.to_string(),
            script_type,
            file_path: Some(PathBuf::from(format!("scripts/{}/{}", folder, name))),
            dependencies: Vec::new(),
            created_at: now,
            modified_at: now,
            compiled: true,
            syntax_valid: true,
            api_valid: true,
            reload_status: ReloadStatus::Loaded,
            folder_path: folder.to_string(),
        });
    }
}

impl ScriptManagerBackend for ExampleScriptBackend {
    fn get_scripts(&self) -> Vec<&ScriptMetadata> {
        self.scripts.values().collect()
    }

    fn get_scripts_in_folder(&self, folder: &str) -> Vec<&ScriptMetadata> {
        self.scripts
            .values()
            .filter(|s| s.folder_path == folder)
            .collect()
    }

    fn get_script(&self, id: i64) -> Option<&ScriptMetadata> {
        self.scripts.get(&id)
    }

    fn get_script_mut(&mut self, id: i64) -> Option<&mut ScriptMetadata> {
        self.scripts.get_mut(&id)
    }

    fn create_script(&mut self, template: &ScriptTemplate, folder: &str) -> Result<ScriptMetadata, String> {
        let id = self.next_id;
        self.next_id += 1;

        let now = chrono::Utc::now().timestamp();

        let script = ScriptMetadata {
            id,
            name: template.name.clone(),
            description: Some(template.description.clone()),
            author: Some("Developer".to_string()),
            source: template.default_code.clone(),
            script_type: template.script_type,
            file_path: Some(PathBuf::from(format!("{}/{}", folder, template.name))),
            dependencies: Vec::new(),
            created_at: now,
            modified_at: now,
            compiled: false,
            syntax_valid: false,
            api_valid: false,
            reload_status: ReloadStatus::Unloaded,
            folder_path: folder.to_string(),
        };

        self.scripts.insert(id, script.clone());
        Ok(script)
    }

    fn delete_script(&mut self, id: i64) -> Result<(), String> {
        self.scripts.remove(&id).ok_or_else(|| "Script not found".to_string())?;
        Ok(())
    }

    fn duplicate_script(&mut self, id: i64, new_name: &str) -> Result<ScriptMetadata, String> {
        let original = self.scripts.get(&id).ok_or_else(|| "Script not found".to_string())?;
        let mut new_script = original.clone();
        
        new_script.id = self.next_id;
        self.next_id += 1;
        new_script.name = new_name.to_string();
        new_script.created_at = chrono::Utc::now().timestamp();
        new_script.modified_at = new_script.created_at;
        new_script.reload_status = ReloadStatus::Unloaded;

        self.scripts.insert(new_script.id, new_script.clone());
        Ok(new_script)
    }

    fn rename_script(&mut self, id: i64, new_name: &str) -> Result<(), String> {
        let script = self.scripts.get_mut(&id).ok_or_else(|| "Script not found".to_string())?;
        script.name = new_name.to_string();
        script.modified_at = chrono::Utc::now().timestamp();
        Ok(())
    }

    fn move_script(&mut self, id: i64, folder: &str) -> Result<(), String> {
        let script = self.scripts.get_mut(&id).ok_or_else(|| "Script not found".to_string())?;
        script.folder_path = folder.to_string();
        script.modified_at = chrono::Utc::now().timestamp();
        Ok(())
    }

    fn validate_script(&mut self, id: i64) -> ValidationResult {
        let script = match self.scripts.get(&id) {
            Some(s) => s,
            None => return ValidationResult {
                valid: false,
                errors: vec![ValidationError {
                    line: 0,
                    message: "Script not found".to_string(),
                    error_type: ErrorType::Load,
                }],
                warnings: Vec::new(),
            },
        };

        // Simple validation - check for common Lua syntax issues
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check for unclosed blocks
        let open_count = script.source.matches("function").count();
        let close_count = script.source.matches("end").count();
        if open_count > close_count {
            errors.push(ValidationError {
                line: 0,
                message: format!("Unclosed function blocks: {} missing 'end'", open_count - close_count),
                error_type: ErrorType::Syntax,
            });
        }

        // Check for 'dde' API usage
        if script.source.contains("dde.") && !script.source.contains("local dde") {
            // This is fine - dde is a global API
        }

        // Warn about long functions
        let lines: Vec<_> = script.source.lines().collect();
        if lines.len() > 100 {
            warnings.push(format!("Script is quite long ({} lines)", lines.len()));
        }

        let valid = errors.is_empty();

        if valid {
            if let Some(s) = self.scripts.get_mut(&id) {
                s.syntax_valid = true;
                s.api_valid = true;
            }
        }

        ValidationResult {
            valid,
            errors,
            warnings,
        }
    }

    fn reload_script(&mut self, id: i64) -> Result<(), String> {
        let script = self.scripts.get_mut(&id).ok_or_else(|| "Script not found".to_string())?;
        script.reload_status = ReloadStatus::Reloading;
        
        // Simulate reload
        script.reload_status = ReloadStatus::Loaded;
        Ok(())
    }

    fn reload_all(&mut self) -> Vec<(i64, Result<(), String>)> {
        let ids: Vec<_> = self.scripts.keys().copied().collect();
        ids.into_iter()
            .map(|id| (id, self.reload_script(id)))
            .collect()
    }

    fn get_folders(&self) -> Vec<&ScriptFolder> {
        self.folders.iter().collect()
    }

    fn create_folder(&mut self, name: &str, parent: &str) -> Result<String, String> {
        let path = format!("{}/{}", parent.trim_end_matches('/'), name);
        
        if self.folders.iter().any(|f| f.path == path) {
            return Err("Folder already exists".to_string());
        }

        self.folders.push(ScriptFolder {
            path: path.clone(),
            name: name.to_string(),
            parent: Some(parent.to_string()),
            expanded: true,
        });

        Ok(path)
    }

    fn delete_folder(&mut self, path: &str, move_scripts_to_parent: bool) -> Result<(), String> {
        if path == "/" {
            return Err("Cannot delete root folder".to_string());
        }

        let folder = self.folders.iter().find(|f| f.path == path).ok_or_else(|| "Folder not found".to_string())?;
        let parent = folder.parent.clone().unwrap_or_else(|| "/".to_string());

        if move_scripts_to_parent {
            for script in self.scripts.values_mut() {
                if script.folder_path == path {
                    script.folder_path = parent.clone();
                }
            }
        }

        self.folders.retain(|f| f.path != path);
        Ok(())
    }

    fn toggle_folder(&mut self, path: &str) {
        if let Some(folder) = self.folders.iter_mut().find(|f| f.path == path) {
            folder.expanded = !folder.expanded;
        }
    }

    fn get_error_log(&self) -> Vec<dde_editor::script_manager::ScriptErrorEntry> {
        self.error_log.clone()
    }

    fn clear_error_log(&mut self) {
        self.error_log.clear();
    }

    fn open_in_external_editor(&self, id: i64) -> Result<(), String> {
        let script = self.scripts.get(&id).ok_or_else(|| "Script not found".to_string())?;
        
        // Open with default system editor or VS Code
        let path = script.file_path.as_ref().ok_or_else(|| "No file path".to_string())?;
        
        #[cfg(target_os = "windows")]
        Command::new("cmd")
            .args(["/C", "start", "", path.to_str().unwrap()])
            .spawn()
            .map_err(|e| e.to_string())?;

        #[cfg(target_os = "macos")]
        Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;

        #[cfg(target_os = "linux")]
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    fn get_script_source(&self, id: i64) -> Option<String> {
        self.scripts.get(&id).map(|s| s.source.clone())
    }
}

/// Main application structure
struct App {
    editor: Editor,
    script_backend: ExampleScriptBackend,
}

impl App {
    fn new() -> Self {
        let mut editor = Editor::new();
        editor.script_manager.show();
        
        Self {
            editor,
            script_backend: ExampleScriptBackend::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update script manager
        self.editor.update_script_manager(1.0 / 60.0, &mut self.script_backend);

        // Draw menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();
                    }
                    if ui.button("Exit").clicked() {
                        ui.close_menu();
                    }
                });

                ui.menu_button("Tools", |ui| {
                    self.editor.draw_tools_menu(ui);
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        ui.close_menu();
                    }
                });
            });
        });

        // Draw script manager
        self.editor.draw_script_manager(ctx, &mut self.script_backend);
    }
}

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Script Manager Browser Example",
        options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    ).unwrap();
}
